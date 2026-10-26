use super::{model::WeeklyPeriod, weekly, weekly_insights, weekly_model::*};
use crate::routes::idle_drain::{PhantomPeriod, ValidationStatus};
use chrono::{Duration, TimeZone, Utc};
fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 25, 12, 0, 0).unwrap()
}
fn evidence() -> WeeklyInsights {
    let driving = WeeklyDrivingSample {
        matched_distance_mi: 100.0,
        matched_energy_kwh: 40.0,
        matched_trips: 5,
        distance_coverage_pct: Some(100.0),
        energy_wh_mi: Some(400.0),
        mean_trip_distance_mi: Some(20.0),
        mean_outside_temp_c: Some(20.0),
        temperature_coverage_pct: Some(100.0),
    };
    let charge = WeeklyChargeSample {
        priced_energy_kwh: Some(10.0),
        matched_cost_usd: Some(2.0),
        average_price_usd_kwh: Some(0.2),
        energy_coverage_pct: Some(100.0),
    };
    WeeklyInsights {
        generated_at: now(),
        current_driving: driving.clone(),
        previous_driving: driving,
        current_charging: charge.clone(),
        previous_charging: charge,
        ..Default::default()
    }
}
fn complete() -> WeeklyPeriod {
    WeeklyPeriod {
        trip_count: 5,
        distance_mi: 100.0,
        driving_energy_kwh: Some(40.0),
        charging_sessions: 1,
        charging_energy_kwh: Some(10.0),
        recorded_cost_usd: Some(2.0),
        ..Default::default()
    }
}
fn period(start: chrono::DateTime<Utc>, hours: i64, loss: f64) -> PhantomPeriod {
    let end = start + Duration::hours(hours);
    PhantomPeriod {
        period_start: Some(start),
        period_end: Some(end),
        duration_hours: Some(hours as f64),
        sleep_share_pct: Some(1.0),
        state_coverage_pct: Some(1.0),
        soc_start: Some(80.0),
        soc_end: Some(80.0 - loss),
        soc_lost_pct: Some(loss),
        drain_pct_per_hour: Some(loss / hours as f64),
        range_start_mi: None,
        range_end_mi: None,
        range_lost_mi: None,
        range_lost_per_hour_mi: None,
        energy_drained_kwh: Some(loss),
        avg_power_w: None,
        has_reduced_range: None,
        validation_status: ValidationStatus::Validated,
        validation_reason: None,
        sample_count: 12,
        start_sample_at: Some(start),
        end_sample_at: Some(end),
        movement_detected: false,
        overlaps_trip: false,
        overlaps_charge: false,
    }
}
#[test]
fn efficiency_is_a_matched_sample_and_uses_energy_per_distance_direction() {
    let mut e = evidence();
    e.current_driving.energy_wh_mi = Some(480.0);
    let cards = weekly_insights::cards(&complete(), &complete(), &e);
    assert_eq!(cards[0].status, "comparison");
    assert_eq!(cards[0].change_pct, Some(20.0));
    assert!(cards[0].explanation.contains("lower efficiency"));
}
#[test]
fn unknown_distances_or_low_coverage_prevent_efficiency_improvement_claims() {
    let mut c = complete();
    c.trips_with_unknown_distance = 1;
    let mut e = evidence();
    e.current_driving.energy_wh_mi = Some(200.0);
    assert_eq!(
        weekly_insights::cards(&c, &complete(), &e)[0].change_pct,
        None
    );
    c.trips_with_unknown_distance = 0;
    e.current_driving.distance_coverage_pct = Some(50.0);
    assert_eq!(
        weekly_insights::cards(&c, &complete(), &e)[0].status,
        "partial"
    );
}
#[test]
fn small_samples_and_zero_baselines_do_not_produce_spurious_percentage_changes() {
    let mut e = evidence();
    e.current_driving.matched_trips = 2;
    assert_eq!(
        weekly_insights::cards(&complete(), &complete(), &e)[0].change_pct,
        None
    );
    e.current_driving.matched_trips = 5;
    e.previous_driving.energy_wh_mi = Some(0.0);
    assert_eq!(
        weekly_insights::cards(&complete(), &complete(), &e)[0].change_pct,
        None
    );
}
#[test]
fn recorded_temperature_changes_are_context_not_a_claim_of_causation() {
    let mut e = evidence();
    e.current_driving.mean_outside_temp_c = Some(5.0);
    let card = &weekly_insights::cards(&complete(), &complete(), &e)[0];
    assert!(card
        .context
        .iter()
        .any(|s| s.contains("possible contributor") && s.contains("not been isolated")));
}
#[test]
fn cost_volume_and_price_components_reconcile_to_the_total_change() {
    let mut c = complete();
    c.recorded_cost_usd = Some(6.0);
    c.charging_energy_kwh = Some(20.0);
    let mut e = evidence();
    e.current_charging.priced_energy_kwh = Some(20.0);
    e.current_charging.matched_cost_usd = Some(6.0);
    e.current_charging.average_price_usd_kwh = Some(0.3);
    let card = &weekly_insights::cards(&c, &complete(), &e)[1];
    let a = card.cost_attribution.as_ref().unwrap();
    assert!((a.charging_volume_effect_usd - 2.0).abs() < 1e-8);
    assert!((a.average_price_effect_usd - 2.0).abs() < 1e-8);
    assert!((a.total_change_usd - 4.0).abs() < 1e-8);
}
#[test]
fn missing_costs_are_unknown_and_suppress_cost_attribution() {
    let mut c = complete();
    c.recorded_cost_usd = None;
    c.sessions_with_unknown_cost = 1;
    let card = &weekly_insights::cards(&c, &complete(), &evidence())[1];
    assert_eq!(card.current_value, None);
    assert!(card.cost_attribution.is_none());
    assert!(card.context[0].contains("unavailable"));
}
#[test]
fn free_recorded_charging_remains_a_real_zero() {
    let mut c = complete();
    c.recorded_cost_usd = Some(0.0);
    let mut e = evidence();
    e.current_charging.average_price_usd_kwh = Some(0.0);
    e.current_charging.matched_cost_usd = Some(0.0);
    let card = &weekly_insights::cards(&c, &complete(), &e)[1];
    assert_eq!(card.current_value, Some(0.0));
    assert!(card.cost_attribution.is_some());
}
#[test]
fn parked_loss_is_weighted_by_observed_hours_instead_of_averaging_session_rates() {
    let start = now() - Duration::days(6);
    let periods = [
        period(start, 12, 1.0),
        period(start + Duration::hours(24), 36, 2.0),
    ];
    let p = weekly::parked(&periods, now() - Duration::days(7), now());
    assert_eq!(p.observed_hours, 48.0);
    assert_eq!(p.loss_pct_points, Some(3.0));
    assert_eq!(p.loss_pct_points_day, Some(1.5));
}
#[test]
fn a_cross_week_parked_interval_is_never_counted_twice() {
    let boundary = now() - Duration::days(7);
    let periods = [period(boundary - Duration::hours(12), 24, 2.0)];
    let current = weekly::parked(&periods, boundary, now());
    let previous = weekly::parked(&periods, boundary - Duration::days(7), boundary);
    assert_eq!(current.loss_pct_points, None);
    assert_eq!(previous.loss_pct_points, None);
    assert_eq!(current.excluded_periods, 1);
    assert_eq!(previous.excluded_periods, 1);
}
#[test]
fn movement_charging_or_distant_anchors_prevent_parked_loss_claims() {
    let start = now() - Duration::days(2);
    let mut moved = period(start, 12, 1.0);
    moved.movement_detected = true;
    let mut charging = period(start + Duration::hours(12), 12, 1.0);
    charging.overlaps_charge = true;
    let mut old = period(start + Duration::hours(24), 12, 1.0);
    old.start_sample_at = Some(start - Duration::hours(3));
    let p = weekly::parked(&[moved, charging, old], now() - Duration::days(7), now());
    assert_eq!(p.loss_pct_points, None);
    assert_eq!(p.excluded_periods, 3);
}
#[test]
fn zero_observed_loss_and_missing_observations_remain_distinct() {
    let zero = weekly::parked(
        &[period(now() - Duration::days(2), 12, 0.0)],
        now() - Duration::days(7),
        now(),
    );
    assert_eq!(zero.loss_pct_points_day, Some(0.0));
    assert_eq!(
        weekly::parked(&[], now() - Duration::days(7), now()).loss_pct_points_day,
        None
    );
}
#[test]
fn parked_comparison_does_not_call_battery_loss_degradation() {
    let mut e = evidence();
    e.current_parked = WeeklyParkedSample {
        validated_periods: 2,
        observed_hours: 24.0,
        loss_pct_points: Some(2.0),
        loss_pct_points_day: Some(2.0),
        ..Default::default()
    };
    e.previous_parked = WeeklyParkedSample {
        validated_periods: 2,
        observed_hours: 24.0,
        loss_pct_points: Some(1.0),
        loss_pct_points_day: Some(1.0),
        ..Default::default()
    };
    let card = &weekly_insights::cards(&complete(), &complete(), &e)[2];
    assert_eq!(card.status, "comparison");
    assert!(card
        .context
        .iter()
        .any(|s| s.contains("not all parked time or battery degradation")));
}
