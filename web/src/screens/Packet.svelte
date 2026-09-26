<!--
  Screen 8, Your packet: the engine's packet (Markdown) rendered safely, ready to print. The print
  stylesheet starts each section on a new page and keeps it legible in black and white.
-->
<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import PlanGate from '../components/PlanGate.svelte';
  import { renderMarkdown } from '../lib/markdown';

  /** In-page links inside the packet (notes, sources) scroll without touching the page's route. */
  function followAnchor(e: MouseEvent) {
    const link = (e.target as HTMLElement | null)?.closest('a');
    const target = link?.getAttribute('href');
    if (!link || !target || !target.startsWith('#') || target.startsWith('#/')) return;
    e.preventDefault();
    const el = document.getElementById(target.slice(1));
    if (el) {
      if (!el.hasAttribute('tabindex')) el.setAttribute('tabindex', '-1');
      el.scrollIntoView({ block: 'start' });
      el.focus({ preventScroll: true });
    }
  }
</script>

<div class="page packet-page">
  <div class="toolbar no-print">
    <h1 id="page-title" tabindex="-1">Your packet</h1>
    <p class="lead">
      Everything in your plan on paper: risks, targets, checklists, a family plan to fill in, a maintenance calendar and every source. Print
      it, or choose "Save as PDF" in the print window. Keep a copy somewhere you can reach without power.
    </p>
    <p class="button-row">
      <button type="button" class="button button--primary" onclick={() => window.print()}><Icon name="print" /> Print or save as PDF</button>
      <span class="small muted">Works on Letter and A4 paper, in black and white.</span>
    </p>
  </div>
  <PlanGate>
    {#snippet children(output)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <article class="packet card" aria-label="Preparedness packet" onclick={followAnchor}>
        {@html renderMarkdown(output.packet_markdown, { idPrefix: 'pk', headingOffset: 1 })}
      </article>
    {/snippet}
  </PlanGate>
</div>

<style>
  .toolbar {
    max-width: var(--w-text);
    margin-bottom: var(--s5);
  }
  .packet {
    max-width: 52rem;
  }
  .packet :global(h2.md-h1) {
    font-size: var(--text-2xl);
    margin-top: 0;
  }
  .packet :global(h3.md-h2) {
    margin-top: var(--s7);
    padding-top: var(--s4);
    border-top: 2px solid var(--border);
    font-size: var(--text-xl);
  }
  .packet :global(h4) {
    margin-top: var(--s5);
  }
  .packet :global(blockquote) {
    margin: var(--s4) 0;
    padding: var(--s3) var(--s4);
    border-left: 4px solid var(--accent);
    background: var(--surface-2);
    border-radius: var(--r1);
  }
  .packet :global(blockquote > :last-child) {
    margin-bottom: 0;
  }
  .packet :global(li:has(> input[type='checkbox'])) {
    list-style: none;
    margin-left: -1.4em;
  }
  .packet :global(input[type='checkbox']) {
    margin-right: var(--s2);
    vertical-align: -0.2em;
  }
  .packet :global(td:empty) {
    min-width: 12rem;
  }
  .packet :global(.footnotes) {
    margin-top: var(--s5);
    font-size: var(--text-sm);
    border-top: 1px solid var(--border);
  }
  .packet :global(sup a) {
    text-decoration: none;
    padding: 0 0.15em;
  }
  @media print {
    .packet {
      border: 0 !important;
      box-shadow: none !important;
      padding: 0 !important;
      max-width: none !important;
    }
  }
</style>
