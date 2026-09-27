<!--
  Screen 8, Your packet: the engine's packet (Markdown) rendered safely, ready to print, with the
  county map at the start of "Your risks". The print stylesheet keeps it legible in black and
  white, lets sections follow on from each other, and sets the sources in two small columns.

  `#/packet/<section>` opens the packet at one section (`#/packet/wallet-cards` from the family
  plan's "Print wallet cards"). The wallet cards are the packet's "Wallet cards" section (or, if a
  packet has none, its family plan); "Print only the wallet cards" prints that section alone, each
  card (a block quote in the packet) boxed with a cut line and never split across pages.

  The maps panel (web-maps, DESIGN-DELTA-v3 §9.4) is mounted here for now: a toolbar button and a
  panel above the packet, both marked "web-maps mount". web-binder moves it into the binder.
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
  // --- web-maps mount (DESIGN-DELTA-v3 §9.4): moves into the binder screen with web-binder ---
  // The panel and everything it uses load on first use (their own chunk, precached for offline).
  import type { CountyOutline } from '../lib/maps/compose';
  import { hasChildren, suggestedLayers } from '../lib/maps/rules';
  import type { MapsState } from '../lib/maps/state';
  // --- end web-maps mount ---

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

  const cardsSlug = $derived(app.result.output ? cardsSection(packetSections(app.result.output.prepare_markdown)) : undefined);

  // Opened at a section (#/packet/wallet-cards): go there once the packet is on the page. A
  // request for the wallet cards finds them under whatever heading this packet gives them.
  $effect(() => {
    const wanted = router.current.id === 'packet' ? router.current.param : undefined;
    const output = app.result.output;
    if (!wanted || !output) return;
    const slugs = packetSections(output.prepare_markdown).map((sec) => sec.slug);
    const slug = slugs.includes(wanted) ? wanted : wanted === 'wallet-cards' ? cardsSlug : undefined;
    if (!slug) return;
    const timer = setTimeout(() => jumpTo(`packet-${slug}`, { focus: 'h2, h3, h4' }), 0);
    return () => clearTimeout(timer);
  });

  // --- web-maps mount (DESIGN-DELTA-v3 §9.4): moves into the binder screen with web-binder ---
  /** Each toolbar press opens the panel afresh at the consent screen. */
  let mapsRequest = $state(0);
  // awaiting: web-interview3 (`SavedPlan.maps?: MapsState`; until then the field is read and
  // written untyped, and a reload drops it, so stored maps show as needing a refresh).
  const planMaps = $derived((app.plan as { maps?: MapsState } | null)?.maps);
  function setPlanMaps(next: MapsState | undefined) {
    const plan = app.plan as { maps?: MapsState } | null;
    if (!plan) return;
    if (next) plan.maps = next;
    else delete plan.maps;
  }
  async function countyOutline(fips: string): Promise<CountyOutline | null> {
    const shapes = await app.countyShapes();
    const county = shapes?.byFips.get(fips);
    return county ? { rings: county.rings, bbox: county.bbox } : null;
  }
  // --- end web-maps mount ---

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
    return packetSections(output.prepare_markdown).map((section) => {
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
      <!-- web-maps mount: the toolbar button (DESIGN-DELTA-v3 §9.4) -->
      <button type="button" class="button" onclick={() => (mapsRequest += 1)}>{planMaps?.fetched_on ? 'Refresh maps' : 'Add maps'}</button>
      <span class="small muted">Works on Letter and A4 paper, in black and white.</span>
    </p>
  </div>
  <PlanGate>
    {#snippet children(output)}
      <!-- web-maps mount: the maps panel above the packet (DESIGN-DELTA-v3 §9.4). Not printed with
           the v2 packet; web-binder prints the maps in the binder's map slots. -->
      {#if mapsRequest > 0 || planMaps}
        <div class="packet__maps no-print">
          {#await import('../components/maps/MapsPanel.svelte')}
            <p class="small muted" aria-busy="true">Loading the maps panel…</p>
          {:then { default: MapsPanel }}
            {#key mapsRequest}
              <MapsPanel
                location={output.location}
                suggested={suggestedLayers(output, app.plan?.input.dials.horizon_years ?? 10)}
                children={hasChildren(app.plan?.input.people)}
                maps={planMaps}
                onchange={setPlanMaps}
                today={app.today}
                loadCounty={() => countyOutline(output.location.county_fips)}
                start={mapsRequest > 0}
              />
            {/key}
          {:catch}
            <p class="small">The maps panel could not be loaded. Reload the page and try again.</p>
          {/await}
        </div>
      {/if}
      <!-- end web-maps mount -->
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
