/**
 * Registers the service worker (production builds only) and tracks whether a new version is
 * waiting. The new version never takes over by itself: the page shows "Reload to update", and
 * only when the person chooses it does the page ask the worker to switch and then reload.
 *
 * On a first visit the data files load before the worker exists. When it first takes control, the
 * page asks for the ones it already has again (`dataUrls`): the browser answers from its own
 * cache, and the worker keeps a copy, so the ZIP tables or the map fetched early also work offline.
 */
export const updates = $state({ ready: false });

let waiting: ServiceWorker | null = null;
let requested = false;

export async function registerServiceWorker(dataUrls: () => string[] = () => []): Promise<void> {
  if (!('serviceWorker' in navigator)) return;
  const base = import.meta.env.BASE_URL;
  const hadController = !!navigator.serviceWorker.controller;
  let warmed = false;
  try {
    const registration = await navigator.serviceWorker.register(`${base}sw.js`, { scope: base });
    const offer = (worker: ServiceWorker | null) => {
      if (worker && navigator.serviceWorker.controller) {
        waiting = worker;
        updates.ready = true;
      }
    };
    offer(registration.waiting);
    registration.addEventListener('updatefound', () => {
      const installing = registration.installing;
      installing?.addEventListener('statechange', () => {
        if (installing.state === 'installed') offer(installing);
      });
    });
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      if (requested) {
        window.location.reload();
        return;
      }
      if (!hadController && !warmed) {
        warmed = true;
        for (const url of dataUrls()) void fetch(url).catch(() => undefined);
      }
    });
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') registration.update().catch(() => {});
    });
  } catch (e) {
    console.warn('Offline support is unavailable in this browser.', e);
  }
}

export function applyUpdate(): void {
  if (!waiting) return;
  requested = true;
  waiting.postMessage({ type: 'SKIP_WAITING' });
}
