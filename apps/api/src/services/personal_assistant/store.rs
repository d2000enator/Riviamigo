use super::{engine, model::*};
use crate::{errors::AppError, routes::range_normalization::normalize_remaining_range_miles};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn snapshot(pool: &PgPool, vehicle_id: Uuid) -> Result<Snapshot, AppError> {
    let mut raw: Value = sqlx::query_scalar(r#"
      SELECT jsonb_build_object(
        'is_demo',v.rivian_vehicle_id LIKE 'demo-%','model',v.model,
        'soc_pct',s.battery_level,'soc_at',s.battery_level_ts,
        'range_mi',s.distance_to_empty_mi,'range_at',s.distance_to_empty_mi_ts,
        'capacity_wh',COALESCE(s.battery_capacity_wh,v.battery_capacity_wh),
        'battery_capacity_kwh',COALESCE(s.battery_capacity_wh,v.battery_capacity_wh)/1000.0,
        'charge_limit_pct',s.battery_limit,'charge_limit_at',s.battery_limit_ts,
        'charger_state',s.charger_state,'charger_state_at',s.charger_state_ts,
        'charger_status',s.charger_status,'charger_status_at',s.charger_status_ts,
        'wiper_fluid_low',s.wiper_fluid_low,'wiper_fluid_at',f.wiper_at,
        'brake_fluid_low',s.brake_fluid_low,'brake_fluid_at',f.brake_at,
        'odometer_mi',s.odometer_miles,'odometer_at',s.odometer_miles_ts,
        'last_event_at',s.ts,
        'latest_activity_at',GREATEST(
            (SELECT max(COALESCE(t.ended_at,now())) FROM riviamigo.trips t WHERE t.vehicle_id=v.id),
            (SELECT max(COALESCE(c.ended_at,now())) FROM riviamigo.charge_sessions c WHERE c.vehicle_id=v.id)))
      FROM riviamigo.vehicles v
      LEFT JOIN riviamigo.vehicle_latest_status s ON s.vehicle_id=v.id
      LEFT JOIN LATERAL (
        SELECT max(ts) FILTER (WHERE wiper_fluid_low IS NOT NULL) AS wiper_at,
               max(ts) FILTER (WHERE brake_fluid_low IS NOT NULL) AS brake_at
        FROM timeseries.telemetry WHERE vehicle_id=v.id AND ts>=now()-interval '3 hours'
      ) f ON true WHERE v.id=$1
    "#).bind(vehicle_id).fetch_optional(pool).await?.ok_or(AppError::NotFound)?;
    // PostgreSQL serializes non-finite floats as JSON strings. Treat those
    // historical bad readings as unavailable rather than failing the whole page.
    for field in [
        "soc_pct",
        "range_mi",
        "capacity_wh",
        "battery_capacity_kwh",
        "charge_limit_pct",
        "odometer_mi",
    ] {
        if !raw[field].is_number() {
            raw[field] = Value::Null;
        }
    }
    let mut result: Snapshot =
        serde_json::from_value(raw.clone()).map_err(|e| AppError::Internal(e.into()))?;
    result.range_mi = normalize_remaining_range_miles(
        result.range_mi,
        result.soc_pct,
        raw.get("capacity_wh").and_then(Value::as_f64),
        raw.get("model").and_then(Value::as_str),
    );
    let mut charging = Vec::new();
    for (name, time) in [
        ("charger_state", "charger_state_at"),
        ("charger_status", "charger_status_at"),
    ] {
        let text = raw
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let state = match text.as_str() {
            "charging" | "charging_active" | "active" | "ac_charging" | "dc_charging" => Some(true),
            "idle" | "standby" | "disconnected" | "not_connected" | "connected" | "complete"
            | "completed" | "stopped" => Some(false),
            _ => None,
        };
        let at = raw
            .get(time)
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|v| v.with_timezone(&Utc));
        if let (Some(state), Some(at)) = (state, at) {
            charging.push((at, state));
        }
    }
    charging.sort_by_key(|row| row.0);
    if let Some((at, state)) = charging.last() {
        result.charging = Some(*state);
        result.charging_at = Some(*at);
    }
    Ok(result)
}

pub async fn document(
    pool: &PgPool,
    user: Uuid,
    vehicle: Uuid,
) -> Result<(i64, Document, Observation), AppError> {
    let row=sqlx::query_as::<_,(i64,Value,Value)>("SELECT revision,document,observation FROM riviamigo.personal_assistant_documents WHERE user_id=$1 AND vehicle_id=$2")
        .bind(user).bind(vehicle).fetch_optional(pool).await?;
    match row {
        Some((revision, doc, observation)) => Ok((
            revision,
            serde_json::from_value(doc).map_err(|e| AppError::Internal(e.into()))?,
            serde_json::from_value(observation).map_err(|e| AppError::Internal(e.into()))?,
        )),
        None => Ok((0, Document::default(), Observation::default())),
    }
}

pub async fn save_document(
    pool: &PgPool,
    user: Uuid,
    vehicle: Uuid,
    revision: i64,
    doc: Document,
    reset_vacation: bool,
) -> Result<(), AppError> {
    let data = snapshot(pool, vehicle).await?;
    engine::validate(&doc, data.is_demo).map_err(AppError::Validation)?;
    let mut tx = pool.begin().await?;
    // The row lock is shared by edits, monitoring and receipt uploads. Revision
    // checks prevent an old phone tab from silently replacing newer Mac edits.
    let current=sqlx::query_as::<_,(i64,Value,Value)>("SELECT revision,document,observation FROM riviamigo.personal_assistant_documents WHERE user_id=$1 AND vehicle_id=$2 FOR UPDATE")
        .bind(user).bind(vehicle).fetch_optional(&mut *tx).await?;
    let payload = serde_json::to_value(&doc).map_err(|e| AppError::Internal(e.into()))?;
    if let Some((current_revision, old_doc, mut observation)) = current {
        if current_revision != revision {
            return Err(AppError::Conflict(
                "These plans changed on another device. Reload before saving your edits.".into(),
            ));
        }
        let old: Document =
            serde_json::from_value(old_doc).map_err(|e| AppError::Internal(e.into()))?;
        if reset_vacation || old.preferences.vacation_enabled != doc.preferences.vacation_enabled {
            let mut o: Observation =
                serde_json::from_value(observation).map_err(|e| AppError::Internal(e.into()))?;
            o.vacation_started_at = None;
            o.vacation_start_soc = None;
            o.vacation_baseline_mi = None;
            observation = serde_json::to_value(o).map_err(|e| AppError::Internal(e.into()))?;
        }
        sqlx::query("UPDATE riviamigo.personal_assistant_documents SET document=$3,observation=$4,revision=revision+1,updated_at=now() WHERE user_id=$1 AND vehicle_id=$2")
            .bind(user).bind(vehicle).bind(payload).bind(observation).execute(&mut *tx).await?;
    } else {
        if revision != 0 {
            return Err(AppError::Conflict(
                "The saved planning document changed; reload before saving.".into(),
            ));
        }
        let inserted=sqlx::query("INSERT INTO riviamigo.personal_assistant_documents(user_id,vehicle_id,document) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(user).bind(vehicle).bind(payload).execute(&mut *tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(AppError::Conflict(
                "Plans were created on another device; reload before saving.".into(),
            ));
        }
    }
    let entry_ids: Vec<Uuid> = doc.maintenance.iter().map(|entry| entry.id).collect();
    sqlx::query("DELETE FROM riviamigo.personal_assistant_receipts WHERE user_id=$1 AND vehicle_id=$2 AND NOT(entry_id=ANY($3))")
        .bind(user).bind(vehicle).bind(&entry_ids).execute(&mut *tx).await?;
    tx.commit().await?;
    evaluate_and_store(pool, user, vehicle).await?;
    Ok(())
}

pub async fn evaluate_and_store(pool: &PgPool, user: Uuid, vehicle: Uuid) -> Result<(), AppError> {
    let data = snapshot(pool, vehicle).await?;
    let mut tx = pool.begin().await?;
    let row=sqlx::query_as::<_,(Value,Value)>("SELECT document,observation FROM riviamigo.personal_assistant_documents WHERE user_id=$1 AND vehicle_id=$2 FOR UPDATE")
        .bind(user).bind(vehicle).fetch_optional(&mut *tx).await?;
    let Some((doc, obs)) = row else {
        return Ok(());
    };
    let doc: Document = serde_json::from_value(doc).map_err(|e| AppError::Internal(e.into()))?;
    let obs: Observation = serde_json::from_value(obs).map_err(|e| AppError::Internal(e.into()))?;
    let evaluation =
        engine::evaluate(&doc, &data, &obs, Utc::now()).map_err(AppError::Validation)?;
    sqlx::query("UPDATE riviamigo.personal_assistant_documents SET observation=$3 WHERE user_id=$1 AND vehicle_id=$2")
        .bind(user).bind(vehicle).bind(serde_json::to_value(&evaluation.observation).map_err(|e|AppError::Internal(e.into()))?).execute(&mut *tx).await?;
    if doc.preferences.alerts_enabled {
        let active: Vec<String> = evaluation
            .conditions
            .iter()
            .map(|c| c.key.clone())
            .collect();
        for c in &evaluation.conditions {
            sqlx::query("INSERT INTO riviamigo.personal_assistant_alerts(id,user_id,vehicle_id,rule_key,severity,title,message,is_demo) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(user_id,vehicle_id,rule_key) WHERE resolved_at IS NULL DO UPDATE SET severity=EXCLUDED.severity,title=EXCLUDED.title,message=EXCLUDED.message,last_seen_at=now()")
                .bind(Uuid::new_v4()).bind(user).bind(vehicle).bind(&c.key).bind(&c.severity).bind(&c.title).bind(&c.message).bind(evaluation.is_demo).execute(&mut *tx).await?;
        }
        // Missing/stale sensor data does not mean a warning has cleared.
        sqlx::query("UPDATE riviamigo.personal_assistant_alerts SET resolved_at=now() WHERE user_id=$1 AND vehicle_id=$2 AND resolved_at IS NULL AND rule_key=ANY($3) AND NOT(rule_key=ANY($4))")
            .bind(user).bind(vehicle).bind(&evaluation.resolvable_rule_keys).bind(&active).execute(&mut *tx).await?;
        let plans: Vec<String> = doc
            .plans
            .iter()
            .flat_map(|p| {
                [
                    format!("trip:{}", p.id),
                    format!("trip_time:{}", p.id),
                    format!("trip_limit:{}", p.id),
                    format!("departure:{}", p.id),
                ]
            })
            .collect();
        let maintenance: Vec<String> = doc
            .maintenance
            .iter()
            .map(|e| format!("maintenance:{}", e.id))
            .collect();
        sqlx::query("UPDATE riviamigo.personal_assistant_alerts SET resolved_at=now() WHERE user_id=$1 AND vehicle_id=$2 AND resolved_at IS NULL AND ((rule_key ~ '^(trip|trip_time|trip_limit|departure):' AND NOT(rule_key=ANY($3))) OR (rule_key LIKE 'maintenance:%' AND NOT(rule_key=ANY($4))))")
            .bind(user).bind(vehicle).bind(&plans).bind(&maintenance).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn period(
    pool: &PgPool,
    user: Uuid,
    vehicle: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    rate: Option<f64>,
) -> Result<WeeklyPeriod, AppError> {
    let value:Value=sqlx::query_scalar(r#"
      SELECT jsonb_build_object('trip_count',t.n,'distance_mi',COALESCE(t.distance,0),'trips_with_unknown_distance',t.unknown_distance,
        'driving_energy_kwh',t.energy/1000.0,'trips_with_unknown_energy',t.unknown_energy,
        'charging_sessions',c.n,'charging_energy_kwh',c.energy,
        'sessions_with_unknown_energy',c.unknown_energy,'recorded_cost_usd',c.cost,
        'estimated_home_cost_usd',c.estimate,'sessions_with_unknown_cost',c.unknown_cost)
      FROM (SELECT count(*) n,SUM(distance_miles) FILTER(WHERE distance_miles>=0 AND distance_miles<10000) distance,
          count(*) FILTER(WHERE distance_miles IS NULL OR distance_miles<0 OR distance_miles>=10000) unknown_distance,
          SUM(energy_wh) FILTER(WHERE energy_wh>=0 AND energy_wh<10000000) energy,
          count(*) FILTER(WHERE energy_wh IS NULL OR energy_wh<0 OR energy_wh>=10000000) unknown_energy
        FROM riviamigo.trips WHERE vehicle_id=$1 AND started_at>=$2 AND started_at<$3 AND ended_at IS NOT NULL) t
      CROSS JOIN (SELECT count(*) n,SUM(COALESCE(kwh_added,energy_added_wh/1000.0)) FILTER(WHERE COALESCE(kwh_added,energy_added_wh/1000.0)>=0 AND COALESCE(kwh_added,energy_added_wh/1000.0)<10000000) energy,
          count(*) FILTER(WHERE COALESCE(kwh_added,energy_added_wh/1000.0) IS NULL OR NOT(COALESCE(kwh_added,energy_added_wh/1000.0)>=0 AND COALESCE(kwh_added,energy_added_wh/1000.0)<10000000)) unknown_energy,
          SUM(cost_usd) FILTER(WHERE cost_usd>-1000000 AND cost_usd<1000000) cost,
          SUM(CASE WHEN cost_usd IS NULL AND is_home AND $4::float8 IS NOT NULL AND COALESCE(kwh_added,energy_added_wh/1000.0)>=0 AND COALESCE(kwh_added,energy_added_wh/1000.0)<10000000 THEN COALESCE(kwh_added,energy_added_wh/1000.0)*$4 END) estimate,
          count(*) FILTER(WHERE cost_usd IS NULL AND NOT(COALESCE(is_home,false) AND $4::float8 IS NOT NULL AND COALESCE(kwh_added,energy_added_wh/1000.0) IS NOT NULL)) unknown_cost
        FROM (SELECT c.started_at,c.ended_at,c.vehicle_id,
            CASE WHEN c.kwh_added>=0 AND c.kwh_added<10000000 THEN c.kwh_added END kwh_added,
            CASE WHEN c.energy_added_wh>=0 AND c.energy_added_wh<10000000000 THEN c.energy_added_wh END energy_added_wh,
            COALESCE(a.is_home,c.is_home) is_home,CASE WHEN COALESCE(a.cost_usd,c.cost_usd)>-1000000 AND COALESCE(a.cost_usd,c.cost_usd)<1000000 THEN COALESCE(a.cost_usd,c.cost_usd) END cost_usd
          FROM riviamigo.charge_sessions c LEFT JOIN riviamigo.charge_session_user_annotations a
          ON a.charge_session_id=c.id AND a.user_id=$5) scoped
          WHERE vehicle_id=$1 AND started_at>=$2 AND started_at<$3 AND ended_at IS NOT NULL) c
    "#).bind(vehicle).bind(start).bind(end).bind(rate).bind(user).fetch_one(pool).await?;
    serde_json::from_value(value).map_err(|e| AppError::Internal(e.into()))
}

pub async fn state(pool: &PgPool, user: Uuid, vehicle: Uuid) -> Result<AssistantState, AppError> {
    let (revision, doc, obs) = document(pool, user, vehicle).await?;
    let data = snapshot(pool, vehicle).await?;
    let evaluation =
        engine::evaluate(&doc, &data, &obs, Utc::now()).map_err(AppError::Validation)?;
    let end = if data.is_demo {
        data.last_event_at.unwrap_or_else(Utc::now).min(Utc::now()) + Duration::seconds(1)
    } else {
        Utc::now()
    };
    let start = end - Duration::days(7);
    let current = period(
        pool,
        user,
        vehicle,
        start,
        end,
        doc.preferences.home_rate_per_kwh,
    )
    .await?;
    let previous = period(
        pool,
        user,
        vehicle,
        start - Duration::days(7),
        start,
        doc.preferences.home_rate_per_kwh,
    )
    .await?;
    let change = if previous.distance_mi > 0.0
        && previous.trips_with_unknown_distance == 0
        && current.trips_with_unknown_distance == 0
    {
        Some((current.distance_mi - previous.distance_mi) / previous.distance_mi * 100.0)
    } else {
        None
    };
    let narrative=format!("{} recorded trips covering {:.0} miles, and {} completed charging sessions in this seven-day window. {} trip distances, {} trip energy readings and {} charging cost records are unavailable. Home cost estimates use your entered tariff; recorded and estimated amounts stay separate.",current.trip_count,current.distance_mi,current.charging_sessions,current.trips_with_unknown_distance,current.trips_with_unknown_energy,current.sessions_with_unknown_cost);
    let alerts=sqlx::query_as::<_,SavedAlert>("SELECT id,rule_key,severity,title,message,is_demo,first_seen_at,last_seen_at,acknowledged_at,resolved_at FROM riviamigo.personal_assistant_alerts WHERE user_id=$1 AND vehicle_id=$2 ORDER BY first_seen_at DESC LIMIT 200")
        .bind(user).bind(vehicle).fetch_all(pool).await?;
    let receipts=sqlx::query_as::<_,ReceiptMetadata>("SELECT id,entry_id,filename,mime_type,octet_length(payload)::bigint byte_count,encode(sha256,'hex') sha256,created_at FROM riviamigo.personal_assistant_receipts WHERE user_id=$1 AND vehicle_id=$2 ORDER BY created_at DESC")
        .bind(user).bind(vehicle).fetch_all(pool).await?;
    Ok(AssistantState {
        revision,
        document: doc,
        evaluation,
        weekly: WeeklySummary {
            starts_at: start,
            ends_at: end,
            is_demo: data.is_demo,
            current,
            previous,
            distance_change_pct: change,
            narrative,
            insights: None,
        },
        alerts,
        receipts,
        notification_delivery: "dashboard_only",
    })
}
