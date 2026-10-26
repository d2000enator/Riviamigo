import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';

const fixture=vi.hoisted(()=>({userId:'owner-1',vehicleId:'r2-1',model:'R2'}));
vi.mock('@riviamigo/hooks',()=>({
  useMe:()=>({data:{user_id:fixture.userId}}),
  useResolvedVehicleSelection:()=>({effectiveVehicleId:fixture.vehicleId,vehicles:[{id:fixture.vehicleId,model:fixture.model,is_demo:true}]}),
}));
vi.mock('../DashboardPage',()=>({DashboardPage:()=> <div>Saved classic dashboard</div>}));
vi.mock('../DashboardPageShell',()=>({DashboardPageShell:({renderTitleAction,renderActions,renderBeforeDashboard}:{renderTitleAction:()=>React.ReactNode;renderActions:()=>React.ReactNode;renderBeforeDashboard:(value:unknown)=>React.ReactNode})=><div>{renderTitleAction()}{renderActions()}{renderBeforeDashboard({vehicleId:fixture.vehicleId,ctx:{vehicleId:fixture.vehicleId}})}</div>}));
vi.mock('./R2OverviewContent',()=>({R2OverviewContent:({ctx}:{ctx:{vehicleId:string}})=><div data-testid="r2-body">{ctx.vehicleId}</div>}));
import { OverviewDashboardPage } from '../OverviewDashboardPage';
describe('R2 overview selection and Classic escape',()=>{
  beforeEach(()=>{localStorage.clear();fixture.userId='owner-1';fixture.vehicleId='r2-1';fixture.model='R2';});
  it('opens the actual R2 composition and labels demo data',()=>{
    render(<OverviewDashboardPage navKey="dashboard" slug="dashboard"/>);
    expect(screen.getByTestId('r2-body')).toHaveTextContent('r2-1');expect(screen.getByText('DEMO · SAMPLE DATA')).toBeInTheDocument();
  });
  it('preserves Classic and scopes the browser preference to this account',()=>{
    const result=render(<OverviewDashboardPage navKey="dashboard" slug="dashboard"/>);
    fireEvent.click(screen.getByRole('button',{name:'Classic overview'}));
    expect(screen.getByText('Saved classic dashboard')).toBeInTheDocument();
    expect(localStorage.getItem('rm-r2-overview-v020:owner-1')).toBe('classic');
    fixture.userId='owner-2';result.rerender(<OverviewDashboardPage navKey="dashboard" slug="dashboard"/>);
    expect(screen.getByTestId('r2-body')).toBeInTheDocument();
  });
  it('keeps existing non-R2 dashboards on their existing renderer',()=>{
    fixture.model='R1T';render(<OverviewDashboardPage navKey="dashboard" slug="dashboard"/>);
    expect(screen.getByText('Saved classic dashboard')).toBeInTheDocument();expect(screen.queryByTestId('r2-body')).not.toBeInTheDocument();
  });
});
