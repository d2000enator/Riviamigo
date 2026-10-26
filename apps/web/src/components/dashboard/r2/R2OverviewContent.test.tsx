import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { useTrips, useChargeCurve, useChargeSessions, useSocHistory, useEfficiencySummary } from '@riviamigo/hooks';
import type { Vehicle } from '@riviamigo/types';
import type { WidgetCtx } from '@riviamigo/dashboards';

const fixture=vi.hoisted(() => ({
  navigate:vi.fn(),
  trip:{id:'trip-1',vehicle_id:'vehicle-1',started_at:'2026-10-05T12:00:00Z',ended_at:'2026-10-05T12:30:00Z',distance_mi:18,duration_min:30,energy_used_kwh:6,efficiency_wh_mi:333,start_place:'Home',end_place:'Work'},
  charge:{id:'charge-1',vehicle_id:'vehicle-1',started_at:'2026-10-05T05:00:00Z',ended_at:'2026-10-05T08:00:00Z',location_name:'Home',charger_type:'ac',energy_added_kwh:24,duration_min:180,soc_start:40,soc_end:70},
}));
const ready=vi.hoisted(() => ({isError:false,isPending:false,isPlaceholderData:false,refetch:vi.fn()}));
vi.mock('@tanstack/react-router',()=>({useNavigate:()=>fixture.navigate}));
vi.mock('@riviamigo/hooks',()=>({
  useCurrentVehicleStatus:vi.fn(()=>({...ready,data:{vehicle_id:'vehicle-1',battery_level:0,range_miles:200,battery_limit:80,last_updated:'2026-10-05T12:00:00Z'}})),
  useSocHistory:vi.fn(()=>({...ready,data:[{ts:'2026-10-05T10:00:00Z',value:60},{ts:'2026-10-05T11:00:00Z',value:null},{ts:'2026-10-05T12:00:00Z',value:50}]})),
  useTrips:vi.fn(()=>({...ready,data:{items:[fixture.trip],total:1}})),
  useEfficiencySummary:vi.fn(()=>({...ready,data:{total_miles:100,avg:250,efficiency_miles:100,coverage_percent:100}})),
  useChargingSummary:vi.fn(()=>({...ready,data:{total_energy_kwh:24}})),
  useChargeSessions:vi.fn(()=>({...ready,data:{items:[fixture.charge],total:1}})),
  useChargeCurve:vi.fn(()=>({...ready,data:[{minutes_elapsed:0,soc_pct:40,power_kw:7},{minutes_elapsed:60,soc_pct:50,power_kw:8}]})),
  usePhantomDrain:vi.fn(()=>({...ready,data:[{total_soc_lost:1,hours_parked:24}]})),
}));
vi.mock('@riviamigo/ui/primitives',()=>({ChartSection:({title,children,actions}:{title:string;children:React.ReactNode;actions?:React.ReactNode})=><section><h2>{title}</h2>{actions}{children}</section>}));
vi.mock('@riviamigo/ui/charts',()=>({RichTimeSeriesChart:({rendererId,series,connectGaps}:{rendererId:string;series:unknown;connectGaps:boolean})=><div data-testid={rendererId} data-series={JSON.stringify(series)} data-connect-gaps={String(connectGaps)}/>}));
vi.mock('../../../features/assistant/AssistantOverviewCard',()=>({AssistantOverviewCard:()=> <div data-testid="assistant-entry"/>}));
import { R2OverviewContent } from './R2OverviewContent';
const ctx={vehicleId:'vehicle-1',from:'2026-10-01T00:00:00Z',to:'2026-10-25T23:59:59Z'} as WidgetCtx;
const vehicle={id:'vehicle-1',model:'R2',is_demo:true,display_name:'Demo R2'} as Vehicle;
describe('authenticated R2 overview composition',()=>{
  beforeEach(()=>{vi.clearAllMocks();});
  it('uses the selected vehicle/range and genuine zero readings with a visible demo label',()=>{
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(useTrips).toHaveBeenCalledWith('vehicle-1',ctx.from,ctx.to,1,3);
    expect(useSocHistory).toHaveBeenCalledWith('vehicle-1',ctx.from,ctx.to);
    expect(screen.getByText('0%')).toBeInTheDocument();expect(screen.getByText('Demo snapshot')).toBeInTheDocument();
    expect(screen.getByText(/Sample readings · Demo vehicle/)).toBeInTheDocument();
    expect(screen.getByAltText('R2 illustration in Launch Green')).toHaveAttribute('src','/r2-dashboard/r2-launch-green-v0.1.0.png');
  });
  it('preserves null battery samples and disables gap connection',()=>{
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.getByTestId('r2-overview-soc')).toHaveAttribute('data-connect-gaps','false');
    expect(screen.getByTestId('r2-overview-soc').getAttribute('data-series')).toContain('60,null,50');
  });
  it('opens real existing trip and charge routes, without a new data/credentials service',()=>{
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    fireEvent.click(screen.getByText('Home → Work'));
    expect(fixture.navigate).toHaveBeenCalledWith({to:'/trips/$tripId',params:{tripId:'trip-1'}});
    fireEvent.click(screen.getByRole('button',{name:'Open last charge'}));
    expect(fixture.navigate).toHaveBeenCalledWith({to:'/charging/$sessionId',params:{sessionId:'charge-1'}});
  });
  it('does not relabel previous-range placeholder readings as the newly selected period',()=>{
    vi.mocked(useEfficiencySummary).mockReturnValueOnce({...ready,isPlaceholderData:true,data:{total_miles:999,avg:250,efficiency_miles:999}} as never);
    vi.mocked(useTrips).mockReturnValueOnce({...ready,isPlaceholderData:true,data:{items:[fixture.trip],total:1}} as never);
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.queryByText('999 mi')).not.toBeInTheDocument();expect(screen.queryByText('Home → Work')).not.toBeInTheDocument();
    expect(screen.getByText('Updating the selected period…')).toBeInTheDocument();
  });
  it('rejects a history row for another selected vehicle',()=>{
    vi.mocked(useTrips).mockReturnValueOnce({...ready,data:{items:[{...fixture.trip,vehicle_id:'other-vehicle'}],total:1}} as never);
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.queryByText('Home → Work')).not.toBeInTheDocument();expect(screen.getByText('No recorded trips in this period')).toBeInTheDocument();
  });
  it('renders current vehicle-scoped API list DTOs that omit row vehicle_id',()=>{
    // routes/trips.rs TripRow and routes/charging.rs SessionRow omit it;
    // normalizeTrip/normalizeChargeSession expose the missing field as ''.
    vi.mocked(useTrips).mockReturnValueOnce({...ready,data:{items:[{...fixture.trip,vehicle_id:''}],total:1}} as never);
    vi.mocked(useChargeSessions).mockReturnValueOnce({...ready,data:{items:[{...fixture.charge,vehicle_id:''}],total:1}} as never);
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.getByText('Home → Work')).toBeInTheDocument();
    expect(screen.getByRole('button',{name:'Open last charge'})).toBeInTheDocument();
    expect(screen.getByTestId('r2-overview-charge-power')).toBeInTheDocument();
  });
  it('rejects an explicitly conflicting charge row',()=>{
    vi.mocked(useChargeSessions).mockReturnValueOnce({...ready,data:{items:[{...fixture.charge,vehicle_id:'other-vehicle'}],total:1}} as never);
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.queryByRole('button',{name:'Open last charge'})).not.toBeInTheDocument();
    expect(screen.getByText('No charging session in this period')).toBeInTheDocument();
  });
  it('does not plot normalized missing charge power as measured zero',()=>{
    vi.mocked(useChargeCurve).mockReturnValueOnce({...ready,data:[{minutes_elapsed:0,soc_pct:40,power_kw:0},{minutes_elapsed:60,soc_pct:50,power_kw:0}]} as never);
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.getByText('No recorded power samples')).toBeInTheDocument();
    expect(screen.queryByTestId('r2-overview-charge-power')).not.toBeInTheDocument();
  });
  it('retains measured zero power and identifies SOC-derived estimates',()=>{
    vi.mocked(useChargeCurve).mockReturnValueOnce({...ready,data:[{minutes_elapsed:0,soc_pct:40,power_kw:0,power_method:'recorded'},{minutes_elapsed:60,soc_pct:50,power_kw:8,power_method:'soc_delta'}]} as never);
    render(<R2OverviewContent ctx={ctx} vehicle={vehicle}/>);
    expect(screen.getByTestId('r2-overview-charge-power').getAttribute('data-series')).toContain('0,8');
    expect(screen.getByText('Includes estimates from battery-percentage changes')).toBeInTheDocument();
  });
});
