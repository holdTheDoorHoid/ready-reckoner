import { describe, expect, it } from 'vitest';

import { formatHash, href, parseHash, ROUTES, screenFor } from './router.svelte';

describe('the family plan and packet addresses (v0.2.0)', () => {
  it('has a family-plan screen, outside the numbered interview steps', () => {
    expect(parseHash('#/family')).toEqual({ id: 'family' });
    expect(ROUTES.family.title).toBe('Your family plan');
    expect(ROUTES.family.step).toBeUndefined();
    expect(href('family')).toBe('#/family');
  });

  it('opens the family plan at one part, and the packet at one section', () => {
    expect(parseHash('#/family/circle')).toEqual({ id: 'family', param: 'circle' });
    expect(formatHash({ id: 'family', param: 'lawyer' })).toBe('#/family/lawyer');
    expect(parseHash('#/packet/wallet-cards')).toEqual({ id: 'packet', param: 'wallet-cards' });
    expect(href('packet', 'wallet-cards')).toBe('#/packet/wallet-cards');
    // Other screens still take no second part.
    expect(parseHash('#/risks/extra')).toEqual({ id: 'missing' });
  });

  it('sends a problem in the family plan to its screen', () => {
    expect(screenFor('family_plan.routes')).toBe('family');
    expect(screenFor('people[2].access_needs[0]')).toBe('who');
    expect(screenFor('housing.cooking')).toBe('where');
    expect(screenFor('finances.benefits[0]')).toBe('money');
    expect(screenFor('dials.rare_opt_in[1]')).toBe('risks');
  });
});
