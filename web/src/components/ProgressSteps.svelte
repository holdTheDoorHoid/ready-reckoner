<!-- Where you are in the interview. Every step is a link: nothing is locked. -->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import { STEP_IDS, type StepId } from '../lib/persistence';
  import { href, ROUTES } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  let { current }: { current: StepId } = $props();
  const app = useApp();
  const index = $derived(STEP_IDS.indexOf(current));
  const completed = $derived(app.plan?.progress.completed ?? []);
</script>

<nav class="steps no-print" aria-label="Interview steps">
  <p class="steps__count">Step {index + 1} of {STEP_IDS.length}</p>
  <div class="steps__bar" aria-hidden="true"><div class="steps__fill" style:width="{((index + 1) / STEP_IDS.length) * 100}%"></div></div>
  <ol class="steps__list">
    {#each STEP_IDS as step, i (step)}
      <li class:current={step === current} class:done={completed.includes(step)}>
        <a href={href(step)} aria-current={step === current ? 'step' : undefined}>
          <span class="steps__num" aria-hidden="true">
            {#if completed.includes(step) && step !== current}<Icon name="check" size="0.95em" />{:else}{i + 1}{/if}
          </span>
          <span class="steps__name">{ROUTES[step].title}</span>
          {#if completed.includes(step)}<span class="visually-hidden"> (answered)</span>{/if}
        </a>
      </li>
    {/each}
  </ol>
</nav>

<style>
  .steps {
    margin-bottom: var(--s5);
  }
  .steps__count {
    margin: 0 0 var(--s2);
    font-size: var(--text-sm);
    font-weight: 650;
    color: var(--text-muted);
  }
  .steps__bar {
    height: 6px;
    border-radius: 999px;
    background: var(--gauge-track);
    overflow: hidden;
    margin-bottom: var(--s3);
  }
  .steps__fill {
    height: 100%;
    background: var(--accent);
  }
  .steps__list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s2);
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .steps__list li + li {
    margin-top: 0;
  }
  .steps__list a {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    min-height: 36px;
    padding: 0.15rem 0.6rem 0.15rem 0.2rem;
    border-radius: 999px;
    color: var(--text-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }
  .steps__list a:hover {
    background: var(--surface-2);
  }
  .steps__num {
    width: 1.6rem;
    height: 1.6rem;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1.5px solid var(--border-strong);
    font-weight: 700;
    font-size: 0.8rem;
  }
  .done .steps__num {
    border-color: var(--good);
    color: var(--good);
  }
  .current a {
    color: var(--text);
    font-weight: 650;
    background: var(--accent-soft);
  }
  .current .steps__num {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  @media (max-width: 40rem) {
    .steps__list li:not(.current) .steps__name {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
      white-space: nowrap;
    }
    .steps__list a {
      padding-right: 0.2rem;
    }
  }
</style>
