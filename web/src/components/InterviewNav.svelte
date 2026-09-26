<!--
  Back and Continue for an interview step. If answers on this step look wrong, Continue first
  lists them (with links to each field); "Continue anyway" always goes on. Warn, never block.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import { useApp } from '../lib/app.svelte';
  import type { StepId } from '../lib/persistence';
  import type { FieldProblem } from '../lib/ui-types';
  import { href, ROUTES, stepAfter, stepBefore, useRouter } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  let { step, problems = [], nextLabel }: { step: StepId; problems?: FieldProblem[]; nextLabel?: string } = $props();
  const app = useApp();
  const router = useRouter();

  let showSummary = $state(false);
  let summary: HTMLDivElement | undefined = $state();
  const next = $derived(stepAfter(step));
  const back = $derived(stepBefore(step));

  $effect(() => {
    if (problems.length === 0) showSummary = false;
  });

  async function go() {
    if (problems.length > 0 && !showSummary) {
      showSummary = true;
      await tick();
      summary?.focus();
      return;
    }
    if (problems.length > 0) {
      summary?.focus();
      return;
    }
    proceed();
  }

  function proceed() {
    app.completeStep(step);
    router.go(next);
  }

  function focusField(e: MouseEvent, id: string) {
    e.preventDefault();
    const el = document.getElementById(id);
    el?.focus();
    el?.scrollIntoView({ block: 'center' });
  }
</script>

{#if showSummary && problems.length > 0}
  <div class="error-summary" tabindex="-1" bind:this={summary} role="alert" aria-labelledby="error-summary-title">
    <h2 id="error-summary-title"><Icon name="alert" /> Check {problems.length === 1 ? 'this answer' : `these ${problems.length} answers`}</h2>
    <ul>
      {#each problems as p (p.id + p.message)}
        <li><a href="#{p.id}" onclick={(e) => focusField(e, p.id)}>{p.message}</a></li>
      {/each}
    </ul>
    <p class="small">You can fix them now or later. Your plan can't be worked out until they're fixed, but nothing is lost.</p>
    <button type="button" class="button" onclick={proceed}>Continue anyway</button>
  </div>
{/if}

<div class="interview-nav no-print">
  <a class="button" href={href(back)}><Icon name="chevron-left" /> Back</a>
  <div class="interview-nav__next">
    <button type="button" class="button button--primary" onclick={go}>
      {nextLabel ?? 'Continue'} <Icon name="chevron-right" />
    </button>
    <span class="small muted">Next: {ROUTES[next].title}</span>
  </div>
</div>

<style>
  .interview-nav {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--s3);
    margin-top: var(--s6);
    padding-top: var(--s4);
    border-top: 1px solid var(--border);
  }
  .interview-nav__next {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--s1);
  }
  .error-summary {
    margin-top: var(--s5);
    padding: var(--s4);
    border: 2px solid var(--danger);
    border-radius: var(--r2);
    background: var(--danger-soft);
  }
  .error-summary h2 {
    margin: 0 0 var(--s2);
    font-size: var(--text-lg);
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--danger);
  }
  .error-summary a {
    color: var(--text);
    font-weight: 600;
  }
</style>
