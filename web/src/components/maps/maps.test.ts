import axe from 'axe-core';
import { IDBFactory } from 'fake-indexeddb';
import { flushSync, mount, tick, unmount, type Component } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { until } from '../../test/helpers';
import fixture from '../../lib/maps/fixtures/overpass-philadelphia.json';
import type { MapLocation } from '../../lib/maps/slots';
import { MAP_ORIGINS, RECIPIENTS, SURGE_NOTE } from '../../lib/maps/sources';
import { emptyMapsState, type MapsState } from '../../lib/maps/state';
import type { MapRecord } from '../../lib/maps/store';
import { fakeEnv, imageResponse, jsonResponse, type Route } from '../../lib/maps/testing';
import type { CreateView, ViewOptions, ViewPin } from '../../lib/maps/view';
import { AddressSearch as Search } from '../../lib/maps/nominatim';
import AddressSearch from './AddressSearch.svelte';
import MapFigure from './MapFigure.svelte';
import MapsConsent from './MapsConsent.svelte';
import MapsPanel from './MapsPanel.svelte';
import PinMap from './PinMap.svelte';
import PinMapButton from './PinMapButton.svelte';

const PHILLY: MapLocation = { county_fips: '42101', county_name: 'Philadelphia', state_abbr: 'PA', state_name: 'Pennsylvania', centroid: { lat: 40.0094, lon: -75.1333 }, zip_centroid: { lat: 39.9364, lon: -75.1534 } };
const HOME = { lat: 39.9364, lon: -75.1534 };

const allGood: Route = (url) => {
  const host = new URL(url).host;
  if (host === 'tile.openstreetmap.org') return imageResponse('tile');
  if (host.startsWith('overpass')) return jsonResponse(fixture);
  if (host === 'hazards.fema.gov') return imageResponse('flood');
  if (host === 'imagery.geoplatform.gov') return imageResponse('wildfire');
  if (host === 'tigerweb.geo.census.gov') return imageResponse('census');
  return new Error(`unexpected ${url}`);
};

/** A map library stand-in: remembers what it was told and lets a test click or drag. */
function stubView(center = HOME) {
  const state = { opts: null as ViewOptions | null, pins: [] as ViewPin[], routes: [] as (readonly { lat: number; lon: number }[])[], center, destroyed: false, views: [] as { lat: number; lon: number }[] };
  const create: CreateView = async (_el, opts) => {
    state.opts = opts;
    return {
      setPins: (p) => (state.pins = [...p]),
      setRoutes: (r) => (state.routes = r.map((x) => [...x])),
      getCenter: () => state.center,
      setView: (c) => {
        state.center = c;
        state.views.push(c);
      },
      destroy: () => (state.destroyed = true),
    };
  };
  return { create, state };
}

interface Mounted {
  target: HTMLElement;
  text(): string;
  click(label: string | RegExp): void;
  button(label: string | RegExp): HTMLButtonElement | undefined;
  cleanup(): void;
}

function render<P extends Record<string, unknown>>(C: Component<P>, props: P): Mounted {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(C, { target, props });
  flushSync();
  const button = (label: string | RegExp) =>
    [...target.querySelectorAll('button')].find((b) => (typeof label === 'string' ? b.textContent?.trim() === label : label.test(b.textContent ?? ''))) as HTMLButtonElement | undefined;
  return {
    target,
    text: () => target.textContent?.replace(/\s+/g, ' ') ?? '',
    button,
    click(label) {
      const b = button(label);
      if (!b) throw new Error(`no button "${label}" in: ${target.textContent?.replace(/\s+/g, ' ').slice(0, 400)}`);
      b.click();
      flushSync();
    },
    cleanup() {
      unmount(component);
      target.remove();
    },
  };
}

async function noAxeViolations(target: HTMLElement, what: string) {
  const results = await axe.run(target, { rules: { 'color-contrast': { enabled: false }, region: { enabled: false } } });
  expect(
    results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).slice(0, 3).join(', ')}`),
    what,
  ).toEqual([]);
}

const originalIdb = globalThis.indexedDB;
let mounted: Mounted[] = [];
beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
  mounted = [];
});
afterEach(() => {
  for (const m of mounted) m.cleanup();
  globalThis.indexedDB = originalIdb;
  vi.restoreAllMocks();
});

function keep(m: Mounted): Mounted {
  mounted.push(m);
  return m;
}

// ---------------------------------------------------------------------------------------------

describe('The consent screen', () => {
  it('names every recipient with what it receives, ticks the street map and places, and the layers the rule suggests', async () => {
    const onfetch = vi.fn();
    const m = keep(render(MapsConsent, { suggested: { flood: true, surge: true, wildfire: false }, onfetch, oncancel: vi.fn() }));
    for (const r of RECIPIENTS) {
      expect(m.text()).toContain(r.layer);
      expect(m.text()).toContain(r.who);
    }
    const boxes = [...m.target.querySelectorAll<HTMLInputElement>('input[type=checkbox]')];
    expect(boxes.map((b) => b.checked)).toEqual([true, true, true, false]);
    expect(m.text()).toContain(SURGE_NOTE);
    expect(m.text()).toContain('internet (IP) address');
    expect(m.text()).toContain('Nothing has been sent yet');
    await noAxeViolations(m.target, 'the consent screen');
    boxes[1]!.click(); // untick places
    boxes[3]!.click(); // tick wildfire
    flushSync();
    m.click('Fetch maps');
    expect(onfetch).toHaveBeenCalledWith({ base: true, places: false, flood: true, surge: false, wildfire: true });
  });

  it('warns, never blocks: without the street map it says what the maps will lack, and still fetches', () => {
    const onfetch = vi.fn();
    const m = keep(render(MapsConsent, { suggested: { flood: false, surge: false, wildfire: false }, offline: true, onfetch, oncancel: vi.fn() }));
    m.target.querySelector<HTMLInputElement>('input[type=checkbox]')!.click();
    flushSync();
    expect(m.text()).toContain('Without the street map there is nothing to place your pins on');
    expect(m.text()).toContain('seems to be offline');
    m.click('Fetch maps');
    expect(onfetch).toHaveBeenCalledWith({ base: false, places: true, flood: false, surge: false, wildfire: false });
  });

  it('for the pin map alone, names only the street map and has no boxes', async () => {
    const onfetch = vi.fn();
    const oncancel = vi.fn();
    const m = keep(render(MapsConsent, { mode: 'pins', suggested: { flood: true, surge: true, wildfire: true }, onfetch, oncancel }));
    expect(m.target.querySelectorAll('input[type=checkbox]')).toHaveLength(0);
    expect(m.text()).toContain(RECIPIENTS[0]!.who);
    expect(m.text()).not.toContain('FEMA');
    await noAxeViolations(m.target, 'the pin-map consent screen');
    m.click('Not now');
    expect(oncancel).toHaveBeenCalled();
    m.click('Show the map');
    expect(onfetch).toHaveBeenCalledWith({ base: true, places: false, flood: false, surge: false, wildfire: false });
  });
});

// ---------------------------------------------------------------------------------------------

function panelProps(overrides: Record<string, unknown> = {}) {
  const fake = fakeEnv(allGood);
  const view = stubView();
  const changes: (MapsState | undefined)[] = [];
  const props = {
    location: PHILLY,
    suggested: { flood: true, surge: true, wildfire: false },
    children: true,
    maps: undefined as MapsState | undefined,
    onchange: (next: MapsState | undefined) => changes.push(next),
    today: () => '2026-10-01',
    loadCounty: async () => null,
    env: fake.env,
    createView: view.create,
    ...overrides,
  };
  return { props, fake, view, changes };
}

describe('The maps panel: nothing is fetched before "Fetch maps", on every press', () => {
  it('opens the consent screen on "Add maps", fetches nothing on "Not now", and asks again next time', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const { props, fake } = panelProps();
    const m = keep(render(MapsPanel, props));
    await until(() => m.target.querySelector('[data-maps-status="none"]') !== null, 'the store read');
    m.click('Add maps');
    expect(m.text()).toContain('Before we fetch your maps');
    m.click('Not now');
    expect(m.text()).not.toContain('Before we fetch your maps');
    m.click('Add maps');
    expect(m.text()).toContain('Before we fetch your maps');
    expect(fake.requests).toEqual([]);
    expect(fetchSpy).not.toHaveBeenCalled();
    await noAxeViolations(m.target, 'the panel with its consent screen');
  });

  it('first time: consent, then the pin map, then the three maps, asking only the listed origins', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const { props, fake, view, changes } = panelProps();
    const m = keep(render(MapsPanel, props));
    await until(() => m.target.querySelector('[data-maps-status="none"]') !== null, 'the store read');
    m.click('Add maps');
    m.click('Fetch maps');
    await until(() => view.state.opts !== null, 'the pin map');
    expect(m.text()).toContain('Place your pins and ways out');
    // The pin map is the stub: still nothing composed.
    expect(fake.requests).toEqual([]);
    view.state.opts!.onClick(HOME);
    flushSync();
    m.click('Done');
    await until(() => m.target.querySelector('[data-maps-phase="idle"] img') !== null, 'the maps', 5000);
    expect(fake.requests.length).toBeGreaterThan(0);
    for (const r of fake.requests) expect(MAP_ORIGINS).toContain(new URL(r.url).origin);
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(m.target.querySelectorAll('figure img')).toHaveLength(3);
    expect(changes.at(-1)).toMatchObject({ home: HOME, fetched_on: '2026-10-01', layers: { base: true, places: true, flood: true, surge: false, wildfire: false } });
    expect(m.text()).toContain('Your maps are ready (fetched October 1, 2026)');
    expect(m.text()).toContain('Rite Aid');
    await noAxeViolations(m.target, 'the panel with its maps');
  });

  it('refresh: the consent screen again (nothing remembered), then straight to the maps when the home pin is placed', async () => {
    const maps: MapsState = { ...emptyMapsState(), home: HOME };
    const { props, fake, view } = panelProps({ maps });
    const m = keep(render(MapsPanel, props));
    await until(() => m.target.querySelector('[data-maps-status="none"]') !== null, 'the store read');
    m.click('Add maps');
    // Every press starts from the defaults: flood suggested, wildfire not.
    expect([...m.target.querySelectorAll<HTMLInputElement>('.consent input[type=checkbox]')].map((b) => b.checked)).toEqual([true, true, true, false]);
    m.click('Fetch maps');
    await until(() => m.target.querySelector('[data-maps-phase="idle"] img') !== null, 'the maps', 5000);
    expect(view.state.opts).toBeNull();
    const before = fake.requests.length;
    m.click('Refresh maps');
    expect(m.text()).toContain('Before we fetch your maps');
    expect(fake.requests.length).toBe(before);
    m.click('Not now');
    expect(fake.requests.length).toBe(before);
  });

  it('a press where nothing answers saves nothing and leaves the placeholders', async () => {
    const dead = fakeEnv(() => new Error('offline'));
    const { props, changes } = panelProps({ maps: { ...emptyMapsState(), home: HOME }, env: dead.env });
    const m = keep(render(MapsPanel, props));
    await until(() => m.target.querySelector('[data-maps-status="none"]') !== null, 'the store read');
    m.click('Add maps');
    m.click('Fetch maps');
    await until(() => m.text().includes('could not be fetched'), 'the failure', 5000);
    expect(m.text()).toContain('The maps could not be fetched on October 1, 2026');
    expect(m.target.querySelectorAll('figure img')).toHaveLength(0);
    expect(m.target.querySelectorAll('.map-figure__placeholder')).toHaveLength(3);
    expect(changes).toEqual([]);
  });

  it('a plan with pins but no images says the maps need refreshing', async () => {
    const { props } = panelProps({ maps: { ...emptyMapsState(), home: HOME, fetched_on: '2026-09-01' } });
    const m = keep(render(MapsPanel, props));
    await until(() => m.target.querySelector('[data-maps-status="stale"]') !== null, 'the store read');
    expect(m.text()).toContain('Maps need refreshing');
    expect(m.button('Refresh maps')).toBeDefined();
  });

  it('"Remove maps" clears the images and, if asked, the pins', async () => {
    const { props, changes } = panelProps({ maps: { ...emptyMapsState(), home: HOME } });
    const m = keep(render(MapsPanel, props));
    await until(() => m.target.querySelector('[data-maps-status="none"]') !== null, 'the store read');
    m.click('Add maps');
    m.click('Fetch maps');
    await until(() => m.target.querySelector('[data-maps-phase="idle"] img') !== null, 'the maps', 5000);
    m.click('Remove maps');
    const dialog = document.querySelector('dialog[open]')!;
    dialog.querySelector<HTMLInputElement>('input[type=checkbox]')!.click();
    flushSync();
    [...dialog.querySelectorAll('button')].find((b) => b.textContent === 'Remove maps')!.click();
    await until(() => changes.at(-1) === undefined && changes.length > 1, 'the removal');
    expect(m.text()).toContain('The maps and your pins were removed from this device.');
  });
});

// ---------------------------------------------------------------------------------------------

describe('The pin map', () => {
  it('places the chosen pin where the map is clicked, or at the cross, and hands pins and lines back on "Done"', async () => {
    const view = stubView({ lat: 39.94, lon: -75.15 });
    const ondone = vi.fn();
    const m = keep(render(PinMap, { value: undefined, center: HOME, zoom: 14, ondone, oncancel: vi.fn(), createView: view.create }));
    await until(() => view.state.opts !== null, 'the map');
    await tick();
    flushSync();
    // Home is chosen first; a click places it.
    view.state.opts!.onClick(HOME);
    flushSync();
    expect(view.state.pins).toEqual([{ id: 'home', at: HOME, mark: 'H', label: 'Home pin' }]);
    expect(m.text()).toContain('Home: placed.');
    // The meeting place near home, at the cross.
    m.target.querySelector<HTMLInputElement>('input[value="meeting_near"]')!.click();
    flushSync();
    m.click('Put it at the cross');
    expect(view.state.pins.map((p) => p.id)).toEqual(['home', 'meeting_near']);
    // A way out: three clicks, one undone.
    m.target.querySelector<HTMLInputElement>('input[value="route1"]')!.click();
    flushSync();
    view.state.opts!.onClick(HOME);
    view.state.opts!.onClick({ lat: 39.95, lon: -75.19 });
    view.state.opts!.onClick({ lat: 40.02, lon: -75.33 });
    flushSync();
    expect(m.text()).toContain('3 points');
    m.click('Undo last point');
    expect(view.state.routes[0]).toHaveLength(2);
    // Dragging a pin moves it.
    view.state.opts!.onPinMoved('home', { lat: 39.937, lon: -75.154 });
    flushSync();
    await noAxeViolations(m.target, 'the pin map');
    m.click('Done');
    expect(ondone).toHaveBeenCalledWith({
      ...emptyMapsState(),
      home: { lat: 39.937, lon: -75.154 },
      meeting_near: { lat: 39.94, lon: -75.15 },
      routes: [[HOME, { lat: 39.95, lon: -75.19 }]],
    });
  });

  it('keeps what was saved, starts with the next pin to place, and "Cancel" changes nothing', async () => {
    const view = stubView();
    const ondone = vi.fn();
    const oncancel = vi.fn();
    const value: MapsState = { ...emptyMapsState(), home: HOME, routes: [[HOME, { lat: 40, lon: -75.3 }]] };
    const m = keep(render(PinMap, { value, center: HOME, zoom: 16, ondone, oncancel, createView: view.create }));
    await until(() => view.state.pins.length === 1, 'the saved pins');
    expect(view.state.routes[0]).toHaveLength(2);
    expect(m.target.querySelector<HTMLInputElement>('input[value="meeting_near"]')!.checked).toBe(true);
    m.click('Cancel');
    expect(oncancel).toHaveBeenCalled();
    expect(ondone).not.toHaveBeenCalled();
  });

  it('says so when the map library cannot load, and the address search is still there', async () => {
    const m = keep(render(PinMap, { value: undefined, center: HOME, zoom: 14, ondone: vi.fn(), oncancel: vi.fn(), createView: async () => Promise.reject(new Error('offline')) }));
    await until(() => m.text().includes('could not be loaded'), 'the failure');
    expect(m.button('Type an address instead')).toBeDefined();
  });
});

describe('Type an address instead (D6)', () => {
  function searchStub(answer: unknown[]) {
    const calls: string[] = [];
    const s = new Search({
      fetch: async (url) => {
        calls.push(url);
        return { ok: true, status: 200, json: async () => answer } as unknown as Response;
      },
      now: () => 0,
      sleep: async () => undefined,
    });
    return { s, calls };
  }

  it('warns first, sends nothing until "Search", then offers the matches to place the pin', async () => {
    const { s, calls } = searchStub([
      { display_name: '1400, John F. Kennedy Boulevard, Philadelphia', lat: '39.9533', lon: '-75.1636' },
      { display_name: 'Dilworth Park Café, Philadelphia', lat: '39.9532', lon: '-75.1644' },
    ]);
    const onchoose = vi.fn();
    const m = keep(render(AddressSearch, { onchoose, search: s }));
    m.click('Type an address instead');
    expect(m.text()).toContain('Before you type an address');
    expect(m.text()).toContain('no names');
    await noAxeViolations(m.target, 'the address warning');
    m.click('Continue');
    const input = m.target.querySelector<HTMLInputElement>('input')!;
    input.value = '1400 JFK Blvd, Philadelphia';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(calls).toEqual([]);
    m.click('Search');
    await until(() => m.text().includes('Choose the right one'), 'the matches');
    expect(calls).toHaveLength(1);
    expect(new URL(calls[0]!).searchParams.get('q')).toBe('1400 JFK Blvd, Philadelphia');
    await noAxeViolations(m.target, 'the address search with matches');
    m.click(/^1400, John F\. Kennedy/);
    expect(onchoose).toHaveBeenCalledWith({ label: '1400, John F. Kennedy Boulevard, Philadelphia', at: { lat: 39.9533, lon: -75.1636 } });
  });
});

// ---------------------------------------------------------------------------------------------

function record(extra: Partial<MapRecord> = {}): MapRecord {
  return {
    slot: 'neighbourhood',
    image: 'data:image/jpeg;base64,AAAA',
    width: 800,
    height: 560,
    bytes: 3,
    zoom: 16,
    legend: [
      { mark: 'H', name: 'Home', kind: 'Your plan', own: true },
      { mark: '1', name: 'Dr. <b>Bold</b> & Sons', kind: 'Pharmacy', address: '1 Main St', phone: '+1 215 555 0100' },
    ],
    keys: [
      { pattern: 'stripes', text: 'FEMA high-risk flood zone: about a 1 in 100 chance of flooding each year.' },
      { pattern: 'dots', text: 'FEMA moderate flood zone: about a 1 in 500 chance each year.' },
    ],
    statuses: [{ layer: 'surge', state: 'unavailable', text: SURGE_NOTE }],
    notes: [],
    credits: ['Map data © OpenStreetMap contributors, openstreetmap.org/copyright', 'Flood zones: FEMA National Flood Hazard Layer, retrieved October 1, 2026'],
    scale: 'The bars show 500 ft and 200 m.',
    fetched_on: '2026-10-01',
    key: 'k',
    ...extra,
  };
}

describe('A map in its slot', () => {
  it('shows the image with words for it, the numbered legend as text, the pattern keys, statuses and dated credits', async () => {
    const m = keep(render(MapFigure, { slot: 'neighbourhood', caption: 'Your neighbourhood', record: record() }));
    const img = m.target.querySelector('img')!;
    expect(img.getAttribute('alt')).toBe('Map of your neighbourhood, showing H home, 1 places numbered as in the table below. The bars show 500 ft and 200 m.');
    const rows = [...m.target.querySelectorAll('tbody tr')].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent?.trim()));
    expect(rows).toEqual([
      ['H', 'Home', 'Your plan', '—'],
      ['1', 'Dr. <b>Bold</b> & Sons', 'Pharmacy', '1 Main St · +1 215 555 0100'],
    ]);
    expect(m.target.querySelector('b')).toBeNull();
    expect(m.target.querySelectorAll('.map-figure__swatch')).toHaveLength(2);
    expect(m.text()).toContain(SURGE_NOTE);
    expect(m.text()).toContain('retrieved October 1, 2026');
    await noAxeViolations(m.target, 'a map figure');
  });

  it('draws the dashed placeholder without an image, and says when maps need refreshing', async () => {
    const none = keep(render(MapFigure, { slot: 'area', caption: 'Your city or county' }));
    expect(none.text()).toContain('Map: your city or county. Add it in the app, or paste a printed map here.');
    const stale = keep(render(MapFigure, { slot: 'region', caption: 'Getting out', status: 'stale' }));
    expect(stale.text()).toContain('Maps need refreshing');
    await noAxeViolations(stale.target, 'a placeholder');
  });
});

describe('The step 7 button', () => {
  it('opens the short consent screen, then the pin map, and hands the pins back', async () => {
    const view = stubView();
    const onchange = vi.fn();
    const m = keep(render(PinMapButton, { maps: undefined, location: PHILLY, onchange, createView: view.create }));
    expect(m.text()).toContain('Nothing placed yet');
    m.click('Set your home point and meeting places on a map');
    expect(m.text()).toContain('Before the map opens');
    expect(view.state.opts).toBeNull();
    m.click('Show the map');
    await until(() => view.state.opts !== null, 'the pin map');
    // Opened at the middle of the ZIP code.
    expect(view.state.opts!.center).toEqual(PHILLY.zip_centroid);
    expect(view.state.opts!.zoom).toBe(14);
    view.state.opts!.onClick(HOME);
    flushSync();
    m.click('Done');
    expect(onchange).toHaveBeenCalledWith({ ...emptyMapsState(), home: HOME });
  });
});
