import React from 'react';
import type { AssistantViewContext } from './viewTypes';
import { Button, Input } from '@riviamigo/ui/primitives';
import { Metric } from './Metric';
import { displayNumber as n } from './format';
export function VacationSection({ context }: { context: AssistantViewContext }) {
  const { evaluation, prefs, preference, persist } = context;
  return (
    <>
      <div className="rm-assistant-section-heading">
        <div>
          <h3>Keep an eye on the parked battery</h3>
          <p>A passive watch with your own threshold. No vehicle wake-ups.</p>
        </div>
      </div>
      <div className="rm-assistant-grid">
        <article className="rm-assistant-panel">
          <label className="rm-assistant-check">
            <input
              type="checkbox"
              checked={prefs.vacation_enabled}
              onChange={(e) => preference('vacation_enabled', e.target.checked)}
            />
            Vacation watch
          </label>
          <Input
            label={`Expected return (${prefs.timezone}, optional)`}
            type="datetime-local"
            value={prefs.vacation_return_local ?? ''}
            onChange={(e) => preference('vacation_return_local', e.target.value || null)}
          />
          <Input
            label="Alert above battery loss (% points/day)"
            type="number"
            min={0.1}
            max={20}
            step={0.1}
            value={prefs.parked_loss_threshold_pct_day}
            onChange={(e) => preference('parked_loss_threshold_pct_day', Number(e.target.value))}
          />
          <div className="rm-assistant-actions">
            <Button
              variant="secondary"
              onClick={() => {
                void persist(true);
              }}
            >
              Save and start a fresh baseline
            </Button>
          </div>
          <p>
            Needs a current battery reading. Driving, charging, or a battery increase invalidates
            the parked-loss interval.
          </p>
        </article>
        <article className="rm-assistant-panel">
          <h4>{evaluation.vacation.status.replaceAll('_', ' ')}</h4>
          <div className="rm-assistant-metrics">
            <Metric
              label="Observation time"
              value={`${n(evaluation.vacation.observed_hours, 1)} h`}
            />
            <Metric
              label="Observed battery loss"
              value={
                evaluation.vacation.observed_loss_pct_day == null
                  ? '—'
                  : `${n(evaluation.vacation.observed_loss_pct_day, 1)}% / day`
              }
            />
            <Metric
              label="Estimated charge on return"
              value={
                evaluation.vacation.projected_return_soc_pct == null
                  ? '—'
                  : `${n(evaluation.vacation.projected_return_soc_pct)}%`
              }
            />
          </div>
          <p>{evaluation.vacation.explanation}</p>
        </article>
      </div>
    </>
  );
}
