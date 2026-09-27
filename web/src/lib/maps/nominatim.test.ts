import { describe, expect, it } from 'vitest';

import { AddressSearch, SearchError, searchUrl, tidyQuery } from './nominatim';

/** A clock that only moves when the search sleeps. */
function deps(answer: (url: string) => unknown = () => [], fail?: 'network' | 'status') {
  let now = 0;
  const calls: { url: string; init: RequestInit; at: number }[] = [];
  return {
    calls,
    deps: {
      fetch: async (url: string, init: RequestInit) => {
        calls.push({ url, init, at: now });
        if (fail === 'network') throw new TypeError('Failed to fetch');
        return { ok: fail !== 'status', status: fail === 'status' ? 429 : 200, json: async () => answer(url) } as unknown as Response;
      },
      now: () => now,
      sleep: async (ms: number) => {
        now += ms;
      },
    },
  };
}

const CITY_HALL = [
  { display_name: '1400, John F. Kennedy Boulevard, Center City, Philadelphia, Philadelphia County, Pennsylvania, 19107, United States', lat: '39.9533060', lon: '-75.1635770' },
  { display_name: 'Dilworth Park Café, 1400, John F. Kennedy Boulevard, Philadelphia', lat: '39.9532060', lon: '-75.1643762' },
  { display_name: 'Somewhere else', lat: '39.9', lon: '-75.1' },
  { display_name: 'A fourth, never shown', lat: '39.8', lon: '-75.0' },
];

describe('Address search (Nominatim, D6)', () => {
  it('asks for at most three US matches, as JSON, with only the typed address', () => {
    const url = new URL(searchUrl('1400 John F Kennedy Blvd, Philadelphia'));
    expect(`${url.origin}${url.pathname}`).toBe('https://nominatim.openstreetmap.org/search');
    expect(Object.fromEntries(url.searchParams)).toEqual({ q: '1400 John F Kennedy Blvd, Philadelphia', format: 'jsonv2', limit: '3', countrycodes: 'us', addressdetails: '0' });
  });

  it('sends the site’s address as the Referer (the policy asks for it), and no cookies', async () => {
    const d = deps(() => CITY_HALL);
    const matches = await new AddressSearch(d.deps).search('1400 John F Kennedy Blvd');
    expect(d.calls[0]?.init).toEqual({ mode: 'cors', credentials: 'omit', referrerPolicy: 'strict-origin-when-cross-origin', cache: 'default' });
    expect(matches).toHaveLength(3);
    expect(matches[0]).toEqual({ label: CITY_HALL[0]!.display_name, at: { lat: 39.95331, lon: -75.16358 } });
  });

  it('never sends empty words, and tidies spaces', async () => {
    const d = deps();
    const s = new AddressSearch(d.deps);
    expect(await s.search('   ')).toEqual([]);
    expect(d.calls).toHaveLength(0);
    expect(tidyQuery('  12   Main  St \n Anytown ')).toBe('12 Main St Anytown');
    expect(tidyQuery('x'.repeat(300))).toHaveLength(200);
  });

  it('answers the same words from memory instead of asking again', async () => {
    const d = deps(() => CITY_HALL);
    const s = new AddressSearch(d.deps);
    await s.search('1400 JFK Blvd');
    await s.search('1400  jfk blvd ');
    expect(d.calls).toHaveLength(1);
    expect(s.sent).toBe(1);
  });

  it('waits so that no two requests are less than a second apart, even when pressed quickly', async () => {
    const d = deps(() => []);
    const s = new AddressSearch(d.deps);
    await Promise.all([s.search('one'), s.search('two'), s.search('three')]);
    expect(d.calls.map((c) => c.at)).toEqual([0, 1100, 2200]);
  });

  it('says plainly what went wrong, and remembers nothing from a failure', async () => {
    const down = deps(() => [], 'network');
    await expect(new AddressSearch(down.deps).search('x')).rejects.toBeInstanceOf(SearchError);
    const busy = deps(() => [], 'status');
    const s = new AddressSearch(busy.deps);
    await expect(s.search('x')).rejects.toThrow('The address search is busy');
    await expect(s.search('x')).rejects.toThrow('The address search is busy');
    expect(busy.calls).toHaveLength(2);
  });

  it('drops matches without a position or a name', async () => {
    const d = deps(() => [{ display_name: 'No position' }, { lat: '1', lon: '1' }, { display_name: 'Fine', lat: '40', lon: '-75' }]);
    expect(await new AddressSearch(d.deps).search('x')).toEqual([{ label: 'Fine', at: { lat: 40, lon: -75 } }]);
  });
});
