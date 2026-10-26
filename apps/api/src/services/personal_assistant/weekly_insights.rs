//! Arithmetic explanations, with explicit coverage gates. Descriptive context
//! never becomes a claim that temperature or a particular system caused loss.
use super::{model::WeeklyPeriod, weekly_model::*};
fn percent(current: Option<f64>, previous: Option<f64>) -> Option<f64> {
    current
        .zip(previous)
        .filter(|(c, p)| c.is_finite() && p.is_finite() && *p > 0.0)
        .map(|(c, p)| (c - p) / p * 100.0)
}
fn known_cost(p: &WeeklyPeriod) -> Option<f64> {
    if p.charging_sessions == 0 {
        Some(0.0)
    } else if p.recorded_cost_usd.is_some() || p.estimated_home_cost_usd.is_some() {
        Some(p.recorded_cost_usd.unwrap_or(0.0) + p.estimated_home_cost_usd.unwrap_or(0.0))
    } else {
        None
    }
}
fn card(
    category: &str,
    title: &str,
    unit: &str,
    current: Option<f64>,
    previous: Option<f64>,
) -> WeeklyInsight {
    WeeklyInsight {
        category: category.into(),
        title: title.into(),
        status: "waiting_for_history".into(),
        unit: unit.into(),
        current_value: current,
        previous_value: previous,
        change_pct: None,
        explanation: "More comparable recorded history is needed.".into(),
        context: Vec::new(),
        cost_attribution: None,
    }
}
fn efficiency(
    current: &WeeklyPeriod,
    previous: &WeeklyPeriod,
    c: &WeeklyDrivingSample,
    p: &WeeklyDrivingSample,
) -> WeeklyInsight {
    let mut out = card(
        "efficiency",
        "Energy per distance",
        "wh_per_mile",
        c.energy_wh_mi,
        p.energy_wh_mi,
    );
    out.context.push(format!("Matched sample: {} trips / {:.0} miles now; {} trips / {:.0} miles previously. Distance coverage: {:.0}% / {:.0}%.",c.matched_trips,c.matched_distance_mi,p.matched_trips,p.matched_distance_mi,c.distance_coverage_pct.unwrap_or(0.0),p.distance_coverage_pct.unwrap_or(0.0)));
    let comparable = c.matched_trips >= 3
        && p.matched_trips >= 3
        && c.distance_coverage_pct.is_some_and(|v| v >= 80.0)
        && p.distance_coverage_pct.is_some_and(|v| v >= 80.0)
        && current.trips_with_unknown_distance == 0
        && previous.trips_with_unknown_distance == 0;
    if comparable {
        if let Some(change) = percent(c.energy_wh_mi, p.energy_wh_mi) {
            out.change_pct = Some(change);
            out.status = "comparison".into();
            out.explanation = if change.abs() < 1.0 {
                "Recorded energy use per distance was roughly unchanged (less than 1%).".into()
            } else {
                format!("Recorded energy use per distance was {:.1}% {}. {} energy per distance means {} efficiency in this matched sample.",change.abs(),if change>0.0{"higher"}else{"lower"},if change>0.0{"Higher"}else{"Lower"},if change>0.0{"lower"}else{"higher"})
            };
        } else {
            out.explanation="The previous sample has no positive energy-per-distance baseline, so a percentage change would be misleading.".into();
        }
    } else {
        out.status = if c.energy_wh_mi.is_some() {
            "partial"
        } else {
            "waiting_for_history"
        }
        .into();
        out.explanation="A fair comparison needs at least three matched trips and 80% distance coverage in both windows, with all trip distances known. Current sample figures are shown without an improvement claim.".into();
    }
    if comparable
        && c.temperature_coverage_pct.is_some_and(|v| v >= 80.0)
        && p.temperature_coverage_pct.is_some_and(|v| v >= 80.0)
    {
        if let Some((now, before)) = c.mean_outside_temp_c.zip(p.mean_outside_temp_c) {
            if (now - before).abs() >= 3.0 {
                out.context.push(format!("Recorded outside temperature averaged {:.0}°F ({:.0}°C) versus {:.0}°F ({:.0}°C). This is a possible contributor; its effect has not been isolated.",now*9.0/5.0+32.0,now,before*9.0/5.0+32.0,before));
            }
        }
    }
    if comparable {
        if let Some((now, before)) = c
            .mean_trip_distance_mi
            .zip(p.mean_trip_distance_mi)
            .filter(|(_, p)| *p > 0.0)
        {
            if ((now - before) / before).abs() >= 0.2 {
                out.context.push(format!("Mean trip length in this sample changed from {:.1} to {:.1} miles. Route, speed, wind and cabin-use effects have not been isolated.",before,now));
            }
        }
    }
    out
}
fn cost(
    current: &WeeklyPeriod,
    previous: &WeeklyPeriod,
    c: &WeeklyChargeSample,
    p: &WeeklyChargeSample,
) -> WeeklyInsight {
    let mut out = card(
        "charging_cost",
        "Known charging cost",
        "usd",
        known_cost(current),
        known_cost(previous),
    );
    let money = |v: Option<f64>| {
        v.map(|v| format!("${v:.2}"))
            .unwrap_or_else(|| "unavailable".into())
    };
    out.context.push(format!(
        "Recorded cost: {} / {}; home-tariff estimate: {} / {} (current / previous, USD).",
        money(current.recorded_cost_usd),
        money(previous.recorded_cost_usd),
        money(current.estimated_home_cost_usd),
        money(previous.estimated_home_cost_usd)
    ));
    let complete = current.charging_sessions > 0
        && previous.charging_sessions > 0
        && current.sessions_with_unknown_energy == 0
        && previous.sessions_with_unknown_energy == 0
        && current.sessions_with_unknown_cost == 0
        && previous.sessions_with_unknown_cost == 0
        && c.energy_coverage_pct.is_some_and(|v| v >= 99.99)
        && p.energy_coverage_pct.is_some_and(|v| v >= 99.99);
    if complete {
        if let Some(((e_now, e_prev), (r_now, r_prev))) = c
            .priced_energy_kwh
            .zip(p.priced_energy_kwh)
            .zip(c.average_price_usd_kwh.zip(p.average_price_usd_kwh))
        {
            let volume = (e_now - e_prev) * r_prev;
            let price = e_now * (r_now - r_prev);
            out.cost_attribution = Some(WeeklyCostAttribution {
                charging_volume_effect_usd: volume,
                average_price_effect_usd: price,
                total_change_usd: volume + price,
            });
            out.status = "comparison".into();
            out.change_pct = percent(out.current_value, out.previous_value);
            out.explanation=format!("Charged energy changed from {:.1} to {:.1} kWh, and average priced energy from ${:.3} to ${:.3}/kWh. At the previous average price, the volume change accounts for ${:+.2}; the average-price change accounts for ${:+.2} of the difference (USD).",e_prev,e_now,r_prev,r_now,volume,price);
            out.context.push("This is an arithmetic breakdown, not proof of a tariff change. Charging locations, session mix and entered estimates can alter the average price.".into());
        }
    } else {
        out.status = if out.current_value.is_some() {
            "partial"
        } else {
            "waiting_for_history"
        }
        .into();
        out.explanation=format!("{} current and {} previous session prices are unavailable; {} / {} session energy readings are unavailable. Known amounts are shown, but incomplete or absent charging history cannot support a full-week cost explanation.",current.sessions_with_unknown_cost,previous.sessions_with_unknown_cost,current.sessions_with_unknown_energy,previous.sessions_with_unknown_energy);
    }
    out
}
fn drain(c: &WeeklyParkedSample, p: &WeeklyParkedSample) -> WeeklyInsight {
    let mut out = card(
        "parked_loss",
        "Observed parked battery loss",
        "pct_points_day",
        c.loss_pct_points_day,
        p.loss_pct_points_day,
    );
    out.context.push(format!("Observed parked time: {:.1} hours in {} validated intervals now; {:.1} hours in {} previously. Excluded intervals: {} / {}.",c.observed_hours,c.validated_periods,p.observed_hours,p.validated_periods,c.excluded_periods,p.excluded_periods));
    let comparable = c.observed_hours >= 12.0
        && p.observed_hours >= 12.0
        && c.validated_periods > 0
        && p.validated_periods > 0;
    if comparable {
        if let Some((now, before)) = c.loss_pct_points_day.zip(p.loss_pct_points_day) {
            out.status = "comparison".into();
            out.change_pct = percent(Some(now), Some(before));
            out.explanation=format!("Battery loss in measured parked intervals averaged {:.2} versus {:.2} percentage points per day, a change of {:+.2} points/day. Total observed loss was {:.2} versus {:.2} points; different amounts of observed parked time also affect those totals.",now,before,now-before,c.loss_pct_points.unwrap_or(0.0),p.loss_pct_points.unwrap_or(0.0));
        }
    } else {
        out.status = if c.loss_pct_points_day.is_some() {
            "partial"
        } else {
            "waiting_for_history"
        }
        .into();
        out.explanation="At least twelve observed parked hours in each window are needed for a rate comparison. Missing or cross-window intervals are not treated as zero battery loss.".into();
    }
    out.context.push("This describes validated observed intervals, not all parked time or battery degradation. The available data does not identify which vehicle system used the energy.".into());
    out
}
pub(super) fn cards(
    current: &WeeklyPeriod,
    previous: &WeeklyPeriod,
    evidence: &WeeklyInsights,
) -> Vec<WeeklyInsight> {
    vec![
        efficiency(
            current,
            previous,
            &evidence.current_driving,
            &evidence.previous_driving,
        ),
        cost(
            current,
            previous,
            &evidence.current_charging,
            &evidence.previous_charging,
        ),
        drain(&evidence.current_parked, &evidence.previous_parked),
    ]
}
