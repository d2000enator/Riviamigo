use super::{engine, model::*};
use chrono::{Duration, TimeZone, Utc};
use uuid::Uuid;
fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 25, 23, 0, 0).unwrap()
}
fn snapshot() -> Snapshot {
    Snapshot {
        soc_pct: Some(78.0),
        soc_at: Some(now()),
        range_mi: Some(234.0),
        range_at: Some(now()),
        battery_capacity_kwh: Some(100.0),
        charge_limit_pct: Some(85.0),
        charge_limit_at: Some(now()),
        charging: Some(false),
        charging_at: Some(now()),
        odometer_mi: Some(12000.0),
        odometer_at: Some(now()),
        last_event_at: Some(now()),
        ..Default::default()
    }
}
fn doc() -> Document {
    Document {
        plans: vec![TripPlan {
            id: Uuid::new_v4(),
            title: "Airport example".into(),
            departure_local: "2026-10-26T09:00".into(),
            distance_mi: 60.0,
            round_trip: false,
        }],
        ..Default::default()
    }
}
fn keys(e: &Evaluation) -> Vec<&str> {
    e.conditions.iter().map(|c| c.key.as_str()).collect()
}
#[test]
fn ready_is_an_estimate_with_reserve_and_buffer() {
    let e = engine::evaluate(&doc(), &snapshot(), &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "ready");
    assert!((e.plans[0].required_soc_pct.unwrap() - 47.6).abs() < 0.001);
    assert!(!e.vehicle_commands_enabled);
}
#[test]
fn round_trip_doubles_the_planning_distance() {
    let mut d = doc();
    d.plans[0].round_trip = true;
    let e = engine::evaluate(&d, &snapshot(), &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].distance_mi, 120.0);
    assert!((e.plans[0].required_soc_pct.unwrap() - 75.2).abs() < 0.001);
}
#[test]
fn low_charge_adds_a_readiness_warning() {
    let mut s = snapshot();
    s.soc_pct = Some(20.0);
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "charge_needed");
    assert!(e.conditions.iter().any(|a| a.key.starts_with("trip:")));
}
#[test]
fn old_battery_data_cannot_produce_ready_or_low_charge_claims() {
    let mut s = snapshot();
    s.soc_at = Some(now() - Duration::hours(2));
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "waiting_for_data");
    assert!(!e.conditions.iter().any(|a| a.key.starts_with("trip:")));
    assert!(keys(&e).contains(&"telemetry_freshness"));
}
#[test]
fn a_long_trip_requires_a_stop_not_an_impossible_charge_setting() {
    let mut d = doc();
    d.plans[0].distance_mi = 400.0;
    let e = engine::evaluate(&d, &snapshot(), &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "charging_stop_needed");
    assert!(e.plans[0].required_soc_pct.unwrap() > 100.0);
}
#[test]
fn missing_capacity_can_use_a_fresh_reported_range() {
    let mut s = snapshot();
    s.battery_capacity_kwh = None;
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].method, "reported_range_estimate");
    assert!(e.plans[0].estimated_energy_kwh.is_none());
}
#[test]
fn zero_battery_does_not_turn_range_into_zero_trip_energy() {
    let mut s = snapshot();
    s.battery_capacity_kwh = None;
    s.soc_pct = Some(0.0);
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "waiting_for_data");
}
#[test]
fn only_current_vehicle_fluid_warnings_trigger() {
    let mut s = snapshot();
    s.wiper_fluid_low = Some(true);
    s.wiper_fluid_at = Some(now());
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert!(keys(&e).contains(&"wiper_fluid"));
    s.wiper_fluid_at = Some(now() - Duration::hours(3));
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert!(!keys(&e).contains(&"wiper_fluid"));
    assert!(!e.resolvable_rule_keys.iter().any(|k| k == "wiper_fluid"));
}
#[test]
fn a_fresh_false_fluid_reading_can_resolve_an_old_warning() {
    let mut s = snapshot();
    s.wiper_fluid_low = Some(false);
    s.wiper_fluid_at = Some(now());
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert!(e.resolvable_rule_keys.iter().any(|k| k == "wiper_fluid"));
    assert!(!keys(&e).contains(&"wiper_fluid"));
}
#[test]
fn charging_interruption_requires_a_sustained_previous_observation() {
    let o = Observation {
        was_charging: true,
        interruption_since: Some(now() - Duration::minutes(2)),
        ..Default::default()
    };
    let e = engine::evaluate(&doc(), &snapshot(), &o, now()).unwrap();
    assert!(!keys(&e).contains(&"charge_interrupted"));
    let o = Observation {
        interruption_since: Some(now() - Duration::minutes(4)),
        ..o
    };
    let e = engine::evaluate(&doc(), &snapshot(), &o, now()).unwrap();
    assert!(keys(&e).contains(&"charge_interrupted"));
}
#[test]
fn charging_at_the_target_does_not_raise_an_interruption() {
    let mut s = snapshot();
    s.soc_pct = Some(90.0);
    let o = Observation {
        was_charging: true,
        interruption_since: Some(now() - Duration::minutes(4)),
        ..Default::default()
    };
    let e = engine::evaluate(&doc(), &s, &o, now()).unwrap();
    assert!(!keys(&e).contains(&"charge_interrupted"));
}
#[test]
fn vacation_loss_waits_for_sufficient_observations() {
    let mut d = doc();
    d.preferences.vacation_enabled = true;
    let e = engine::evaluate(&d, &snapshot(), &Observation::default(), now()).unwrap();
    assert_eq!(e.vacation.status, "observing");
    assert!(e.vacation.observed_loss_pct_day.is_none());
}
#[test]
fn vacation_projection_is_explicitly_linear_and_configured() {
    let mut d = doc();
    d.preferences.vacation_enabled = true;
    d.preferences.vacation_return_local = Some("2026-10-28T18:00".into());
    let mut s = snapshot();
    s.soc_pct = Some(80.0);
    let o = Observation {
        vacation_started_at: Some(now() - Duration::days(2)),
        vacation_start_soc: Some(95.0),
        vacation_baseline_mi: Some(12000.0),
        ..Default::default()
    };
    let e = engine::evaluate(&d, &s, &o, now()).unwrap();
    assert_eq!(e.vacation.observed_loss_pct_day, Some(7.5));
    assert!(keys(&e).contains(&"vacation_drain"));
    assert!(e.vacation.projected_return_soc_pct.unwrap() < 80.0);
}
#[test]
fn driving_or_charging_invalidates_a_parked_loss_baseline() {
    let mut d = doc();
    d.preferences.vacation_enabled = true;
    let mut s = snapshot();
    s.latest_activity_at = Some(now() - Duration::hours(2));
    let o = Observation {
        vacation_started_at: Some(now() - Duration::days(2)),
        vacation_start_soc: Some(95.0),
        vacation_baseline_mi: Some(12000.0),
        ..Default::default()
    };
    let e = engine::evaluate(&d, &s, &o, now()).unwrap();
    assert_eq!(e.vacation.status, "baseline_changed");
    assert!(e.vacation.observed_loss_pct_day.is_none());
}
#[test]
fn demo_scenarios_are_rejected_for_a_real_vehicle() {
    let mut d = doc();
    d.preferences.demo_scenario = DemoScenario::FluidWarning;
    assert!(engine::evaluate(&d, &snapshot(), &Observation::default(), now()).is_err());
}
#[test]
fn demo_scenarios_are_labeled_and_never_enable_commands() {
    let mut d = doc();
    d.preferences.demo_scenario = DemoScenario::FluidWarning;
    let mut s = snapshot();
    s.is_demo = true;
    let e = engine::evaluate(&d, &s, &Observation::default(), now()).unwrap();
    assert!(e.is_demo);
    assert!(keys(&e).contains(&"wiper_fluid"));
    assert!(!e.vehicle_commands_enabled);
}
#[test]
fn daylight_saving_gaps_and_ambiguities_are_rejected() {
    assert!(engine::local_time("2026-03-08T02:30", "America/Chicago").is_err());
    assert!(engine::local_time("2026-11-01T01:30", "America/Chicago").is_err());
    assert!(engine::local_time("2026-11-01T02:30", "America/Chicago").is_ok());
}
#[test]
fn invalid_numeric_inputs_are_rejected() {
    let mut d = doc();
    d.preferences.assumed_efficiency_wh_mi = f64::NAN;
    assert!(engine::validate(&d, false).is_err());
    d.preferences.assumed_efficiency_wh_mi = 400.0;
    d.preferences.estimated_charger_kw = Some(0.0);
    assert!(engine::validate(&d, false).is_err());
}
#[test]
fn arbitrary_command_fields_are_not_accepted_as_configuration() {
    let mut v = serde_json::to_value(doc()).unwrap();
    v["preferences"]["vehicle_commands_enabled"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Document>(v).is_err());
}
#[test]
fn document_serialization_preserves_user_plans() {
    let d = doc();
    let rebuilt: Document = serde_json::from_value(serde_json::to_value(&d).unwrap()).unwrap();
    assert_eq!(rebuilt.plans[0].id, d.plans[0].id);
    assert_eq!(rebuilt.plans[0].departure_local, d.plans[0].departure_local);
}

#[test]
fn invalid_recorded_capacity_can_use_the_entered_fallback() {
    let mut d = doc();
    d.preferences.battery_capacity_kwh = Some(90.0);
    let mut s = snapshot();
    s.battery_capacity_kwh = Some(0.0);
    let e = engine::evaluate(&d, &s, &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].method, "entered_capacity_and_assumed_efficiency");
    assert!(e.plans[0].required_soc_pct.is_some());
}
#[test]
fn ready_label_does_not_spend_part_of_the_entered_arrival_reserve() {
    let mut d = doc();
    d.preferences.weather_buffer_pct = 0.0;
    d.plans[0].distance_mi = 146.0;
    let e = engine::evaluate(&d, &snapshot(), &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "charge_needed");
}
#[test]
fn future_battery_timestamps_are_not_current_evidence() {
    let mut s = snapshot();
    s.soc_at = Some(now() + Duration::hours(1));
    let e = engine::evaluate(&doc(), &s, &Observation::default(), now()).unwrap();
    assert_eq!(e.plans[0].status, "waiting_for_data");
}

#[test]
fn fresh_low_charge_limit_is_an_actionable_departure_warning() {
    let mut d = doc();
    d.plans[0].distance_mi = 120.0;
    let mut s = snapshot();
    s.soc_pct = Some(25.0);
    s.charge_limit_pct = Some(50.0);
    let e = engine::evaluate(&d, &s, &Observation::default(), now()).unwrap();
    assert!(keys(&e).iter().any(|k| k.starts_with("trip_limit:")));
    s.charge_limit_at = Some(now() - Duration::hours(2));
    let e = engine::evaluate(&d, &s, &Observation::default(), now()).unwrap();
    assert!(!keys(&e).iter().any(|k| k.starts_with("trip_limit:")));
}

#[test]
fn reaching_the_departure_charge_resolves_an_old_limit_warning_without_a_new_limit_reading() {
    let mut d = doc();
    d.plans[0].distance_mi = 120.0;
    let mut s = snapshot();
    s.soc_pct = Some(90.0);
    s.charge_limit_pct = Some(50.0);
    s.charge_limit_at = None;
    let e = engine::evaluate(&d, &s, &Observation::default(), now()).unwrap();
    assert!(e
        .resolvable_rule_keys
        .iter()
        .any(|k| k.starts_with("trip_limit:")));
    assert!(!keys(&e).iter().any(|k| k.starts_with("trip_limit:")));
}
