import React, { useEffect, useState } from 'react';
import { useMe, useResolvedVehicleSelection } from '@riviamigo/hooks';
import { DashboardPage, type DashboardPageProps } from './DashboardPage';
import { DashboardPageShell } from './DashboardPageShell';
import { R2OverviewContent } from './r2/R2OverviewContent';
import { isPersonalR2Model } from '../PersonalR2Artwork';

export function OverviewDashboardPage({ navKey, slug, title }: DashboardPageProps) {
  const me = useMe();
  const selection = useResolvedVehicleSelection();
  const activeVehicle = selection.vehicles.find((vehicle) => vehicle.id === selection.effectiveVehicleId);
  const isR2 = isPersonalR2Model(activeVehicle?.model);
  const userId = me.data?.user_id;
  const preferenceKey = userId ? `rm-r2-overview-v020:${userId}` : null;
  const [classic, setClassic] = useState(false);
  useEffect(() => {
    try { setClassic(preferenceKey ? localStorage.getItem(preferenceKey) === 'classic' : false); }
    catch { setClassic(false); }
  }, [preferenceKey]);
  function changePresentation(nextClassic: boolean) {
    setClassic(nextClassic);
    try { if (preferenceKey) localStorage.setItem(preferenceKey, nextClassic ? 'classic' : 'r2'); } catch { /* A blocked browser store must not break navigation. */ }
  }
  if (isR2 && !classic) {
    return <DashboardPageShell
      navKey={navKey} slug={slug} title="My R2" presentation="r2"
      enableDashboardEditing={false} showEfficiencyDisplayToggle
      renderTitleAction={() => <span className="rm-r2-badge">{activeVehicle?.is_demo ? 'DEMO · SAMPLE DATA' : 'PRIVATE DASHBOARD'}</span>}
      renderActions={() => <button type="button" className="rm-r2-view-switch" onClick={() => changePresentation(true)}>Classic overview</button>}
      renderBeforeDashboard={(state) => <R2OverviewContent key={state.vehicleId} ctx={state.ctx} vehicle={activeVehicle} />}
      renderDashboard={() => false}
    />;
  }
  return (
    <>
      {isR2 && classic ? <button type="button" className="fixed bottom-4 right-4 z-50 rounded-lg border border-border bg-bg-surface px-3 py-2 text-sm text-fg shadow-lg" onClick={() => changePresentation(false)}>Use R2 dashboard</button> : null}
      <DashboardPage navKey={navKey} slug={slug} title={title} showEfficiencyDisplayToggle />
    </>
  );
}
