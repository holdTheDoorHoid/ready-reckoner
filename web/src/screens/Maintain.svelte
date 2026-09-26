<!--
  Screen 9, Keep it up: what to use and replace, check and practise, and when, from the dates you
  recorded; a calendar file for your own calendar (this app never contacts you); and your data:
  save a copy, open a saved copy, or forget everything.
-->
<script lang="ts">
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import type { Problem } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { addMonths, formatDate } from '../lib/format';
  import { calendarFile, drillItems, intervalLabel, maintenanceTasks, type Task } from '../lib/maintenance';
  import { engineInput, EXPORT_FILENAME, exportText, parseImport, type SavedPlan } from '../lib/persistence';
  import { useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();
  let confirmForget = $state(false);
  let confirmImport = $state(false);
  let pending = $state<SavedPlan | null>(null);
  let message = $state('');
  let importError = $state('');
  let fileInput: HTMLInputElement | undefined = $state();

  const today = $derived(app.today());
  const tasks = $derived(app.plan && app.catalogue ? maintenanceTasks($state.snapshot(app.plan) as SavedPlan, app.catalogue) : []);
  const due = $derived(tasks.filter((t) => t.due <= today));
  const soon = $derived(tasks.filter((t) => t.due > today && t.due <= addMonths(today, 12)));
  const planIds = $derived(new Set(app.result.output?.plan.months.flatMap((m) => m.items.map((i) => i.item_id)) ?? []));
  const drills = $derived(app.catalogue ? drillItems(app.catalogue, planIds) : []);

  function done(task: Task) {
    if (!app.plan) return;
    if (task.kind === 'review') app.plan.reviewed_on = today;
    app.markDone(task.key, today);
    message = `Marked as done today: ${task.title}.`;
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

  function exportPlan() {
    if (!app.plan) return;
    app.saveNow();
    download(EXPORT_FILENAME, 'application/json', exportText($state.snapshot(app.plan) as SavedPlan));
    message = `Saved a copy as ${EXPORT_FILENAME} in your downloads.`;
  }

  function exportCalendar() {
    download('ready-reckoner-calendar.ics', 'text/calendar', calendarFile(tasks, today));
    message = 'Saved ready-reckoner-calendar.ics. Open it to add the dates to your calendar.';
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
        const problems = ((check.error.details as { problems?: Problem[] } | undefined)?.problems ?? []).filter((p) => p.code === 'schema');
        if (problems.length) {
          importError = `The household details in this file don't match what Ready Reckoner expects (first problem: ${problems[0]!.field}: ${problems[0]!.message})`;
          return;
        }
      }
    }
    pending = parsed.plan;
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
  <p class="visually-hidden" aria-live="polite">{message}</p>

  {#if app.plan && app.catalogue}
    <section aria-labelledby="due-title">
      <h2 id="due-title">Due now</h2>
      {#if due.length === 0}
        <p class="card"><Icon name="check" /> Nothing is due. {soon[0] ? `Next: ${soon[0].title.toLowerCase()} on ${formatDate(soon[0].due)}.` : ''}</p>
      {:else}
        <ul class="tasks">
          {#each due as t (t.key)}
            <li class="card task">
              <div>
                <p class="task__title">{t.title}</p>
                <p class="small muted">
                  {intervalLabel(t.interval_months)}.
                  {#if t.last}Last done {formatDate(t.last)}.{:else if t.from_inventory}You had this before the plan; check its date.{/if}
                </p>
              </div>
              <button type="button" class="button button--small" onclick={() => done(t)}><Icon name="check" /> Done today<span class="visually-hidden">: {t.title}</span></button>
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
              <button type="button" class="button button--small" onclick={() => practised(d.id, d.name)}>Practised today<span class="visually-hidden">: {d.name}</span></button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section aria-labelledby="reminders-title">
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

  <section aria-labelledby="data-title" class="data">
    <h2 id="data-title">Your data</h2>
    <p>
      Your plan is kept in this browser only{app.storageAvailable ? '' : ' (this browser is not keeping it: private browsing or blocked storage, so save a copy)'}.
      {#if app.saveFailed}<strong>The last change could not be saved in this browser. Save a copy to keep it.</strong>{/if}
    </p>
    <div class="data__actions">
      <div>
        <button type="button" class="button" onclick={exportPlan} disabled={!app.plan}><Icon name="download" /> Save a copy of your plan</button>
        <p class="small muted">Saves {EXPORT_FILENAME}. Keep it somewhere safe; it holds your household details.</p>
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

<ConfirmDialog bind:open={confirmForget} title="Forget everything?" confirmLabel="Forget everything" cancelLabel="Keep my plan" onconfirm={forget}>
  <p>This deletes your household details, your plan, your check-offs and your settings from this browser. It cannot be undone.</p>
  <p>If you want a copy, cancel and choose "Save a copy of your plan" first. The app itself stays available offline.</p>
</ConfirmDialog>

<ConfirmDialog bind:open={confirmImport} title="Replace your plan with the file?" confirmLabel="Open the file" cancelLabel="Keep my plan" onconfirm={finishImport}>
  <p>Your current plan in this browser will be replaced by the one in the file.</p>
</ConfirmDialog>

<style>
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
