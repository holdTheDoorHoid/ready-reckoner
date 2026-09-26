/**
 * A small hash router: `#/where`, `#/who`, ... `#/learn/myths`. Hash routes work on GitHub Pages
 * under any base path and offline from the service worker, because there is only ever one page.
 * Hashes that do not start with `#/` are in-page anchors and leave the route alone.
 */
import { getContext, setContext } from 'svelte';

import { STEP_IDS, type StepId } from './persistence';

export const ROUTE_IDS = [
  'start',
  'where',
  'who',
  'travel',
  'money',
  'have',
  'risks',
  'plan',
  'packet',
  'maintain',
  'learn',
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
  /** Interview step number, 1–5. */
  step?: number;
}

export const ROUTES: Record<RouteId, RouteInfo> = {
  start: { path: '', title: 'Start' },
  where: { path: 'where', title: 'Where you live', step: 1 },
  who: { path: 'who', title: 'Who is in your household', step: 2 },
  travel: { path: 'travel', title: 'How you get around', step: 3 },
  money: { path: 'money', title: 'Money', step: 4 },
  have: { path: 'have', title: 'What you already have', step: 5 },
  risks: { path: 'risks', title: 'Your risks' },
  plan: { path: 'plan', title: 'Your plan' },
  packet: { path: 'packet', title: 'Your packet' },
  maintain: { path: 'maintain', title: 'Keep it up' },
  learn: { path: 'learn', title: 'Learn' },
  about: { path: 'about', title: 'About and method' },
  missing: { path: 'missing', title: 'Page not found' },
};

const BY_PATH = new Map(Object.entries(ROUTES).map(([id, info]) => [info.path, id as RouteId]));

/** The route a hash names, or null when it is an in-page anchor (not starting with `#/`). */
export function parseHash(hash: string): Route | null {
  if (hash === '' || hash === '#' || hash === '#/') return { id: 'start' };
  if (!hash.startsWith('#/')) return null;
  let path = hash.slice(2);
  try {
    path = decodeURIComponent(path);
  } catch {
    return { id: 'missing' };
  }
  const [head = '', ...rest] = path.replace(/\/+$/, '').split('/');
  const id = BY_PATH.get(head);
  if (!id || id === 'missing') return { id: 'missing' };
  const param = rest.join('/');
  if (param && id !== 'learn') return { id: 'missing' };
  return param ? { id, param } : { id };
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
    const next = parseHash(this.#win.location.hash);
    if (next) this.current = next;
  };

  constructor(win: Window = window) {
    this.#win = win;
    this.current = parseHash(win.location.hash) ?? { id: 'start' };
    win.addEventListener('hashchange', this.#onChange);
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
  if (/^people\[\d+\]\.commute/.test(field) || field.startsWith('mobility')) return 'travel';
  if (field.startsWith('people') || field.startsWith('pets')) return 'who';
  if (field.startsWith('finances')) return 'money';
  if (field.startsWith('dials')) return 'risks';
  return 'start';
}
