/**
 * The service worker's decisions, kept free of worker globals so they can be unit-tested.
 * `sw.js` imports this; `vite-plugins/pwa.ts` bundles both into `dist/sw.js` at build time.
 *
 * Caching rules:
 * - Built assets (the app shell, scripts, styles, icons, the WebAssembly engine in `pkg/`) and
 *   `data/manifest.json` are precached when a new version installs and served cache-first. The
 *   manifest belongs to the build: an installed version always reads the data it was built with,
 *   and new data arrives with the next version ("Reload to update").
 * - The data the engine needs at start (`data/core/*` except the ZIP tables) is precached too, by
 *   its versioned address (`?v=<pack_version>`), in a data cache that survives app updates: files
 *   whose version has not changed are not downloaded again.
 * - Other versioned data (the ZIP tables, the map) is cached the first time it is fetched, then
 *   served from the cache: a versioned file never changes.
 * - Unversioned data is fetched fresh, with the cached copy as the offline fallback.
 * - Page navigations get the cached `index.html` (the router is hash-based, so there is only one page).
 * - Anything else, including every other origin, goes straight to the network untouched.
 */

/** How the service worker answers one request. */
export type Strategy = 'data' | 'precache' | 'navigate' | 'network';

/** Every cache this app creates starts with this, so it never touches another site's caches on a shared origin. */
export const CACHE_PREFIX = 'rr-';
const PRECACHE_PREFIX = `${CACHE_PREFIX}precache-`;
/** Data packs live in one long-lived cache, keyed by their versioned address. */
export const DATA_CACHE = `${CACHE_PREFIX}data-v1`;
/** The data manifest, precached with the app so each build reads the data it was built with. */
export const DATA_MANIFEST = 'data/manifest.json';

export function precacheName(version: string): string {
  return `${PRECACHE_PREFIX}${version}`;
}

/**
 * Which built files to precache, as paths relative to the site root. Leaves out the service worker
 * itself, source maps (large and only for developers), dotfiles and data packs (precached by
 * version, or cached when used), but keeps the data manifest.
 */
export function precacheList(files: readonly string[]): string[] {
  return files
    .filter((f) => f !== 'sw.js' && !f.endsWith('.map') && (!f.startsWith('data/') || f === DATA_MANIFEST) && !/(^|\/)\./.test(f))
    .sort();
}

/** The data files to precache, by versioned address relative to the site root. */
export function dataPrecacheList(paths: readonly string[], packVersion: string): string[] {
  const v = packVersion ? `?v=${encodeURIComponent(packVersion)}` : '';
  return paths.map((p) => `data/${p}${v}`);
}

/**
 * Decide how to answer a GET request. `scope` is the service worker's registration scope (the
 * site root, which includes the Pages base path); `precached` holds root-relative paths.
 */
export function strategyFor(url: URL, scope: URL, precached: ReadonlySet<string>, mode: string): Strategy {
  if (url.origin !== scope.origin || !url.pathname.startsWith(scope.pathname)) return 'network';
  const rel = url.pathname.slice(scope.pathname.length);
  if (mode === 'navigate') return 'navigate';
  if (precached.has(rel)) return 'precache';
  if (rel.startsWith('data/')) return 'data';
  return 'network';
}

/** Old precaches to delete when a new version activates. Only ever this app's own caches. */
export function staleCaches(names: readonly string[], current: string): string[] {
  return names.filter((n) => n.startsWith(PRECACHE_PREFIX) && n !== current);
}

/** Cached data addresses from other data versions, to delete when a new version activates. */
export function staleDataEntries(urls: readonly string[], packVersion: string): string[] {
  return urls.filter((u) => {
    const v = new URL(u, 'https://x/').searchParams.get('v');
    return v !== null && v !== packVersion;
  });
}
