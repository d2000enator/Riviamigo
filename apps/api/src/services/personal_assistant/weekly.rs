//! Detailed weekly analysis is lazy: the Overview's minute poll never runs the
//! parked-session query. Only the Weekly screen requests this richer endpoint.
use super::{model::*, store, weekly_insights, weekly_model::*};
use crate::{
    errors::AppError,
    routes::idle_drain::{self, PhantomPeriod, ValidationStatus},
};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

async fn driving(
    pool: &PgPool,
    vehicle: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<WeeklyDrivingSample, AppError> {
    let (distance,energy,trips,total_distance,temp_distance,temp_sum)=sqlx::query_as::<_,(f64,f64,i64,f64,f64,Option<f64>)>(r#"
      WITH trips AS (SELECT distance_miles,energy_wh,outside_temp_c,
         distance_miles>0 AND distance_miles<10000 AND energy_wh>=0 AND energy_wh<10000000 AS matched
        FROM riviamigo.trips WHERE vehicle_id=$1 AND started_at>=$2 AND started_at<$3 AND ended_at IS NOT NULL)
      SELECT COALESCE(sum(distance_miles) FILTER(WHERE matched),0)::float8,
        COALESCE(sum(energy_wh) FILTER(WHERE matched),0)::float8/1000,
        count(*) FILTER(WHERE matched),
        COALESCE(sum(distance_miles) FILTER(WHERE distance_miles>=0 AND distance_miles<10000),0)::float8,
        COALESCE(sum(distance_miles) FILTER(WHERE matched AND outside_temp_c BETWEEN -80 AND 80),0)::float8,
        sum(outside_temp_c*distance_miles) FILTER(WHERE matched AND outside_temp_c BETWEEN -80 AND 80)::float8
      FROM trips
    "#).bind(vehicle).bind(from).bind(to).fetch_one(pool).await?;
    Ok(WeeklyDrivingSample {
        matched_distance_mi: distance,
        matched_energy_kwh: energy,
        matched_trips: trips,
        distance_coverage_pct: (total_distance > 0.0).then_some(distance / total_distance * 100.0),
        energy_wh_mi: (distance > 0.0).then_some(energy * 1000.0 / distance),
        mean_trip_distance_mi: (trips > 0).then_some(distance / trips as f64),
        mean_outside_temp_c: temp_sum
            .zip((temp_distance > 0.0).then_some(temp_distance))
            .map(|(sum, n)| sum / n),
        temperature_coverage_pct: (distance > 0.0).then_some(temp_distance / distance * 100.0),
    })
}
async fn charging(
    pool: &PgPool,
    user: Uuid,
    vehicle: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    rate: Option<f64>,
    total_energy: Option<f64>,
) -> Result<WeeklyChargeSample, AppError> {
    let (energy,cost)=sqlx::query_as::<_,(Option<f64>,Option<f64>)>(r#"
      WITH scoped AS (SELECT COALESCE(CASE WHEN c.kwh_added>=0 AND c.kwh_added<10000000 THEN c.kwh_added END,CASE WHEN c.energy_added_wh>=0 AND c.energy_added_wh<10000000000 THEN c.energy_added_wh/1000 END) AS energy,
          CASE WHEN COALESCE(a.cost_usd,c.cost_usd)>-1000000 AND COALESCE(a.cost_usd,c.cost_usd)<1000000 THEN COALESCE(a.cost_usd,c.cost_usd) END AS recorded,
          COALESCE(a.is_home,c.is_home,false) AS home
        FROM riviamigo.charge_sessions c LEFT JOIN riviamigo.charge_session_user_annotations a
          ON a.charge_session_id=c.id AND a.user_id=$5
        WHERE c.vehicle_id=$1 AND c.started_at>=$2 AND c.started_at<$3 AND c.ended_at IS NOT NULL),
      priced AS (SELECT energy,CASE WHEN recorded>-1000000 AND recorded<1000000 THEN recorded
        WHEN recorded IS NULL AND home AND $4::float8 IS NOT NULL THEN energy*$4 END AS cost FROM scoped
        WHERE energy>=0 AND energy<10000000)
      SELECT sum(energy) FILTER(WHERE cost IS NOT NULL)::float8,
        sum(cost) FILTER(WHERE cost IS NOT NULL)::float8 FROM priced
    "#).bind(vehicle).bind(from).bind(to).bind(rate).bind(user).fetch_one(pool).await?;
    Ok(WeeklyChargeSample {
        priced_energy_kwh: energy,
        matched_cost_usd: cost,
        average_price_usd_kwh: cost
            .zip(energy)
            .filter(|(_, e)| *e > 0.0)
            .map(|(c, e)| c / e),
        energy_coverage_pct: energy
            .zip(total_energy)
            .filter(|(_, t)| *t > 0.0)
            .map(|(e, t)| e / t * 100.0),
    })
}

/// Keep entire validated parked intervals inside one window. We never allocate
/// one interval's loss to both weeks or invent a within-session loss curve.
pub(super) fn parked(
    periods: &[PhantomPeriod],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> WeeklyParkedSample {
    let mut out = WeeklyParkedSample::default();
    let mut loss = 0.0;
    let mut energy = 0.0;
    let mut energy_known = 0;
    for p in periods {
        let (Some(start), Some(end)) = (p.period_start, p.period_end) else {
            continue;
        };
        if end <= from || start >= to {
            continue;
        }
        let bounded = start >= from && end <= to;
        let anchors = p
            .start_sample_at
            .zip(p.end_sample_at)
            .is_some_and(|(a, b)| {
                (a - start).num_seconds().abs() <= 45 * 60
                    && (b - end).num_seconds().abs() <= 45 * 60
            });
        let measured = p
            .soc_lost_pct
            .filter(|v| v.is_finite() && (0.0..=100.0).contains(v));
        let hours = p
            .duration_hours
            .filter(|v| v.is_finite() && *v >= 6.0 && *v <= 168.0);
        if p.validation_status != ValidationStatus::Validated
            || !bounded
            || !anchors
            || measured.is_none()
            || hours.is_none()
            || p.movement_detected
            || p.overlaps_trip
            || p.overlaps_charge
        {
            out.excluded_periods += 1;
            continue;
        }
        out.validated_periods += 1;
        out.observed_hours += hours.unwrap();
        loss += measured.unwrap();
        if let Some(kwh) = p
            .energy_drained_kwh
            .filter(|v| v.is_finite() && *v >= 0.0 && *v < 1000.0)
        {
            energy += kwh;
            energy_known += 1;
        } else {
            out.periods_without_energy += 1;
        }
    }
    if out.validated_periods > 0 {
        out.loss_pct_points = Some(loss);
        out.loss_pct_points_day = Some(loss / out.observed_hours * 24.0);
    }
    if energy_known > 0 {
        out.soc_based_energy_kwh = Some(energy);
    }
    out
}

pub async fn summary(pool: &PgPool, user: Uuid, vehicle: Uuid) -> Result<WeeklySummary, AppError> {
    let (_, doc, _) = store::document(pool, user, vehicle).await?;
    let data = store::snapshot(pool, vehicle).await?;
    let end = if data.is_demo {
        data.last_event_at.unwrap_or_else(Utc::now).min(Utc::now()) + Duration::seconds(1)
    } else {
        Utc::now()
    };
    let start = end - Duration::days(7);
    let previous_start = start - Duration::days(7);
    let current = store::period(
        pool,
        user,
        vehicle,
        start,
        end,
        doc.preferences.home_rate_per_kwh,
    )
    .await?;
    let previous = store::period(
        pool,
        user,
        vehicle,
        previous_start,
        start,
        doc.preferences.home_rate_per_kwh,
    )
    .await?;
    let driving_current = driving(pool, vehicle, start, end).await?;
    let driving_previous = driving(pool, vehicle, previous_start, start).await?;
    let charging_current = charging(
        pool,
        user,
        vehicle,
        start,
        end,
        doc.preferences.home_rate_per_kwh,
        current.charging_energy_kwh,
    )
    .await?;
    let charging_previous = charging(
        pool,
        user,
        vehicle,
        previous_start,
        start,
        doc.preferences.home_rate_per_kwh,
        previous.charging_energy_kwh,
    )
    .await?;
    // Reuse the dashboard's canonical validated-session detector. No Parallax,
    // account connection, wake-up or new polling subscription is required.
    let periods = idle_drain::fetch_idle_drain_periods(
        pool,
        vehicle,
        previous_start - Duration::days(7),
        end,
        6.0,
        500,
        true,
    )
    .await?;
    let parked_current = parked(&periods, start, end);
    let parked_previous = parked(&periods, previous_start, start);

    let change = if previous.distance_mi > 0.0
        && previous.trips_with_unknown_distance == 0
        && current.trips_with_unknown_distance == 0
    {
        Some((current.distance_mi - previous.distance_mi) / previous.distance_mi * 100.0)
    } else {
        None
    };
    let narrative=format!("{} completed trips over {:.0} known miles and {} completed charging sessions. Comparisons below use matched records and explain their coverage. Missing measurements remain unavailable.",current.trip_count,current.distance_mi,current.charging_sessions);
    let mut result=WeeklySummary{starts_at:start,ends_at:end,is_demo:data.is_demo,current,previous,distance_change_pct:change,narrative,
        insights:Some(WeeklyInsights{generated_at:Utc::now(),current_driving:driving_current,previous_driving:driving_previous,current_charging:charging_current,previous_charging:charging_previous,current_parked:parked_current,previous_parked:parked_previous,cards:Vec::new(),
            methodology:vec!["Coverage refers to available recorded history; it does not prove every real trip or charge was captured.".into(),"Rolling seven-day windows; completed trips/charges are assigned by their start time. Demo windows end at the last recorded demo event.".into(),"Energy per distance uses the same trips for both energy and mileage. A comparison needs at least three matched trips and 80% distance coverage in each window, with no unknown trip distances.".into(),"Cost attribution requires complete prices and energy in both windows. Charging-volume effect uses the previous average unit price; the remaining change is the average-price effect. Entered home tariff estimates use the same current tariff in both windows.".into(),"Parked loss uses fully contained validated sessions of at least six hours with charge anchors within 45 minutes. Overlapping, moving, charging, incomplete and cross-window intervals are excluded. Rates describe observed intervals, not every parked hour or battery degradation.".into(),"Temperature and trip-length differences are descriptive context. Their causal effects, route, wind, speed and cabin use are not isolated; parked loss is not attributed to a vehicle system.".into()]})};
    let insights = result.insights.as_mut().expect("Constructed above");
    insights.cards = weekly_insights::cards(&result.current, &result.previous, insights);
    Ok(result)
}
