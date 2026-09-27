<!--
  The maps panel (DESIGN-DELTA-v3 §9.4): "Add maps" → the consent screen (every time) → the pin map
  (the first time, or after "Edit pins") → fetch → the three maps, with "Refresh maps", "Edit pins"
  and "Remove maps". A source that fails leaves its layer out and the map says so; a press where
  nothing at all answered saves nothing and leaves the placeholders. The images are kept only on
  this device (`store.ts`); the pins and choices go back to the caller for `SavedPlan.maps`.

  Mounted on the Packet screen for now; web-binder moves it into the binder.
-->
<script lang="ts">
  import { onMount, tick } from 'svelte';

  import type { IsoDate } from '../../engine/types';
  import { browserEnv, type ComposeEnv, type ComposeResult, composeMaps, type CountyOutline, type LayerChoice, longDate } from '../../lib/maps/compose';
  import type { AddressSearch } from '../../lib/maps/nominatim';
  import { homePoint, MAP_SLOT_FIXTURES, type MapLocation, type MapSlotBlock, type MapSlotKind, type SuggestedLayers } from '../../lib/maps/slots';
  import { emptyMapsState, type MapsState } from '../../lib/maps/state';
  import { clearMaps, type MapRecord, mapRecordsFor, type MapsStatus, mapsKey, saveMaps, toRecord } from '../../lib/maps/store';
  import type { CreateView } from '../../lib/maps/view';
  import ConfirmDialog from '../ConfirmDialog.svelte';
  import MapFigure from './MapFigure.svelte';
  import MapsConsent from './MapsConsent.svelte';
  import PinMap from './PinMap.svelte';

  let {
    location,
    suggested,
    children,
    maps,
    onchange,
    today,
    loadCounty,
    zoneLink = null,
    slots = MAP_SLOT_FIXTURES,
    headingLevel = 2,
    start = false,
    env,
    createView,
    search,
  }: {
    location: MapLocation;
    /** The §9.2 rule for this household. */
    suggested: SuggestedLayers;
    /** Schools and child care are shown when the household has children. */
    children: boolean;
    /** `SavedPlan.maps`. */
    maps: MapsState | undefined;
    /** Called with the new `SavedPlan.maps` (undefined when the household removes the pins too). */
    onchange: (next: MapsState | undefined) => void;
    today: () => IsoDate;
    /** The county outline for the area and region maps (from the app's own `geo` pack). */
    loadCounty: () => Promise<CountyOutline | null>;
    /** The state's evacuation-zone page, printed where storm surge would be (awaiting: binder). */
    zoneLink?: { name: string; url: string } | null;
    /** The binder's `map_slot` blocks (captions); the three stand-ins until the binder lands. */
    slots?: readonly MapSlotBlock[];
    headingLevel?: 2 | 3;
    /** Open straight at the consent screen (the toolbar's "Add maps"). */
    start?: boolean;
    /** Tests only: the fetch and canvas environment. */
    env?: ComposeEnv;
    /** Tests only: the map library for the pin map. */
    createView?: CreateView;
    /** Tests only: the address search service. */
    search?: AddressSearch;
  } = $props();

  const uid = $props.id();
  type Phase = 'idle' | 'consent' | 'pins' | 'fetching';
  let phase = $state<Phase>('idle');
  let intent = $state<'add' | 'refresh' | 'edit'>('add');
  let chosen = $state<LayerChoice>({ base: true, ...emptyMapsState().layers });
  let status = $state<MapsStatus>('none');
  let records = $state.raw<Partial<Record<MapSlotKind, MapRecord>>>({});
  let progress = $state({ done: 0, total: 0 });
  let message = $state('');
  let removing = $state(false);
  let forgetPins = $state(false);
  let controller: AbortController | null = null;
  /** The last press, for the report and the end-to-end check. */
  let last = $state.raw<ComposeResult | null>(null);
  let panel: HTMLElement | undefined = $state();
  /** Maps made in a browser that keeps none: shown for this visit only. */
  let memory: { key: string; records: Partial<Record<MapSlotKind, MapRecord>> } | null = null;

  const offline = () => typeof navigator !== 'undefined' && navigator.onLine === false;
  const hasMaps = $derived(status === 'ready' && Object.keys(records).length > 0);
  const key = $derived(mapsKey(maps, location.county_fips));

  async function reload() {
    const wanted = key;
    const found = await mapRecordsFor({ maps, countyFips: location.county_fips });
    if (wanted !== key) return;
    if (found.status !== 'ready' && memory && memory.key === wanted) {
      status = 'ready';
      records = memory.records;
      return;
    }
    status = found.status;
    records = found.records;
  }

  // The stored maps follow the plan: new pins, a new county or an imported plan re-read them.
  $effect(() => {
    void key;
    void reload();
  });

  onMount(() => {
    if (start) begin('add');
    return () => controller?.abort();
  });

  function begin(what: 'add' | 'refresh' | 'edit') {
    intent = what;
    message = '';
    phase = 'consent';
  }

  function consented(layers: LayerChoice) {
    chosen = layers;
    if (layers.base && (intent === 'edit' || !maps?.home)) phase = 'pins';
    else void fetchMaps(maps);
  }

  const pinCenter = $derived.by(() => {
    const h = homePoint(maps, location);
    return { at: h.at, zoom: h.basis === 'pin' ? 16 : h.basis === 'zip' ? 14 : 11 };
  });

  async function fetchMaps(pins: MapsState | undefined) {
    phase = 'fetching';
    message = '';
    progress = { done: 0, total: 0 };
    controller = new AbortController();
    const signal = controller.signal;
    const day = today();
    const base: MapsState = { ...(pins ?? emptyMapsState()) };
    try {
      const county = await loadCounty().catch(() => null);
      const result = await composeMaps(
        { maps: base, location, county, layers: chosen, suggested, children, today: day, zoneLink },
        { ...(env ?? browserEnv()), signal, onProgress: (done, total) => (progress = { done, total }) },
      );
      last = result;
      if (signal.aborted) {
        message = 'Stopped. Nothing was saved.';
        if (pins !== maps) onchange(pins);
        return;
      }
      const asked = Object.values(result.requests).reduce((n, c) => n + c, 0);
      if (asked > 0 && result.succeeded === 0) {
        message = `The maps could not be fetched on ${longDate(day)}: none of the services answered. Nothing was saved. Try again later.`;
        if (pins !== maps) onchange(pins);
        return;
      }
      // The saved plan keeps §9.5's four layers; the street-map choice belonged to this press.
      const next: MapsState = { ...base, layers: { places: chosen.places, flood: chosen.flood, surge: chosen.surge, wildfire: chosen.wildfire }, fetched_on: day };
      const nextKey = mapsKey(next, location.county_fips);
      const made = result.maps.map((m) => toRecord(m, nextKey));
      const kept = await saveMaps(made);
      records = Object.fromEntries(made.map((r) => [r.slot, r]));
      memory = kept ? null : { key: nextKey, records };
      status = 'ready';
      onchange(next);
      message = kept
        ? `Your maps are ready (fetched ${longDate(day)}). They are kept only on this device, not in your saved file.`
        : 'Your maps are ready, but this browser will not keep them: they will be gone when you leave this page. Print the binder now if you want them on paper.';
    } catch {
      message = 'Something went wrong while making the maps. Nothing was saved. Try again later.';
    } finally {
      controller = null;
      phase = 'idle';
      void tick().then(() => panel?.focus());
    }
  }

  async function remove() {
    await clearMaps();
    memory = null;
    records = {};
    status = 'none';
    onchange(forgetPins || !maps ? undefined : { ...maps, fetched_on: undefined });
    message = forgetPins ? 'The maps and your pins were removed from this device.' : 'The maps were removed from this device. Your pins are kept.';
    forgetPins = false;
  }
</script>

<section class="maps-panel card" aria-labelledby="{uid}-title" data-maps-phase={phase} data-maps-status={status}>
  <svelte:element this={`h${headingLevel}`} id="{uid}-title" class="maps-panel__title" tabindex="-1" bind:this={panel}>Maps of your area</svelte:element>
  <p>
    Three maps for your binder: your neighbourhood, your city or county, and your region with the ways out. They come from outside
    services and are fetched only when you ask; you will see exactly who gets what first.
  </p>

  {#if phase === 'consent'}
    <MapsConsent {suggested} hasHomePin={!!maps?.home} offline={offline()} onfetch={consented} oncancel={() => (phase = 'idle')} headingLevel={3} />
  {:else if phase === 'pins'}
    <PinMap
      value={maps}
      center={pinCenter.at}
      zoom={pinCenter.zoom}
      ondone={(next) => void fetchMaps(next)}
      oncancel={() => (phase = 'idle')}
      headingLevel={3}
      {...createView ? { createView } : {}}
      {...search ? { search } : {}}
    />
  {:else if phase === 'fetching'}
    <div class="maps-panel__progress" role="status">
      <p>Fetching your maps{progress.total ? `: ${progress.done} of ${progress.total} requests` : '…'}</p>
      <progress max={progress.total || 1} value={progress.done}></progress>
      <button type="button" class="button button--small" onclick={() => controller?.abort()}>Stop</button>
    </div>
  {:else}
    {#if status === 'stale'}
      <p class="maps-panel__note">Maps need refreshing: your pins changed, or this plan came from a file, and this device has no maps for it yet.</p>
    {:else if status === 'unavailable'}
      <p class="maps-panel__note">This browser does not keep maps (a private window, or storage is blocked). You can still make them and print the binder straight away.</p>
    {:else if status === 'ready' && maps?.fetched_on}
      <p class="small muted">Fetched {longDate(maps.fetched_on)}. Kept only on this device, never in your saved file.</p>
    {/if}
    <div class="button-row">
      {#if hasMaps}
        <button type="button" class="button button--primary" onclick={() => begin('refresh')}>Refresh maps</button>
        <button type="button" class="button" onclick={() => begin('edit')}>Edit pins</button>
        <button type="button" class="button button--quiet" onclick={() => (removing = true)}>Remove maps</button>
      {:else}
        <button type="button" class="button button--primary" onclick={() => begin(status === 'stale' ? 'refresh' : 'add')}>{status === 'stale' ? 'Refresh maps' : 'Add maps'}</button>
        {#if maps?.home}
          <button type="button" class="button" onclick={() => begin('edit')}>Edit pins</button>
        {/if}
      {/if}
    </div>
  {/if}

  <p class="maps-panel__message" aria-live="polite">{message}</p>

  {#if phase === 'idle' && (hasMaps || status === 'stale' || message.startsWith('The maps could not'))}
    <div class="maps-panel__figures">
      {#each slots as s (s.id)}
        <MapFigure slot={s.kind} caption={s.caption} record={records[s.kind] ?? null} status={records[s.kind] ? 'ready' : status === 'ready' ? 'none' : status} />
      {/each}
    </div>
  {/if}
  {#if last}
    <span class="visually-hidden" data-maps-requests={JSON.stringify(last.requests)} data-maps-tiles={last.tiles}></span>
  {/if}
</section>

<ConfirmDialog bind:open={removing} title="Remove the maps?" confirmLabel="Remove maps" onconfirm={remove}>
  <p>The three maps are removed from this device. You can fetch them again at any time.</p>
  <label class="choice">
    <input type="checkbox" bind:checked={forgetPins} />
    <span class="choice__text">Also forget the pins (your home point, meeting places and ways out)</span>
  </label>
</ConfirmDialog>

<style>
  .maps-panel {
    margin-bottom: var(--s5);
    max-width: 52rem;
  }
  .maps-panel__title {
    margin-top: 0;
  }
  .maps-panel__title:focus {
    outline: none;
  }
  .maps-panel__note {
    padding: var(--s3) var(--s4);
    border-left: 4px solid var(--note-edge);
    background: var(--note-soft);
    border-radius: var(--r1);
  }
  .maps-panel__progress progress {
    width: 100%;
    max-width: 24rem;
    display: block;
    margin-bottom: var(--s2);
  }
  .maps-panel__message:empty {
    display: none;
  }
  .maps-panel__figures {
    margin-top: var(--s5);
  }
</style>
