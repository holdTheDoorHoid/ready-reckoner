<!--
  A text answer with its label and help (the family plan's free text). What is typed is saved as it
  is typed; leaving the field tidies it (`onleave`). `maxlength` is the engine's limit, so nothing
  typed is ever cut later.

  `multiline` is for the longer notes: a box that grows with what is written, so a whole note can
  be read back. The note still prints on one line (a packet table cell), so Enter adds no line
  break and pasted line breaks become spaces.
-->
<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Field from './Field.svelte';

  let {
    id,
    label,
    help,
    value,
    maxlength,
    oninput,
    onleave,
    autocomplete = 'off',
    inputmode,
    width,
    placeholder,
    multiline = false,
    context,
  }: {
    id: string;
    label: string;
    help?: string;
    value: string | undefined;
    maxlength: number;
    oninput: (text: string) => void;
    onleave?: () => void;
    autocomplete?: HTMLInputAttributes['autocomplete'];
    inputmode?: HTMLInputAttributes['inputmode'];
    width?: 'short' | 'medium';
    placeholder?: string;
    multiline?: boolean;
    /** Words only screen readers hear after the label (see Field). */
    context?: string;
  } = $props();

  let box: HTMLTextAreaElement | undefined = $state();

  /** Grow the box to fit its text (jsdom reports no height, so tests leave it alone). */
  function fit() {
    if (!box || !box.scrollHeight) return;
    box.style.height = 'auto';
    box.style.height = `${box.scrollHeight + 2}px`;
  }

  $effect(() => {
    void value;
    fit();
  });

  function typed(el: HTMLTextAreaElement) {
    const text = el.value;
    const oneLine = text.replace(/[\r\n]+/g, ' ');
    if (oneLine !== text) {
      const at = el.selectionStart;
      el.value = oneLine;
      el.setSelectionRange(at, at);
    }
    oninput(oneLine);
    fit();
  }
</script>

<Field {id} {label} {help} {context}>
  {#snippet children({ describedBy })}
    {#if multiline}
      <textarea
        bind:this={box}
        {id}
        class="input text-field__note{width ? ` input--${width}` : ''}"
        rows="1"
        {maxlength}
        {autocomplete}
        {placeholder}
        aria-describedby={describedBy}
        value={value ?? ''}
        onkeydown={(e) => {
          if (e.key === 'Enter') e.preventDefault();
        }}
        oninput={(e) => typed(e.currentTarget as HTMLTextAreaElement)}
        onblur={() => onleave?.()}
      ></textarea>
    {:else}
      <input
        {id}
        class="input{width ? ` input--${width}` : ''}"
        type="text"
        {maxlength}
        {autocomplete}
        {inputmode}
        {placeholder}
        aria-describedby={describedBy}
        value={value ?? ''}
        oninput={(e) => oninput((e.currentTarget as HTMLInputElement).value)}
        onblur={() => onleave?.()}
      />
    {/if}
  {/snippet}
</Field>

<style>
  .text-field__note {
    resize: none;
    overflow: hidden;
    line-height: 1.45;
  }
</style>
