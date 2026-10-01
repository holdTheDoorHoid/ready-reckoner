<!--
  "Save a copy of your plan" once the plan holds sensitive answers (DESIGN-DELTA-v3 §7): says what
  the file will hold, and offers "Protect this file with a passphrase", on by default, with a
  passphrase and its confirmation. Unticking it is allowed (warn, don't block): a one-sentence
  warning then says what anyone with the file could read. A forgotten passphrase cannot be
  recovered, so the dialog says the printed binder is the backup. The passphrase is used once to
  seal the file and never kept; the fields empty whenever the dialog closes.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import type { SavedPlan } from '../lib/persistence';
  import { exportText } from '../lib/persistence';
  import { canProtect, PASSPHRASE_MIN, protectedExportText } from '../lib/protect';
  import Icon from './Icon.svelte';

  let {
    open = $bindable(false),
    plan,
    onsave,
  }: {
    open?: boolean;
    /** The plan to save, read when "Save the file" is pressed. */
    plan: () => SavedPlan | null;
    /** The file's text, ready to download, and whether it is protected. */
    onsave: (text: string, protectedFile: boolean) => void;
  } = $props();

  const uid = $props.id();
  let dialog: HTMLDialogElement | undefined = $state();
  let protect = $state(true);
  let passphrase = $state('');
  let confirm = $state('');
  let tried = $state(false);
  let busy = $state(false);
  let failed = $state('');

  const available = canProtect();
  const tooShort = $derived(protect && [...passphrase].length < PASSPHRASE_MIN);
  const mismatch = $derived(protect && !tooShort && confirm !== passphrase);

  function reset() {
    passphrase = '';
    confirm = '';
    tried = false;
    busy = false;
    failed = '';
    protect = available;
  }

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      reset();
      dialog.showModal();
      void tick().then(() => document.getElementById(protect ? `${uid}-passphrase` : `${uid}-protect`)?.focus());
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    tried = true;
    if (tooShort || mismatch) {
      await tick();
      document.getElementById(tooShort ? `${uid}-passphrase` : `${uid}-confirm`)?.focus();
      return;
    }
    const current = plan();
    if (!current) return;
    busy = true;
    try {
      const text = protect ? await protectedExportText(current, passphrase) : exportText(current);
      onsave(text, protect);
      open = false;
    } catch {
      failed = 'The file could not be protected in this browser. Try again, or save it without protection and keep it somewhere safe.';
    } finally {
      busy = false;
      passphrase = '';
      confirm = '';
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="{uid}-title" onclose={() => (open = false)}>
  <form onsubmit={submit} novalidate>
    <h2 id="{uid}-title">Save a copy of your plan</h2>
    <p>The file will hold names, phone numbers, medical details, insurance IDs and your address.</p>
    {#if available}
      <label class="choice check-row">
        <input id="{uid}-protect" type="checkbox" bind:checked={protect} onchange={() => (tried = false)} />
        <span class="choice__text">
          <span>Protect this file with a passphrase</span>
          <span class="choice__help">Only someone who knows the passphrase can open the file.</span>
        </span>
      </label>
    {/if}
    {#if protect}
      <div class="field">
        <label for="{uid}-passphrase">Passphrase</label>
        <span class="help" id="{uid}-passphrase-help">At least {PASSPHRASE_MIN} characters. Three or four words you will remember work well.</span>
        <input
          bind:value={passphrase}
          id="{uid}-passphrase"
          class="input"
          type="password"
          autocomplete="new-password"
          spellcheck="false"
          aria-invalid={tried && tooShort}
          aria-describedby="{uid}-passphrase-help{tried && tooShort ? ` ${uid}-short` : ''}"
        />
        {#if tried && tooShort}
          <p class="error-text" id="{uid}-short"><Icon name="alert" /><span>Use at least {PASSPHRASE_MIN} characters.</span></p>
        {/if}
      </div>
      <div class="field">
        <label for="{uid}-confirm">Type the passphrase again</label>
        <input
          bind:value={confirm}
          id="{uid}-confirm"
          class="input"
          type="password"
          autocomplete="new-password"
          spellcheck="false"
          aria-invalid={tried && mismatch}
          aria-describedby={tried && mismatch ? `${uid}-mismatch` : undefined}
        />
        {#if tried && mismatch}
          <p class="error-text" id="{uid}-mismatch"><Icon name="alert" /><span>The two passphrases are not the same.</span></p>
        {/if}
      </div>
      <p class="small">Nobody can recover a forgotten passphrase, not even Ready Reckoner. Keep your printed binder as the backup.</p>
    {:else}
      <p class="warning" role="note">
        <Icon name="alert" />
        <span>
          {available
            ? 'Without a passphrase, anyone who gets the file can read everything in it.'
            : 'This browser cannot protect files, so anyone who gets this file can read everything in it.'}
        </span>
      </p>
    {/if}
    {#if failed}<p class="error-text" role="alert"><Icon name="alert" /><span>{failed}</span></p>{/if}
    <div class="button-row">
      <button type="button" class="button" onclick={() => (open = false)}>Cancel</button>
      <button type="submit" class="button button--primary" disabled={busy}><Icon name="download" /> {busy ? 'Protecting the file…' : 'Save the file'}</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    max-width: min(34rem, calc(100vw - 2rem));
    border: 1px solid var(--border);
    border-radius: var(--r3);
    padding: var(--s5);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-2);
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }
  h2 {
    margin-top: 0;
  }
  .field {
    margin-bottom: var(--s4);
  }
  .field label {
    display: block;
  }
  .check-row {
    margin-bottom: var(--s4);
  }
  .warning {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    padding: var(--s3);
    border-left: 4px solid var(--warning-edge, var(--border-strong));
    background: var(--warning-soft, var(--surface-2));
    border-radius: var(--r1);
  }
</style>
