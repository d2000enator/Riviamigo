import type { Trip, VehicleStatus } from '@riviamigo/types';
import type { UnitPreferences } from '@riviamigo/ui/lib/utils';

/** Preserve missing measurements. A zero battery reading is a valid reading. */
export function finite(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

/** The existing curve transport replaces missing power with zero. The current
 * API marks actual/derived samples with power_method; an unmarked zero cannot
 * establish a measurement. Preserve explicitly sourced zero and legacy >0. */
export function chargePowerValue(point: { power_kw: number; power_method?: string }): number | null {
  const value = finite(point.power_kw);
  return value === 0 && !point.power_method ? null : value;
}

/** Null gap markers are presentation metadata, never invented measurements. */
export function withHistoryGaps(rows: Array<{ts:string;value:number|null}>, maximumGapMs = 6 * 60 * 60 * 1000) {
  const ordered = rows.filter((row) => Number.isFinite(Date.parse(row.ts))).slice().sort((a,b) => Date.parse(a.ts)-Date.parse(b.ts));
  const output: Array<{ts:string;value:number|null}> = [];
  for (const row of ordered) {
    const previous = output.at(-1);
    if (previous && Date.parse(row.ts)-Date.parse(previous.ts) > maximumGapMs) {
      output.push({ts:new Date((Date.parse(previous.ts)+Date.parse(row.ts))/2).toISOString(),value:null});
    }
    output.push(row);
  }
  return output;
}

/** Same documented upstream parked-window numerator and denominator, weighted by time. */
export function dailyParkedDrain(rows: Array<{ total_soc_lost: number | null; hours_parked: number | null }>) {
  const valid = rows.filter((row) => finite(row.total_soc_lost) !== null && finite(row.hours_parked) !== null && row.hours_parked! > 0);
  const hours = valid.reduce((sum, row) => sum + row.hours_parked!, 0);
  return hours > 0 ? valid.reduce((sum, row) => sum + row.total_soc_lost!, 0) / hours * 24 : null;
}

export function efficiencyBarValue(whPerMile: number | null, preferences: UnitPreferences): number | null {
  if (whPerMile === null || !Number.isFinite(whPerMile) || whPerMile <= 0) return null;
  if (preferences.efficiency_display === 'energy_per_distance') {
    return preferences.distance_unit === 'kilometers' ? whPerMile / 1.609344 : whPerMile;
  }
  return (preferences.distance_unit === 'kilometers' ? 1609.344 : 1000) / whPerMile;
}

export function tripLabel(trip: Trip): string {
  const start = trip.start_place ?? trip.start_address;
  const end = trip.end_place ?? trip.end_address;
  return start && end ? `${start} → ${end}` : end ?? start ?? 'Recorded trip';
}

/** A browser socket being online does not establish that vehicle readings are fresh. */
export function vehicleReadingState(status: VehicleStatus | null | undefined, demo: boolean, now = Date.now()) {
  if (!status) return 'No readings yet';
  if (demo) return 'Demo snapshot';
  if (status.auth_state === 'needs_reauth') return 'Sign-in required';
  const timestamp = status.last_event_at ?? status.last_updated;
  const age = timestamp ? now - Date.parse(timestamp) : Number.NaN;
  if (status.telemetry_stale || !Number.isFinite(age) || age > 15 * 60 * 1000) return 'Stale readings';
  if (status.power_state === 'charging' || status.charger_state === 'Charging') return 'Charging';
  if (status.power_state === 'drive' || status.power_state === 'go') return 'Driving';
  if (status.power_state === 'sleep') return 'Asleep';
  return 'Parked';
}
