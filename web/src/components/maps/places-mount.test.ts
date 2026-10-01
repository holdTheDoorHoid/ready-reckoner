import axe from 'axe-core';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { FIXTURES } from '../../engine/fixtures';
import { RECIPIENTS } from '../../lib/maps/sources';
import { emptyMapsState } from '../../lib/maps/state';
import Places from '../../screens/Places.svelte';
import { render, savedFor, until, type Rendered } from '../../test/helpers';

let current: Rendered | null = null;

afterEach(() => {
  current?.cleanup();
  current = null;
  vi.restoreAllMocks();
});

const LABEL = 'Set your home point and meeting places on a map';

function button(r: Rendered, label: string): HTMLButtonElement | undefined {
  return [...r.target.querySelectorAll('button')].find((b) => b.textContent?.trim() === label);
}

describe('Step 7, Your places: the pin map button in the Getting out card (DESIGN-DELTA-v3 §2.2, §9.4)', () => {
  it('offers the pin map, opens the short consent screen first, and fetches nothing until it is accepted', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    current = await render(Places, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'places' });
    await until(() => !!button(current!, LABEL), 'the pin map button');
    const leave = current.target.querySelector('#places-leave');
    expect(leave?.contains(button(current, LABEL)!)).toBe(true);
    expect(current.text()).toContain('Nothing placed yet');
    button(current, LABEL)!.click();
    flushSync();
    await until(() => current!.text().includes('Before the map opens'), 'the consent screen');
    expect(current.text()).toContain(RECIPIENTS[0]!.who);
    const results = await axe.run(current.target, { rules: { 'color-contrast': { enabled: false }, region: { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
    button(current, 'Not now')!.click();
    flushSync();
    await until(() => !!button(current!, LABEL), 'the button again');
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('says what is already placed, from the saved plan’s maps', async () => {
    const plan = savedFor(FIXTURES['philadelphia-renters-4']);
    plan.maps = { ...emptyMapsState(), home: { lat: 39.9366, lon: -75.1479 }, where_go: { lat: 40.0379, lon: -76.3055 }, routes: [[{ lat: 39.9366, lon: -75.1479 }, { lat: 40.0379, lon: -76.3055 }]] };
    current = await render(Places, { plan, route: 'places' });
    await until(() => !!button(current!, LABEL), 'the pin map button');
    expect(current.text()).toContain('Placed: H home, D where you would go, 1 way out.');
  });
});
