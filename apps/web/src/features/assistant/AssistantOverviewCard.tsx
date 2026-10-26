import React from 'react';
import { useNavigate } from '@tanstack/react-router';
import { usePersonalAssistant } from '@riviamigo/hooks';
import { Button } from '@riviamigo/ui/primitives';
import { planStatusLabel } from './format';
import './style.css';
export function AssistantOverviewCard({ vehicleId }: { vehicleId: string }) {
  const query = usePersonalAssistant(vehicleId);
  const navigate = useNavigate();
  const next = query.data?.evaluation.plans.find((p) => p.status !== 'departed');
  const count = query.data?.alerts.filter((a) => !a.acknowledged_at && !a.resolved_at).length ?? 0;
  return (
    <section className="rm-assistant-overview">
      <div>
        <span className="rm-assistant-eyebrow">YOUR DAY, READY</span>
        <h3>{next ? next.title : 'A little planning. A quieter inbox.'}</h3>
        <p>
          {next
            ? planStatusLabel(next.status)
            : 'Trip readiness, vacation watch, weekly summaries and your service notebook.'}
          {count ? ` · ${count} new dashboard ${count === 1 ? 'alert' : 'alerts'}` : ''}
        </p>
      </div>
      <Button
        variant="secondary"
        onClick={() => {
          void navigate({ to: '/assistant' });
        }}
      >
        Open assistant
      </Button>
    </section>
  );
}
