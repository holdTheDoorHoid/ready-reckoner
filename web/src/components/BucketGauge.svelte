<!--
  A duration bucket: the target with its range ("about 3 days (2–5)"), a meter with what the
  household has now (`covered_today`: what it owns and has checked off) and, lighter and striped,
  where the plan takes it once every step is done (`covered`), with both in words beside it, the
  relief rating, and what drives it. A target of 0 days means the bucket does not apply at these
  settings, and says so instead of drawing an empty meter.
-->
<script lang="ts">
  import type { BucketAssessment, Target } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { dayPhrase, targetDays } from '../lib/format';
  import { hazardName, lowerFirst } from '../lib/lookup';
  import ExplainButton from './ExplainButton.svelte';
  import Icon from './Icon.svelte';
  import Sources from './Sources.svelte';

  let { bucket, compact = false }: { bucket: BucketAssessment; compact?: boolean } = $props();
  const app = useApp();
  const uid = $props.id();

  const days = (t: Target) => (t.kind === 'days' ? t.value : 0);
  const target = $derived(bucket.target.kind === 'days' ? bucket.target : null);
  const notNeeded = $derived(target !== null && target.value <= 0);
  const planned = $derived(days(bucket.covered));
  const now = $derived(Math.min(days(bucket.covered_today), planned));
  const doneNow = $derived(target !== null && !notNeeded && now >= target.value);
  const donePlan = $derived(target !== null && !notNeeded && planned >= target.value);
  const pct = (d: number) => (target && !notNeeded ? Math.min(100, (d / target.value) * 100) : 0);

  /** "2 of 3 days", "half a day of 3 days", "1 week of 2 weeks": the unit said once when both share it. */
  function ofTarget(d: number, t: number): string {
    const tp = dayPhrase(t);
    if (d <= 0) return `0 of ${tp}`;
    const dp = dayPhrase(d);
    const [dn, du] = [dp.split(' ')[0], dp.split(' ').slice(1).join(' ')];
    const tu = tp.split(' ').slice(1).join(' ');
    return d !== 0.5 && du.replace(/s$/, '') === tu.replace(/s$/, '') ? `${dn} of ${tp}` : `${dp} of ${tp}`;
  }
  const nowText = $derived.by(() => {
    if (!target || notNeeded) return '';
    return doneNow ? `You have all ${dayPhrase(target.value)} now` : `You have ${ofTarget(now, target.value)} now`;
  });
  const planText = $derived.by(() => {
    if (!target || notNeeded || doneNow) return '';
    return donePlan ? `Your plan covers all ${dayPhrase(target.value)}` : `Your plan covers ${ofTarget(planned, target.value)}`;
  });
  const targetText = $derived(target ? targetDays(target.value, target.low, target.high) : '');
  const mainText = $derived(target ? `about ${dayPhrase(target.value)}` : '');
</script>

{#if target}
  <article class="gauge" class:card={!compact} class:compact aria-labelledby="{uid}-name" data-bucket={bucket.id} data-target={JSON.stringify(bucket.target)}>
    <h3 id="{uid}-name" class="gauge__name">{bucket.name}</h3>
    {#if notNeeded}
      <p class="gauge__target"><span class="big">Not needed</span></p>
      <p class="small muted">At your settings this does not apply to your home, so there is nothing to prepare for it.</p>
    {:else}
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
        aria-valuenow={Math.min(now, target.value)}
        aria-valuetext={planText ? `${nowText}. ${planText}.` : nowText}
      >
        {#if planned > now}<div class="meter__plan" style:width="{pct(planned)}%"></div>{/if}
        <div class="meter__fill" class:meter__fill--done={doneNow} style:width="{pct(now)}%"></div>
      </div>
      <p class="gauge__covered" class:is-done={doneNow}>
        {#if doneNow}<Icon name="check" />{/if}
        <span>{nowText}</span>
      </p>
      {#if planText}
        <p class="gauge__plan small">
          <span class="gauge__swatch" aria-hidden="true"></span>
          <span>{planText}</span>
        </p>
      {/if}
    {/if}
    {#if compact}
      <Sources ids={[...bucket.sources, ...(bucket.relief?.sources ?? [])]} variant="inline" what="{bucket.name}: be ready for {targetText}" />
    {/if}
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
        <Sources ids={[...bucket.sources, ...(bucket.relief?.sources ?? [])]} what={bucket.name} />
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
    position: relative;
    height: 0.75rem;
    border-radius: 999px;
    background: var(--gauge-track);
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .meter__fill,
  .meter__plan {
    position: absolute;
    inset: 0 auto 0 0;
    height: 100%;
    border-radius: 999px;
  }
  .meter__fill {
    background: var(--gauge-fill);
  }
  /* Where the plan takes you: striped, so it reads without colour. */
  .meter__plan,
  .gauge__swatch {
    background: repeating-linear-gradient(135deg, var(--gauge-plan) 0 3px, transparent 3px 6px);
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
  .gauge__plan {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--s1);
    color: var(--text-muted);
  }
  .gauge__swatch {
    display: inline-block;
    width: 1.25rem;
    height: 0.75rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    flex: none;
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
