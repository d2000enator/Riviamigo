import React from 'react';
import type { AssistantViewContext } from './viewTypes';
import { Input, Badge } from '@riviamigo/ui/primitives';
export function DepartureSection({ context }: { context: AssistantViewContext }) {
  const { evaluation, prefs, fahrenheit, time, preference } = context;
  return (
    <>
      <div className="rm-assistant-section-heading">
        <div>
          <h3>A plan for stepping out the door</h3>
          <p>Choose when cabin preparation would be useful. Actual vehicle controls remain off.</p>
        </div>
        <Badge variant="default">Planning only</Badge>
      </div>
      <div className="rm-assistant-grid">
        <article className="rm-assistant-panel">
          <Input
            label={`Cabin preference (${fahrenheit ? '°F' : '°C'})`}
            type="number"
            min={fahrenheit ? 50 : 10}
            max={fahrenheit ? 86 : 30}
            value={Number(
              (fahrenheit ? (prefs.cabin_target_c * 9) / 5 + 32 : prefs.cabin_target_c).toFixed(1)
            )}
            onChange={(e) =>
              preference(
                'cabin_target_c',
                fahrenheit ? ((Number(e.target.value) - 32) * 5) / 9 : Number(e.target.value)
              )
            }
          />
          <Input
            label="Preparation time before departure (minutes)"
            type="number"
            min={0}
            max={120}
            value={prefs.preparation_minutes}
            onChange={(e) => preference('preparation_minutes', Number(e.target.value))}
          />
          <p>
            Save or preview to update the schedule. These settings do not change the Rivian app or
            vehicle.
          </p>
        </article>
        <article className="rm-assistant-panel">
          <h4>No command has been sent</h4>
          <p>
            Use your Rivian app for actual cabin preparation. Calendar integration and automatic
            vehicle control need separate setup and verified R2 support.
          </p>
        </article>
      </div>
      {evaluation.plans
        .filter((p) => p.status !== 'departed')
        .map((plan) => (
          <article className="rm-assistant-panel" key={plan.id}>
            <h4>{plan.title}</h4>
            <p>Preparation reminder: {time(plan.preparation_at)}</p>
            <p>Departure: {time(plan.departure_at)}</p>
            <Badge variant="default">
              {evaluation.is_demo ? 'Demo preparation plan' : 'Manual preparation reminder'}
            </Badge>
          </article>
        ))}
      {!evaluation.plans.length ? (
        <p>Add a trip under Tomorrow to see its preparation time.</p>
      ) : null}
    </>
  );
}
