import { describe, expect, it } from 'vitest';
import type { UnitPreferences } from '@riviamigo/ui/lib/utils';
import type { VehicleStatus } from '@riviamigo/types';
import { dailyParkedDrain, efficiencyBarValue, finite, vehicleReadingState, withHistoryGaps } from './metrics';

const preferences: UnitPreferences = { mode:'imperial',distance_unit:'miles',speed_unit:'mph',temperature_unit:'fahrenheit',pressure_unit:'psi',altitude_unit:'feet',place_radius_unit:'feet',efficiency_display:'distance_per_energy' };
describe('R2 overview measurement contracts', () => {
  it('does not draw unobserved battery history across a long silent interval', () => {
    const rows=[{ts:'2026-10-25T00:00:00Z',value:70},{ts:'2026-10-25T12:00:00Z',value:60}];
    const output=withHistoryGaps(rows);expect(output.map((point)=>point.value)).toEqual([70,null,60]);expect(rows).toHaveLength(2);
  });
  it('preserves legitimate zero and rejects missing/nonfinite measurements', () => {
    expect(finite(0)).toBe(0);expect(finite(null)).toBeNull();expect(finite(undefined)).toBeNull();expect(finite(Number.NaN)).toBeNull();expect(finite('78')).toBeNull();
  });
  it('weights parked loss by valid observed hours, not a mean of daily ratios', () => {
    expect(dailyParkedDrain([{total_soc_lost:1,hours_parked:24},{total_soc_lost:2,hours_parked:12},{total_soc_lost:null,hours_parked:10},{total_soc_lost:99,hours_parked:0}])).toBe(2);
    expect(dailyParkedDrain([])).toBeNull();expect(dailyParkedDrain([{total_soc_lost:0,hours_parked:12}])).toBe(0);
  });
  it('honors every existing distance/efficiency preference combination', () => {
    expect(efficiencyBarValue(250,preferences)).toBe(4);
    expect(efficiencyBarValue(250,{...preferences,distance_unit:'kilometers'})).toBeCloseTo(6.437376);
    expect(efficiencyBarValue(250,{...preferences,efficiency_display:'energy_per_distance'})).toBe(250);
    expect(efficiencyBarValue(250,{...preferences,distance_unit:'kilometers',efficiency_display:'energy_per_distance'})).toBeCloseTo(155.3428);
    expect(efficiencyBarValue(0,preferences)).toBeNull();expect(efficiencyBarValue(null,preferences)).toBeNull();
  });
  it('does not confuse an online browser socket with fresh vehicle readings', () => {
    const now=Date.parse('2026-10-25T20:00:00Z');
    const status={is_online:true,last_updated:'2026-10-25T20:00:00Z',last_event_at:'2026-10-24T20:00:00Z',power_state:'ready'} as VehicleStatus;
    expect(vehicleReadingState(status,false,now)).toBe('Stale readings');
    expect(vehicleReadingState(status,true,now)).toBe('Demo snapshot');
    expect(vehicleReadingState(undefined,false,now)).toBe('No readings yet');
  });
  it('keeps sign-in, sleep, driving and charging states distinct', () => {
    const now=Date.parse('2026-10-25T20:00:00Z');const base={last_updated:'2026-10-25T20:00:00Z',is_online:true} as VehicleStatus;
    expect(vehicleReadingState({...base,power_state:'sleep'},false,now)).toBe('Asleep');
    expect(vehicleReadingState({...base,power_state:'drive'},false,now)).toBe('Driving');
    expect(vehicleReadingState({...base,charger_state:'Charging'},false,now)).toBe('Charging');
    expect(vehicleReadingState({...base,auth_state:'needs_reauth'},false,now)).toBe('Sign-in required');
  });
});
