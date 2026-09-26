<!--
  A labelled form field: label, help text, and a validation message, wired to the control with
  aria-describedby and aria-invalid. The control itself is passed in as a snippet.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    id,
    label,
    help,
    error,
    optional = false,
    context,
    children,
  }: {
    id: string;
    label: string;
    help?: string;
    error?: string;
    optional?: boolean;
    /** Words only screen readers hear after the label, when the same label repeats ("of person 2"). */
    context?: string;
    children: Snippet<[{ id: string; describedBy: string | undefined; invalid: boolean }]>;
  } = $props();

  const describedBy = $derived([help ? `${id}-help` : '', error ? `${id}-error` : ''].filter(Boolean).join(' ') || undefined);
</script>

<div class="field">
  <label for={id}>{label}{#if context}<span class="visually-hidden">{' '}{context}</span>{/if}{#if optional}{' '}<span class="optional">(optional)</span>{/if}</label>
  {#if help}<span class="help" id="{id}-help">{help}</span>{/if}
  {@render children({ id, describedBy, invalid: !!error })}
  {#if error}
    <p class="error-text" id="{id}-error"><Icon name="alert" /><span><span class="visually-hidden">Problem: </span>{error}</span></p>
  {/if}
</div>

<style>
  .field {
    margin-bottom: var(--s5);
  }
  .field > label {
    display: block;
  }
  .optional {
    font-weight: 400;
    color: var(--text-muted);
  }
</style>
