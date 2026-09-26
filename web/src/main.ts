import './styles/tokens.css';
import './styles/base.css';
import './styles/print.css';

import { mount } from 'svelte';

import App from './App.svelte';
import { AppState } from './lib/app.svelte';
import { Router } from './lib/router.svelte';
import { registerServiceWorker } from './lib/sw-register.svelte';

const app = new AppState();
const router = new Router();

mount(App, { target: document.getElementById('app')!, props: { app, router } });

// Offline support is for the built site; the dev server always serves fresh files. The service
// worker is registered once the county data is in, so its first install copies the files this
// visit already downloaded (from the browser's cache) instead of downloading them a second time.
void app
  .init()
  .then(() => app.dataSettled())
  .then(() => {
    if (import.meta.env.PROD) void registerServiceWorker(() => app.dataUrlsFetched());
  });

// Save straight away if the page is being hidden or closed mid-change.
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') app.saveNow();
});

export default app;
