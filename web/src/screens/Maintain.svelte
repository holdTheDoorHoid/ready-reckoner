<!--
  Screen 9, Keep it up: what to use and replace, check, test and practise, and when, from the dates
  you recorded; the seasonal anchors month by month (have it before the season starts, or check it
  then); a calendar file for your own calendar (this app never contacts you); and your data: save a
  copy, open a saved copy, or forget everything. "Tested today" records the day an item was tried
  (contract v2 `Owned.tested_on`), the same date the Have screen shows.

  Saving (DESIGN-DELTA-v3 §7): a plan with sensitive answers (medical details, insurance IDs, an
  address, accounts…) opens the save dialog, which protects the file with a passphrase unless the
  household unticks it; any other plan saves as a plain file at once, as before. Opening a protected
  file asks for its passphrase. "Forget everything" also deletes the maps store.
-->
<script lang="ts">
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import PassphraseDialog from '../components/PassphraseDialog.svelte';
  import SavePlanDialog from '../components/SavePlanDialog.svelte';
  import type { Problem } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { addMonths, formatDate } from '../lib/format';
  import { calendarFile, drillItems, intervalLabel, maintenanceTasks, SEASON_WORDS, seasonalAnchors, type Task } from '../lib/maintenance';
  import { engineInput, EXPORT_FILENAME, exportText, setTestedOn, type SavedPlan } from '../lib/persistence';
  import { type EncryptedPlanFile, hasSensitiveAnswers, readPlanFile } from '../lib/protect';
  import { useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();
  let confirmForget = $state(false);
  let confirmImport = $state(false);
  let pending = $state<SavedPlan | null>(null);
  let message = $state('');
  let importError = $state('');
  let fileInput: HTMLInputElement | undefined = $state();
  let saving = $state(false);
  /** A protected file waiting for its passphrase. */
  let locked = $state<EncryptedPlanFile | null>(null);

  const today = $derived(app.today());
  const tasks = $derived(app.plan && app.catalogue ? maintenanceTasks($state.snapshot(app.plan) as SavedPlan, app.catalogue) : []);
  const due = $derived(tasks.filter((t) => t.due <= today));
  const soon = $derived(tasks.filter((t) => t.due > today && t.due <= addMonths(today, 12)));
  const planIds = $derived(new Set(app.result.output?.plan.months.flatMap((m) => m.items.map((i) => i.item_id)) ?? []));
  const drills = $derived(app.catalogue ? drillItems(app.catalogue, planIds) : []);
  /** What the household has and what its plan still holds, for the seasonal view. */
  const heldOrPlanned = $derived(
    new Set([
      ...planIds,
      ...(app.result.output?.plan.long_horizon ?? []).map((i) => i.item_id),
      ...(app.plan?.input.existing ?? []).filter((o) => o.qty > 0).map((o) => o.item_id),
    ]),
  );
  const seasons = $derived(app.catalogue ? seasonalAnchors(app.catalogue, heldOrPlanned) : []);

  function done(task: Task) {
    if (!app.plan) return;
    if (task.kind === 'review') app.plan.reviewed_on = today;
    if (task.kind === 'test' && task.item_id) setTestedOn(app.plan, task.item_id, today);
    app.markDone(task.key, today);
    message = task.kind === 'test' ? `Tested today: ${task.title.replace(/^Test: /, '')}.` : `Marked as done today: ${task.title}.`;
  }

  /** The line under a task: how often, and when it was last done or tested. */
  function lastLine(t: Task): string {
    if (t.kind === 'test') return t.last ? `Last tested ${formatDate(t.last)}.` : 'Not tested yet: try it and record the day.';
    if (t.last) return `Last done ${formatDate(t.last)}.`;
    return t.from_inventory ? 'You had this before the plan; check its date.' : '';
  }

  function practised(itemId: string, name: string) {
    if (!app.plan) return;
    app.markDone(`check:${itemId}`, today);
    const planned = app.result.output?.plan.months.flatMap((m) => m.items).find((i) => i.item_id === itemId);
    if (planned && !planned.done) app.record(planned);
    message = `Practised today: ${name}.`;
  }

  function lastPractised(itemId: string): string | undefined {
    const plan = app.plan;
    if (!plan) return undefined;
    return plan.done_dates[`check:${itemId}`] ?? plan.purchases.find((p) => p.item_id === itemId)?.date;
  }

  function download(name: string, type: string, text: string) {
    const url = URL.createObjectURL(new Blob([text], { type }));
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function snapshot(): SavedPlan | null {
    return app.plan ? ($state.snapshot(app.plan) as SavedPlan) : null;
  }

  /** "Save a copy": the save dialog when the plan holds sensitive answers, otherwise the plain file at once. */
  function exportPlan() {
    const plan = snapshot();
    if (!plan) return;
    app.saveNow();
    if (hasSensitiveAnswers(plan)) {
      saving = true;
      return;
    }
    download(EXPORT_FILENAME, 'application/json', exportText(plan));
    message = `Saved a copy as ${EXPORT_FILENAME} in your downloads.`;
  }

  function saved(text: string, protectedFile: boolean) {
    download(EXPORT_FILENAME, 'application/json', text);
    message = protectedFile
      ? `Saved a protected copy as ${EXPORT_FILENAME} in your downloads. It opens only with your passphrase.`
      : `Saved a copy as ${EXPORT_FILENAME} in your downloads. It is not protected, so keep it somewhere safe.`;
  }

  function exportCalendar() {
    download('ready-reckoner-calendar.ics', 'text/calendar', calendarFile(tasks, today));
    message = 'Saved ready-reckoner-calendar.ics. Open it to add the dates to your calendar.';
  }

  async function readFile(file: File) {
    importError = '';
    const read = readPlanFile(await file.text());
    if (read.kind === 'error') {
      importError = read.reason;
      return;
    }
    if (read.kind === 'protected') {
      locked = read.file;
      return;
    }
    await accept(read.plan);
  }

  /** A plan read from a file (opened with its passphrase if it was protected): checked, then opened. */
  async function accept(plan: SavedPlan) {
    if (app.engine) {
      const check = await app.engine.assess(engineInput(plan));
      if (!check.ok && check.error.code === 'bad_input') {
        const problems = ((check.error.details as { problems?: Problem[] } | undefined)?.problems ?? []).filter((p) => p.code === 'schema');
        if (problems.length) {
          importError = `The household details in this file don't match what Ready Reckoner expects (first problem: ${problems[0]!.field}: ${problems[0]!.message})`;
          return;
        }
      }
    }
    pending = plan;
    if (app.plan) confirmImport = true;
    else finishImport();
  }

  function finishImport() {
    if (!pending) return;
    app.load(pending);
    pending = null;
    message = 'Plan opened from the file.';
  }

  function forget() {
    app.forget();
    router.go('start');
    app.status = 'Everything was deleted from this browser.';
  }
</script>

<div class="page page--narrow">
  <h1 id="page-title" tabindex="-1">Keep it up</h1>
  <p class="lead">A few minutes a month keeps your plan working: use and replace, check, and practise. Missing a month is fine.</p>
  {#if app.plan && app.catalogue}
    <p class="button-row no-print">
      <button type="button" class="button" onclick={() => window.print()}><Icon name="print" /> Print this calendar</button>
      <span class="small muted">What is due, the year ahead, the seasons and the drills, on one sheet to keep with your binder.</span>
    </p>
    <p class="print-only maintain-print-date">Ready Reckoner · Keep it up · printed {formatDate(today)}</p>
  {/if}
  <p class="visually-hidden" aria-live="polite">{message}</p>

  {#if app.plan && app.catalogue}
    <section aria-labelledby="due-title">
      <h2 id="due-title">Due now</h2>
      {#if due.length === 0}
        <p class="card"><Icon name="check" /> Nothing is due. {soon[0] ? `The next one is on ${formatDate(soon[0].due)}: ${soon[0].title}.` : ''}</p>
      {:else}
        <ul class="tasks">
          {#each due as t (t.key)}
            <li class="card task">
              <div>
                <p class="task__title">{t.title}</p>
                <p class="small muted">
                  {intervalLabel(t.interval_months)}.
                  {lastLine(t)}
                </p>
              </div>
              <button type="button" class="button button--small no-print" onclick={() => done(t)}
                ><Icon name="check" /> {t.kind === 'test' ? 'Tested today' : 'Done today'}<span class="visually-hidden">: {t.title}</span></button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section aria-labelledby="soon-title">
      <h2 id="soon-title">Coming up in the next year</h2>
      {#if soon.length === 0}
        <p>Nothing yet. As you check items off your plan, their rotation dates appear here.</p>
      {:else}
        <!-- Wide tables scroll sideways on phones; a focusable, labelled region lets keyboard users scroll it. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div class="table-wrap" tabindex="0" role="region" aria-label="Coming up">
          <table>
            <thead><tr><th scope="col">When</th><th scope="col">What</th><th scope="col">How often</th></tr></thead>
            <tbody>
              {#each soon as t (t.key)}
                <tr><td class="num">{formatDate(t.due)}</td><td>{t.title}</td><td>{intervalLabel(t.interval_months)}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    {#if seasons.length}
      <section aria-labelledby="seasons-title">
        <h2 id="seasons-title">Through the year</h2>
        <p class="section-intro">Some things belong to a season: have them ready before it starts, or check them then.</p>
        <ul class="seasons">
          {#each seasons as s (s.season)}
            <li class="card season">
              <p class="season__when"><strong>{SEASON_WORDS[s.season].month}</strong> <span class="muted">before {SEASON_WORDS[s.season].name}</span></p>
              <ul class="season__items">
                {#each s.items as i (i.id)}<li>{i.name}</li>{/each}
              </ul>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if drills.length}
      <section aria-labelledby="drills-title">
        <h2 id="drills-title">Practise</h2>
        <p class="section-intro">Ten-minute drills make the real thing automatic. They count toward your plan.</p>
        <ul class="tasks">
          {#each drills as d (d.id)}
            {@const last = lastPractised(d.id)}
            <li class="card task">
              <div>
                <p class="task__title">{d.name}</p>
                <p class="small muted">
                  {d.maintenance?.check_months ? `${intervalLabel(d.maintenance.check_months)}. ` : ''}{last ? `Last practised ${formatDate(last)}.` : 'Not practised yet.'}
                </p>
              </div>
              <button type="button" class="button button--small no-print" onclick={() => practised(d.id, d.name)}>Practised today<span class="visually-hidden">: {d.name}</span></button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section aria-labelledby="reminders-title" class="no-print">
      <h2 id="reminders-title">Reminders</h2>
      <p>
        This app never contacts you, so it cannot send reminders. Add the dates to your own calendar instead. The file lists item names and
        dates only, nothing about where you live or who is in your household.
      </p>
      <button type="button" class="button" onclick={exportCalendar} disabled={tasks.length === 0}><Icon name="calendar" /> Download calendar file (.ics)</button>
    </section>
  {:else if !app.plan}
    <p class="card">Start a plan first, and its maintenance calendar appears here.</p>
  {/if}

  <section aria-labelledby="data-title" class="data no-print">
    <h2 id="data-title">Your data</h2>
    <p>
      Your plan is kept in this browser only{app.storageAvailable ? '' : ' (this browser is not keeping it: private browsing or blocked storage, so save a copy)'}.
      {#if app.saveFailed}<strong>The last change could not be saved in this browser. Save a copy to keep it.</strong>{/if}
    </p>
    <div class="data__actions">
      <div>
        <button type="button" class="button" onclick={exportPlan} disabled={!app.plan}><Icon name="download" /> Save a copy of your plan</button>
        <p class="small muted">
          Saves {EXPORT_FILENAME}. Keep it somewhere safe: it can hold names, phone numbers, medical details, insurance IDs and your address.
          Once it does, you can protect it with a passphrase.
        </p>
      </div>
      <div>
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
        <button type="button" class="button" onclick={() => fileInput?.click()}><Icon name="upload" /> Open a saved plan</button>
        {#if importError}<p class="error-text" role="alert"><Icon name="alert" /><span>{importError}</span></p>{/if}
      </div>
      <div>
        <button type="button" class="button button--danger" onclick={() => (confirmForget = true)}><Icon name="trash" /> Forget everything</button>
        <p class="small muted">Deletes your plan and settings from this browser.</p>
      </div>
    </div>
  </section>
</div>

<SavePlanDialog bind:open={saving} plan={snapshot} onsave={saved} />
<PassphraseDialog bind:file={locked} onopened={(plan) => void accept(plan)} onerror={(reason) => (importError = reason)} />

<ConfirmDialog bind:open={confirmForget} title="Forget everything?" confirmLabel="Forget everything" cancelLabel="Keep my plan" onconfirm={forget}>
  <p>This deletes your household details, your plan, your check-offs, your maps and your settings from this browser. It cannot be undone.</p>
  <p>If you want a copy, cancel and choose "Save a copy of your plan" first. The app itself stays available offline.</p>
</ConfirmDialog>

<ConfirmDialog bind:open={confirmImport} title="Replace your plan with the file?" confirmLabel="Open the file" cancelLabel="Keep my plan" onconfirm={finishImport}>
  <p>Your current plan in this browser will be replaced by the one in the file.</p>
</ConfirmDialog>

<style>
  /* The Keep it up sheet (DESIGN-DELTA-v3 §6): printing this tab prints its calendar alone, compact,
     black on white; the buttons, the calendar file and Your data stay on the screen. */
  @media print {
    .maintain-print-date {
      font-size: 8.5pt;
      font-weight: 700;
      border-bottom: 1.5pt solid #000;
      padding-bottom: 3pt;
      margin: 0 0 8pt;
    }
    .card {
      border: 0.75pt solid #000 !important;
      box-shadow: none !important;
      padding: 4pt 6pt !important;
      break-inside: avoid;
    }
    h2 {
      font-size: 13pt;
      margin: 12pt 0 4pt;
    }
  }
  .tasks {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--s2);
  }
  .tasks li + li {
    margin-top: 0;
  }
  .task {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
  }
  .task p {
    margin: 0;
  }
  .task__title {
    font-weight: 650;
  }
  .seasons {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--s2);
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 14rem), 1fr));
  }
  .seasons > li + li {
    margin-top: 0;
  }
  .season {
    padding: var(--s3) var(--s4);
  }
  .season__when {
    margin: 0 0 var(--s1);
  }
  .season__items {
    margin: 0;
    padding-left: 1.2em;
  }
  .data {
    margin-top: var(--s7);
    padding-top: var(--s4);
    border-top: 1px solid var(--border);
  }
  .data__actions {
    display: grid;
    gap: var(--s4);
  }
  .data__actions p {
    margin: var(--s1) 0 0;
  }
</style>
