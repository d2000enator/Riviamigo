import React from 'react';
import { usePersonalAssistantWeekly } from '@riviamigo/hooks';
import { WeeklyInsightCards } from './WeeklyInsightCards';
import type { AssistantViewContext } from './viewTypes';
import { Button, Input } from '@riviamigo/ui/primitives';
import { Download } from 'lucide-react';
import { Metric } from './Metric';
import { weeklyText, downloadText, optionalNumber, displayNumber as n } from './format';
export function WeeklySection({ context }: { context: AssistantViewContext }) {
  const { data, prefs, time, distance, preference, tz } = context;
  const detailed = usePersonalAssistantWeekly(context.vehicleId);
  const summary = detailed.data ?? data.weekly;
  return (
    <>
      <div className="rm-assistant-section-heading">
        <div>
          <h3>Your week, in perspective</h3>
          <p>
            {summary.is_demo ? 'EXAMPLE VEHICLE HISTORY · ' : ''}
            {time(summary.starts_at)} – {time(summary.ends_at)}
          </p>
        </div>
        <Button
          variant="secondary"
          iconLeft={<Download size={16} />}
          onClick={() =>
            downloadText(
              `riviamigo-weekly-${summary.ends_at.slice(0, 10)}.txt`,
              weeklyText(summary, tz)
            )
          }
        >
          Download summary
        </Button>
      </div>
      {detailed.isPending ? <p role="status">Loading the detailed weekly comparisons…</p> : null}
      {detailed.isError ? (
        <div role="alert" className="rm-assistant-panel">
          Detailed comparisons are unavailable. The basic history totals remain below.{' '}
          <Button
            variant="secondary"
            onClick={() => {
              void detailed.refetch();
            }}
          >
            Retry weekly comparisons
          </Button>
        </div>
      ) : null}
      {summary.insights ? (
        <WeeklyInsightCards insights={summary.insights} metric={context.metric} timezone={tz} />
      ) : null}
      <div className="rm-assistant-panel">
        <p>{summary.narrative}</p>
        <div className="rm-assistant-metrics">
          <Metric label="Recorded trips" value={n(summary.current.trip_count)} />
          <Metric label="Driving distance" value={distance(summary.current.distance_mi)} />
          <Metric
            label="Driving energy"
            value={`${n(summary.current.driving_energy_kwh, 1)} kWh`}
          />
          <Metric label="Completed charges" value={n(summary.current.charging_sessions)} />
          <Metric
            label="Charging energy"
            value={`${n(summary.current.charging_energy_kwh, 1)} kWh`}
          />
          <Metric
            label="Recorded charging cost"
            value={`$${n(summary.current.recorded_cost_usd, 2)} USD`}
          />
          <Metric
            label="Estimated home cost"
            value={`$${n(summary.current.estimated_home_cost_usd, 2)} USD`}
          />
          <Metric
            label="Distance vs prior seven days"
            value={
              summary.distance_change_pct == null
                ? 'Not enough history'
                : `${n(summary.distance_change_pct, 1)}%`
            }
          />
        </div>
      </div>
      <div className="rm-assistant-panel">
        <Input
          label="Home tariff for unpriced home sessions ($/kWh, optional)"
          type="number"
          min={0}
          max={5}
          step={0.01}
          value={prefs.home_rate_per_kwh ?? ''}
          onChange={(e) => preference('home_rate_per_kwh', optionalNumber(e.target.value))}
        />
        <p>
          Save to update estimates. Existing recorded session costs are preserved. Missing energy or
          prices stay unavailable.
        </p>
      </div>
    </>
  );
}
