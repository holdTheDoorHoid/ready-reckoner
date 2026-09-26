<!-- A native modal dialog for a choice that cannot be undone. The safe choice has focus first. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    open = $bindable(false),
    title,
    confirmLabel,
    cancelLabel = 'Cancel',
    onconfirm,
    children,
  }: {
    open?: boolean;
    title: string;
    confirmLabel: string;
    cancelLabel?: string;
    onconfirm: () => void;
    children: Snippet;
  } = $props();

  const uid = $props.id();
  let dialog: HTMLDialogElement | undefined = $state();
  let cancel: HTMLButtonElement | undefined = $state();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      cancel?.focus();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });
</script>

<dialog bind:this={dialog} aria-labelledby="{uid}-title" onclose={() => (open = false)}>
  <h2 id="{uid}-title">{title}</h2>
  {@render children()}
  <div class="button-row">
    <button type="button" class="button" bind:this={cancel} onclick={() => (open = false)}>{cancelLabel}</button>
    <button
      type="button"
      class="button button--danger"
      onclick={() => {
        onconfirm();
        open = false;
      }}>{confirmLabel}</button
    >
  </div>
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
  }
</style>
