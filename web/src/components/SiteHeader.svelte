<!-- The site header: name, main sections, and a menu button on narrow screens. -->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import { STEP_IDS } from '../lib/persistence';
  import { href, isStep, type RouteId, useRouter } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  const app = useApp();
  const router = useRouter();
  let open = $state(false);

  const answersTarget = $derived.by((): RouteId => {
    const done = app.plan?.progress.completed ?? [];
    return STEP_IDS.find((s) => !done.includes(s)) ?? 'where';
  });

  const links = $derived<{ id: RouteId; label: string; current: boolean }[]>([
    { id: app.plan ? answersTarget : 'start', label: app.plan ? 'Your answers' : 'Start', current: isStep(router.current.id) || router.current.id === 'start' },
    { id: 'family', label: 'Family plan', current: router.current.id === 'family' },
    { id: 'risks', label: 'Risks', current: router.current.id === 'risks' },
    { id: 'plan', label: 'Plan', current: router.current.id === 'plan' },
    { id: 'packet', label: 'Packet', current: router.current.id === 'packet' },
    { id: 'maintain', label: 'Keep it up', current: router.current.id === 'maintain' },
    { id: 'learn', label: 'Learn', current: router.current.id === 'learn' },
    { id: 'about', label: 'About', current: router.current.id === 'about' },
  ]);

  $effect(() => {
    void router.current;
    open = false;
  });
</script>

<header class="site-header">
  <div class="site-header__inner">
    <a class="brand" href={href('start')}>
      <svg class="brand__mark" viewBox="0 0 32 32" aria-hidden="true">
        <rect width="32" height="32" rx="8" fill="var(--accent)" />
        <path d="M8 16.5 L16 9.5 L24 16.5 M10.5 14.5 V23.5 H21.5 V14.5" fill="none" stroke="var(--on-accent)" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round" />
        <path d="M13 18.5 l2.2 2.2 L19.5 16" fill="none" stroke="var(--on-accent)" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <span>Ready Reckoner</span>
    </a>
    <button
      type="button"
      class="button button--quiet menu-button"
      aria-expanded={open}
      aria-controls="site-nav"
      onclick={() => (open = !open)}
    >
      <Icon name={open ? 'close' : 'menu'} /> Menu
    </button>
    <nav id="site-nav" class="site-nav" class:open aria-label="Main">
      <ul>
        {#each links as link (link.label)}
          <li><a href={href(link.id)} aria-current={link.current ? 'page' : undefined}>{link.label}</a></li>
        {/each}
      </ul>
    </nav>
  </div>
</header>

<style>
  .site-header {
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .site-header__inner {
    max-width: var(--w-wide);
    margin: 0 auto;
    padding: var(--s2) var(--s4);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2);
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    min-height: var(--tap);
    color: var(--text);
    text-decoration: none;
    font-weight: 750;
    font-size: var(--text-lg);
  }
  .brand__mark {
    width: 2rem;
    height: 2rem;
  }
  .menu-button {
    display: none;
  }
  .site-nav ul {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1);
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .site-nav li + li {
    margin-top: 0;
  }
  .site-nav a {
    display: inline-flex;
    align-items: center;
    min-height: var(--tap);
    padding: 0 var(--s3);
    border-radius: var(--r1);
    color: var(--text);
    text-decoration: none;
    font-weight: 560;
  }
  .site-nav a:hover {
    background: var(--surface-2);
  }
  .site-nav a[aria-current='page'] {
    background: var(--accent-soft);
    box-shadow: inset 0 -3px 0 var(--accent);
    font-weight: 680;
  }
  @media (max-width: 52rem) {
    .menu-button {
      display: inline-flex;
    }
    .site-nav {
      display: none;
      flex-basis: 100%;
    }
    .site-nav.open {
      display: block;
    }
    .site-nav ul {
      flex-direction: column;
      padding-bottom: var(--s2);
    }
    .site-nav a {
      width: 100%;
    }
  }
</style>
