import { afterEach, describe, expect, it } from 'vitest';

import { formatHash, parseHash, ROUTE_IDS, ROUTES, Router, screenFor, stepAfter, stepBefore } from './router.svelte';

describe('hash routes', () => {
  it('round-trips every route', () => {
    for (const id of ROUTE_IDS) {
      const route = { id };
      expect(parseHash(formatHash(route))).toEqual(id === 'missing' ? { id: 'missing' } : route);
    }
    expect(formatHash({ id: 'learn', param: 'myths' })).toBe('#/learn/myths');
    expect(parseHash('#/learn/myths')).toEqual({ id: 'learn', param: 'myths' });
  });

  it('treats the empty hash and #/start as the start screen', () => {
    for (const h of ['', '#', '#/', '#/start', '#/start/']) expect(parseHash(h)).toEqual({ id: 'start' });
  });

  it('sends unknown paths, and extra path parts, to "page not found"', () => {
    expect(parseHash('#/nowhere')).toEqual({ id: 'missing' });
    expect(parseHash('#/prepare/extra')).toEqual({ id: 'missing' });
    expect(parseHash('#/%E0%A4%A')).toEqual({ id: 'missing' });
  });

  it('leaves in-page anchors alone', () => {
    expect(parseHash('#main')).toBeNull();
    expect(parseHash('#pk-fn-mock_nri_county')).toBeNull();
  });

  it('numbers the eight interview steps, the last three optional, and links them in order', () => {
    const steps = ['where', 'who', 'travel', 'money', 'have', 'people', 'places', 'contacts'] as const;
    expect(steps.map((s) => ROUTES[s].step)).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(steps.map((s) => ROUTES[s].optional ?? false)).toEqual([false, false, false, false, false, true, true, true]);
    expect(stepAfter('where')).toBe('who');
    expect(stepAfter('have')).toBe('people');
    expect(stepAfter('contacts')).toBe('risks');
    expect(stepBefore('where')).toBe('start');
    expect(stepBefore('money')).toBe('travel');
    expect(stepBefore('people')).toBe('have');
  });

  it('points engine problems at the screen where the answer lives', () => {
    expect(screenFor('location.zip')).toBe('where');
    expect(screenFor('housing.floor')).toBe('where');
    expect(screenFor('housing.alarms.co')).toBe('have');
    expect(screenFor('people[1].commute.distance_km')).toBe('travel');
    expect(screenFor('people[0].medical.powered_device.other.watts')).toBe('who');
    expect(screenFor('pets.dogs')).toBe('who');
    expect(screenFor('mobility.vehicles[0].fuel')).toBe('travel');
    expect(screenFor('finances.monthly_budget_usd')).toBe('money');
    expect(screenFor('existing[2].qty')).toBe('have');
    expect(screenFor('dials.horizon_years')).toBe('risks');
  });
});

describe('Router', () => {
  let router: Router | undefined;
  afterEach(() => router?.destroy());

  it('follows the address bar and navigates', async () => {
    window.location.hash = '#/money';
    router = new Router(window);
    expect(router.current).toEqual({ id: 'money' });
    router.go('learn', 'myths');
    expect(router.current).toEqual({ id: 'learn', param: 'myths' });
    expect(window.location.hash).toBe('#/learn/myths');
    window.location.hash = '#/about';
    await new Promise((r) => setTimeout(r, 0));
    expect(router.current).toEqual({ id: 'about' });
    window.location.hash = '#top';
    await new Promise((r) => setTimeout(r, 0));
    expect(router.current).toEqual({ id: 'about' });
  });
});
