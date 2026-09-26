// Service worker for Ready Reckoner. Bundled by vite-plugins/pwa.ts into dist/sw.js, which
// replaces __RR_PRECACHE__ (built files), __RR_DATA_PRECACHE__ (the start-up data, by versioned
// address), __RR_DATA_VERSION__ (the data's pack version) and __RR_SW_VERSION__ (a hash of it
// all). A new version installs in the background and waits; the page shows "Reload to update"
// and sends SKIP_WAITING when the person chooses to. See sw-core.ts for the caching rules.
import { DATA_CACHE, precacheName, staleCaches, staleDataEntries, strategyFor } from './sw-core.ts';

/* global __RR_PRECACHE__, __RR_DATA_PRECACHE__, __RR_DATA_VERSION__, __RR_SW_VERSION__ */
const PRECACHE = __RR_PRECACHE__;
const DATA_PRECACHE = __RR_DATA_PRECACHE__;
const DATA_VERSION = __RR_DATA_VERSION__;
const CACHE = precacheName(__RR_SW_VERSION__);
const scope = new URL(self.registration.scope);
const precached = new Set(PRECACHE);

self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE);
      await cache.addAll(PRECACHE.map((path) => new URL(path, scope).href));
      // Start-up data: only what this device does not have yet (an app update rarely changes it).
      const data = await caches.open(DATA_CACHE);
      await Promise.all(
        DATA_PRECACHE.map(async (path) => {
          const url = new URL(path, scope).href;
          if (!(await data.match(url))) await data.add(url);
        }),
      );
    })(),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const names = await caches.keys();
      await Promise.all(staleCaches(names, CACHE).map((name) => caches.delete(name)));
      if (DATA_VERSION) {
        const data = await caches.open(DATA_CACHE);
        const keys = await data.keys();
        const stale = new Set(staleDataEntries(keys.map((r) => r.url), DATA_VERSION));
        await Promise.all(keys.filter((r) => stale.has(r.url)).map((r) => data.delete(r)));
      }
      await self.clients.claim();
    })(),
  );
});

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') self.skipWaiting();
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') return;
  const strategy = strategyFor(new URL(request.url), scope, precached, request.mode);
  if (strategy === 'data') event.respondWith(dataFile(request));
  else if (strategy === 'navigate') event.respondWith(appShell(request));
  else if (strategy === 'precache') event.respondWith(cacheFirst(request));
});

async function cacheFirst(request) {
  const cache = await caches.open(CACHE);
  return (await cache.match(request, { ignoreSearch: true })) ?? fetch(request);
}

async function appShell(request) {
  const cache = await caches.open(CACHE);
  return (await cache.match(new URL('index.html', scope).href)) ?? fetch(request);
}

// A versioned data file never changes: the cached copy answers, and a first fetch is kept.
// Unversioned data is fetched fresh, with the cached copy as the offline fallback.
async function dataFile(request) {
  const cache = await caches.open(DATA_CACHE);
  const versioned = new URL(request.url).searchParams.has('v');
  if (versioned) {
    const cached = await cache.match(request);
    if (cached) return cached;
  }
  try {
    const response = await fetch(request);
    if (response.ok) await cache.put(request, response.clone());
    return response;
  } catch (e) {
    const cached = versioned ? undefined : await cache.match(request);
    if (cached) return cached;
    throw e;
  }
}
