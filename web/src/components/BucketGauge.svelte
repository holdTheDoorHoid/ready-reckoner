<!--
  A duration bucket: the target with its range ("about 3 days (2–5)"), how much is covered now as
  a meter with the words beside it, the relief rating, and what drives it.
-->
<script lang="ts">
  import type { BucketAssessment } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { dayPhrase, targetDays } from '../lib/format';
  import { hazardName, lowerFirst } from '../lib/lookup';
  import ExplainButton from './ExplainButton.svelte';
  import Icon from './Icon.svelte';
  import Sources from './Sources.svelte';

  let { bucket, compact = false }: { bucket: BucketAssessment; compact?: boolean } = $props();
  const app = useApp();
  const uid = $props.id();

  const target = $derived(bucket.target.kind === 'days' ? bucket.target : null);
  const covered = $derived(bucket.covered.kind === 'days' ? bucket.covered.value : 0);
  const done = $derived(target !== null && covered >= target.value);
  const pct = $derived(target ? Math.min(100, (covered / target.value) * 100) : 0);
  const coveredText = $derived.by(() => {
    if (!target) return '';
    if (done) return `Covered: all ${dayPhrase(target.value)}`;
    const t = dayPhrase(target.value);
    const c = dayPhrase(covered);
    const [cn, cu] = [c.split(' ')[0], c.split(' ').slice(1).join(' ')];
    const tu = t.split(' ').slice(1).join(' ');
    return covered > 0 && cu.replace(/s$/, '') === tu.replace(/s$/, '') ? `${cn} of ${t} covered` : `${covered === 0 ? '0' : c} of ${t} covered`;
  });
  const targetText = $derived(target ? targetDays(target.value, target.low, target.high) : '');
  const mainText = $derived(target ? `about ${dayPhrase(target.value)}` : '');
</script>

{#if target}
  <article class="gauge" class:card={!compact} class:compact aria-labelledby="{uid}-name">
    <h3 id="{uid}-name" class="gauge__name">{bucket.name}</h3>
    {#if !compact}
      <p class="gauge__target">
        <span class="visually-hidden">Be ready for </span>
        <span class="big">{mainText}</span>
        <span class="range">{targetText.slice(mainText.length).trim()}</span>
      </p>
    {/if}
    <div
      class="meter"
      role="meter"
      aria-labelledby="{uid}-name"
      aria-valuemin="0"
      aria-valuemax={target.value}
      aria-valuenow={Math.min(covered, target.value)}
      aria-valuetext={coveredText}
    >
      <div class="meter__fill" class:meter__fill--done={done} style:width="{pct}%"></div>
    </div>
    <p class="gauge__covered" class:is-done={done}>
      {#if done}<Icon name="check" />{/if}
      <span>{coveredText}</span>
    </p>
    {#if !compact}
      {#if bucket.relief}
        <p class="gauge__relief small">
          <Icon name="clock" />
          <span>Help likely arrives in about {dayPhrase(bucket.relief.help_arrives_days)}; service mostly back in about {dayPhrase(bucket.relief.mostly_restored_days)}.</span>
        </p>
      {/if}
      {#if bucket.contributions.length}
        <p class="small muted">Mostly from: {bucket.contributions.slice(0, 3).map((c) => lowerFirst(hazardName(app.catalogue, c.hazard))).join(', ')}.</p>
      {/if}
      <footer class="gauge__foot">
        <ExplainButton kind="bucket" id={bucket.id} />
        <Sources ids={[...bucket.sources, ...(bucket.relief?.sources ?? [])]} />
      </footer>
    {/if}
  </article>
{/if}

<style>
  .gauge {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .gauge__name {
    font-size: var(--text-base);
    margin: 0;
  }
  .gauge__target {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 var(--s2);
  }
  .big {
    font-size: var(--text-xl);
    font-weight: 700;
  }
  .range {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .meter {
    height: 0.75rem;
    border-radius: 999px;
    background: var(--gauge-track);
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .meter__fill {
    height: 100%;
    background: var(--gauge-fill);
    border-radius: 999px;
  }
  .meter__fill--done {
    background: var(--gauge-over);
  }
  .gauge__covered {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--s1);
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .gauge__covered.is-done {
    color: var(--good);
  }
  .gauge__relief {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    margin: 0;
    padding: var(--s2) var(--s3);
    background: var(--surface-2);
    border-radius: var(--r1);
  }
  .gauge__foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
    margin-top: auto;
  }
  .compact {
    padding: var(--s2) 0;
    border-bottom: 1px solid var(--border);
  }
</style>
