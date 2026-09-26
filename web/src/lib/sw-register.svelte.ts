/**
 * Registers the service worker (production builds only) and tracks whether a new version is
 * waiting. The new version never takes over by itself: the page shows "Reload to update", and
 * only when the person chooses it does the page ask the worker to switch and then reload.
 */
export const updates = $state({ ready: false });

let waiting: ServiceWorker | null = null;
let requested = false;

export async function registerServiceWorker(): Promise<void> {
  if (!('serviceWorker' in navigator)) return;
  const base = import.meta.env.BASE_URL;
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
      if (requested) window.location.reload();
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
