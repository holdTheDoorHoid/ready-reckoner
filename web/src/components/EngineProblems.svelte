<!--
  When the engine cannot work out a plan: says what to fix, with a link to the screen where each
  answer lives. Nothing is lost; the answers stay as they are.
-->
<script lang="ts">
  import type { EngineError, Problem } from '../engine/types';
  import { href, ROUTES, screenFor } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  let { error }: { error: EngineError } = $props();

  const problems = $derived.by((): Problem[] => {
    if (error.code !== 'bad_input') return [];
    const details = error.details as { problems?: Problem[] } | undefined;
    return details?.problems ?? [];
  });
  const locationError = $derived(error.code === 'unknown_zip' || error.code === 'unknown_county' || error.code === 'ambiguous_zip');
</script>

<div class="problems card" role="status">
  <h2><Icon name="info" /> We can't work out your plan yet</h2>
  <p>{error.message}</p>
  {#if problems.length}
    <ul>
      {#each problems as p, i (i)}
        <li>{p.message} <a href={href(screenFor(p.field))}>Fix it on "{ROUTES[screenFor(p.field)].title}"</a></li>
      {/each}
    </ul>
  {:else if locationError}
    <p><a class="button button--primary" href={href('where')}>Go to "Where you live"</a></p>
  {:else}
    <p>Your answers are safe. Reload the page to try again; if it keeps happening, export your plan from "Keep it up" and report the problem.</p>
  {/if}
</div>

<style>
  .problems {
    border-left: 6px solid var(--note-edge);
  }
  .problems h2 {
    margin-top: 0;
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--text-lg);
  }
</style>
