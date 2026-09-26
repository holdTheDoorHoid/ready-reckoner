<!--
  Screen 8, Your packet: the engine's packet (Markdown) rendered safely, ready to print, with the
  county map at the start of "Your risks". The print stylesheet keeps it legible in black and
  white, lets sections follow on from each other, and sets the sources in two small columns.

  `#/packet/<section>` opens the packet at one section (`#/packet/wallet-cards` from the family
  plan's "Print wallet cards"). The wallet cards are the packet's "Wallet cards" section (or, if a
  packet has none, its family plan); "Print only the wallet cards" prints that section alone, each
  card (a block quote in the packet) boxed with a cut line and never split across pages.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import CountyMap from '../components/CountyMap.svelte';
  import Icon from '../components/Icon.svelte';
  import PlanGate from '../components/PlanGate.svelte';
  import type { PlanOutput } from '../engine/types';
  import { followInPageAnchor, jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import { cardsSection, countTables, packetSections, renderMarkdown, splitIntro } from '../lib/markdown';
  import { useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();
  /** Printing only the wallet cards (until the print window closes). */
  let cardsOnly = $state(false);

  interface Rendered {
    slug: string;
    /** The whole section, or its heading and opening paragraphs when the map follows them. */
    html: string;
    /** After the map (the risks section only). */
    rest?: string;
  }

  const cardsSlug = $derived(app.result.output ? cardsSection(packetSections(app.result.output.packet_markdown)) : undefined);

  // Opened at a section (#/packet/wallet-cards): go there once the packet is on the page. A
  // request for the wallet cards finds them under whatever heading this packet gives them.
  $effect(() => {
    const wanted = router.current.id === 'packet' ? router.current.param : undefined;
    const output = app.result.output;
    if (!wanted || !output) return;
    const slugs = packetSections(output.packet_markdown).map((sec) => sec.slug);
    const slug = slugs.includes(wanted) ? wanted : wanted === 'wallet-cards' ? cardsSlug : undefined;
    if (!slug) return;
    const timer = setTimeout(() => jumpTo(`packet-${slug}`, { focus: 'h2, h3, h4' }), 0);
    return () => clearTimeout(timer);
  });

  function afterPrint() {
    cardsOnly = false;
  }

  async function printCards() {
    cardsOnly = true;
    await tick();
    window.addEventListener('afterprint', afterPrint, { once: true });
    window.print();
  }

  function printAll() {
    cardsOnly = false;
    window.print();
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

<div class="page packet-page" class:cards-only={cardsOnly}>
  <div class="toolbar no-print">
    <h1 id="page-title" tabindex="-1">Your packet</h1>
    <p class="lead">
      Everything in your plan on paper: risks, targets, checklists, your family plan, a maintenance calendar and every source. Print
      it, or choose "Save as PDF" in the print window. Keep a copy somewhere you can reach without power.
    </p>
    <p class="button-row">
      <button type="button" class="button button--primary" onclick={printAll}><Icon name="print" /> Print or save as PDF</button>
      {#if cardsSlug}
        <button type="button" class="button" onclick={printCards}><Icon name="print" /> Print only the wallet cards</button>
      {/if}
      <span class="small muted">Works on Letter and A4 paper, in black and white.</span>
    </p>
  </div>
  <PlanGate>
    {#snippet children(output)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <article class="packet card" aria-label="Preparedness packet" onclick={followInPageAnchor}>
        {#each render(output) as section (section.slug)}
          <div id="packet-{section.slug}" class="packet-section packet-section--{section.slug}" class:is-cards={section.slug === cardsSlug}>
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
  /* Wallet cards: each card (a block quote) boxed with a cut line. */
  .packet :global(.is-cards blockquote) {
    border: 1px dashed var(--border-strong);
    border-radius: var(--r1);
    break-inside: avoid;
    max-width: 26rem;
  }
  .packet :global(.is-cards blockquote ul) {
    padding-left: 1.1em;
    margin: 0;
  }
  @media print {
    .packet {
      border: 0 !important;
      box-shadow: none !important;
      padding: 0 !important;
      max-width: none !important;
    }
    /* Two cards across on paper, about wallet width, never split across pages. */
    .packet :global(.is-cards blockquote) {
      display: inline-block;
      vertical-align: top;
      width: 48%;
      margin: 0 1.5% 8pt 0 !important;
      border: 1pt dashed #000 !important;
      font-size: 9pt;
      break-inside: avoid;
    }
    .cards-only :global(.packet-section:not(.is-cards)) {
      display: none !important;
    }
  }
</style>
