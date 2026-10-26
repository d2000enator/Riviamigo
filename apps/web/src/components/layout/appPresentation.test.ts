import { describe, expect, it } from 'vitest';
import { layoutPresentation } from './appPresentation';

describe('scoped layout presentation',()=>{
  it('leaves standard layout classes and theme scope unchanged',()=>{
    const attributes=layoutPresentation('standard','metric',false);
    expect(attributes.className).toBe('min-h-screen bg-bg-page text-fg');
    expect(attributes['data-r2-presentation']).toBeUndefined();
    expect(attributes['data-r2-sidebar-collapsed']).toBeUndefined();
    expect(attributes['data-unit-system']).toBe('metric');
  });
  it('scopes the R2 layout and preserves sidebar collapse state',()=>{
    expect(layoutPresentation('r2','imperial',false)['data-r2-sidebar-collapsed']).toBe('false');
    expect(layoutPresentation('r2','imperial',true)['data-r2-sidebar-collapsed']).toBe('true');
    expect(layoutPresentation('r2','imperial',true)['data-r2-presentation']).toBe('true');
  });
});
