<!--
  The shell: skip link, sample-data banner, header, the current screen, footer, the update banner
  and a polite live region. On every route change the page title updates and focus moves to the
  new screen's heading, so keyboard and screen-reader users land where the content starts.
-->
<script lang="ts">
  import { tick, type Component } from 'svelte';
  import MockBanner from './components/MockBanner.svelte';
  import SiteFooter from './components/SiteFooter.svelte';
  import SiteHeader from './components/SiteHeader.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import Icon from './components/Icon.svelte';
  import type { AppState } from './lib/app.svelte';
  import { provideApp } from './lib/app.svelte';
  import { formatHash, provideRouter, ROUTES, type RouteId, type Router } from './lib/router.svelte';
  import About from './screens/About.svelte';
  import Have from './screens/Have.svelte';
  import Learn from './screens/Learn.svelte';
  import Maintain from './screens/Maintain.svelte';
  import Money from './screens/Money.svelte';
  import NotFound from './screens/NotFound.svelte';
  import Packet from './screens/Packet.svelte';
  import PlanScreen from './screens/PlanScreen.svelte';
  import Risks from './screens/Risks.svelte';
  import Start from './screens/Start.svelte';
  import Travel from './screens/Travel.svelte';
  import Where from './screens/Where.svelte';
  import Who from './screens/Who.svelte';
  import { article } from './learn/articles';

  let { app, router }: { app: AppState; router: Router } = $props();
  // The app state and router are created once in main.ts and never replaced.
  // svelte-ignore state_referenced_locally
  provideApp(app);
  // svelte-ignore state_referenced_locally
  provideRouter(router);

  const SCREENS: Record<RouteId, Component> = {
    start: Start,
    where: Where,
    who: Who,
    travel: Travel,
    money: Money,
    have: Have,
    risks: Risks,
    plan: PlanScreen,
    packet: Packet,
    maintain: Maintain,
    learn: Learn,
    about: About,
    missing: NotFound,
  };

  const Screen = $derived(SCREENS[router.current.id]);
  let first = true;
  let lastRoute = '';

  $effect(() => {
    const route = router.current;
    const key = formatHash(route);
    if (key === lastRoute) return;
    lastRoute = key;
    const title = route.id === 'learn' && route.param ? (article(route.param)?.title ?? ROUTES.learn.title) : ROUTES[route.id].title;
    document.title = route.id === 'start' ? 'Ready Reckoner' : `${title} · Ready Reckoner`;
    if (route.id !== 'start' && route.id !== 'missing') app.rememberRoute(key);
    if (first) {
      first = false;
      return;
    }
    if (route.id !== 'start') app.status = '';
    void tick().then(() => {
      window.scrollTo(0, 0);
      document.getElementById('page-title')?.focus();
    });
  });

  function skipToContent(e: MouseEvent) {
    e.preventDefault();
    const main = document.getElementById('main');
    main?.focus();
    main?.scrollIntoView();
  }
</script>

<a class="skip-link" href="#main" onclick={skipToContent}>Skip to main content</a>
<MockBanner />
{#if app.storageDamaged}
  <section class="notice" aria-label="Saved plan notice"><p>A saved plan was found in this browser but it could not be read, so it was left untouched. You can start a new plan or open a saved file.</p></section>
{:else if !app.storageAvailable}
  <section class="notice" aria-label="Storage notice"><p>This browser is not keeping your answers (private browsing or blocked storage). Use "Save a copy of your plan" on the Keep it up screen before you close it.</p></section>
{/if}
<SiteHeader />
<main id="main" tabindex="-1">
  {#if app.status}
    <div class="status-banner page page--narrow"><p class="card" role="status"><Icon name="check" /> {app.status}</p></div>
  {/if}
  {#if app.ready}
    <Screen />
  {:else}
    <div class="page page--narrow"><p class="muted" aria-busy="true">Loading…</p></div>
  {/if}
</main>
<SiteFooter />
<UpdateBanner />

<style>
  .skip-link {
    position: absolute;
    left: var(--s3);
    top: -4rem;
    z-index: 20;
    padding: var(--s2) var(--s4);
    background: var(--surface);
    border: 2px solid var(--focus);
    border-radius: var(--r1);
    font-weight: 650;
  }
  .skip-link:focus {
    top: var(--s3);
  }
  main:focus {
    outline: none;
  }
  .notice {
    background: var(--note-soft);
    border-bottom: 1px solid var(--note-edge);
    font-size: var(--text-sm);
  }
  .notice p {
    max-width: var(--w-wide);
    margin: 0 auto;
    padding: var(--s2) var(--s4);
  }
  .status-banner {
    padding-bottom: 0;
  }
  .status-banner p {
    display: flex;
    gap: var(--s2);
    align-items: center;
    margin: 0;
    color: var(--good);
    font-weight: 600;
  }
</style>
