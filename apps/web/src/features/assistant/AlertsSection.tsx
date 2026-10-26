import React from 'react';
import type { AssistantViewContext } from './viewTypes';
import { Button, Input, Badge } from '@riviamigo/ui/primitives';
import { errorText } from './format';
export function AlertsSection({ context }: { context: AssistantViewContext }) {
  const { data, evaluation, prefs, time, preference, acknowledge, setNotice, previewResult } =
    context;
  return (
    <>
      <div className="rm-assistant-section-heading">
        <div>
          <h3>A quieter alert inbox</h3>
          <p>Saved on your NAS and checked about once a minute, even when this page is closed.</p>
        </div>
        <Badge variant={prefs.alerts_enabled ? 'success' : 'default'}>
          {prefs.alerts_enabled ? 'Monitoring enabled' : 'Monitoring paused'}
        </Badge>
      </div>
      <div className="rm-assistant-panel">
        <label className="rm-assistant-check">
          <input
            type="checkbox"
            checked={prefs.alerts_enabled}
            onChange={(e) => preference('alerts_enabled', e.target.checked)}
          />
          Monitor saved plans and vehicle signals on this dashboard
        </label>
        <Input
          label="Treat battery data as stale after (minutes)"
          type="number"
          min={5}
          max={180}
          value={prefs.stale_after_minutes}
          onChange={(e) => preference('stale_after_minutes', Number(e.target.value))}
        />
        <p>
          Save changes to apply monitoring settings. No email, phone push or external notification
          service is configured.
        </p>
      </div>
      {!data.alerts.length ? (
        <div className="rm-assistant-empty">
          Your saved inbox is empty. Enable monitoring and save a plan, or save a demo warning
          scenario to try it.
        </div>
      ) : (
        <div className="rm-assistant-stack">
          {data.alerts.map((a) => (
            <article className="rm-assistant-panel" key={a.id}>
              <div className="rm-assistant-section-heading">
                <h4>{a.title}</h4>
                <Badge
                  variant={
                    a.resolved_at
                      ? 'default'
                      : a.severity === 'attention'
                        ? 'danger'
                        : a.severity === 'warning'
                          ? 'warning'
                          : 'default'
                  }
                >
                  {a.resolved_at ? 'Inactive' : a.acknowledged_at ? 'Acknowledged' : 'New'}
                </Badge>
              </div>
              {a.is_demo ? <span className="rm-assistant-eyebrow">DEMO ALERT</span> : null}
              <p>{a.message}</p>
              <small>
                First seen {time(a.first_seen_at)} · Last confirmed {time(a.last_seen_at)}
              </small>
              {!a.acknowledged_at ? (
                <Button
                  variant="secondary"
                  loading={acknowledge.isPending}
                  onClick={() => {
                    acknowledge.mutate(a.id, { onError: (e) => setNotice(errorText(e)) });
                  }}
                >
                  Acknowledge
                </Button>
              ) : null}
            </article>
          ))}
        </div>
      )}
      {previewResult ? (
        <div className="rm-assistant-panel">
          <h4>Unsaved preview conditions</h4>
          {evaluation.conditions.map((c) => (
            <p key={c.key}>
              <strong>{c.title}</strong> — {c.message}
            </p>
          ))}
          {!evaluation.conditions.length ? (
            <p>No conditions currently trigger in this preview.</p>
          ) : null}
        </div>
      ) : null}
    </>
  );
}
