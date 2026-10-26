import type { AssistantPlanAssessment, AssistantWeeklySummary } from '@riviamigo/types';
export function displayNumber(value: number | null | undefined, digits = 0): string {
  return typeof value === 'number' && Number.isFinite(value)
    ? value.toLocaleString(undefined, { maximumFractionDigits: digits })
    : '—';
}
export function displayTime(value: string | null | undefined, timezone: string): string {
  if (!value) return 'Unavailable';
  try {
    return new Intl.DateTimeFormat(undefined, {
      timeZone: timezone,
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(value));
  } catch {
    return 'Invalid date or timezone';
  }
}
export function localExampleDate(timezone: string, days = 1, hour = '08:00'): string {
  try {
    const parts = new Intl.DateTimeFormat('en-CA', {
      timeZone: timezone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    }).formatToParts(new Date(Date.now() + days * 86_400_000));
    const value = (key: string) => parts.find((p) => p.type === key)?.value ?? '';
    return `${value('year')}-${value('month')}-${value('day')}T${hour}`;
  } catch {
    return '';
  }
}
export function planStatusLabel(status: AssistantPlanAssessment['status']): string {
  return {
    ready: 'Ready with your reserve',
    charge_needed: 'More charge needed',
    charging_stop_needed: 'Plan a charging stop',
    waiting_for_data: 'Waiting for current data',
    departed: 'Past departure',
  }[status];
}
export function weeklyText(summary: AssistantWeeklySummary, timezone: string): string {
  const p = summary.current;
  const insights = summary.insights
    ? '\nWEEKLY COMPARISONS\n' +
      summary.insights.cards
        .map((c) => `${c.title} [${c.status}]: ${c.explanation}\n${c.context.join('\n')}`)
        .join('\n\n') +
      '\n\nMethod: ' +
      summary.insights.methodology.join(' ') +
      '\n'
    : '';
  return `${summary.is_demo ? 'DEMO VEHICLE HISTORY\n' : ''}Riviamigo weekly summary\n${displayTime(summary.starts_at, timezone)} to ${displayTime(summary.ends_at, timezone)}\n\n${summary.narrative}\n\nDistance: ${displayNumber(p.distance_mi, 1)} mi\nDriving energy: ${displayNumber(p.driving_energy_kwh, 1)} kWh\nCharging energy: ${displayNumber(p.charging_energy_kwh, 1)} kWh\nRecorded charging cost: $${displayNumber(p.recorded_cost_usd, 2)} USD\nEstimated home cost: $${displayNumber(p.estimated_home_cost_usd, 2)} USD\nMissing trip energy records: ${p.trips_with_unknown_energy}\nMissing charging cost records: ${p.sessions_with_unknown_cost}\n\n${insights}Amounts marked estimated use your entered assumptions. No vehicle command was sent.\n`;
}
export function downloadText(name: string, contents: string, type = 'text/plain;charset=utf-8') {
  const url = URL.createObjectURL(new Blob([contents], { type }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 30_000);
}

export const optionalNumber = (value: string) => (value.trim() ? Number(value) : null);
export const errorText = (error: unknown) =>
  error instanceof Error ? error.message : 'Could not complete that action. Please retry.';
export const copy = <T>(value: T): T => structuredClone(value);
