<!--
  The paragraph at the top of each optional step (DESIGN-DELTA-v3 §2): what its answers are for
  and where they are kept, passed in as `children`. Under it, in small print, the caveat about the
  shared web address (warn, don't block; docs/DESIGN.md §10).
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="step-privacy">
  <p class="step-privacy__main"><Icon name="lock" /> <span>{@render children()}</span></p>
  <p class="small">
    Until Ready Reckoner has a web address of its own, other pages at the same address could in principle read what this browser keeps. If
    that matters to you, write these details on the printed binder instead.
  </p>
</div>

<style>
  .step-privacy {
    padding: var(--s3) var(--s4);
    background: var(--surface-2);
    border-radius: var(--r2);
    margin-bottom: var(--s5);
    max-width: var(--w-text);
  }
  .step-privacy p {
    margin: 0;
  }
  .step-privacy__main {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    margin-bottom: var(--s2) !important;
  }
  .step-privacy__main :global(svg) {
    flex: none;
    margin-top: 0.2em;
  }
</style>
