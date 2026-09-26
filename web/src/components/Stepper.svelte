<!-- A small count with minus and plus buttons, for pets and similar. -->
<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    label,
    help,
    value,
    onchange,
    min = 0,
    max = 255,
    noun = 'one',
  }: {
    label: string;
    help?: string;
    value: number;
    onchange: (value: number) => void;
    min?: number;
    max?: number;
    /** For the button names: "One fewer {noun}". */
    noun?: string;
  } = $props();

  const uid = $props.id();

  function set(n: number) {
    onchange(Math.min(max, Math.max(min, Math.round(n))));
  }
</script>

<div class="stepper">
  <div class="stepper__label">
    <label for="{uid}-input">{label}</label>
    {#if help}<span class="help" id="{uid}-help">{help}</span>{/if}
  </div>
  <div class="stepper__control">
    <button type="button" class="button button--small" onclick={() => set(value - 1)} disabled={value <= min} aria-label="One fewer {noun}">
      <Icon name="minus" />
    </button>
    <input
      id="{uid}-input"
      class="input num"
      type="text"
      inputmode="numeric"
      aria-describedby={help ? `${uid}-help` : undefined}
      value={value}
      onchange={(e) => {
        const n = Number((e.currentTarget as HTMLInputElement).value);
        if (Number.isFinite(n)) set(n);
        else (e.currentTarget as HTMLInputElement).value = String(value);
      }}
    />
    <button type="button" class="button button--small" onclick={() => set(value + 1)} disabled={value >= max} aria-label="One more {noun}">
      <Icon name="plus" />
    </button>
  </div>
</div>

<style>
  .stepper {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    padding: var(--s3) 0;
    border-bottom: 1px solid var(--border);
  }
  .stepper__label {
    flex: 1 1 12rem;
  }
  .stepper__label .help {
    margin-bottom: 0;
  }
  .stepper__control {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .stepper__control .input {
    width: 4rem;
    text-align: center;
  }
  .stepper__control .button {
    min-width: var(--tap);
    min-height: var(--tap);
  }
</style>
