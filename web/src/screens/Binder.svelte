<!--
  Your binder (#/binder; DESIGN-DELTA-v3 §6): the during-event document, drawn on screen from the
  engine's binder tree (never from Markdown), with a table of contents beside it that stays in
  view. `#/binder/<page-id>` opens it at a page; `#/binder/wallet-cards` (the old request for the
  wallet cards) lands on the wallet cards page whatever its id.

  The toolbar: Download PDF (Letter or A4, and "printed on both sides" so each tab starts on a
  right-hand page), Print (the browser's own, with the same page breaks where the browser keeps
  them), and the maps: "Add maps" opens web-maps' panel at its consent screen on every press,
  and the maps it makes appear in the binder's map slots (the neighborhood page, getting out).
  Each page can also be printed alone, so it can be reprinted when something changes.

  The PDF is made in this browser by a separate chunk loaded on the first press (pdfmake and its
  fonts, precached by the service worker so it works offline). Nothing is sent anywhere.
-->
<script lang="ts">
  import { tick } from 'svelte';

  import BinderPage from '../components/binder/BinderPage.svelte';
  import BinderToc from '../components/binder/BinderToc.svelte';
  import type { BinderView } from '../components/binder/view';
  import Icon from '../components/Icon.svelte';
  import PlanGate from '../components/PlanGate.svelte';
  import { hospitalListMissing } from '../engine/loader';
  import type { Binder, MapSlot, PlanOutput } from '../engine/types';
  import { followInPageAnchor, jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import { binderPages, findPage, flatBlocks, HOSPITAL_LIST_MISSING, pageDomId, pageIndex, partDomId, pdfFileName } from '../lib/binder/model';
  import type { MapsStatus, Paper, PdfMap } from '../lib/binder/pdf/doc';
  import type { CountyOutline } from '../lib/maps/compose';
  import { hasChildren, suggestedLayers } from '../lib/maps/rules';
  import type { MapsState } from '../lib/maps/state';
  import { type MapRecord, mapRecordsFor } from '../lib/maps/store';
  import { formatHash, useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();

  // ---- the page on screen ---------------------------------------------------------------------

  /** The page the address names, or the one last jumped to (marked in the contents). */
  let current = $state('');
  const wanted = $derived(router.current.id === 'binder' ? router.current.param : undefined);

  /** The address last gone to: a binder that is drawn again (the hospitals arriving) does not move the page. */
  let jumpedTo: string | undefined;

  // Opened at a page (#/binder/home, #/binder/wallet-cards): go there once it is on the page.
  $effect(() => {
    const param = wanted;
    const binder = app.result.output?.binder;
    if (!param) jumpedTo = undefined;
    if (!param || !binder || param === jumpedTo) return;
    const entry = findPage(binder, param);
    if (!entry) return;
    current = entry.page.id;
    const timer = setTimeout(() => {
      jumpedTo = param;
      jumpTo(pageDomId(entry.page.id), { focus: 'h3' });
    }, 0);
    return () => clearTimeout(timer);
  });

  // The county hospitals for the Neighborhood page come with their own small pack, fetched the
  // first time the binder is shown; the binder is drawn again when they are in.
  $effect(() => {
    app.loadPlaces();
  });
  /** The list could not be had (the binder first opened offline, or the site has no hospital file):
   * the Neighborhood page and the PDF's message say so in one sentence (verify3 R4-05). */
  const hospitalsMissing = $derived(hospitalListMissing(app.data?.places, app.manifest));

  /** A contents link to the page already in the address changes nothing the router sees: jump anyway. */
  function onjump(id: string, e: MouseEvent) {
    const target = formatHash({ id: 'binder', param: id });
    if (window.location.hash !== target) return;
    e.preventDefault();
    current = id;
    jumpTo(pageDomId(id), { focus: 'h3' });
  }

  /** Clicks inside the binder: citations jump within the page; page links as the contents do. */
  function onBinderClick(e: MouseEvent) {
    const link = (e.target as HTMLElement | null)?.closest('a');
    const page = link?.getAttribute('data-page');
    if (page) onjump(page, e);
    else followInPageAnchor(e);
  }

  // ---- printing ------------------------------------------------------------------------------

  /** A page printed on its own ("Print this page"), until the print window closes. */
  let printingOne = $state<string | null>(null);

  async function printPage(id: string) {
    printingOne = id;
    await tick();
    window.addEventListener('afterprint', () => (printingOne = null), { once: true });
    window.print();
  }

  function printAll() {
    printingOne = null;
    window.print();
  }

  // ---- maps (web-maps' panel, moved here from the v0.2 packet screen) ------------------------

  /** Each "Add maps" press opens the panel afresh at its consent screen. */
  let mapsRequest = $state(0);
  /** `SavedPlan.maps`: saved with the plan, never in `input` (DESIGN-DELTA-v3 §9.5). */
  const planMaps = $derived(app.plan?.maps);
  function setPlanMaps(next: MapsState | undefined) {
    if (!app.plan) return;
    if (next) app.plan.maps = next;
    else delete app.plan.maps;
  }
  async function countyOutline(fips: string): Promise<CountyOutline | null> {
    const shapes = await app.countyShapes();
    const county = shapes?.byFips.get(fips);
    return county ? { rings: county.rings, bbox: county.bbox } : null;
  }
  function addMaps() {
    mapsRequest += 1;
    void tick().then(() => document.getElementById('binder-maps')?.scrollIntoView?.({ block: 'start' }));
  }

  /** The stored maps for this plan (from this device's `rr-maps` store), re-read when the pins change. */
  let maps = $state.raw<{ status: MapsStatus; records: Partial<Record<MapSlot['kind'], MapRecord>> }>({ status: 'none', records: {} });
  const countyFips = $derived(app.result.output?.location.county_fips ?? '');
  const mapsSignature = $derived(JSON.stringify([countyFips, planMaps ?? null]));
  $effect(() => {
    void mapsSignature;
    const owner = { maps: $state.snapshot(planMaps) as MapsState | undefined, countyFips };
    let stale = false;
    void mapRecordsFor(owner).then((found) => {
      if (!stale) maps = found;
    });
    return () => {
      stale = true;
    };
  });

  /** The binder's own map slots (the panel shows its figures itself only where the slots cannot). */
  function slotsOf(b: Binder): MapSlot[] {
    return binderPages(b).flatMap((e) => flatBlocks(e.page.blocks).flatMap((bl) => ('map_slot' in bl ? [bl.map_slot] : [])));
  }

  function viewOf(binder: Binder): BinderView {
    return { binder, pages: pageIndex(binder), maps, addMaps, hospitalsMissing };
  }

  // ---- the PDF --------------------------------------------------------------------------------

  /** Letter in the Americas and the Philippines, A4 elsewhere; the household can change it. */
  function defaultPaper(): Paper {
    const lang = typeof navigator !== 'undefined' ? navigator.language : 'en-US';
    const region = /[-_]([A-Z]{2})\b/i.exec(lang)?.[1]?.toUpperCase() ?? 'US';
    return ['US', 'CA', 'MX', 'PH', 'PR', 'CL', 'CO', 'CR', 'GT', 'PA', 'VE', 'DO', 'SV', 'NI'].includes(region) ? 'LETTER' : 'A4';
  }

  let paper = $state<Paper>(defaultPaper());
  let doubleSided = $state(false);
  let pdfBusy = $state(false);
  let pdfMessage = $state('');

  function save(blob: Blob, name: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }

  function pdfMaps(): Partial<Record<MapSlot['kind'], PdfMap>> {
    const out: Partial<Record<MapSlot['kind'], PdfMap>> = {};
    for (const [kind, r] of Object.entries(maps.records) as [MapSlot['kind'], MapRecord][]) {
      out[kind] = { dataUrl: r.image, width: r.width, height: r.height, legend: r.legend, keys: r.keys, statuses: r.statuses, notes: r.notes, credits: r.credits, scale: r.scale, fetched_on: r.fetched_on };
    }
    return out;
  }

  async function downloadPdf(output: PlanOutput) {
    if (pdfBusy) return;
    pdfBusy = true;
    pdfMessage = 'Making your PDF. This takes a few seconds…';
    const name = pdfFileName(output.binder, output.location);
    try {
      const { makeBinderPdf } = await import('../lib/binder/pdf/browser');
      const made = await makeBinderPdf(output.binder, { paper, doubleSided, appVersion: __RR_APP_VERSION__, maps: pdfMaps(), mapsStatus: maps.status });
      save(made.blob, name);
      const pages = `${made.pages} pages, ${paper === 'LETTER' ? 'Letter' : 'A4'}${doubleSided ? ', for printing on both sides' : ''}`;
      pdfMessage = `Saved ${name} (${pages}) in your downloads.`;
      if (made.missing.length) {
        pdfMessage += ` Some letters in your answers are not in the PDF's typeface and show as empty boxes: ${made.missing.join(' ')}. The Print button uses this device's own typefaces instead.`;
      }
      if (hospitalsMissing) pdfMessage += ` ${HOSPITAL_LIST_MISSING}`;
    } catch (e) {
      const why = e instanceof Error && e.message ? ` (${e.message})` : '';
      pdfMessage = `The PDF could not be made${why}. You can still use Print, and choose "Save as PDF" there.`;
    } finally {
      pdfBusy = false;
    }
  }
</script>

<div class="page binder-screen" class:printing-one={printingOne !== null}>
  <div class="binder-intro no-print">
    <h1 id="page-title" tabindex="-1">Your binder</h1>
    <p class="lead">
      What to reach for when something happens: what to do first, who to call, where to go, and a page for each person. Download it as a
      PDF, print it, and keep it in a binder with ten tabs where everyone can find it.
    </p>
  </div>
  <PlanGate>
    {#snippet children(output)}
      {@const binder = output.binder}
      {@const view = viewOf(binder)}
      {@const entries = binderPages(binder)}
      <form class="pdf-options no-print" aria-label="Download or print the binder" onsubmit={(e) => (e.preventDefault(), void downloadPdf(output))}>
        <fieldset>
          <legend>Paper</legend>
          <label class="choice"><input type="radio" name="paper" value="LETTER" bind:group={paper} /> <span>Letter</span></label>
          <label class="choice"><input type="radio" name="paper" value="A4" bind:group={paper} /> <span>A4</span></label>
        </fieldset>
        <label class="choice"><input type="checkbox" bind:checked={doubleSided} /> <span>Printing on both sides of the paper</span></label>
        <p class="button-row">
          <button type="submit" class="button button--primary" disabled={pdfBusy} aria-describedby="pdf-status"><Icon name="download" /> {pdfBusy ? 'Making the PDF…' : 'Download PDF'}</button>
          <button type="button" class="button" onclick={printAll}><Icon name="print" /> Print</button>
          <button type="button" class="button" onclick={addMaps}>{planMaps?.fetched_on ? 'Refresh maps' : 'Add maps'}</button>
        </p>
        <p id="pdf-status" class="pdf-status" role="status" aria-live="polite">{pdfMessage}</p>
      </form>
      {#if mapsRequest > 0 || planMaps}
        <div class="binder-maps no-print" id="binder-maps">
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
                slots={maps.status === 'unavailable' ? slotsOf(binder) : []}
                start={mapsRequest > 0}
              />
            {/key}
          {:catch}
            <p class="small">The maps panel could not be loaded. Reload the page and try again.</p>
          {/await}
        </div>
      {/if}
      <div class="binder-layout">
        <div class="binder-side no-print">
          <BinderToc {binder} {current} {onjump} />
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="binder-doc" onclick={onBinderClick}>
          {#each binder.parts as part (part.id)}
            <section class="binder-part" aria-labelledby={partDomId(part.id)}>
              <h2 class="binder-part__title" id={partDomId(part.id)}><span class="binder-part__tab">Tab {part.tab}</span> {part.title}</h2>
              {#each entries.filter((e) => e.part.id === part.id) as entry (entry.page.id)}
                <div class="binder-pagewrap" class:print-this={printingOne === entry.page.id}>
                  <BinderPage {entry} {view} onprint={printPage} />
                </div>
              {/each}
            </section>
          {/each}
        </div>
      </div>
    {/snippet}
  </PlanGate>
</div>

<style>
  .binder-intro {
    max-width: var(--w-text);
  }
  .pdf-options {
    max-width: var(--w-text);
    margin-bottom: var(--s5);
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2) var(--s5);
    align-items: center;
  }
  .pdf-options fieldset {
    display: flex;
    gap: var(--s4);
    align-items: center;
    border: 0;
    padding: 0;
    margin: 0;
  }
  .pdf-options legend {
    float: left;
    margin-right: var(--s3);
    font-weight: 700;
  }
  .pdf-options .button-row {
    flex-basis: 100%;
    margin: 0;
  }
  .pdf-status:empty {
    display: none;
  }
  .pdf-status {
    flex-basis: 100%;
    margin: 0;
  }
  .binder-maps {
    margin-bottom: var(--s5);
  }
  .binder-layout {
    display: grid;
    grid-template-columns: minmax(14rem, 17rem) minmax(0, 52rem);
    gap: var(--s6);
    align-items: start;
  }
  .binder-side {
    position: sticky;
    top: var(--s4);
    max-height: calc(100vh - 2rem);
    overflow-y: auto;
    padding-right: var(--s2);
  }
  .binder-part__title {
    font-size: var(--text-lg);
    margin: var(--s6) 0 var(--s3);
  }
  .binder-part:first-child .binder-part__title {
    margin-top: 0;
  }
  .binder-part__tab {
    display: inline-block;
    padding: 0 0.4em;
    background: var(--text);
    color: var(--bg);
    border-radius: 2px;
  }
  @media (max-width: 60rem) {
    .binder-layout {
      grid-template-columns: minmax(0, 1fr);
      gap: var(--s3);
    }
    .binder-side {
      position: static;
      max-height: none;
      overflow: visible;
      padding: var(--s3) var(--s4);
      border: 1px solid var(--border);
      border-radius: var(--r2);
      background: var(--surface);
    }
  }
  @media print {
    .binder-layout {
      display: block;
    }
    .binder-part__title {
      display: none;
    }
    .binder-screen :global(.binder-page) {
      border: 0 !important;
      padding: 0 !important;
      margin: 0 !important;
      background: none !important;
      break-before: page;
    }
    .binder-part:first-child .binder-pagewrap:first-child :global(.binder-page) {
      break-before: auto;
    }
    .printing-one .binder-pagewrap:not(.print-this) {
      display: none;
    }
    .printing-one .binder-pagewrap.print-this :global(.binder-page) {
      break-before: auto;
    }
  }
</style>
