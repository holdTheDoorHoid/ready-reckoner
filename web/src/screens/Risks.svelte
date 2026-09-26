<!--
  Screen 6, Your risks: the register and what it means for this household. Settings (the dials)
  sit in a drawer at the top and re-run the assessment as they change. Directly under them, the
  risk matrix lists every hazard in one table, most likely first, each name a jump to its card.
  Named scenarios appear only when the engine offers them. Duration buckets are gauges, readiness
  buckets are have/not-yet cards, money buckets are on a separate savings track, and rare
  catastrophic hazards have their own box. Every hazard is shown with what in the plan answers it.
-->
<script lang="ts">
  import BucketGauge from '../components/BucketGauge.svelte';
  import CheckRow from '../components/CheckRow.svelte';
  import ChoiceGroup from '../components/ChoiceGroup.svelte';
  import CountyMap from '../components/CountyMap.svelte';
  import Dial from '../components/Dial.svelte';
  import HazardCard from '../components/HazardCard.svelte';
  import Icon from '../components/Icon.svelte';
  import PlanGate from '../components/PlanGate.svelte';
  import RareBox from '../components/RareBox.svelte';
  import ReadinessCard from '../components/ReadinessCard.svelte';
  import RiskMatrix from '../components/RiskMatrix.svelte';
  import SavingsTrack from '../components/SavingsTrack.svelte';
  import ScenarioToggle from '../components/ScenarioToggle.svelte';
  import Sources from '../components/Sources.svelte';
  import Warning from '../components/Warning.svelte';
  import type { ClimateHorizon, Dials, PlanItem, PlanOutput, ReturnPeriod, WaterLevel } from '../engine/types';
  import { CLIMATE_HORIZONS, WATER_LEVELS } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { dayPhrase, targetDays } from '../lib/format';
  import { helpsFor } from '../lib/helps';
  import { CLIMATE, dialSentence, HORIZONS, RETURN_PERIOD, stageLine, WATER_LEVEL } from '../lib/labels';
  import { allPlanItems } from '../lib/lookup';
  import { href } from '../lib/router.svelte';
  import type { ComparisonRow } from '../lib/ui-types';

  const app = useApp();
  let settingsOpen = $state(false);
  let showAll = $state(false);
  let announcement = $state('');
  let comparisons = $state<Record<string, ComparisonRow[]>>({});

  const dials = $derived(app.plan?.input.dials);
  const years = $derived(dials?.horizon_years ?? 10);

  function setDial<K extends 'return_period' | 'climate' | 'water_level' | 'horizon_years'>(key: K, value: NonNullable<Dials[K]>, words: string) {
    if (!app.plan) return;
    app.plan.input.dials[key] = value;
    announcement = `Updated for ${words}.`;
  }

  function setScenario(id: string, on: boolean) {
    if (!app.plan) return;
    const d = app.plan.input.dials;
    d.scenario_overrides = [...(d.scenario_overrides ?? []).filter((t) => t.id !== id), { id, on }];
    announcement = on ? 'Scenario switched on; targets updated.' : 'Scenario switched off; targets updated.';
  }

  function setRareCatastrophicOptIn(on: boolean) {
    if (!app.plan) return;
    app.plan.input.dials.rare_catastrophic_opt_in = on;
    announcement = on ? 'Rare-catastrophe budget allowed.' : 'Rare-catastrophe budget switched off.';
  }

  function bucketItems(output: PlanOutput, bucketId: string): PlanItem[] {
    const seen = new Set<string>();
    return allPlanItems(output)
      .map((x) => x.item)
      .filter((i) => i.buckets.includes(bucketId as PlanItem['buckets'][number]))
      .filter((i) => (seen.has(i.item_id) ? false : (seen.add(i.item_id), true)));
  }

  // The plan with and without each named scenario: one extra assessment per scenario, flipped.
  let compareSeq = 0;
  $effect(() => {
    const output = app.result.output;
    const input = app.engineInput;
    const engine = app.engine;
    if (!output || !input || !engine || output.scenarios.length === 0) {
      comparisons = {};
      return;
    }
    const seq = ++compareSeq;
    void (async () => {
      const next: Record<string, ComparisonRow[]> = {};
      for (const s of output.scenarios) {
        const flipped = JSON.parse(JSON.stringify(input)) as typeof input;
        flipped.dials.scenario_overrides = [...(flipped.dials.scenario_overrides ?? []).filter((t) => t.id !== s.id), { id: s.id, on: !s.on }];
        const r = await engine.assess(flipped);
        if (!r.ok) continue;
        const rows: ComparisonRow[] = [];
        for (const b of output.buckets) {
          const other = r.value.buckets.find((x) => x.id === b.id);
          if (b.target.kind !== 'days' || other?.target.kind !== 'days' || other.target.value === b.target.value) continue;
          const mine = dayPhrase(b.target.value);
          const theirs = dayPhrase(other.target.value);
          rows.push({ bucket: b.name, with: s.on ? mine : theirs, without: s.on ? theirs : mine });
        }
        next[s.id] = rows;
      }
      if (seq === compareSeq) comparisons = next;
    })();
  });
</script>

<div class="page">
  <h1 id="page-title" tabindex="-1">Your risks</h1>
  <PlanGate>
    {#snippet children(output)}
      {@const ranked = output.register.filter((h) => h.display === 'ranked')}
      {@const rare = output.register.filter((h) => h.display === 'rare_catastrophic')}
      {@const duration = output.buckets.filter((b) => b.target.kind === 'days')}
      {@const readiness = output.buckets.filter((b) => (b.target.kind === 'readiness' || b.target.kind === 'evacuate') && b.id !== 'home_loss')}
      {@const warnings = output.warnings}
      <div class="intro">
        <div>
          <p class="lead">
            For {output.location.county_name}, {output.location.state_name}: what is most likely to disrupt a household like yours, and how long
            to be ready for. {stageLine(app.plan?.input.stage)}
          </p>
          {#if dials}
            <section class="settings card" aria-labelledby="settings-title">
              <div class="settings__summary">
                <div>
                  <h2 id="settings-title">Your settings</h2>
                  <p class="small">
                    Ready for <strong>{RETURN_PERIOD[dials.return_period].label.toLowerCase()} events ({RETURN_PERIOD[dials.return_period].jargon})</strong>;
                    {CLIMATE[dials.climate].label.toLowerCase()}; {WATER_LEVEL[dials.water_level ?? 'basic'].label.toLowerCase()} water; chances over
                    {dials.horizon_years} {dials.horizon_years === 1 ? 'year' : 'years'}.
                  </p>
                </div>
                <button
                  type="button"
                  class="button"
                  aria-expanded={settingsOpen}
                  aria-controls="settings-panel"
                  onclick={() => (settingsOpen = !settingsOpen)}
                >
                  {settingsOpen ? 'Close settings' : 'Change settings'}
                </button>
              </div>
              <div id="settings-panel" class="settings__panel" hidden={!settingsOpen}>
                <Dial value={dials.return_period} onchange={(rp: ReturnPeriod) => setDial('return_period', rp, `${RETURN_PERIOD[rp].label} (${RETURN_PERIOD[rp].jargon})`)} />
                <div class="settings__grid">
                  <ChoiceGroup
                    legend="Climate"
                    name="climate"
                    help="Around 2050 uses projections for your region, labelled as such."
                    options={CLIMATE_HORIZONS.map((v) => ({ value: v, label: CLIMATE[v].label }))}
                    value={dials.climate}
                    onchange={(v: ClimateHorizon) => setDial('climate', v, CLIMATE[v].label)}
                    columns={2}
                  />
                  <ChoiceGroup
                    legend="Show chances over"
                    name="horizon"
                    options={HORIZONS.map((v) => ({ value: String(v), label: v === 1 ? '1 year' : `${v} years` }))}
                    value={String(dials.horizon_years)}
                    onchange={(v: string) => setDial('horizon_years', Number(v), `${v} years`)}
                    columns={3}
                  />
                </div>
                <ChoiceGroup
                  legend="Water per person"
                  name="water-level"
                  help="Official advice ranges widely because it measures different things."
                  options={WATER_LEVELS.map((v) => ({ value: v, label: WATER_LEVEL[v].label, help: WATER_LEVEL[v].help }))}
                  value={dials.water_level ?? 'basic'}
                  onchange={(v: WaterLevel) => setDial('water_level', v, `${WATER_LEVEL[v].label} water`)}
                  columns={3}
                />
                <CheckRow
                  label="Allow up to 10% of my budget for rare catastrophes (off by default)"
                  help="Covers items like a radiation meter, potassium iodide only on official instruction, or Faraday storage. See Rare but severe below."
                  checked={dials.rare_catastrophic_opt_in ?? false}
                  onchange={setRareCatastrophicOptIn}
                />
                <div class="settings__live" aria-hidden="true">
                  {#each duration.filter((b) => ['power', 'water_out', 'supplies'].includes(b.id)) as b (b.id)}
                    {#if b.target.kind === 'days'}
                      <p><span class="muted">{b.name}:</span> <strong>{targetDays(b.target.value, b.target.low, b.target.high)}</strong></p>
                    {/if}
                  {/each}
                </div>
              </div>
            </section>
          {/if}
          <RiskMatrix {ranked} {rare} {years} />
        </div>
        <CountyMap location={output.location} />
      </div>
      <p class="visually-hidden" aria-live="polite">{announcement}</p>

      {#if warnings.length}
        <section aria-label="Things to look at">
          {#each warnings as w (w.id)}<Warning warning={w} />{/each}
        </section>
      {/if}

      {#if output.scenarios.length}
        <section aria-labelledby="scenarios-title">
          <h2 id="scenarios-title">Named scenarios for your area</h2>
          <p class="section-intro">One big event can decide most of your targets here. You can plan with it or without it; both are reasonable.</p>
          <div class="scenarios">
            {#each output.scenarios as s (s.id)}
              <ScenarioToggle scenario={s} comparison={comparisons[s.id] ?? null} onchange={(on) => setScenario(s.id, on)} />
            {/each}
          </div>
        </section>
      {/if}

      <section aria-labelledby="likely-title">
        <h2 id="likely-title">Most likely to affect you</h2>
        <p class="section-intro">Ranked by how likely and how serious. Chances are for households like yours in your county.</p>
        <div class="featured">
          {#each ranked.slice(0, 3) as h (h.id)}
            <HazardCard hazard={h} featured helps={helpsFor(output, app.catalogue, h.id)} {years} backToTable />
          {/each}
        </div>
        {#if ranked.length > 3}
          <details class="all-risks" bind:open={showAll}>
            <summary>All {ranked.length} risks, ranked</summary>
            <div class="grid">
              {#each ranked.slice(3) as h (h.id)}
                <HazardCard hazard={h} helps={helpsFor(output, app.catalogue, h.id)} {years} backToTable />
              {/each}
            </div>
          </details>
        {/if}
      </section>

      <section aria-labelledby="days-title">
        <h2 id="days-title">How long to be ready for</h2>
        <p class="section-intro">
          The days your household should be able to manage for each kind of disruption, at your settings. The solid bar is what you have
          now; the striped bar is how much of it your plan covers once every step in it is done.
        </p>
        {#if dials}<p class="section-intro dial-sentence">{dialSentence(dials.return_period)}</p>{/if}
        <div class="grid">
          {#each duration as b (b.id)}<BucketGauge bucket={b} />{/each}
        </div>
      </section>

      <section aria-labelledby="ready-title">
        <h2 id="ready-title">Things to have ready</h2>
        <p class="section-intro">Some disruptions are not about days: you either have a way to deal with them or you don't.</p>
        <div class="grid">
          {#each readiness as b (b.id)}<ReadinessCard bucket={b} items={bucketItems(output, b.id)} />{/each}
        </div>
      </section>

      <RareBox hazards={rare} {years} backToTable />

      <SavingsTrack
        income={output.buckets.find((b) => b.id === 'income')}
        track={output.plan.savings_track}
        homeLoss={output.buckets.find((b) => b.id === 'home_loss')}
        homeItems={bucketItems(output, 'home_loss')}
        planningDate={app.plan?.input.planning_date}
        doneMonth={output.plan.done_month}
      />

      <section class="support card" aria-labelledby="support-title">
        <h2 id="support-title">If this feels like a lot</h2>
        <p>Reading about risks can be stressful, and that is normal. You can talk to someone at any hour:</p>
        <ul class="support__lines">
          <li>
            <strong>988 Suicide and Crisis Lifeline:</strong> call, text or chat 988. Support 24/7 for mental health, substance use and more.
            <Sources ids={['samhsa_988']} variant="inline" what="the 988 Lifeline" />
          </li>
          <li>
            <strong>Disaster Distress Helpline:</strong> call or text 1-800-985-5990. Toll-free, in many languages, 24/7, for anyone in the U.S.
            and its territories feeling distress after a natural or human-caused disaster.
            <Sources ids={['samhsa_disaster_distress']} variant="inline" what="the Disaster Distress Helpline" />
          </li>
        </ul>
      </section>

      <p class="next button-row">
        <a class="button button--primary" href={href('plan')}>See your plan <Icon name="chevron-right" /></a>
        <span class="small muted">Free steps first, then what to buy each month.</span>
      </p>
    {/snippet}
  </PlanGate>
</div>

<style>
  .intro {
    display: grid;
    gap: var(--s5);
    align-items: start;
  }
  @media (min-width: 60rem) {
    .intro {
      grid-template-columns: minmax(0, 1fr) 18rem;
    }
  }
  .settings {
    margin-top: var(--s4);
  }
  .settings__summary {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: var(--s3);
  }
  .settings__summary h2 {
    margin: 0 0 var(--s1);
    font-size: var(--text-lg);
  }
  .settings__summary p {
    margin: 0;
  }
  .settings__panel {
    margin-top: var(--s4);
    padding-top: var(--s4);
    border-top: 1px solid var(--border);
  }
  .settings__grid {
    display: grid;
    gap: 0;
  }
  .settings__live {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s5);
    padding: var(--s3) var(--s4);
    background: var(--surface-2);
    border-radius: var(--r2);
    font-size: var(--text-sm);
  }
  .settings__live p {
    margin: 0;
  }
  .scenarios {
    display: grid;
    gap: var(--s4);
    max-width: 48rem;
  }
  .featured {
    display: grid;
    gap: var(--s4);
  }
  @media (min-width: 64rem) {
    .featured {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  .all-risks {
    margin-top: var(--s4);
  }
  .all-risks .grid {
    margin-top: var(--s3);
  }
  .next {
    margin-top: var(--s6);
  }
  .support {
    margin-top: var(--s6);
    max-width: 48rem;
  }
  .support h2 {
    margin-top: 0;
    font-size: var(--text-lg);
  }
  .support__lines li + li {
    margin-top: var(--s2);
  }
</style>
