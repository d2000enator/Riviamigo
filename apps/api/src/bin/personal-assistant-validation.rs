//! Explicit isolated-lab acceptance runner. Refuses production origins/databases.
//! It is included in the test image only and never invoked by normal startup.
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Duration, Utc};
use reqwest::{Client, Method};
use riviamigo_api::services::personal_assistant::model::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

struct Probe {
    http: Client,
    base: String,
}
impl Probe {
    async fn call(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
        expected: u16,
    ) -> Result<Value> {
        let mut request = self.http.request(method, format!("{}{}", self.base, path));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await?;
        ensure!(
            response.status().as_u16() == expected,
            "{} returned HTTP{}; expected{}",
            path,
            response.status(),
            expected
        );
        if path.contains("/assistant") {
            ensure!(
                response
                    .headers()
                    .get("cache-control")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.contains("no-store")),
                "Private assistant response on {path} must not be cached"
            );
            let vary = response
                .headers()
                .get_all("vary")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .collect::<Vec<_>>()
                .join(",")
                .to_ascii_lowercase();
            ensure!(
                vary.contains("authorization") && vary.contains("cookie"),
                "Private assistant response on {path} must vary by authentication"
            );
        }
        if expected == 200 || expected == 201 {
            Ok(response.json().await?)
        } else {
            Ok(Value::Null)
        }
    }
    async fn login(&self, email: &str, password: &str) -> Result<String> {
        let result = self
            .call(
                Method::POST,
                "/v1/auth/login",
                None,
                Some(json!({"email":email,"password":password})),
                200,
            )
            .await?;
        Ok(result["access_token"]
            .as_str()
            .context("Lab login did not return a token")?
            .into())
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    ensure!(
        std::env::var("R2_ASSISTANT_VALIDATION").as_deref() == Ok("isolated-lab-v032"),
        "Explicit isolated-lab guard required"
    );
    let db = std::env::var("DATABASE_URL")?;
    ensure!(
        db.contains("@timescaledb:5432/riviamigo"),
        "Only the internal lab database is allowed"
    );
    ensure!(
        std::env::var("ALLOWED_ORIGINS").as_deref() == Ok("https://validation.invalid"),
        "Refusing a production or browser origin"
    );
    let pool = PgPool::connect(&db).await?;
    let credentials: i64 = sqlx::query_scalar("SELECT count(*) FROM riviamigo.vehicle_credentials")
        .fetch_one(&pool)
        .await?;
    let real: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM riviamigo.vehicles WHERE rivian_vehicle_id NOT LIKE 'demo-%'",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        credentials == 0 && real == 0,
        "Lab must contain no real vehicle or provider credentials"
    );
    let users: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM riviamigo.users WHERE email<>'validation.owner@example.invalid'",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        users == 0,
        "Unexpected lab user; preserve existing data and stop"
    );
    let p = Probe {
        http: Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(30))
            .build()?,
        base: "http://candidate:8080".into(),
    };
    let password = std::fs::read_to_string("/validation-config/owner-password")?;
    let token = p
        .login("validation.owner@example.invalid", password.trim())
        .await?;
    let (owner,hash)=sqlx::query_as::<_,(Uuid,String)>("SELECT id,password_hash FROM riviamigo.users WHERE email='validation.owner@example.invalid'").fetch_one(&pool).await?;
    let other = Uuid::new_v4();
    let vehicle = Uuid::new_v4();
    let plan = Uuid::new_v4();
    let entry = Uuid::new_v4();
    let other_email = format!("assistant-{}@example.invalid", other);
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO riviamigo.users(id,email,password_hash,role) VALUES($1,$2,$3,'user')")
        .bind(other)
        .bind(&other_email)
        .bind(hash)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO riviamigo.vehicles(id,user_id,rivian_vehicle_id,model,name) VALUES($1,$2,$3,'R2','Assistant acceptance fixture')").bind(vehicle).bind(owner).bind(format!("demo-assistant-validation-{vehicle}")).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO riviamigo.vehicle_memberships(vehicle_id,user_id,role) VALUES($1,$2,'owner')",
    )
    .bind(vehicle)
    .bind(owner)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    println!("CREATED_SYNTHETIC_FIXTURE vehicle={vehicle} other_user={other} owner={owner}");
    let path = format!("/v1/vehicles/{vehicle}/assistant");
    p.call(Method::GET, &path, None, None, 401).await?;
    let second = p.login(&other_email, password.trim()).await?;
    p.call(Method::GET, &path, Some(&second), None, 403).await?;
    let key=p.call(Method::POST,"/v1/api-keys",Some(&token),Some(json!({"vehicle_id":vehicle,"name":"Isolated assistant denial test","expires_in_days":1})),201).await?;
    p.call(
        Method::GET,
        &path,
        Some(key["key"].as_str().context("Lab API key")?),
        None,
        403,
    )
    .await?;
    println!("PASS unsigned and nonmember access denied");
    p.call(Method::GET, &format!("{path}/weekly"), None, None, 401)
        .await?;
    p.call(
        Method::GET,
        &format!("{path}/weekly"),
        Some(&second),
        None,
        403,
    )
    .await?;
    let initial = p.call(Method::GET, &path, Some(&token), None, 200).await?;
    ensure!(
        initial["revision"] == 0 && initial["notification_delivery"] == "dashboard_only",
        "Initial private document contract"
    );
    let mut doc: Document = serde_json::from_value(initial["document"].clone())?;
    doc.preferences.alerts_enabled = true;
    doc.preferences.demo_scenario = DemoScenario::LowCharge;
    let departure = (Utc::now() + Duration::hours(10))
        .with_timezone(&chrono_tz::America::Chicago)
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    doc.plans.push(TripPlan {
        id: plan,
        title: "Airport · isolated fixture".into(),
        departure_local: departure,
        distance_mi: 60.0,
        round_trip: true,
    });
    doc.maintenance.push(MaintenanceEntry {
        id: entry,
        title: "Test service receipt".into(),
        performed_on: "2026-10-25".into(),
        odometer_mi: Some(100.0),
        cost_usd: Some(25.0),
        receipt_reference: "Synthetic fixture".into(),
        notes: "Not actual service".into(),
        next_due_on: None,
        next_due_odometer_mi: None,
    });
    let preview = p
        .call(
            Method::POST,
            &format!("{path}/preview"),
            Some(&token),
            Some(json!({"document":doc})),
            200,
        )
        .await?;
    ensure!(
        preview["plans"][0]["status"] == "charge_needed"
            && preview["vehicle_commands_enabled"] == false,
        "Preview must assess sample charge without commands"
    );
    ensure!(
        p.call(Method::GET, &path, Some(&token), None, 200).await?["revision"] == 0,
        "Preview wrote saved state"
    );
    let saved = p
        .call(
            Method::PUT,
            &path,
            Some(&token),
            Some(json!({"revision":0,"document":doc})),
            200,
        )
        .await?;
    ensure!(
        saved["revision"] == 1 && saved["alerts"].as_array().context("Alerts")?.len() == 1,
        "Saved alert occurrence"
    );
    p.call(
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({"revision":0,"document":doc})),
        409,
    )
    .await?;
    let ack = saved["alerts"][0]["id"].as_str().context("Alert ID")?;
    p.call(
        Method::POST,
        &format!("{path}/alerts/{ack}/acknowledge"),
        Some(&token),
        None,
        200,
    )
    .await?;
    // No browser is involved. Wait for the actual server worker's next tick,
    // rather than invoking its library directly and claiming that proves scheduling.
    let last_seen = saved["alerts"][0]["last_seen_at"]
        .as_str()
        .context("Last seen")?
        .to_string();
    println!("WAIT actual background worker tick (65 seconds)");
    tokio::time::sleep(std::time::Duration::from_secs(65)).await;
    let repeated = p.call(Method::GET, &path, Some(&token), None, 200).await?;
    ensure!(
        repeated["alerts"].as_array().unwrap().len() == 1
            && !repeated["alerts"][0]["acknowledged_at"].is_null(),
        "Deduplication must retain acknowledgment"
    );
    ensure!(
        repeated["alerts"][0]["last_seen_at"]
            .as_str()
            .context("Updated last seen")?
            > last_seen.as_str(),
        "Actual monitor did not refresh the saved occurrence"
    );
    println!("PASS preview isolation, revision conflict, saved deduplication and acknowledgment");
    sqlx::query(
        "INSERT INTO riviamigo.vehicle_memberships(vehicle_id,user_id,role) VALUES($1,$2,'viewer')",
    )
    .bind(vehicle)
    .bind(other)
    .execute(&pool)
    .await?;
    let private = p.call(Method::GET, &path, Some(&second), None, 200).await?;
    ensure!(
        private["revision"] == 0 && private["alerts"].as_array().unwrap().is_empty(),
        "Member must not read another user's personal records"
    );
    p.call(
        Method::POST,
        &format!("{path}/alerts/{ack}/acknowledge"),
        Some(&second),
        None,
        404,
    )
    .await?;
    // >2MiB tests the JSON extractor limit as well as decoded receipt bounds.
    let mut bytes = b"%PDF-1.4\n% isolated fixture, not a real receipt\n".to_vec();
    bytes.resize(2_200_000, b' ');
    bytes.extend_from_slice(b"\n%%EOF");
    let encoded = STANDARD.encode(&bytes);
    let receipt=p.call(Method::POST,&format!("{path}/maintenance/{entry}/receipts"),Some(&token),Some(json!({"filename":"fixture.pdf","mime_type":"application/pdf","data_base64":encoded})),200).await?;
    let id = receipt["id"].as_str().context("Receipt ID")?;
    let downloaded = p
        .call(
            Method::GET,
            &format!("{path}/receipts/{id}"),
            Some(&token),
            None,
            200,
        )
        .await?;
    ensure!(
        STANDARD.decode(downloaded["data_base64"].as_str().unwrap())? == bytes,
        "Receipt bytes changed"
    );
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ensure!(downloaded["sha256"] == digest, "Receipt digest mismatch");
    p.call(
        Method::GET,
        &format!("{path}/receipts/{id}"),
        Some(&second),
        None,
        404,
    )
    .await?;
    p.call(Method::POST,&format!("{path}/maintenance/{entry}/receipts"),Some(&token),Some(json!({"filename":"fake.pdf","mime_type":"application/pdf","data_base64":STANDARD.encode(b"<script>bad</script>")})),422).await?;
    println!(
        "PASS large receipt round trip, exact hash, content validation and per-user isolation"
    );
    doc.preferences.demo_scenario = DemoScenario::FluidWarning;
    p.call(
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({"revision":1,"document":doc})),
        200,
    )
    .await?;
    let warnings = p.call(Method::GET, &path, Some(&token), None, 200).await?;
    ensure!(
        warnings["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["rule_key"] == "wiper_fluid" && a["resolved_at"].is_null()),
        "Fluid demo warning missing"
    );
    // Recorded fixture has no fresh fluid observation: it must not clear the warning.
    doc.preferences.demo_scenario = DemoScenario::Recorded;
    p.call(
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({"revision":2,"document":doc})),
        200,
    )
    .await?;
    let unknown = p.call(Method::GET, &path, Some(&token), None, 200).await?;
    ensure!(
        unknown["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["rule_key"] == "wiper_fluid" && a["resolved_at"].is_null()),
        "Missing fluid input incorrectly cleared alert"
    );
    doc.preferences.demo_scenario = DemoScenario::Ready;
    p.call(
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({"revision":3,"document":doc})),
        200,
    )
    .await?;
    let cleared = p.call(Method::GET, &path, Some(&token), None, 200).await?;
    ensure!(
        cleared["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["rule_key"] == "wiper_fluid" && !a["resolved_at"].is_null()),
        "Fresh false input did not resolve warning"
    );
    println!("PASS stale input retains warning; fresh false input resolves it");
    // Source-backed weekly calculations: completed periods, missing data, and the
    // authenticated user's own annotations must agree with known synthetic rows.
    let charge = Uuid::new_v4();
    sqlx::query("INSERT INTO riviamigo.trips(vehicle_id,started_at,ended_at,distance_miles,energy_wh) VALUES($1,now()-interval '1 day',now()-interval '23 hours',20,8000),($1,now()-interval '2 days',now()-interval '47 hours',NULL,NULL),($1,now()-interval '8 days',now()-interval '191 hours',40,16000)").bind(vehicle).execute(&pool).await?;
    sqlx::query("INSERT INTO riviamigo.charge_sessions(id,vehicle_id,started_at,ended_at,kwh_added,is_home) VALUES($1,$2,now()-interval '2 days',now()-interval '47 hours',10,true)").bind(charge).bind(vehicle).execute(&pool).await?;
    sqlx::query("INSERT INTO riviamigo.charge_session_user_annotations(charge_session_id,user_id,cost_usd) VALUES($1,$2,3.25)").bind(charge).bind(owner).execute(&pool).await?;
    let weekly = p.call(Method::GET, &path, Some(&token), None, 200).await?;
    ensure!(
        weekly["weekly"]["current"]["trip_count"] == 2
            && weekly["weekly"]["current"]["distance_mi"].as_f64() == Some(20.0)
            && weekly["weekly"]["current"]["trips_with_unknown_distance"] == 1
            && weekly["weekly"]["current"]["driving_energy_kwh"].as_f64() == Some(8.0)
            && weekly["weekly"]["current"]["recorded_cost_usd"] == 3.25
            && weekly["weekly"]["distance_change_pct"].is_null(),
        "Weekly history or missing-distance contract"
    );
    let detailed = p
        .call(
            Method::GET,
            &format!("{path}/weekly"),
            Some(&token),
            None,
            200,
        )
        .await?;
    ensure!(
        detailed["insights"]["cards"]
            .as_array()
            .context("Weekly cards")?
            .len()
            == 3
            && detailed["insights"]["current_driving"]["energy_wh_mi"].as_f64() == Some(400.0)
            && detailed["insights"]["cards"][0]["status"] == "partial"
            && detailed["insights"]["current_parked"]["loss_pct_points_day"].is_null(),
        "Detailed weekly matched/unknown coverage contract"
    );
    sqlx::query("UPDATE riviamigo.charge_sessions SET cost_usd='NaN'::float8 WHERE id=$1")
        .bind(charge)
        .execute(&pool)
        .await?;
    let secondary = p
        .call(
            Method::GET,
            &format!("{path}/weekly"),
            Some(&second),
            None,
            200,
        )
        .await?;
    ensure!(
        secondary["current"]["recorded_cost_usd"].is_null(),
        "Another user's cost annotation leaked into the weekly summary"
    );
    sqlx::query("UPDATE riviamigo.charge_sessions SET cost_usd=NULL WHERE id=$1")
        .bind(charge)
        .execute(&pool)
        .await?;
    println!("PASS weekly recorded aggregates, missing readings and private cost annotation");
    // Baseline ledger must remain upstream-only, while extension tables are present.
    let ledger: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public._sqlx_migrations WHERE success")
            .fetch_one(&pool)
            .await?;
    ensure!(ledger == 27, "Upstream migration ledger changed");
    let schema: i32 = sqlx::query_scalar(
        "SELECT version FROM riviamigo.personal_assistant_schema WHERE singleton",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(schema == 1, "Extension storage version");
    // Keep this synthetic owner's records for restart and isolated restore checks.
    // Remove only the newly generated secondary user; preserve all earlier fixtures.
    sqlx::query("DELETE FROM riviamigo.users WHERE id=$1 AND email=$2")
        .bind(other)
        .bind(other_email)
        .execute(&pool)
        .await?;
    println!(
        "RECOVERY_FIXTURE vehicle={vehicle} owner={owner} revision=4 receipt={id} sha256={digest}"
    );
    println!("PASS upstream ledger27 preserved; synthetic extension fixture retained for restart/restore");
    Ok(())
}
