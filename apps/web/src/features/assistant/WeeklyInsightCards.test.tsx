import React from 'react';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import type { AssistantWeeklyInsights } from '@riviamigo/types';
import { WeeklyInsightCards } from './WeeklyInsightCards';
const fixture: AssistantWeeklyInsights = {
  generated_at: '2026-10-26T00:00:00Z',
  current_driving: {
    matched_distance_mi: 100,
    matched_energy_kwh: 40,
    matched_trips: 5,
    distance_coverage_pct: 100,
    energy_wh_mi: 400,
    mean_trip_distance_mi: 20,
    mean_outside_temp_c: null,
    temperature_coverage_pct: 0,
  },
  previous_driving: {
    matched_distance_mi: 100,
    matched_energy_kwh: 30,
    matched_trips: 5,
    distance_coverage_pct: 100,
    energy_wh_mi: 300,
    mean_trip_distance_mi: 20,
    mean_outside_temp_c: null,
    temperature_coverage_pct: 0,
  },
  current_charging: {
    priced_energy_kwh: null,
    matched_cost_usd: null,
    average_price_usd_kwh: null,
    energy_coverage_pct: null,
  },
  previous_charging: {
    priced_energy_kwh: null,
    matched_cost_usd: null,
    average_price_usd_kwh: null,
    energy_coverage_pct: null,
  },
  current_parked: {
    validated_periods: 0,
    excluded_periods: 1,
    observed_hours: 0,
    loss_pct_points: null,
    loss_pct_points_day: null,
    soc_based_energy_kwh: null,
    periods_without_energy: 0,
  },
  previous_parked: {
    validated_periods: 0,
    excluded_periods: 0,
    observed_hours: 0,
    loss_pct_points: null,
    loss_pct_points_day: null,
    soc_based_energy_kwh: null,
    periods_without_energy: 0,
  },
  cards: [
    {
      category: 'efficiency',
      title: 'Energy per distance',
      status: 'comparison',
      unit: 'wh_per_mile',
      current_value: 400,
      previous_value: 300,
      change_pct: 33.3,
      explanation: 'More recorded energy per distance in this matched sample.',
      context: ['Temperature effects have not been isolated.'],
    },
    {
      category: 'charging_cost',
      title: 'Known charging cost',
      status: 'partial',
      unit: 'usd',
      current_value: 0,
      previous_value: null,
      change_pct: null,
      explanation: 'One session price is missing; no full-week comparison.',
      context: ['No invented prices.'],
    },
    {
      category: 'parked_loss',
      title: 'Observed parked battery loss',
      status: 'waiting_for_history',
      unit: 'pct_points_day',
      current_value: null,
      previous_value: null,
      change_pct: null,
      explanation: 'No comparable parked intervals.',
      context: ['Missing intervals are not zero loss.'],
    },
  ],
  methodology: ['Use matched samples; no vehicle command was sent.'],
};
describe('weekly evidence cards', () => {
  it('shows actual comparisons, partial coverage, true zero prices and unavailable observations distinctly', () => {
    render(<WeeklyInsightCards insights={fixture} metric={false} timezone="America/Chicago" />);
    expect(screen.getByText('400 Wh/mi')).toBeInTheDocument();
    expect(screen.getByText('+33.3%')).toBeInTheDocument();
    expect(screen.getByText('$0 USD')).toBeInTheDocument();
    expect(screen.getByText('Partial coverage')).toBeInTheDocument();
    expect(screen.getAllByText('Unavailable')).toHaveLength(3);
    expect(screen.getByText('Missing intervals are not zero loss.')).toBeInTheDocument();
  });
  it('converts energy per distance consistently for metric display and retains its relative change', () => {
    render(<WeeklyInsightCards insights={fixture} metric={true} timezone="America/Chicago" />);
    expect(screen.getByText('249 Wh/km')).toBeInTheDocument();
    expect(screen.getByText('+33.3%')).toBeInTheDocument();
  });
  it('does not invent a percentage change from a genuinely free cost baseline', () => {
    const data = structuredClone(fixture);
    data.cards[1]!.status = 'comparison';
    data.cards[1]!.previous_value = 0;
    render(<WeeklyInsightCards insights={data} metric={false} timezone="America/Chicago" />);
    expect(screen.getByText('No percentage baseline')).toBeInTheDocument();
  });
});
