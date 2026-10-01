<!--
  Opening a protected plan file (DESIGN-DELTA-v3 §7): asks for its passphrase, says plainly when the
  passphrase does not open the file (and lets the person try again), and explains once that a
  forgotten passphrase cannot be recovered, so the printed binder is the backup. The passphrase is
  used once to open the file and never kept.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import type { SavedPlan } from '../lib/persistence';
  import { type EncryptedPlanFile, openProtected } from '../lib/protect';
  import Icon from './Icon.svelte';

  let {
    file = $bindable(null),
    onopened,
    onerror,
  }: {
    /** The protected file to open; the dialog shows while it is set, and clears it when done. */
    file?: EncryptedPlanFile | null;
    onopened: (plan: SavedPlan) => void;
    /** The file opened but is not a plan this version can read, or it is damaged. */
    onerror: (reason: string) => void;
  } = $props();

  const uid = $props.id();
  let dialog: HTMLDialogElement | undefined = $state();
  let field: HTMLInputElement | undefined = $state();
  let passphrase = $state('');
  let wrong = $state(false);
  let busy = $state(false);

  $effect(() => {
    if (!dialog) return;
    if (file && !dialog.open) {
      passphrase = '';
      wrong = false;
      dialog.showModal();
      void tick().then(() => field?.focus());
    } else if (!file && dialog.open) {
      dialog.close();
    }
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!file || busy) return;
    busy = true;
    const result = await openProtected(file, passphrase);
    busy = false;
    if (result.kind === 'wrong_passphrase') {
      wrong = true;
      await tick();
      field?.select();
      field?.focus();
      return;
    }
    passphrase = '';
    file = null;
    if (result.kind === 'plan') onopened(result.plan);
    else if (result.kind === 'error') onerror(result.reason);
  }

  function cancel() {
    passphrase = '';
    file = null;
  }
</script>

<dialog bind:this={dialog} aria-labelledby="{uid}-title" aria-describedby="{uid}-about" onclose={cancel}>
  <form onsubmit={submit}>
    <h2 id="{uid}-title"><Icon name="lock" /> Open a protected plan</h2>
    <p id="{uid}-about">
      This plan file is protected with a passphrase. Nobody can recover a forgotten passphrase, not even Ready Reckoner; if it is lost, your
      printed binder is your copy.
    </p>
    <div class="field">
      <label for="{uid}-passphrase">Passphrase</label>
      <input
        bind:this={field}
        bind:value={passphrase}
        id="{uid}-passphrase"
        class="input"
        type="password"
        autocomplete="off"
        spellcheck="false"
        aria-invalid={wrong}
        aria-describedby={wrong ? `${uid}-wrong` : undefined}
        oninput={() => (wrong = false)}
      />
      {#if wrong}
        <p class="error-text" id="{uid}-wrong" role="alert"><Icon name="alert" /><span>That passphrase does not open this file. Check it and try again: capital letters and spaces count.</span></p>
      {/if}
    </div>
    <div class="button-row">
      <button type="button" class="button" onclick={cancel}>Cancel</button>
      <button type="submit" class="button button--primary" disabled={busy || passphrase === ''}>{busy ? 'Opening…' : 'Open the file'}</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    max-width: min(32rem, calc(100vw - 2rem));
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
    display: flex;
    gap: var(--s2);
    align-items: center;
  }
  .field {
    margin-bottom: var(--s4);
  }
  .field label {
    display: block;
    margin-bottom: var(--s1);
  }
</style>
