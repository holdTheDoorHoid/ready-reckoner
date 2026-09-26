// Service worker for Ready Reckoner. Bundled by vite-plugins/pwa.ts into dist/sw.js, which
// replaces __RR_PRECACHE__ (built files) and __RR_SW_VERSION__ (a hash of their contents).
// A new version installs in the background and waits; the page shows "Reload to update" and
// sends SKIP_WAITING when the person chooses to. See sw-core.ts for the caching rules.
import { DATA_CACHE, precacheName, staleCaches, strategyFor } from './sw-core.ts';

/* global __RR_PRECACHE__, __RR_SW_VERSION__ */
const PRECACHE = __RR_PRECACHE__;
const CACHE = precacheName(__RR_SW_VERSION__);
const scope = new URL(self.registration.scope);
const precached = new Set(PRECACHE);

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(PRECACHE.map((path) => new URL(path, scope).href))),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const names = await caches.keys();
      await Promise.all(staleCaches(names, CACHE).map((name) => caches.delete(name)));
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
  if (strategy === 'data') event.respondWith(staleWhileRevalidate(event));
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

async function staleWhileRevalidate(event) {
  const cache = await caches.open(DATA_CACHE);
  const cached = await cache.match(event.request);
  const fresh = fetch(event.request)
    .then((response) => {
      if (response.ok) cache.put(event.request, response.clone());
      return response;
    })
    .catch(() => undefined);
  if (cached) {
    event.waitUntil(fresh);
    return cached;
  }
  return (await fresh) ?? new Response('This data pack is not available offline yet.', { status: 503 });
}
