//! Weekly comparison DTOs. Quantities retain their own coverage denominators.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeeklyDrivingSample {
    pub matched_distance_mi: f64,
    pub matched_energy_kwh: f64,
    pub matched_trips: i64,
    pub distance_coverage_pct: Option<f64>,
    pub energy_wh_mi: Option<f64>,
    pub mean_trip_distance_mi: Option<f64>,
    pub mean_outside_temp_c: Option<f64>,
    pub temperature_coverage_pct: Option<f64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeeklyChargeSample {
    pub priced_energy_kwh: Option<f64>,
    pub matched_cost_usd: Option<f64>,
    pub average_price_usd_kwh: Option<f64>,
    pub energy_coverage_pct: Option<f64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeeklyParkedSample {
    pub validated_periods: i64,
    pub excluded_periods: i64,
    pub observed_hours: f64,
    pub loss_pct_points: Option<f64>,
    pub loss_pct_points_day: Option<f64>,
    pub soc_based_energy_kwh: Option<f64>,
    pub periods_without_energy: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyCostAttribution {
    pub charging_volume_effect_usd: f64,
    pub average_price_effect_usd: f64,
    pub total_change_usd: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyInsight {
    pub category: String,
    pub title: String,
    pub status: String,
    pub unit: String,
    pub current_value: Option<f64>,
    pub previous_value: Option<f64>,
    pub change_pct: Option<f64>,
    pub explanation: String,
    pub context: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_attribution: Option<WeeklyCostAttribution>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeeklyInsights {
    pub generated_at: DateTime<Utc>,
    pub current_driving: WeeklyDrivingSample,
    pub previous_driving: WeeklyDrivingSample,
    pub current_charging: WeeklyChargeSample,
    pub previous_charging: WeeklyChargeSample,
    pub current_parked: WeeklyParkedSample,
    pub previous_parked: WeeklyParkedSample,
    pub cards: Vec<WeeklyInsight>,
    pub methodology: Vec<String>,
}
