/**
 * A small hash router: `#/where`, `#/who`, ... `#/learn/myths`. Hash routes work on GitHub Pages
 * under any base path and offline from the service worker, because there is only ever one page.
 * Hashes that do not start with `#/` are in-page anchors and leave the route alone. Some routes
 * take a second part: a Learn article (`#/learn/myths`), a page of the binder
 * (`#/binder/wallet-cards`) and a card of an optional step (`#/places/home`, `#/contacts/circle`).
 *
 * v0.3.0 (DESIGN-DELTA-v3 §1): the interview has eight steps, the last three optional (`people`,
 * `places`, `contacts`); the Plan tab is `prepare` and the packet is the `binder`. Addresses from
 * earlier versions still work: `#/plan` opens Prepare, `#/packet` (and `#/packet/<anything>`) the
 * binder (`#/packet/wallet-cards` its wallet cards), and `#/family` step 7, each part of the old
 * family plan (`#/family/circle`) the card that now holds its questions. The address bar is
 * rewritten to the new address without adding a history entry.
 */
import { getContext, setContext } from 'svelte';

import { isOptionalStep, STEP_IDS, type StepId } from './persistence';

export const ROUTE_IDS = [
  'start',
  'where',
  'who',
  'travel',
  'money',
  'have',
  'people',
  'places',
  'contacts',
  'risks',
  'prepare',
  'binder',
  'maintain',
  'learn',
  'validation',
  'about',
  'missing',
] as const;
export type RouteId = (typeof ROUTE_IDS)[number];

export interface Route {
  id: RouteId;
  /** The rest of the path, for example the article slug in `#/learn/myths`. */
  param?: string;
}

export interface RouteInfo {
  path: string;
  title: string;
  /** Interview step number, 1–8. */
  step?: number;
  /** An optional interview step (6–8): it has "Skip for now", and the plan never waits for it. */
  optional?: true;
}

export const ROUTES: Record<RouteId, RouteInfo> = {
  start: { path: '', title: 'Start' },
  where: { path: 'where', title: 'Where you live', step: 1 },
  who: { path: 'who', title: 'Who is in your household', step: 2 },
  travel: { path: 'travel', title: 'How you get around', step: 3 },
  money: { path: 'money', title: 'Money', step: 4 },
  have: { path: 'have', title: 'What you already have', step: 5 },
  people: { path: 'people', title: 'Your people', step: 6, optional: true },
  places: { path: 'places', title: 'Your places', step: 7, optional: true },
  contacts: { path: 'contacts', title: 'Contacts, pets, vehicles and documents', step: 8, optional: true },
  risks: { path: 'risks', title: 'Your risks' },
  prepare: { path: 'prepare', title: 'Prepare: what to do before' },
  binder: { path: 'binder', title: 'Your binder' },
  maintain: { path: 'maintain', title: 'Keep it up' },
  learn: { path: 'learn', title: 'Learn' },
  validation: { path: 'validation', title: 'How well do these numbers hold up?' },
  about: { path: 'about', title: 'About and method' },
  missing: { path: 'missing', title: 'Page not found' },
};

/** Routes whose address may carry a second part (an article, a binder page, or a card to open at). */
const WITH_PARAM: ReadonlySet<RouteId> = new Set<RouteId>(['learn', 'binder', 'places', 'contacts']);

const BY_PATH = new Map(Object.entries(ROUTES).map(([id, info]) => [info.path, id as RouteId]));

/**
 * The parts of the v0.2.0 family-plan screen (`#/family/<part>`) and the card on step 7 or 8 that
 * holds the same questions now.
 */
export const FAMILY_REDIRECTS: Readonly<Record<string, Route>> = {
  contact: { id: 'places', param: 'touch' },
  children: { id: 'places', param: 'touch' },
  shelter: { id: 'places', param: 'home' },
  leave: { id: 'places', param: 'leave' },
  home: { id: 'places', param: 'home' },
  circle: { id: 'contacts', param: 'circle' },
  lawyer: { id: 'contacts', param: 'lawyer' },
};

/** Where an address from an earlier version leads now (DESIGN-DELTA-v3 §1), or undefined. */
function redirect(head: string, param: string): Route | undefined {
  switch (head) {
    case 'plan':
      return { id: 'prepare' };
    case 'packet':
      return param === 'wallet-cards' ? { id: 'binder', param: 'wallet-cards' } : { id: 'binder' };
    case 'family':
      return FAMILY_REDIRECTS[param] ?? { id: 'places' };
    default:
      return undefined;
  }
}

/** The path of a route hash, decoded and without trailing slashes; null when it cannot be read. */
function pathOf(hash: string): string | null {
  try {
    return decodeURIComponent(hash.slice(2)).replace(/\/+$/, '');
  } catch {
    return null;
  }
}

/** The route a hash names, or null when it is an in-page anchor (not starting with `#/`). */
export function parseHash(hash: string): Route | null {
  if (hash === '' || hash === '#' || hash === '#/') return { id: 'start' };
  if (!hash.startsWith('#/')) return null;
  const path = pathOf(hash);
  if (path === null) return { id: 'missing' };
  const [head = '', ...rest] = path.split('/');
  if (head === 'start' && rest.length === 0) return { id: 'start' };
  const param = rest.join('/');
  const id = BY_PATH.get(head);
  if (!id) return redirect(head, param) ?? { id: 'missing' };
  if (id === 'missing') return { id: 'missing' };
  if (param && !WITH_PARAM.has(id)) return { id: 'missing' };
  return param ? { id, param } : { id };
}

/** True when `hash` is an address from an earlier version that now leads somewhere else. */
export function isRedirect(hash: string): boolean {
  if (!hash.startsWith('#/')) return false;
  const head = pathOf(hash)?.split('/')[0] ?? '';
  return !BY_PATH.has(head) && redirect(head, '') !== undefined;
}

export function formatHash(route: Route): string {
  const path = ROUTES[route.id].path;
  if (!path) return '#/';
  return route.param ? `#/${path}/${encodeURIComponent(route.param)}` : `#/${path}`;
}

export function href(id: RouteId, param?: string): string {
  return formatHash(param ? { id, param } : { id });
}

export function isStep(id: RouteId): id is StepId {
  return (STEP_IDS as readonly string[]).includes(id);
}

/** An optional interview step (6–8). */
export function isOptional(id: RouteId): boolean {
  return isOptionalStep(id);
}

export function stepAfter(id: StepId): RouteId {
  const i = STEP_IDS.indexOf(id);
  return STEP_IDS[i + 1] ?? 'risks';
}

export function stepBefore(id: StepId): RouteId {
  const i = STEP_IDS.indexOf(id);
  return i === 0 ? 'start' : STEP_IDS[i - 1]!;
}

/** The current route, kept in step with `location.hash`. */
export class Router {
  current = $state<Route>({ id: 'start' });
  #win: Window;
  #onChange = () => {
    const next = this.#read();
    if (next) this.current = next;
  };

  constructor(win: Window = window) {
    this.#win = win;
    this.current = this.#read() ?? { id: 'start' };
    win.addEventListener('hashchange', this.#onChange);
  }

  /** The route in the address bar; an old address is rewritten to its new one in place. */
  #read(): Route | null {
    const hash = this.#win.location.hash;
    const route = parseHash(hash);
    if (route && isRedirect(hash)) {
      try {
        this.#win.history.replaceState(this.#win.history.state, '', formatHash(route));
      } catch {
        // The route still opens; only the address bar keeps the old address.
      }
    }
    return route;
  }

  go(id: RouteId, param?: string): void {
    const hash = href(id, param);
    this.current = parseHash(hash)!;
    if (this.#win.location.hash !== hash) this.#win.location.hash = hash;
  }

  destroy(): void {
    this.#win.removeEventListener('hashchange', this.#onChange);
  }
}

// ---------------------------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------------------------

export const ROUTER_CONTEXT = 'rr-router';

export function provideRouter(router: Router): void {
  setContext(ROUTER_CONTEXT, router);
}

export function useRouter(): Router {
  const router = getContext<Router | undefined>(ROUTER_CONTEXT);
  if (!router) throw new Error('Router missing from context');
  return router;
}

/** Which screen holds the answer an engine problem points at (its JSON field path). */
export function screenFor(field: string): RouteId {
  if (field.startsWith('housing.alarms') || field.startsWith('existing')) return 'have';
  if (field.startsWith('location') || field.startsWith('housing') || field === '') return 'where';
  if (/^people\[\d+\]\.profile/.test(field)) return 'people';
  if (/^people\[\d+\]\.commute/.test(field) || field.startsWith('mobility')) return 'travel';
  if (field.startsWith('people') || field.startsWith('pets')) return 'who';
  if (field.startsWith('finances')) return 'money';
  if (field.startsWith('dials')) return 'risks';
  if (/^family_plan\.(trusted_circle|lawyer|pets|vehicles|documents|who_takes_animals)/.test(field)) return 'contacts';
  if (field.startsWith('family_plan')) return 'places';
  return 'start';
}
