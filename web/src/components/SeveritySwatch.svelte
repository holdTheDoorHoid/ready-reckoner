<!-- Severity as a word plus a patterned swatch: the pattern and the word carry it, colour only adds to it. -->
<script lang="ts">
  import { severityBand } from '../lib/format';
  let { severity }: { severity: number } = $props();
  const band = $derived(severityBand(severity));
</script>

<span class="severity">
  <span class="swatch sev-{band.level}" aria-hidden="true"></span>
  <span>{band.label}</span>
</span>

<style>
  .severity {
    display: inline-flex;
    align-items: center;
    gap: 0.45em;
    white-space: nowrap;
  }
  .swatch {
    width: 1.1em;
    height: 1.1em;
    border-radius: 3px;
    border: 1px solid var(--border-strong);
    flex: none;
  }
  .sev-1 {
    background: var(--sev-1);
  }
  .sev-2 {
    background:
      radial-gradient(circle, var(--sev-ink) 18%, transparent 20%) 0 0 / 5px 5px,
      var(--sev-2);
  }
  .sev-3 {
    background:
      repeating-linear-gradient(45deg, var(--sev-ink) 0 1.5px, transparent 1.5px 5px),
      var(--sev-3);
  }
  .sev-4 {
    background:
      repeating-linear-gradient(45deg, var(--sev-ink-strong) 0 1.5px, transparent 1.5px 5px),
      repeating-linear-gradient(-45deg, var(--sev-ink-strong) 0 1.5px, transparent 1.5px 5px),
      var(--sev-4);
  }
  .sev-5 {
    background:
      repeating-linear-gradient(45deg, var(--sev-ink-strong) 0 1px, transparent 1px 3px),
      repeating-linear-gradient(-45deg, var(--sev-ink-strong) 0 1px, transparent 1px 3px),
      var(--sev-5);
  }
</style>
