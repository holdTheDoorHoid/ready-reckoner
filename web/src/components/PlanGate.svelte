<!--
  Shared by the result screens: shows a start prompt when there is no plan yet, a calm "working it
  out" while the first answer comes back, the engine's problems when it cannot answer, and the
  screen itself once there is a plan.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { PlanOutput } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { href } from '../lib/router.svelte';
  import EngineProblems from './EngineProblems.svelte';

  let { children }: { children: Snippet<[PlanOutput]> } = $props();
  const app = useApp();
</script>

{#if !app.plan}
  <div class="card gate">
    <p>You haven't started a plan on this device yet. It takes about ten minutes, and nothing you enter leaves your browser.</p>
    <p class="button-row"><a class="button button--primary" href={href('start')}>Start a plan</a></p>
  </div>
{:else if app.result.error}
  <EngineProblems error={app.result.error} />
{:else if app.result.output}
  <div data-screen-ready aria-busy={app.pending}>
    {@render children(app.result.output)}
  </div>
{:else}
  <p class="muted" aria-busy="true">Working out your plan…</p>
{/if}

<style>
  .gate {
    max-width: var(--w-text);
  }
</style>
