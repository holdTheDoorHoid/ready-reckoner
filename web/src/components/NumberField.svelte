<!--
  A number typed as text (no spinner surprises), with its own validation message. The saved value
  only changes when what is typed is a valid number; until then the message says what to fix.
-->
<script lang="ts">
  import Field from './Field.svelte';

  let {
    id,
    label,
    help,
    value,
    onchange,
    min = 0,
    max,
    integer = false,
    optional = false,
    prefix,
    suffix,
    error,
    example = '12',
    width = 'short',
  }: {
    id: string;
    label: string;
    help?: string;
    value: number | undefined;
    onchange: (value: number | undefined) => void;
    min?: number;
    max?: number;
    integer?: boolean;
    optional?: boolean;
    prefix?: string;
    suffix?: string;
    /** A problem reported by the engine for this field. */
    error?: string;
    example?: string;
    width?: 'short' | 'medium';
  } = $props();

  let text = $state('');
  let focused = $state(false);
  let touched = $state(false);

  $effect(() => {
    const v = value;
    if (!focused) text = v === undefined ? '' : String(v);
  });

  function problem(raw: string): string | undefined {
    const t = raw.trim().replace(/[$,]/g, '');
    if (t === '') return optional ? undefined : `Enter a number, like ${example}. Use 0 if none.`;
    const n = Number(t);
    if (!Number.isFinite(n)) return `Enter a number, like ${example}.`;
    if (integer && !Number.isInteger(n)) return 'Enter a whole number.';
    if (n < min) return min === 0 ? 'Enter 0 or more.' : `Enter ${min} or more.`;
    if (max !== undefined && n > max) return `Enter ${max} or less.`;
    return undefined;
  }

  const localError = $derived(touched ? problem(text) : undefined);

  function commit(raw: string) {
    const t = raw.trim().replace(/[$,]/g, '');
    if (problem(raw) !== undefined) return;
    onchange(t === '' ? undefined : Number(t));
  }
</script>

<Field {id} {label} {help} error={localError ?? error} {optional}>
  {#snippet children({ describedBy, invalid })}
    <div class="affix">
      {#if prefix}<span class="affix__text" aria-hidden="true">{prefix}</span>{/if}
      <input
        {id}
        class="input input--{width}"
        type="text"
        inputmode={integer ? 'numeric' : 'decimal'}
        autocomplete="off"
        aria-describedby={describedBy}
        aria-invalid={invalid}
        value={text}
        onfocus={() => (focused = true)}
        onblur={() => {
          focused = false;
          touched = true;
          if (problem(text) === undefined) text = value === undefined ? '' : String(value);
        }}
        oninput={(e) => {
          text = (e.currentTarget as HTMLInputElement).value;
          if (touched || problem(text) === undefined) commit(text);
        }}
      />
      {#if suffix}<span class="affix__text">{suffix}</span>{/if}
    </div>
  {/snippet}
</Field>

<style>
  .affix {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .affix__text {
    color: var(--text-muted);
    font-weight: 600;
  }
</style>
