<!--
  Screen 8, Your packet: the engine's packet (Markdown) rendered safely, ready to print, with the
  county map at the start of "Your risks". The print stylesheet keeps it legible in black and
  white, lets sections follow on from each other, and sets the sources in two small columns.
-->
<script lang="ts">
  import CountyMap from '../components/CountyMap.svelte';
  import Icon from '../components/Icon.svelte';
  import PlanGate from '../components/PlanGate.svelte';
  import type { PlanOutput } from '../engine/types';
  import { followInPageAnchor } from '../lib/anchors';
  import { countTables, packetSections, renderMarkdown, splitIntro } from '../lib/markdown';

  interface Rendered {
    slug: string;
    /** The whole section, or its heading and opening paragraphs when the map follows them. */
    html: string;
    /** After the map (the risks section only). */
    rest?: string;
  }

  /** The packet rendered section by section; the map goes after the opening of "Your risks". */
  function render(output: PlanOutput): Rendered[] {
    let tables = 0;
    const md = (text: string) => {
      const html = renderMarkdown(text, { idPrefix: 'pk', headingOffset: 1, tableStart: tables });
      tables += countTables(html);
      return html;
    };
    return packetSections(output.packet_markdown).map((section) => {
      if (section.slug !== 'your-risks') return { slug: section.slug, html: md(section.markdown) };
      const { intro, rest } = splitIntro(section.markdown);
      return { slug: section.slug, html: md(intro), rest: md(rest) };
    });
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
      <article class="packet card" aria-label="Preparedness packet" onclick={followInPageAnchor}>
        {#each render(output) as section (section.slug)}
          <div class="packet-section packet-section--{section.slug}">
            {@html section.html}
            {#if section.rest !== undefined}
              <div class="packet__map"><CountyMap location={output.location} /></div>
              {@html section.rest}
            {/if}
          </div>
        {/each}
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
  .packet__map {
    max-width: 20rem;
    margin: var(--s4) 0 var(--s5);
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
