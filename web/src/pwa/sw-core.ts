/**
 * The service worker's decisions, kept free of worker globals so they can be unit-tested.
 * `sw.js` imports this; `vite-plugins/pwa.ts` bundles both into `dist/sw.js` at build time.
 *
 * Caching rules:
 * - Built assets (the app shell, scripts, styles, icons, the WebAssembly engine) are precached
 *   when a new version installs and served cache-first, so the app works offline.
 * - Data packs under `data/` are stale-while-revalidate: the cached copy answers at once and a
 *   fresh copy is fetched in the background for next time.
 * - Page navigations get the cached `index.html` (the router is hash-based, so there is only one page).
 * - Anything else, including every other origin, goes straight to the network untouched.
 */

/** How the service worker answers one request. */
export type Strategy = 'data' | 'precache' | 'navigate' | 'network';

/** Every cache this app creates starts with this, so it never touches another site's caches on a shared origin. */
export const CACHE_PREFIX = 'rr-';
const PRECACHE_PREFIX = `${CACHE_PREFIX}precache-`;
/** Data packs live in one long-lived cache; stale-while-revalidate keeps it fresh. */
export const DATA_CACHE = `${CACHE_PREFIX}data-v1`;

export function precacheName(version: string): string {
  return `${PRECACHE_PREFIX}${version}`;
}

/**
 * Which built files to precache, as paths relative to the site root. Leaves out the service worker
 * itself, source maps (large and only for developers), dotfiles and data packs (cached at runtime).
 */
export function precacheList(files: readonly string[]): string[] {
  return files
    .filter((f) => f !== 'sw.js' && !f.endsWith('.map') && !f.startsWith('data/') && !/(^|\/)\./.test(f))
    .sort();
}

/**
 * Decide how to answer a GET request. `scope` is the service worker's registration scope (the
 * site root, which includes the Pages base path); `precached` holds root-relative paths.
 */
export function strategyFor(url: URL, scope: URL, precached: ReadonlySet<string>, mode: string): Strategy {
  if (url.origin !== scope.origin || !url.pathname.startsWith(scope.pathname)) return 'network';
  const rel = url.pathname.slice(scope.pathname.length);
  if (rel.startsWith('data/')) return 'data';
  if (mode === 'navigate') return 'navigate';
  if (precached.has(rel)) return 'precache';
  return 'network';
}

/** Old precaches to delete when a new version activates. Only ever this app's own caches. */
export function staleCaches(names: readonly string[], current: string): string[] {
  return names.filter((n) => n.startsWith(PRECACHE_PREFIX) && n !== current);
}
