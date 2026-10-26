//! Personal planning inputs remain separate from Rivian-reported telemetry.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DemoScenario {
    #[default]
    Recorded,
    Ready,
    LowCharge,
    ChargingInterrupted,
    FluidWarning,
    VacationDrain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Preferences {
    pub timezone: String,
    pub reserve_pct: f64,
    pub assumed_efficiency_wh_mi: f64,
    pub weather_buffer_pct: f64,
    pub battery_capacity_kwh: Option<f64>,
    pub estimated_charger_kw: Option<f64>,
    pub home_rate_per_kwh: Option<f64>,
    pub alerts_enabled: bool,
    pub stale_after_minutes: i64,
    pub parked_loss_threshold_pct_day: f64,
    pub cabin_target_c: f64,
    pub preparation_minutes: i64,
    pub vacation_enabled: bool,
    pub vacation_return_local: Option<String>,
    pub demo_scenario: DemoScenario,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            timezone: "America/Chicago".into(),
            reserve_pct: 20.0,
            assumed_efficiency_wh_mi: 400.0,
            weather_buffer_pct: 15.0,
            battery_capacity_kwh: None,
            estimated_charger_kw: None,
            home_rate_per_kwh: None,
            alerts_enabled: false,
            stale_after_minutes: 30,
            parked_loss_threshold_pct_day: 2.0,
            cabin_target_c: 21.0,
            preparation_minutes: 20,
            vacation_enabled: false,
            vacation_return_local: None,
            demo_scenario: DemoScenario::Recorded,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TripPlan {
    pub id: Uuid,
    pub title: String,
    pub departure_local: String,
    pub distance_mi: f64,
    pub round_trip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceEntry {
    pub id: Uuid,
    pub title: String,
    pub performed_on: String,
    pub odometer_mi: Option<f64>,
    pub cost_usd: Option<f64>,
    pub receipt_reference: String,
    pub notes: String,
    pub next_due_on: Option<String>,
    pub next_due_odometer_mi: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Document {
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub plans: Vec<TripPlan>,
    #[serde(default)]
    pub maintenance: Vec<MaintenanceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Observation {
    pub was_charging: bool,
    pub interruption_since: Option<DateTime<Utc>>,
    pub vacation_started_at: Option<DateTime<Utc>>,
    pub vacation_start_soc: Option<f64>,
    pub vacation_baseline_mi: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub is_demo: bool,
    pub soc_pct: Option<f64>,
    pub soc_at: Option<DateTime<Utc>>,
    pub range_mi: Option<f64>,
    pub range_at: Option<DateTime<Utc>>,
    pub battery_capacity_kwh: Option<f64>,
    pub charge_limit_pct: Option<f64>,
    pub charge_limit_at: Option<DateTime<Utc>>,
    pub charging: Option<bool>,
    pub charging_at: Option<DateTime<Utc>>,
    pub wiper_fluid_low: Option<bool>,
    pub wiper_fluid_at: Option<DateTime<Utc>>,
    pub brake_fluid_low: Option<bool>,
    pub brake_fluid_at: Option<DateTime<Utc>>,
    pub odometer_mi: Option<f64>,
    pub odometer_at: Option<DateTime<Utc>>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub latest_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub key: String,
    pub severity: String,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAssessment {
    pub id: Uuid,
    pub title: String,
    pub departure_at: DateTime<Utc>,
    pub preparation_at: DateTime<Utc>,
    pub distance_mi: f64,
    pub status: String,
    pub method: String,
    pub required_soc_pct: Option<f64>,
    pub arrival_soc_pct: Option<f64>,
    pub estimated_energy_kwh: Option<f64>,
    pub estimated_charge_hours: Option<f64>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacationAssessment {
    pub status: String,
    pub observed_hours: f64,
    pub observed_loss_pct_day: Option<f64>,
    pub projected_return_soc_pct: Option<f64>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub is_demo: bool,
    pub generated_at: DateTime<Utc>,
    pub snapshot: Snapshot,
    pub plans: Vec<PlanAssessment>,
    pub vacation: VacationAssessment,
    pub conditions: Vec<AlertCondition>,
    pub resolvable_rule_keys: Vec<String>,
    pub observation: Observation,
    pub vehicle_commands_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeeklyPeriod {
    pub trip_count: i64,
    pub distance_mi: f64,
    pub driving_energy_kwh: Option<f64>,
    pub trips_with_unknown_energy: i64,
    pub trips_with_unknown_distance: i64,
    pub charging_sessions: i64,
    pub charging_energy_kwh: Option<f64>,
    pub sessions_with_unknown_energy: i64,
    pub recorded_cost_usd: Option<f64>,
    pub estimated_home_cost_usd: Option<f64>,
    pub sessions_with_unknown_cost: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklySummary {
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub is_demo: bool,
    pub current: WeeklyPeriod,
    pub previous: WeeklyPeriod,
    pub distance_change_pct: Option<f64>,
    pub narrative: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insights: Option<super::weekly_model::WeeklyInsights>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SavedAlert {
    pub id: Uuid,
    pub rule_key: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub is_demo: bool,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ReceiptMetadata {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub byte_count: i64,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Serialize)]
pub struct AssistantState {
    pub revision: i64,
    pub document: Document,
    pub evaluation: Evaluation,
    pub weekly: WeeklySummary,
    pub alerts: Vec<SavedAlert>,
    pub receipts: Vec<ReceiptMetadata>,
    pub notification_delivery: &'static str,
}
