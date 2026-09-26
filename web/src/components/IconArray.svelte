<!--
  "Of 100 households like yours": a 10 by 10 grid, filled squares for the households affected.
  Filled versus outlined carries the meaning, so it reads in greyscale; the label says it in words.
-->
<script lang="ts">
  import { per100 } from '../lib/format';

  let { probability, label }: { probability: number; label: string } = $props();
  const count = $derived.by(() => {
    const v = per100(probability);
    return v.kind === 'fewer' ? 0 : v.kind === 'almost_all' ? 100 : v.n;
  });
</script>

<svg class="icon-array" viewBox="0 0 100 100" role="img" aria-label={label}>
  {#each Array.from({ length: 100 }, (_, i) => i) as i (i)}
    {@const x = (i % 10) * 10}
    {@const y = Math.floor(i / 10) * 10}
    {#if i < count}
      <rect class="filled" x={x + 1.2} y={y + 1.2} width="7.6" height="7.6" rx="1.6" />
    {:else}
      <rect class="empty" x={x + 1.8} y={y + 1.8} width="6.4" height="6.4" rx="1.4" />
    {/if}
  {/each}
</svg>

<style>
  .icon-array {
    width: 7.5rem;
    height: 7.5rem;
    flex: none;
  }
  .filled {
    fill: var(--accent);
  }
  .empty {
    fill: none;
    stroke: var(--border-strong);
    stroke-width: 0.9;
  }
</style>
