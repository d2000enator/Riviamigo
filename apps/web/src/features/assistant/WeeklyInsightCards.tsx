import React from 'react';
import type { AssistantWeeklyInsight, AssistantWeeklyInsights } from '@riviamigo/types';
import { Badge } from '@riviamigo/ui/primitives';
import { displayNumber, displayTime } from './format';
function value(number: number | null, unit: AssistantWeeklyInsight['unit'], metric: boolean) {
  if (number == null) return 'Unavailable';
  if (unit === 'usd') return `$${displayNumber(number, 2)} USD`;
  if (unit === 'wh_per_mile')
    return `${displayNumber(metric ? number / 1.609344 : number, 0)} ${metric ? 'Wh/km' : 'Wh/mi'}`;
  return `${displayNumber(number, 2)} points/day`;
}
export function WeeklyInsightCards({
  insights,
  metric,
  timezone,
}: {
  insights: AssistantWeeklyInsights;
  metric: boolean;
  timezone: string;
}) {
  return (
    <section aria-label="Weekly explanations" className="rm-assistant-stack">
      <p>
        Comparisons calculated {displayTime(insights.generated_at, timezone)} · Based on recorded
        history
      </p>
      {insights.cards.map((card) => (
        <article className="rm-assistant-panel" key={card.category}>
          <div className="rm-assistant-section-heading">
            <h4>{card.title}</h4>
            <Badge variant={card.status === 'comparison' ? 'info' : 'default'}>
              {card.status === 'comparison'
                ? 'Comparable samples'
                : card.status === 'partial'
                  ? 'Partial coverage'
                  : 'Waiting for history'}
            </Badge>
          </div>
          <div className="rm-assistant-metrics">
            <div>
              <small>Current seven days</small>
              <strong>{value(card.current_value, card.unit, metric)}</strong>
            </div>
            <div>
              <small>Previous seven days</small>
              <strong>{value(card.previous_value, card.unit, metric)}</strong>
            </div>
            <div>
              <small>Change in this sample</small>
              <strong>
                {card.change_pct == null
                  ? card.status === 'comparison'
                    ? 'No percentage baseline'
                    : 'Not compared'
                  : `${card.change_pct > 0 ? '+' : ''}${displayNumber(card.change_pct, 1)}%`}
              </strong>
            </div>
          </div>
          <p>{card.explanation}</p>
          <ul>
            {card.context.map((text) => (
              <li key={text}>{text}</li>
            ))}
          </ul>
        </article>
      ))}
      <details className="rm-assistant-panel">
        <summary>How these comparisons work</summary>
        <ul>
          {insights.methodology.map((text) => (
            <li key={text}>{text}</li>
          ))}
        </ul>
      </details>
    </section>
  );
}
