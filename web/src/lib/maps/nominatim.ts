/**
 * "Type an address instead" (DESIGN-DELTA-v3 D6): OpenStreetMap's search service, Nominatim, asked
 * once per "Search" press, only after the warning, for at most three matches.
 *
 * The Nominatim Usage Policy (read 2026-09-27; see `sources.ts`) is followed here: no request
 * without a press (never as the person types: autocomplete is forbidden), at most one request a
 * second (a quicker press waits its turn), the same words answered from memory instead of asked
 * again, the site's address sent as the Referer, no cookies, and results limited to the United
 * States. The address is the only thing sent: never a name or anything else from the plan.
 */
import type { LatLon } from '../../engine/types';
import { NOMINATIM } from './sources';
import { checkLatLon } from './state';

export interface AddressMatch {
  /** Nominatim's own description of the place ("1400, John F. Kennedy Boulevard, …"). */
  label: string;
  at: LatLon;
}

/** Why a search gave nothing, in words for the screen. */
export class SearchError extends Error {}

/** The words as they will be sent: spaces tidied, at most 200 characters. */
export function tidyQuery(text: string): string {
  return text.replace(/\s+/g, ' ').trim().slice(0, 200);
}

export function searchUrl(query: string): string {
  const params = new URLSearchParams({ q: query, format: 'jsonv2', limit: String(NOMINATIM.maxResults), countrycodes: NOMINATIM.countryCodes, addressdetails: '0' });
  return `${NOMINATIM.url}?${params.toString()}`;
}

export interface SearchDeps {
  fetch: (url: string, init: RequestInit) => Promise<Response>;
  now: () => number;
  sleep: (ms: number) => Promise<void>;
}

/** One search service for the whole page, so the one-a-second limit and the memory are shared. */
export class AddressSearch {
  #last = -Infinity;
  #queue: Promise<unknown> = Promise.resolve();
  #memory = new Map<string, AddressMatch[]>();
  /** Requests actually sent (for tests and the report). */
  sent = 0;

  constructor(private deps: SearchDeps = { fetch: (u, i) => fetch(u, i), now: () => Date.now(), sleep: (ms) => new Promise((r) => setTimeout(r, ms)) }) {}

  /** Up to three matches for the words; an empty list when there are none. */
  search(text: string): Promise<AddressMatch[]> {
    const query = tidyQuery(text);
    if (!query) return Promise.resolve([]);
    const remembered = this.#memory.get(query.toLowerCase());
    if (remembered) return Promise.resolve(remembered);
    // One at a time, a second apart.
    const run = this.#queue.then(() => this.#send(query));
    this.#queue = run.catch(() => undefined);
    return run;
  }

  async #send(query: string): Promise<AddressMatch[]> {
    const again = this.#memory.get(query.toLowerCase());
    if (again) return again;
    const wait = this.#last + NOMINATIM.minIntervalMs - this.deps.now();
    if (wait > 0) await this.deps.sleep(wait);
    this.#last = this.deps.now();
    this.sent += 1;
    let response: Response;
    try {
      response = await this.deps.fetch(searchUrl(query), { mode: 'cors', credentials: 'omit', referrerPolicy: NOMINATIM.referrerPolicy, cache: 'default' });
    } catch {
      throw new SearchError('The address search did not answer. Check your connection and try again, or place the pin by hand.');
    }
    if (!response.ok) throw new SearchError('The address search is busy. Wait a minute and try again, or place the pin by hand.');
    let json: unknown;
    try {
      json = await response.json();
    } catch {
      throw new SearchError('The address search sent back something this app could not read. Place the pin by hand instead.');
    }
    const matches = (Array.isArray(json) ? json : [])
      .map((r: { display_name?: unknown; lat?: unknown; lon?: unknown }) => {
        const at = checkLatLon({ lat: Number(r.lat), lon: Number(r.lon) });
        const label = typeof r.display_name === 'string' ? r.display_name.replace(/\s+/g, ' ').trim().slice(0, 200) : '';
        return at && label ? { label, at } : null;
      })
      .filter((m): m is AddressMatch => m !== null)
      .slice(0, NOMINATIM.maxResults);
    this.#memory.set(query.toLowerCase(), matches);
    return matches;
  }
}

/** The page's one search service. */
export const addressSearch = new AddressSearch();
