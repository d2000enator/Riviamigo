export type AssistantDemoScenario =
  'recorded' | 'ready' | 'low_charge' | 'charging_interrupted' | 'fluid_warning' | 'vacation_drain';
export interface AssistantPreferences {
  timezone: string;
  reserve_pct: number;
  assumed_efficiency_wh_mi: number;
  weather_buffer_pct: number;
  battery_capacity_kwh: number | null;
  estimated_charger_kw: number | null;
  home_rate_per_kwh: number | null;
  alerts_enabled: boolean;
  stale_after_minutes: number;
  parked_loss_threshold_pct_day: number;
  cabin_target_c: number;
  preparation_minutes: number;
  vacation_enabled: boolean;
  vacation_return_local: string | null;
  demo_scenario: AssistantDemoScenario;
}
export interface AssistantTripPlan {
  id: string;
  title: string;
  departure_local: string;
  distance_mi: number;
  round_trip: boolean;
}
export interface AssistantMaintenanceEntry {
  id: string;
  title: string;
  performed_on: string;
  odometer_mi: number | null;
  cost_usd: number | null;
  receipt_reference: string;
  notes: string;
  next_due_on: string | null;
  next_due_odometer_mi: number | null;
}
export interface AssistantDocument {
  preferences: AssistantPreferences;
  plans: AssistantTripPlan[];
  maintenance: AssistantMaintenanceEntry[];
}
export interface AssistantSnapshot {
  is_demo: boolean;
  soc_pct: number | null;
  soc_at: string | null;
  range_mi: number | null;
  range_at: string | null;
  battery_capacity_kwh: number | null;
  charge_limit_pct: number | null;
  charge_limit_at?: string | null;
  charging: boolean | null;
  charging_at: string | null;
  wiper_fluid_low: boolean | null;
  wiper_fluid_at: string | null;
  brake_fluid_low: boolean | null;
  brake_fluid_at: string | null;
  odometer_mi: number | null;
  odometer_at: string | null;
  last_event_at: string | null;
  latest_activity_at: string | null;
}
export interface AssistantCondition {
  key: string;
  severity: 'info' | 'warning' | 'attention';
  title: string;
  message: string;
}
export interface AssistantPlanAssessment {
  id: string;
  title: string;
  departure_at: string;
  preparation_at: string;
  distance_mi: number;
  status: 'waiting_for_data' | 'departed' | 'charging_stop_needed' | 'ready' | 'charge_needed';
  method: string;
  required_soc_pct: number | null;
  arrival_soc_pct: number | null;
  estimated_energy_kwh: number | null;
  estimated_charge_hours: number | null;
  explanation: string;
}
export interface AssistantVacationAssessment {
  status: string;
  observed_hours: number;
  observed_loss_pct_day: number | null;
  projected_return_soc_pct: number | null;
  explanation: string;
}
export interface AssistantEvaluation {
  is_demo: boolean;
  generated_at: string;
  snapshot: AssistantSnapshot;
  plans: AssistantPlanAssessment[];
  vacation: AssistantVacationAssessment;
  conditions: AssistantCondition[];
  resolvable_rule_keys: string[];
  vehicle_commands_enabled: false;
}
export interface AssistantWeeklyPeriod {
  trip_count: number;
  distance_mi: number;
  driving_energy_kwh: number | null;
  trips_with_unknown_energy: number;
  trips_with_unknown_distance: number;
  charging_sessions: number;
  charging_energy_kwh: number | null;
  sessions_with_unknown_energy: number;
  recorded_cost_usd: number | null;
  estimated_home_cost_usd: number | null;
  sessions_with_unknown_cost: number;
}
export interface AssistantWeeklySummary {
  starts_at: string;
  ends_at: string;
  is_demo: boolean;
  current: AssistantWeeklyPeriod;
  previous: AssistantWeeklyPeriod;
  distance_change_pct: number | null;
  narrative: string;
  insights?: AssistantWeeklyInsights;
}
export interface AssistantSavedAlert extends Omit<AssistantCondition, 'key'> {
  id: string;
  rule_key: string;
  is_demo: boolean;
  first_seen_at: string;
  last_seen_at: string;
  acknowledged_at: string | null;
  resolved_at: string | null;
}
export interface AssistantReceiptMetadata {
  id: string;
  entry_id: string;
  filename: string;
  mime_type: string;
  byte_count: number;
  sha256: string;
  created_at: string;
}
export interface PersonalAssistantState {
  revision: number;
  document: AssistantDocument;
  evaluation: AssistantEvaluation;
  weekly: AssistantWeeklySummary;
  alerts: AssistantSavedAlert[];
  receipts: AssistantReceiptMetadata[];
  notification_delivery: 'dashboard_only';
}
export interface AssistantReceiptDownload {
  filename: string;
  mime_type: string;
  data_base64: string;
  sha256: string;
}

export interface AssistantWeeklyDrivingSample {
  matched_distance_mi: number;
  matched_energy_kwh: number;
  matched_trips: number;
  distance_coverage_pct: number | null;
  energy_wh_mi: number | null;
  mean_trip_distance_mi: number | null;
  mean_outside_temp_c: number | null;
  temperature_coverage_pct: number | null;
}
export interface AssistantWeeklyChargeSample {
  priced_energy_kwh: number | null;
  matched_cost_usd: number | null;
  average_price_usd_kwh: number | null;
  energy_coverage_pct: number | null;
}
export interface AssistantWeeklyParkedSample {
  validated_periods: number;
  excluded_periods: number;
  observed_hours: number;
  loss_pct_points: number | null;
  loss_pct_points_day: number | null;
  soc_based_energy_kwh: number | null;
  periods_without_energy: number;
}
export interface AssistantWeeklyInsight {
  category: 'efficiency' | 'charging_cost' | 'parked_loss';
  title: string;
  status: 'comparison' | 'partial' | 'waiting_for_history';
  unit: 'wh_per_mile' | 'usd' | 'pct_points_day';
  current_value: number | null;
  previous_value: number | null;
  change_pct: number | null;
  explanation: string;
  context: string[];
  cost_attribution?: {
    charging_volume_effect_usd: number;
    average_price_effect_usd: number;
    total_change_usd: number;
  };
}
export interface AssistantWeeklyInsights {
  generated_at: string;
  current_driving: AssistantWeeklyDrivingSample;
  previous_driving: AssistantWeeklyDrivingSample;
  current_charging: AssistantWeeklyChargeSample;
  previous_charging: AssistantWeeklyChargeSample;
  current_parked: AssistantWeeklyParkedSample;
  previous_parked: AssistantWeeklyParkedSample;
  cards: AssistantWeeklyInsight[];
  methodology: string[];
}
