<!--
  A one-line text answer with its label and help (the family plan's free text). What is typed is
  saved as it is typed; leaving the field tidies it (`onleave`). `maxlength` is the engine's limit,
  so nothing typed is ever cut later.
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
  } = $props();
</script>

<Field {id} {label} {help}>
  {#snippet children({ describedBy })}
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
  {/snippet}
</Field>
