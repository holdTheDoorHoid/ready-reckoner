<!-- A single yes/no answer as a tappable row with an optional explanation. -->
<script lang="ts">
  let {
    label,
    help,
    checked = $bindable(false),
    disabled = false,
    onchange,
  }: { label: string; help?: string; checked?: boolean; disabled?: boolean; onchange?: (checked: boolean) => void } = $props();
</script>

<label class="choice check-row" class:disabled>
  <input
    type="checkbox"
    bind:checked
    {disabled}
    onchange={(e) => onchange?.((e.currentTarget as HTMLInputElement).checked)}
  />
  <span class="choice__text">
    <span>{label}</span>
    {#if help}<span class="choice__help">{help}</span>{/if}
  </span>
</label>

<style>
  .check-row {
    margin-bottom: var(--s2);
  }
  .disabled {
    opacity: 0.7;
    cursor: default;
  }
</style>
