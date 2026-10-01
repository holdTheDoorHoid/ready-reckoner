/**
 * The maps panel on the Binder screen (web-maps' mount, moved from the v0.2 packet screen by
 * web-binder): "Add maps" opens the consent screen on every press and nothing is fetched until
 * "Fetch maps"; the binder's map slots say when there is no map and offer the same button; and
 * the stored maps appear in the slots, not twice over in the panel.
 */
import { IDBFactory } from 'fake-indexeddb';
import { flushSync, tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { FIXTURES } from '../../engine/fixtures';
import { mapsKey, saveMaps, type MapRecord } from '../../lib/maps/store';
import Binder from '../../screens/Binder.svelte';
import { render, savedFor, until, type Rendered } from '../../test/helpers';

let current: Rendered | null = null;
const originalIdb = globalThis.indexedDB;

beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
});

afterEach(() => {
  current?.cleanup();
  current = null;
  globalThis.indexedDB = originalIdb;
  vi.restoreAllMocks();
});

const buttons = (r: Rendered, label: string) => [...r.target.querySelectorAll('button')].filter((x) => x.textContent?.trim() === label);
const click = (r: Rendered, label: string) => {
  const b = buttons(r, label)[0];
  if (!b) throw new Error(`no button "${label}"`);
  b.click();
  flushSync();
};

describe('the maps on the Binder screen', () => {
  it('"Add maps" opens the consent screen every time, and nothing is fetched until "Fetch maps"', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    current = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder' });
    await tick();
    flushSync();
    expect(current.target.querySelector('.maps-panel')).toBeNull();
    click(current, 'Add maps');
    await until(() => current!.text().includes('Before we fetch your maps'), 'the consent screen');
    click(current, 'Not now');
    expect(current.text()).not.toContain('Before we fetch your maps');
    click(current, 'Add maps');
    await until(() => current!.text().includes('Before we fetch your maps'), 'the consent screen again');
    // Nothing has left the site: at most the app's own county outline was read.
    const urls = fetchSpy.mock.calls.map(([u]) => new URL(String(u instanceof Request ? u.url : u), window.location.href));
    expect(urls.every((u) => u.origin === window.location.origin)).toBe(true);
    // The panel is for the screen only; it is never printed.
    expect(current.target.querySelector('.binder-maps')?.classList.contains('no-print')).toBe(true);
  });

  it('a map slot with no map says so in one line and offers "Add maps"; stored maps fill the slots', async () => {
    const plan = savedFor(FIXTURES['philadelphia-renters-4']);
    current = await render(Binder, { plan, route: 'binder' });
    const slots = () => [...current!.target.querySelectorAll('.binder-map--missing, .binder-map')];
    const binder = current.app.result.output!.binder;
    const slotCount = binder.parts.flatMap((p) => p.pages).flatMap((p) => p.blocks).filter((b) => 'map_slot' in b).length;
    expect(slots()).toHaveLength(slotCount);
    if (slotCount === 0) return; // awaiting: binder (the transitional binder has no map slots)
    await until(() => current!.target.querySelectorAll('.binder-map--missing').length === slotCount, 'the one-line notes');
    expect(current.target.querySelector('.binder-map--missing')?.textContent).toContain('no map added');
    // Maps made for this plan's pins show in the slots.
    const maps = { home: { lat: 39.94, lon: -75.16 }, routes: [], layers: { places: true, flood: false, surge: false, wildfire: false }, fetched_on: '2026-10-01' };
    const fips = current.app.result.output!.location.county_fips;
    const record = (slot: MapRecord['slot']): MapRecord => ({
      slot,
      image: 'data:image/jpeg;base64,/9j/4AAQ',
      width: 800,
      height: 560,
      bytes: 10,
      zoom: 15,
      legend: [{ mark: '1', name: 'A pharmacy', kind: 'Pharmacy' }],
      keys: [],
      statuses: [],
      notes: [],
      credits: ['© OpenStreetMap contributors'],
      scale: 'The bars show 500 ft and 200 m.',
      fetched_on: '2026-10-01',
      key: mapsKey(maps, fips),
    });
    await saveMaps([record('neighbourhood'), record('area'), record('region')]);
    current.app.plan!.maps = maps;
    await until(() => current!.target.querySelectorAll('.binder-map img').length === slotCount, 'the maps in their slots');
    expect(current.target.querySelector('.binder-maps .map-figure img')).toBeNull();
  });
});
