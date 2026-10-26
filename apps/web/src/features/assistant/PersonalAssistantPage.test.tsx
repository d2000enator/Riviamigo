import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { PersonalAssistantState } from '@riviamigo/types';
const fixture = vi.hoisted(() => ({
  data: null as unknown as PersonalAssistantState,
  save: vi.fn(),
  preview: vi.fn(),
  ack: vi.fn(),
  refetch: vi.fn(),
  saveError: false,
}));
vi.mock('@riviamigo/hooks', () => ({
  useMe: () => ({ data: { user_id: 'owner' } }),
  useAuth: () => vi.fn(),
  useResolvedVehicleSelection: () => ({
    vehicles: [{ id: 'vehicle-1', model: 'R2', display_name: 'My R2' }],
    effectiveVehicleId: 'vehicle-1',
    vehicleSelectionReady: true,
  }),
  usePersonalAssistant: () => ({
    data: fixture.data,
    isPending: false,
    isError: false,
    refetch: fixture.refetch,
  }),
  usePersonalAssistantWeekly: () => ({
    data: fixture.data.weekly,
    isPending: false,
    isError: false,
    refetch: fixture.refetch,
  }),
  useSavePersonalAssistant: () => ({
    mutateAsync: fixture.save,
    isPending: false,
    isError: fixture.saveError,
  }),
  usePreviewPersonalAssistant: () => ({ mutateAsync: fixture.preview, isPending: false }),
  useAcknowledgeAssistantAlert: () => ({ mutate: fixture.ack, isPending: false }),
  useUploadAssistantReceipt: () => ({ mutateAsync: vi.fn(), isPending: false }),
  getAssistantReceipt: vi.fn(),
}));
vi.mock('../../components/layout/AppLayout', () => ({
  AppLayout: ({ children }: { children: React.ReactNode }) => <main>{children}</main>,
}));
vi.mock('@riviamigo/ui/primitives', () => ({
  PageLayout: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  Badge: ({ children }: { children: React.ReactNode }) => <span>{children}</span>,
  Button: ({
    children,
    onClick,
    disabled,
    type,
    role,
    ...props
  }: React.ButtonHTMLAttributes<HTMLButtonElement> & {
    loading?: boolean;
    iconLeft?: React.ReactNode;
    variant?: string;
  }) => (
    <button
      type={type ?? 'button'}
      onClick={onClick}
      disabled={disabled}
      role={role}
      aria-selected={props['aria-selected']}
    >
      {children}
    </button>
  ),
  Input: ({ label, ...props }: React.InputHTMLAttributes<HTMLInputElement> & { label: string }) => (
    <label>
      {label}
      <input {...props} />
    </label>
  ),
  SelectPicker: ({
    options,
    onChange,
    ...props
  }: {
    options: { value: string; label: string }[];
    onChange: (value: string) => void;
    value: string;
    'aria-label': string;
  }) => (
    <select {...props} onChange={(e) => onChange(e.target.value)}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  ),
}));
import { PersonalAssistantPage } from './PersonalAssistantPage';
import { displayNumber, displayTime, weeklyText } from './format';
function state(): PersonalAssistantState {
  const period = {
    trip_count: 0,
    distance_mi: 0,
    driving_energy_kwh: null,
    trips_with_unknown_energy: 0,
    trips_with_unknown_distance: 0,
    charging_sessions: 0,
    charging_energy_kwh: null,
    sessions_with_unknown_energy: 0,
    recorded_cost_usd: null,
    estimated_home_cost_usd: null,
    sessions_with_unknown_cost: 0,
  };
  return {
    revision: 7,
    document: {
      preferences: {
        timezone: 'America/Chicago',
        reserve_pct: 20,
        assumed_efficiency_wh_mi: 400,
        weather_buffer_pct: 15,
        battery_capacity_kwh: null,
        estimated_charger_kw: null,
        home_rate_per_kwh: null,
        alerts_enabled: false,
        stale_after_minutes: 30,
        parked_loss_threshold_pct_day: 2,
        cabin_target_c: 21,
        preparation_minutes: 20,
        vacation_enabled: false,
        vacation_return_local: null,
        demo_scenario: 'recorded',
      },
      plans: [],
      maintenance: [],
    },
    evaluation: {
      is_demo: true,
      generated_at: '2026-10-25T21:00:00Z',
      snapshot: {
        is_demo: true,
        soc_pct: null,
        soc_at: null,
        range_mi: null,
        range_at: null,
        battery_capacity_kwh: null,
        charge_limit_pct: null,
        charging: null,
        charging_at: null,
        wiper_fluid_low: null,
        wiper_fluid_at: null,
        brake_fluid_low: null,
        brake_fluid_at: null,
        odometer_mi: null,
        odometer_at: null,
        last_event_at: null,
        latest_activity_at: null,
      },
      plans: [],
      vacation: {
        status: 'off',
        observed_hours: 0,
        observed_loss_pct_day: null,
        projected_return_soc_pct: null,
        explanation: 'Start vacation watch.',
      },
      conditions: [],
      resolvable_rule_keys: [],
      vehicle_commands_enabled: false,
    },
    weekly: {
      starts_at: '2026-10-18T21:00:00Z',
      ends_at: '2026-10-25T21:00:00Z',
      is_demo: true,
      current: period,
      previous: { ...period },
      distance_change_pct: null,
      narrative: 'No recorded trips; unavailable amounts remain unknown.',
    },
    alerts: [],
    receipts: [],
    notification_delivery: 'dashboard_only',
  };
}
beforeEach(() => {
  vi.clearAllMocks();
  fixture.saveError = false;
  fixture.data = state();
  fixture.save.mockImplementation(async (input) => {
    fixture.data = {
      ...fixture.data,
      document: structuredClone(input.document),
      revision: input.revision + 1,
    };
    return fixture.data;
  });
  fixture.preview.mockImplementation(async () => structuredClone(fixture.data.evaluation));
});
describe('private personal assistant', () => {
  it('opens all six useful sections while keeping demo and controls status visible', async () => {
    render(<PersonalAssistantPage />);
    expect(await screen.findByText('DEMO · SAMPLE DATA')).toBeInTheDocument();
    for (const [tab, title] of [
      ['Tomorrow', 'Ready for your next departure'],
      ['Alerts', 'A quieter alert inbox'],
      ['Vacation', 'Keep an eye on the parked battery'],
      ['Weekly', 'Your week, in perspective'],
      ['Maintenance', 'Your service notebook'],
      ['Departure', 'A plan for stepping out the door'],
    ] as const) {
      fireEvent.click(screen.getByRole('tab', { name: tab }));
      expect(screen.getByText(title)).toBeInTheDocument();
    }
    expect(screen.getByText('Vehicle controls off')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /start climate|unlock|send command/i })
    ).not.toBeInTheDocument();
  });
  it('keeps a scenario as a draft until preview or save and sends the exact revision', async () => {
    render(<PersonalAssistantPage />);
    fireEvent.click(await screen.findByRole('button', { name: 'Add examples' }));
    fireEvent.change(screen.getByLabelText('Demo scenario'), { target: { value: 'low_charge' } });
    expect(fixture.save).not.toHaveBeenCalled();
    expect(fixture.preview).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Preview changes' }));
    await waitFor(() =>
      expect(fixture.preview).toHaveBeenCalledWith(
        expect.objectContaining({
          plans: [expect.objectContaining({ distance_mi: 60, round_trip: true })],
          preferences: expect.objectContaining({
            demo_scenario: 'low_charge',
            alerts_enabled: true,
          }),
        })
      )
    );
    expect(fixture.data.document.plans).toHaveLength(0);
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(await screen.findByText('Saved on your NAS.')).toBeInTheDocument();
    expect(fixture.save).toHaveBeenCalledWith(
      expect.objectContaining({
        revision: 7,
        document: expect.objectContaining({
          maintenance: [expect.objectContaining({ title: 'Tire rotation · example' })],
        }),
      })
    );
  });
  it('retains unsaved edits after a server refresh and reports save conflicts', async () => {
    const view = render(<PersonalAssistantPage />);
    fireEvent.click(await screen.findByRole('button', { name: 'Add examples' }));
    fixture.data = { ...fixture.data, revision: 8 };
    view.rerender(<PersonalAssistantPage />);
    fixture.saveError = true;
    fixture.save.mockRejectedValueOnce(
      new Error('These plans changed on another device. Reload before saving your edits.')
    );
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(await screen.findByText(/These plans changed on another device/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reload saved version' })).toBeInTheDocument();
    expect(fixture.save.mock.calls[0]![0].revision).toBe(7);
    expect(fixture.save.mock.calls[0]![0].document.plans).toHaveLength(1);
  });
  it('does not reuse a saved ready assessment after editing its trip distance', async () => {
    const plan = {
      id: 'plan-1',
      title: 'My trip',
      departure_local: '2026-10-27T09:30',
      distance_mi: 30,
      round_trip: false,
    };
    fixture.data.document.plans = [plan];
    fixture.data.evaluation.plans = [
      {
        id: 'plan-1',
        title: 'My trip',
        departure_at: '2026-10-27T14:30:00Z',
        preparation_at: '2026-10-27T14:10:00Z',
        distance_mi: 30,
        status: 'ready',
        method: 'recorded_capacity_and_assumed_efficiency',
        required_soc_pct: 35,
        arrival_soc_pct: 60,
        estimated_energy_kwh: 15,
        estimated_charge_hours: 0,
        explanation: 'Saved estimate.',
      },
    ];
    render(<PersonalAssistantPage />);
    expect(await screen.findByText('Ready with your reserve')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('One-way driving distance (mi)'), {
      target: { value: '300' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Keep in draft' }));
    expect(screen.queryByText('Ready with your reserve')).not.toBeInTheDocument();
    expect(screen.getByText('Not assessed')).toBeInTheDocument();
  });
  it('never offers simulation controls for a real vehicle', async () => {
    fixture.data.evaluation.is_demo = false;
    fixture.data.evaluation.snapshot.is_demo = false;
    render(<PersonalAssistantPage />);
    expect(await screen.findByText('Private vehicle data')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Add examples' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Demo scenario')).not.toBeInTheDocument();
  });
  it('acknowledges only the selected saved alert and labels simulated warnings', async () => {
    fixture.data.alerts = [
      {
        id: 'alert-1',
        rule_key: 'wiper_fluid',
        severity: 'attention',
        title: 'Washer fluid reported low',
        message: 'A demo warning.',
        is_demo: true,
        first_seen_at: '2026-10-25T21:00:00Z',
        last_seen_at: '2026-10-25T21:00:00Z',
        acknowledged_at: null,
        resolved_at: null,
      },
    ];
    render(<PersonalAssistantPage />);
    await screen.findByText('DEMO · SAMPLE DATA');
    fireEvent.click(screen.getByRole('tab', { name: /Alerts/ }));
    expect(screen.getByText('DEMO ALERT')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Acknowledge' }));
    expect(fixture.ack).toHaveBeenCalledWith('alert-1', expect.any(Object));
  });
  it('keeps missing values distinct from measured zero in display and weekly exports', () => {
    expect(displayNumber(null)).toBe('—');
    expect(displayNumber(0)).toBe('0');
    expect(displayNumber(NaN)).toBe('—');
    expect(displayTime(null, 'America/Chicago')).toBe('Unavailable');
    expect(weeklyText(state().weekly, 'America/Chicago')).toContain(
      'Recorded charging cost: $— USD'
    );
  });
});
