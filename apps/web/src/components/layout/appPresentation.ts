import type { ReactNode } from 'react';

export const collapsedFooterRow = '-mx-1 grid w-[calc(100%+0.5rem)] grid-cols-[24px_24px] items-center justify-between';
export const collapsedFooterCell = 'flex h-8 w-6 items-center justify-center';

export interface AppLayoutProps {
  children: ReactNode;
  activeKey: string;
  /** A scoped presentation; never writes account theme or navigation settings. */
  presentation?: 'standard' | 'r2';
}

/** Keep presentation attributes outside the shared auth/navigation orchestrator. */
export function layoutPresentation(presentation: 'standard' | 'r2', unitSystem: string, collapsed: boolean) {
  return {
    className: `min-h-screen bg-bg-page text-fg${presentation === 'r2' ? ' rm-r2-app' : ''}`,
    'data-r2-presentation': presentation === 'r2' ? 'true' : undefined,
    'data-r2-sidebar-collapsed': presentation === 'r2' ? collapsed ? 'true' : 'false' : undefined,
    'data-unit-system': unitSystem,
  };
}
