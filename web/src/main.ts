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

void app.init();

// Offline support is for the built site; the dev server always serves fresh files.
if (import.meta.env.PROD) void registerServiceWorker();

// Save straight away if the page is being hidden or closed mid-change.
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') app.saveNow();
});

export default app;
