<!--
  Screen 0, Start: what this is, the privacy promise, and three ways in: continue a saved plan,
  start a new one (with two optional questions), or open a saved plan file.
-->
<script lang="ts">
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import type { Problem, Stage } from '../engine/types';
  import { STAGES } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { CONFIDENCE_QUESTION, CONFIDENCE_SCALE, STAGE } from '../lib/labels';
  import { engineInput, parseImport, STEP_IDS, type SavedPlan } from '../lib/persistence';
  import { parseHash, useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();

  let stage = $state<Stage | ''>('');
  let confidence = $state<number | undefined>(undefined);
  let importError = $state('');
  let fileInput: HTMLInputElement | undefined = $state();
  let confirmReplace = $state(false);
  let pendingImport = $state<SavedPlan | null>(null);

  const county = $derived(app.result.output ? `${app.result.output.location.county_name}, ${app.result.output.location.state_abbr}` : '');
  const resumeHash = $derived.by(() => {
    const plan = app.plan;
    if (!plan) return '#/where';
    const last = plan.progress.last ? parseHash(plan.progress.last) : null;
    if (last && last.id !== 'start' && last.id !== 'missing') return plan.progress.last!;
    const next = STEP_IDS.find((s) => !plan.progress.completed.includes(s));
    return next ? `#/${next}` : '#/plan';
  });

  async function startNew() {
    await app.startNew();
    if (app.plan) {
      if (stage) app.plan.input.stage = stage;
      if (confidence !== undefined) app.plan.confidence.before = confidence;
    }
    router.go('where');
  }

  async function readFile(file: File) {
    importError = '';
    const parsed = parseImport(await file.text());
    if (!parsed.ok) {
      importError = parsed.reason;
      return;
    }
    if (app.engine) {
      const check = await app.engine.assess(engineInput(parsed.plan));
      if (!check.ok && check.error.code === 'bad_input') {
        const problems = (check.error.details as { problems?: Problem[] } | undefined)?.problems ?? [];
        const structural = problems.filter((p) => p.code === 'schema');
        if (structural.length > 0) {
          importError = `The household details in this file don't match what Ready Reckoner expects (${structural.length} ${structural.length === 1 ? 'problem' : 'problems'}, first: ${structural[0]!.field || 'the file'}: ${structural[0]!.message})`;
          return;
        }
      }
    }
    if (app.plan) {
      pendingImport = parsed.plan;
      confirmReplace = true;
    } else {
      finishImport(parsed.plan);
    }
  }

  function finishImport(plan: SavedPlan) {
    app.load(plan);
    const done = STEP_IDS.every((s) => plan.progress.completed.includes(s));
    router.go(done ? 'plan' : 'where');
  }
</script>

<div class="page page--narrow start">
  <h1 id="page-title" tabindex="-1">Get ready for what is likely where you live</h1>
  <p class="lead">
    Ready Reckoner works out what is most likely to disrupt your household, what it would do to you, and what to prepare first with
    the money you have. You get a month-by-month plan and a packet to print.
  </p>

  {#if app.plan}
    <section class="card resume" aria-labelledby="resume-title">
      <h2 id="resume-title">Welcome back</h2>
      <p>Your plan is saved on this device{county ? ` for ${county}` : ''}.</p>
      <p class="button-row"><a class="button button--primary" href={resumeHash}>Continue your plan <Icon name="chevron-right" /></a></p>
    </section>
  {/if}

  <section class="begin" aria-labelledby="new-title">
    <h2 id="new-title" class="visually-hidden">{app.plan ? 'Start a new plan or open a file' : 'Start'}</h2>
    <div class="button-row">
      {#if app.plan}
        <button type="button" class="button" onclick={() => (confirmReplace = true)}>Start a new plan</button>
      {:else}
        <button type="button" class="button button--primary button--big" onclick={startNew}>Start a plan <Icon name="chevron-right" /></button>
      {/if}
      <button type="button" class="button" onclick={() => fileInput?.click()}><Icon name="upload" /> Open a saved plan</button>
    </div>
    <p class="small muted">
      About ten minutes, in five short steps: where you live, who lives with you, how you get around, money, and what you already have.
      Stop whenever you like; your answers are saved on this device as you go.
    </p>
    <input
      bind:this={fileInput}
      class="visually-hidden"
      type="file"
      accept="application/json,.json"
      tabindex="-1"
      aria-hidden="true"
      onchange={(e) => {
        const file = (e.currentTarget as HTMLInputElement).files?.[0];
        if (file) void readFile(file);
        (e.currentTarget as HTMLInputElement).value = '';
      }}
    />
    {#if importError}<p class="error-text" role="alert"><Icon name="alert" /><span>{importError}</span></p>{/if}
    <details class="optional-questions">
      <summary>Two optional questions before you start</summary>
      <fieldset class="field-block">
        <legend>Where are you with preparing today?</legend>
        <p class="help">This only changes the wording, never the numbers.</p>
        <div class="choices">
          {#each STAGES as s (s)}
            <label class="choice"><input type="radio" name="stage" value={s} checked={stage === s} onchange={() => (stage = s)} /><span class="choice__text">{STAGE[s].label}</span></label>
          {/each}
        </div>
      </fieldset>
      <fieldset class="field-block">
        <legend>{CONFIDENCE_QUESTION}</legend>
        <p class="help">We ask again when your plan is ready, so you can see the change.</p>
        <div class="scale">
          {#each [1, 2, 3, 4, 5] as n (n)}
            <label class="choice scale__option">
              <input type="radio" name="confidence-before" value={n} checked={confidence === n} onchange={() => (confidence = n)} />
              <span class="choice__text"><span class="scale__num">{n}</span><span class="choice__help">{CONFIDENCE_SCALE[n]}</span></span>
            </label>
          {/each}
        </div>
      </fieldset>
      <p class="small muted">Your answers are kept with your plan when you press {app.plan ? '"Start a new plan"' : '"Start a plan"'}.</p>
    </details>
  </section>

  <section class="card promise" aria-labelledby="promise-title">
    <h2 id="promise-title"><Icon name="lock" /> Private by design</h2>
    <ul>
      <li>Everything you enter stays on this device. There is no account and no tracking.</li>
      <li>Nothing is sent anywhere. Once loaded, it works without an internet connection.</li>
      <li>You can save your plan as a file, and delete it from this browser at any time.</li>
    </ul>
  </section>

  <section aria-labelledby="get-title">
    <h2 id="get-title">What you get</h2>
    <ul class="get-list">
      <li><strong>Your risks, ranked</strong>: how often each one reaches households like yours, in plain numbers, with sources.</li>
      <li><strong>How long to be ready for</strong>: days without power, water or stores, for your household.</li>
      <li><strong>A month-by-month plan</strong> that starts with free steps and spends only your budget.</li>
      <li><strong>A printable packet</strong> with checklists, a family plan and a maintenance calendar.</li>
    </ul>
  </section>
</div>

<ConfirmDialog
  bind:open={confirmReplace}
  title="Replace your saved plan?"
  confirmLabel={pendingImport ? 'Open the file' : 'Start a new plan'}
  cancelLabel="Keep my plan"
  onconfirm={() => {
    if (pendingImport) {
      const plan = pendingImport;
      pendingImport = null;
      finishImport(plan);
    } else {
      void startNew();
    }
  }}
>
  <p>Your current plan on this device will be replaced. If you want a copy, save it first from "Keep it up".</p>
</ConfirmDialog>

<style>
  .promise h2 {
    margin-top: 0;
    display: flex;
    gap: var(--s2);
    align-items: center;
  }
  .promise,
  .resume {
    margin: var(--s5) 0;
  }
  .begin {
    margin: var(--s5) 0;
  }
  .begin .button-row {
    margin-bottom: var(--s3);
  }
  :global(.button--big) {
    min-height: 3.25rem;
    padding: 0.75rem 1.5rem;
    font-size: var(--text-lg);
  }
  .resume h2 {
    margin-top: 0;
  }
  .field-block {
    margin: var(--s4) 0;
  }
  .optional-questions {
    margin-bottom: var(--s4);
  }
  .scale {
    display: grid;
    gap: var(--s2);
    grid-template-columns: repeat(5, minmax(0, 1fr));
  }
  .scale__option {
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: var(--s2);
  }
  .scale__option .choice__text {
    align-items: center;
  }
  .scale__num {
    font-weight: 750;
    font-size: var(--text-lg);
  }
  @media (max-width: 34rem) {
    .scale {
      grid-template-columns: 1fr;
    }
    .scale__option {
      flex-direction: row;
      text-align: left;
    }
    .scale__option .choice__text {
      flex-direction: row;
      gap: var(--s2);
      align-items: baseline;
    }
  }
  .get-list li + li {
    margin-top: var(--s2);
  }
</style>
