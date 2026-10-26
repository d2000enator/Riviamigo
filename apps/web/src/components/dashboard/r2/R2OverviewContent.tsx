import { AssistantOverviewCard } from '../../../features/assistant/AssistantOverviewCard';
import { PersonalR2Artwork } from '../../PersonalR2Artwork';
import React, { useEffect, useMemo, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import {
  useChargeCurve, useChargeSessions, useChargingSummary, useCurrentVehicleStatus,
  useEfficiencySummary, usePhantomDrain, useSocHistory, useTrips,
} from '@riviamigo/hooks';
import type { Vehicle } from '@riviamigo/types';
import type { WidgetCtx } from '@riviamigo/dashboards';
import { ChartSection } from '@riviamigo/ui/primitives';
import { RichTimeSeriesChart } from '@riviamigo/ui/charts';
import {
  formatDuration, formatEfficiency, formatEfficiencyValue, formatKwh, formatMiles,
  getEfficiencyUnitLabel, getUnitPreferences,
} from '@riviamigo/ui/lib/utils';
import { formatAppDateTime, formatAppDate, getAppTimezone } from '@riviamigo/ui/lib/dateTime';
import { ArrowRight, Clock, Home, Leaf, Lock, Route, Zap } from 'lucide-react';
import { chargePowerValue, dailyParkedDrain, efficiencyBarValue, finite, tripLabel, vehicleReadingState, withHistoryGaps } from './metrics';
import './style.css';

type Props = { ctx: WidgetCtx; vehicle: Vehicle | undefined };
type QueryState = { isError: boolean; isPending: boolean; isPlaceholderData?: boolean; refetch: () => unknown };

/** Read-only composition over the same authenticated, selected-vehicle hooks as
 * the standard dashboards. No fixtures, new credential handling or commands. */
export function R2OverviewContent({ ctx, vehicle }: Props) {
  const navigate = useNavigate();
  const id = ctx.vehicleId ?? null;
  const from = ctx.from ?? null;
  const to = ctx.to ?? null;
  const status = useCurrentVehicleStatus(id);
  const history = useSocHistory(id, from, to);
  const recentTrips = useTrips(id, from, to, 1, 3);
  const efficiency = useEfficiencySummary(id, from, to);
  const charging = useChargingSummary(id, from, to);
  const sessions = useChargeSessions(id, from, to, 1, 250);
  const drain = usePhantomDrain(id, from, to);
  const [preferences, setPreferences] = useState(getUnitPreferences);
  useEffect(() => {
    const update = () => setPreferences(getUnitPreferences());
    window.addEventListener('rm-units-change', update);
    window.addEventListener('storage', update);
    return () => { window.removeEventListener('rm-units-change', update); window.removeEventListener('storage', update); };
  }, []);

  // These authenticated list endpoints are scoped by the hook's vehicleId and
  // query key. Their current DTOs omit vehicle_id, which transport normalizes
  // to ''. Accept that sentinel, but reject an explicit conflicting identity.
  // The parent is keyed by vehicleId; range placeholders remain hidden.
  const inScope = (row: { vehicle_id: string }) => !!id && (row.vehicle_id === '' || row.vehicle_id === id);
  const tripRows = recentTrips.isPlaceholderData ? [] : (recentTrips.data?.items ?? []).filter(inScope);
  const chargeRows = sessions.isPlaceholderData ? [] : (sessions.data?.items ?? []).filter(inScope);
  const lastCharge = chargeRows[0];
  const curve = useChargeCurve(lastCharge?.id ?? null, id, lastCharge?.ended_at === null);
  const soc = history.isPlaceholderData ? [] : withHistoryGaps(history.data ?? []);
  const efficiencyData = efficiency.isPlaceholderData ? undefined : efficiency.data;
  const chargingData = charging.isPlaceholderData ? undefined : charging.data;
  const drainValue = drain.isPlaceholderData ? null : dailyParkedDrain(drain.data ?? []);
  const updated = status.data?.last_event_at ?? status.data?.last_updated;
  const demo = vehicle?.is_demo === true;
  const readingState = vehicleReadingState(status.data, demo);
  const intervals = useMemo(() => chargeRows
    .filter((row) => row.ended_at && Date.parse(row.ended_at) > Date.parse(row.started_at))
    .map((row) => ({ id: row.id, start: row.started_at, end: row.ended_at!, label: 'Charging',
      details: `${row.location_name ?? 'Recorded session'} · ${formatKwh(row.energy_added_kwh)}`,
      color: 'var(--rm-chart-success)' })), [chargeRows]);
  const bars = tripRows.map((trip) => ({ trip, value: efficiencyBarValue(trip.efficiency_wh_mi, preferences) }));
  const barMaximum = Math.max(1, ...bars.map((bar) => bar.value ?? 0));
  const curveRows = curve.isPlaceholderData ? [] : curve.data ?? [];
  const elapsed = curveRows.length > 0 && curveRows.every((point) => finite(point.minutes_elapsed) !== null);
  const errors = [status, history, recentTrips, efficiency, charging, sessions, drain].filter((query) => query.isError);
  const anyUpdating = [history, recentTrips, efficiency, charging, sessions, drain].some((query) => query.isPlaceholderData);
  const stats = [
    { label: 'Distance', value: formatMiles(efficiencyData?.total_miles), Icon: Route, action: () => navigate({ to: '/trips' }), note: 'Trip distance in the selected period; not the odometer.' },
    { label: 'Efficiency', value: efficiencyData && efficiencyData.efficiency_miles > 0 && efficiencyData.avg > 0 ? formatEfficiency(efficiencyData.avg) : '—', Icon: Leaf, action: () => navigate({ to: '/efficiency' }), note: 'Upstream distance-weighted driving efficiency for trips with valid readings.' },
    { label: 'Charging energy', value: formatKwh(chargingData?.total_energy_kwh), Icon: Zap, action: () => navigate({ to: '/charging' }), note: 'Vehicle-reported session energy in the selected period; not wall-meter energy.' },
    { label: 'Parked drain', value: drainValue === null ? '—' : `${drainValue.toFixed(1)}% / day`, Icon: Clock, action: () => navigate({ to: '/battery/phantom-drain' }), note: 'Estimated SOC percentage-point loss ÷ observed parked hours × 24, using the existing parked-history service.' },
  ];
  return <div className="rm-r2-dashboard" data-testid="r2-overview" data-vehicle-id={id}>
    {errors.length > 0 ? <div className="rm-r2-notice" role="alert">Some readings could not be loaded. <button type="button" onClick={() => errors.forEach((query) => { void query.refetch(); })}>Try again</button></div> : null}
    {anyUpdating ? <p className="rm-r2-updating" role="status">Updating the selected period…</p> : null}
    <section className="rm-r2-hero" aria-label="Selected vehicle readings">
      <div className="rm-r2-vehicle-art"><PersonalR2Artwork /></div>
      <div className="rm-r2-status">
        <div className="rm-r2-state"><span className={readingState.includes('Stale') || readingState.includes('required') ? 'warning' : ''} /><div><strong>{readingState}</strong><small>{updated ? `As of ${formatAppDateTime(updated)}` : 'Awaiting a vehicle update'}</small></div></div>
        <dl><div><dt>Battery</dt><dd>{finite(status.data?.battery_level) === null ? '—' : `${Math.round(status.data!.battery_level!)}%`}</dd></div>
          <div><dt>Estimated range</dt><dd>{formatMiles(status.data?.range_miles)}</dd></div>
          <div><dt>Charge limit</dt><dd>{finite(status.data?.battery_limit) === null ? '—' : `${Math.round(status.data!.battery_limit!)}%`}</dd></div></dl>
      </div>
      <figure className="rm-r2-scenic"><svg viewBox="1071 85 433 243" role="img" aria-label="Mountain lake. Good to see you. Another day of further places."><image href="/r2-dashboard/approved-reference-v0.1.0.png" width="1536" height="1024" /></svg></figure>
    </section>
    <div className="rm-r2-stats">{stats.map(({label,value,Icon,action,note}) => <button key={label} type="button" onClick={action} title={note} className="rm-r2-stat"><Icon aria-hidden="true"/><span><span>{label}</span><strong>{value === '-' ? '—' : value}</strong><small>Selected period</small></span></button>)}</div>
    <div className="rm-r2-chart-grid">
      <ChartSection title="Battery & charging" className="rm-r2-panel" actions={<span className="rm-r2-legend"><i/>State of charge <b/>Charging</span>}>
        <QueryMessage query={history} empty={soc.length === 0} emptyText="No battery history in this period">
          <RichTimeSeriesChart rendererId="r2-overview-soc" points={soc} series={[{key:'soc',label:'Battery state of charge',values:soc.map((point) => point.value),color:'var(--rm-chart-success)',mode:'line',strokeWidth:2}]}
            height={240} showLegend={false} showPoints={soc.length < 40} connectGaps={false} smoothness="straight" timeFilter="raw" yUnit="%" yRange={[0,100]} intervals={intervals}
            onIntervalClick={(interval) => { void navigate({to:'/charging/$sessionId',params:{sessionId:interval.id}}); }}
            yValueFormatter={(value) => value == null ? 'Unavailable' : `${value.toFixed(0)}%`} />
        </QueryMessage><p className="rm-r2-unit">Recorded samples · Gaps over 6 hours are left blank</p>
      </ChartSection>
      <ChartSection title="Efficiency by trip" className="rm-r2-panel">
        <QueryMessage query={recentTrips} empty={tripRows.length === 0} emptyText="No trips in this period">
          <div className="rm-r2-bars" role="group" aria-label={`Recent trip efficiency, ${getEfficiencyUnitLabel()}`}>
            {bars.map(({trip,value}) => <button key={trip.id} type="button" className="rm-r2-bar" onClick={() => { void navigate({to:'/trips/$tripId',params:{tripId:trip.id}}); }} title={`${tripLabel(trip)} · ${formatEfficiency(trip.efficiency_wh_mi)}`}>
              <span className="rm-r2-bar-space"><strong style={{bottom:`calc(${value === null ? 0 : Math.max(3,value/barMaximum*80)}% + 8px)`}}>{value === null ? '—' : formatEfficiencyValue(trip.efficiency_wh_mi)}</strong><i style={{height:value === null ? 0 : `${Math.max(3,value/barMaximum*80)}%`}}/></span><span className="rm-r2-bar-label">{formatAppDate(trip.started_at,{month:'short',day:'numeric'})}</span>
            </button>)}
          </div><p className="rm-r2-unit">{getEfficiencyUnitLabel()} · Select a trip for details</p>
        </QueryMessage>
      </ChartSection>
    </div>
    <div className="rm-r2-bottom-grid">
      <ChartSection title="Recent trips" className="rm-r2-panel" actions={<button className="rm-r2-arrow" type="button" aria-label="View all trips" onClick={() => { void navigate({to:'/trips'}); }}><ArrowRight size={18}/></button>}>
        <QueryMessage query={recentTrips} empty={tripRows.length === 0} emptyText="No recorded trips in this period">
          <div className="rm-r2-table-scroll"><table><thead><tr><th>Trip</th><th>Distance</th><th>Duration</th><th>Efficiency</th></tr></thead><tbody>{tripRows.map((trip) => <tr key={trip.id}><td><button type="button" title={tripLabel(trip)} onClick={() => { void navigate({to:'/trips/$tripId',params:{tripId:trip.id}}); }}>{tripLabel(trip)}</button></td><td>{formatMiles(trip.distance_mi)}</td><td>{formatDuration(trip.duration_min)}</td><td>{formatEfficiency(trip.efficiency_wh_mi)}</td></tr>)}</tbody></table></div>
        </QueryMessage>
      </ChartSection>
      <ChartSection title="Last charge" className="rm-r2-panel">
        <QueryMessage query={sessions} empty={!lastCharge} emptyText="No charging session in this period">
          {lastCharge ? <div className="rm-r2-last-charge"><button type="button" className="rm-r2-charge-info" aria-label="Open last charge" onClick={() => { void navigate({to:'/charging/$sessionId',params:{sessionId:lastCharge.id}}); }}>
            <span className="rm-r2-charge-place"><Home size={19}/>{lastCharge.location_name ?? 'Recorded session'}{lastCharge.charger_type ? ` · ${lastCharge.charger_type.toUpperCase()}` : ''}</span>
            <small>Energy added</small><strong>{formatKwh(lastCharge.energy_added_kwh)}</strong>
            <span className="rm-r2-charge-stats"><span><small>Duration</small>{formatDuration(lastCharge.duration_min)}</span><span><small>State of charge</small>{lastCharge.soc_start === null ? '—' : `${lastCharge.soc_start}%`} → {lastCharge.soc_end === null ? '—' : `${lastCharge.soc_end}%`}</span></span>
          </button><div className="rm-r2-power"><h3>Charge power</h3><QueryMessage query={curve} empty={curveRows.every((point) => chargePowerValue(point) === null)} emptyText="No recorded power samples">
            <RichTimeSeriesChart rendererId="r2-overview-charge-power" points={curveRows.map((point) => ({ts:elapsed ? point.minutes_elapsed! : point.soc_pct}))} series={[{key:'power',label:'Charge power',values:curveRows.map(chargePowerValue),color:'var(--rm-chart-success)',mode:'area'}]}
              height={190} xTime={false} xUnit={elapsed ? 'min' : '%'} xAxisLabel={elapsed ? 'Minutes elapsed' : 'State of charge'} yUnit="kW" showLegend={false} smoothness="straight" connectGaps={false}
              yValueFormatter={(value) => value == null ? 'Unavailable' : `${value.toFixed(1)} kW`} />
            {curveRows.some((point) => point.power_method === 'soc_delta') ? <p className="rm-r2-unit">Includes estimates from battery-percentage changes</p> : null}
          </QueryMessage></div></div> : null}
        </QueryMessage>
      </ChartSection>
    </div>
    <AssistantOverviewCard vehicleId={ctx.vehicleId!} />
    <footer className="rm-r2-footnote"><span>{demo ? 'Sample readings · Demo vehicle' : 'Selected vehicle · Reported readings'} · {getAppTimezone()}</span><span><Lock size={12}/>Private dashboard · R2 design v0.3.0</span></footer>
  </div>;
}

function QueryMessage({query,empty,emptyText,children}:{query:QueryState;empty:boolean;emptyText:string;children:React.ReactNode}) {
  if (query.isError) return <div className="rm-r2-empty" role="alert">Could not load this history.<button type="button" onClick={() => { void query.refetch(); }}>Try again</button></div>;
  if (query.isPending || query.isPlaceholderData) return <div className="rm-r2-empty" role="status">Loading selected history…</div>;
  if (empty) return <div className="rm-r2-empty">{emptyText}</div>;
  return <>{children}</>;
}
