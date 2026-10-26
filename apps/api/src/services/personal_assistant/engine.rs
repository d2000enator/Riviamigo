//! Deterministic, read-only assessments. This module cannot call Rivian or send
//! messages. Every recommendation carries its assumptions and data freshness.
use super::model::*;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::collections::HashSet;

pub fn local_time(value: &str, timezone: &str) -> Result<DateTime<Utc>, String> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| "Choose a valid IANA timezone".to_string())?;
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| "Use a date and time such as 2026-10-27T09:30".to_string())?;
    tz.from_local_datetime(&naive).single().map(|time| time.with_timezone(&Utc))
        .ok_or_else(|| "This local time is skipped or occurs twice during a clock change; choose an unambiguous time".into())
}
fn in_range(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && value >= min && value <= max
}
fn valid_date(value: &str) -> bool {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}
pub fn validate(doc: &Document, is_demo: bool) -> Result<(), String> {
    let p = &doc.preferences;
    p.timezone
        .parse::<Tz>()
        .map_err(|_| "Choose a valid timezone")?;
    if !in_range(p.reserve_pct, 5.0, 80.0)
        || !in_range(p.assumed_efficiency_wh_mi, 100.0, 1500.0)
        || !in_range(p.weather_buffer_pct, 0.0, 100.0)
        || !in_range(p.parked_loss_threshold_pct_day, 0.1, 20.0)
        || !in_range(p.cabin_target_c, 10.0, 30.0)
        || !(5..=180).contains(&p.stale_after_minutes)
        || !(0..=120).contains(&p.preparation_minutes)
    {
        return Err(
            "Check reserve, efficiency, buffer, freshness, parked loss and cabin settings".into(),
        );
    }
    for (value, min, max) in [
        (p.battery_capacity_kwh, 20.0, 250.0),
        (p.estimated_charger_kw, 0.1, 400.0),
        (p.home_rate_per_kwh, 0.0, 5.0),
    ] {
        if value.is_some_and(|v| !in_range(v, min, max)) {
            return Err(
                "A capacity, charging power or tariff value is outside its supported range".into(),
            );
        }
    }
    if !is_demo && p.demo_scenario != DemoScenario::Recorded {
        return Err("Demo scenarios are available only for demo vehicles".into());
    }
    if let Some(value) = &p.vacation_return_local {
        local_time(value, &p.timezone)?;
    }
    if doc.plans.len() > 100 || doc.maintenance.len() > 250 {
        return Err("Keep at most 100 plans and 250 service entries in this version".into());
    }
    let mut ids = HashSet::new();
    for plan in &doc.plans {
        if !ids.insert(plan.id)
            || plan.title.trim().is_empty()
            || plan.title.chars().count() > 120
            || !in_range(plan.distance_mi, 0.1, 5000.0)
        {
            return Err("Each trip needs a unique ID, short title and valid distance".into());
        }
        local_time(&plan.departure_local, &p.timezone)?;
    }
    ids.clear();
    for entry in &doc.maintenance {
        if !ids.insert(entry.id)
            || entry.title.trim().is_empty()
            || entry.title.chars().count() > 120
            || !valid_date(&entry.performed_on)
            || entry.notes.chars().count() > 1000
            || entry.receipt_reference.chars().count() > 500
            || entry
                .odometer_mi
                .is_some_and(|v| !in_range(v, 0.0, 2_000_000.0))
            || entry
                .next_due_odometer_mi
                .is_some_and(|v| !in_range(v, 0.0, 2_000_000.0))
            || entry
                .cost_usd
                .is_some_and(|v| !in_range(v, 0.0, 1_000_000.0))
            || entry.next_due_on.as_deref().is_some_and(|v| !valid_date(v))
        {
            return Err("Check the service entry date, mileage, amount and note length".into());
        }
    }
    Ok(())
}
fn fresh(at: Option<DateTime<Utc>>, now: DateTime<Utc>, minutes: i64) -> bool {
    at.is_some_and(|at| at <= now + Duration::minutes(2) && at >= now - Duration::minutes(minutes))
}
fn condition(
    key: impl Into<String>,
    severity: &str,
    title: &str,
    message: impl Into<String>,
) -> AlertCondition {
    AlertCondition {
        key: key.into(),
        severity: severity.into(),
        title: title.into(),
        message: message.into(),
    }
}

fn demo_inputs(
    p: &Preferences,
    original: &Snapshot,
    old: &Observation,
    now: DateTime<Utc>,
) -> (Snapshot, Observation) {
    if !original.is_demo || p.demo_scenario == DemoScenario::Recorded {
        return (original.clone(), old.clone());
    }
    let mut s = Snapshot {
        is_demo: true,
        soc_pct: Some(78.0),
        soc_at: Some(now),
        range_mi: Some(234.0),
        range_at: Some(now),
        battery_capacity_kwh: Some(100.0),
        charge_limit_pct: Some(85.0),
        charge_limit_at: Some(now),
        charging: Some(false),
        charging_at: Some(now),
        wiper_fluid_low: Some(false),
        wiper_fluid_at: Some(now),
        brake_fluid_low: Some(false),
        brake_fluid_at: Some(now),
        odometer_mi: Some(12_000.0),
        odometer_at: Some(now),
        last_event_at: Some(now),
        latest_activity_at: None,
    };
    let mut o = old.clone();
    match p.demo_scenario {
        DemoScenario::LowCharge => {
            s.soc_pct = Some(22.0);
            s.range_mi = Some(66.0);
        }
        DemoScenario::ChargingInterrupted => {
            s.soc_pct = Some(40.0);
            s.range_mi = Some(120.0);
            o.was_charging = true;
            o.interruption_since = Some(now - Duration::minutes(4));
        }
        DemoScenario::FluidWarning => {
            s.wiper_fluid_low = Some(true);
        }
        DemoScenario::VacationDrain => {
            s.soc_pct = Some(80.0);
            o.vacation_started_at = Some(now - Duration::days(2));
            o.vacation_start_soc = Some(95.0);
            o.vacation_baseline_mi = s.odometer_mi;
        }
        _ => {}
    }
    (s, o)
}

pub fn evaluate(
    doc: &Document,
    input: &Snapshot,
    previous: &Observation,
    now: DateTime<Utc>,
) -> Result<Evaluation, String> {
    validate(doc, input.is_demo)?;
    let p = &doc.preferences;
    let (s, mut observation) = demo_inputs(p, input, previous, now);
    let mut conditions = Vec::new();
    let mut resolvable = vec!["telemetry_freshness".to_string()];
    let soc = s.soc_pct.filter(|v| in_range(*v, 0.0, 100.0));
    let current_soc = soc.filter(|_| fresh(s.soc_at, now, p.stale_after_minutes));
    if current_soc.is_none() {
        conditions.push(condition("telemetry_freshness", "info", "Waiting for current vehicle data", "Battery data is unavailable or older than your freshness limit. Charge and readiness warnings are paused; a sleeping car can report less often."));
    }
    let mut plans = Vec::new();
    for plan in &doc.plans {
        let departure = local_time(&plan.departure_local, &p.timezone)?;
        let distance = plan.distance_mi * if plan.round_trip { 2.0 } else { 1.0 };
        let energy =
            distance * p.assumed_efficiency_wh_mi / 1000.0 * (1.0 + p.weather_buffer_pct / 100.0);
        let capacity = s
            .battery_capacity_kwh
            .filter(|v| in_range(*v, 20.0, 250.0))
            .or(p.battery_capacity_kwh.filter(|v| in_range(*v, 20.0, 250.0)));
        let mut a = PlanAssessment {
            id: plan.id,
            title: plan.title.clone(),
            departure_at: departure,
            preparation_at: departure - Duration::minutes(p.preparation_minutes),
            distance_mi: distance,
            status: "waiting_for_data".into(),
            method: "unavailable".into(),
            required_soc_pct: None,
            arrival_soc_pct: None,
            estimated_energy_kwh: None,
            estimated_charge_hours: None,
            explanation: "Current battery data is needed before assessing this trip.".into(),
        };
        if current_soc.is_some() || departure < now {
            resolvable.push(format!("trip:{}", plan.id));
            resolvable.push(format!("trip_time:{}", plan.id));
        }
        resolvable.push(format!("departure:{}", plan.id));
        if departure < now
            || (current_soc.is_some() && fresh(s.charge_limit_at, now, p.stale_after_minutes))
        {
            resolvable.push(format!("trip_limit:{}", plan.id));
        }
        if departure < now {
            a.status = "departed".into();
            a.explanation =
                "This departure is in the past. It will not generate a trip-readiness alert."
                    .into();
            plans.push(a);
            continue;
        }
        if let Some(level) = current_soc {
            let needed_pct = if let Some(capacity) = capacity {
                a.method = if s
                    .battery_capacity_kwh
                    .is_some_and(|v| in_range(v, 20.0, 250.0))
                {
                    "recorded_capacity_and_assumed_efficiency"
                } else {
                    "entered_capacity_and_assumed_efficiency"
                }
                .into();
                a.estimated_energy_kwh = Some(energy);
                Some(energy / capacity * 100.0)
            } else if level > 0.0 && fresh(s.range_at, now, p.stale_after_minutes) {
                s.range_mi
                    .filter(|r| in_range(*r, 0.1, 1000.0))
                    .map(|range| {
                        a.method = "reported_range_estimate".into();
                        distance * (1.0 + p.weather_buffer_pct / 100.0) / range * level
                    })
            } else {
                None
            };
            if let Some(needed_pct) = needed_pct {
                let required = needed_pct + p.reserve_pct;
                a.required_soc_pct = Some(required);
                if level >= required {
                    resolvable.push(format!("trip_limit:{}", plan.id));
                }
                a.arrival_soc_pct = Some((level - needed_pct).clamp(0.0, 100.0));
                a.status = if required > 100.0 {
                    "charging_stop_needed"
                } else if level >= required {
                    "ready"
                } else {
                    "charge_needed"
                }
                .into();
                a.explanation = format!("Estimate includes a {:.0}% driving/weather buffer and {:.0}% arrival reserve. Distance and efficiency are planning inputs; this is not a route or weather forecast.", p.weather_buffer_pct, p.reserve_pct);
                if let (Some(capacity), Some(kw)) = (capacity, p.estimated_charger_kw) {
                    a.estimated_charge_hours = Some(
                        ((required.min(100.0) - level).max(0.0) / 100.0 * capacity) / (kw * 0.9),
                    );
                }
                if departure <= now + Duration::hours(36) && a.status != "ready" {
                    if fresh(s.charge_limit_at, now, p.stale_after_minutes)
                        && s.charge_limit_pct.is_some_and(|limit| {
                            in_range(limit, 0.0, 100.0) && limit < required.min(100.0)
                        })
                    {
                        conditions.push(condition(format!("trip_limit:{}",plan.id),"warning","Charge limit below departure estimate",format!("{} needs more charge than the latest reported charging limit permits. Review the limit in the Rivian app. No setting has been changed.",plan.title)));
                    }
                    let title = if required > 100.0 {
                        "Plan needs a charging stop"
                    } else {
                        "More charge needed for departure"
                    };
                    conditions.push(condition(format!("trip:{}", plan.id), "warning", title, format!("{}: {:.0} miles with your reserve and buffer. Review the readiness estimate before leaving.", plan.title, distance)));
                    if a.estimated_charge_hours.is_some_and(|hours| {
                        hours > (departure - now).num_minutes().max(0) as f64 / 60.0 + 0.25
                    }) {
                        conditions.push(condition(format!("trip_time:{}", plan.id), "warning", "Charging time may be too short", format!("{}: your entered charging power may not reach the estimated departure charge in time. Charging power is an assumption, not a promise.", plan.title)));
                    }
                }
            }
        }
        if now >= a.preparation_at && now < departure {
            conditions.push(condition(format!("departure:{}", plan.id), "info", if s.is_demo { "Demo: departure preparation is due" } else { "Departure preparation is due" }, format!("{} departs soon. Your cabin preference is {:.0}°C. No vehicle command was sent; use the Rivian app for actual cabin preparation.", plan.title, p.cabin_target_c)));
        }
        plans.push(a);
    }
    plans.sort_by_key(|p| p.departure_at);
    for (low, at, key, title) in [
        (
            s.wiper_fluid_low,
            s.wiper_fluid_at,
            "wiper_fluid",
            "Washer fluid reported low",
        ),
        (
            s.brake_fluid_low,
            s.brake_fluid_at,
            "brake_fluid",
            "Brake fluid warning reported",
        ),
    ] {
        if low.is_some() && fresh(at, now, p.stale_after_minutes) {
            resolvable.push(key.to_string());
        }
        if low == Some(true) && fresh(at, now, p.stale_after_minutes) {
            conditions.push(condition(key, "attention", title, "A current vehicle-reported warning is present. Review the vehicle's own warning and instructions."));
        }
    }
    if fresh(s.charging_at, now, p.stale_after_minutes) && current_soc.is_some() {
        let plan_target = plans
            .iter()
            .filter(|plan| plan.departure_at >= now)
            .filter_map(|plan| plan.required_soc_pct)
            .next();
        let current_limit = s
            .charge_limit_pct
            .filter(|_| fresh(s.charge_limit_at, now, p.stale_after_minutes));
        let target = match (plan_target, current_limit) {
            (Some(plan), Some(limit)) => Some(plan.max(limit)),
            (plan, limit) => plan.or(limit),
        };
        if s.charging == Some(true) || (s.charging.is_some() && target.is_some()) {
            resolvable.push("charge_interrupted".into());
        }
        match s.charging {
            Some(true) => {
                observation.was_charging = true;
                observation.interruption_since = None;
            }
            Some(false)
                if observation.was_charging
                    && target
                        .zip(current_soc)
                        .is_some_and(|(target, level)| level + 1.0 < target.min(100.0)) =>
            {
                let since = *observation.interruption_since.get_or_insert(now);
                if now - since >= Duration::minutes(3) {
                    conditions.push(condition("charge_interrupted", "warning", "Charging stopped below your target", "Charging was previously observed, then stopped below the planning target. This may be an intentional schedule or limit; check the charging page."));
                }
            }
            Some(false) => {
                observation.was_charging = false;
                observation.interruption_since = None;
            }
            None => {}
        }
    }
    let mut vacation = VacationAssessment { status: "off".into(), observed_hours: 0.0, observed_loss_pct_day: None, projected_return_soc_pct: None, explanation: "Start vacation watch to record a current battery baseline. It does not wake or operate the vehicle.".into() };
    if !p.vacation_enabled || current_soc.is_some() {
        resolvable.push("vacation_drain".into());
    }
    if !p.vacation_enabled {
        observation.vacation_started_at = None;
        observation.vacation_start_soc = None;
        observation.vacation_baseline_mi = None;
    } else if let Some(level) = current_soc {
        if observation.vacation_started_at.is_none() {
            observation.vacation_started_at = Some(now);
            observation.vacation_start_soc = Some(level);
            observation.vacation_baseline_mi = s.odometer_mi;
        }
        if let (Some(start), Some(start_soc)) = (
            observation.vacation_started_at,
            observation.vacation_start_soc,
        ) {
            let hours = (now - start).num_seconds().max(0) as f64 / 3600.0;
            vacation.observed_hours = hours;
            let moved = observation
                .vacation_baseline_mi
                .zip(s.odometer_mi)
                .is_some_and(|(a, b)| b > a + 0.5)
                || s.latest_activity_at.is_some_and(|at| at >= start)
                || s.charging == Some(true);
            if moved || level > start_soc + 1.0 {
                vacation.status = "baseline_changed".into();
                vacation.explanation = "Driving, charging, mileage or a battery increase was recorded since the baseline. Restart vacation watch after parking; this interval cannot be treated as parked drain.".into();
            } else if hours < 6.0 {
                vacation.status = "observing".into();
                vacation.explanation =
                    "Collecting at least six hours of observations before estimating parked loss."
                        .into();
            } else {
                let loss = (start_soc - level).max(0.0) / hours * 24.0;
                vacation.observed_loss_pct_day = Some(loss);
                vacation.status = "monitoring".into();
                vacation.explanation = "Linear estimate from the vacation baseline; it is not a battery-health diagnosis. Climate use and unobserved activity can affect this figure.".into();
                if let Some(value) = &p.vacation_return_local {
                    let end = local_time(value, &p.timezone)?;
                    if end > now {
                        vacation.projected_return_soc_pct = Some(
                            (level - loss * (end - now).num_minutes() as f64 / 1440.0)
                                .clamp(0.0, 100.0),
                        );
                    }
                }
                if loss > p.parked_loss_threshold_pct_day {
                    conditions.push(condition("vacation_drain", "warning", "Parked battery loss is above your threshold", format!("Observed loss is about {:.1} percentage points per day; your configured threshold is {:.1}. Review the interval and data freshness.",loss,p.parked_loss_threshold_pct_day)));
                }
            }
        }
    } else {
        vacation.status = "waiting_for_data".into();
        vacation.explanation = "Current battery data is needed; an old reading cannot start or update the parked-loss estimate.".into();
    }
    let today = now
        .with_timezone(&p.timezone.parse::<Tz>().map_err(|_| "Invalid timezone")?)
        .date_naive();
    for entry in &doc.maintenance {
        let due_date = entry
            .next_due_on
            .as_deref()
            .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
            .is_some_and(|v| v <= today);
        let due_miles = fresh(s.odometer_at, now, p.stale_after_minutes)
            && entry
                .next_due_odometer_mi
                .zip(s.odometer_mi)
                .is_some_and(|(due, current)| current >= due);
        if due_date
            || entry.next_due_odometer_mi.is_none()
            || fresh(s.odometer_at, now, p.stale_after_minutes)
        {
            resolvable.push(format!("maintenance:{}", entry.id));
        }
        if due_date || due_miles {
            conditions.push(condition(format!("maintenance:{}",entry.id),"info","Your service reminder is due",format!("{} reached the date or mileage you entered. This is your reminder, not an official Rivian service interval.",entry.title)));
        }
    }
    Ok(Evaluation {
        is_demo: s.is_demo,
        generated_at: now,
        snapshot: s,
        plans,
        vacation,
        conditions,
        resolvable_rule_keys: resolvable,
        observation,
        vehicle_commands_enabled: false,
    })
}
