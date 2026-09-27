<!--
  "Set your home point and meeting places on a map" (DESIGN-DELTA-v3 §2.2, step 7's Getting out
  card; web-interview3 places it). The press opens the short consent screen (the street map is the
  only thing fetched), then the pin map; "Done" hands the pins back for `SavedPlan.maps`. Moving a
  pin makes any maps already made "need refreshing" on the binder.
-->
<script lang="ts">
  import { tick } from 'svelte';

  import type { AddressSearch } from '../../lib/maps/nominatim';
  import { PIN_MARKS, TOOL_LABELS } from '../../lib/maps/pins';
  import { homePoint, type MapLocation } from '../../lib/maps/slots';
  import { type MapsState, PIN_IDS } from '../../lib/maps/state';
  import type { CreateView } from '../../lib/maps/view';
  import MapsConsent from './MapsConsent.svelte';
  import PinMap from './PinMap.svelte';

  let {
    maps,
    location,
    onchange,
    label = 'Set your home point and meeting places on a map',
    headingLevel = 3,
    createView,
    search,
  }: {
    maps: MapsState | undefined;
    location: MapLocation;
    onchange: (next: MapsState) => void;
    label?: string;
    headingLevel?: 2 | 3 | 4;
    /** Tests only. */
    createView?: CreateView;
    /** Tests only. */
    search?: AddressSearch;
  } = $props();

  let phase = $state<'idle' | 'consent' | 'map'>('idle');
  let button: HTMLButtonElement | undefined = $state();

  const placed = $derived(PIN_IDS.filter((id) => maps?.[id]).map((id) => `${PIN_MARKS[id]} ${TOOL_LABELS[id].toLowerCase()}`));
  const lines = $derived(maps?.routes.length ?? 0);
  const start = $derived.by(() => {
    const h = homePoint(maps, location);
    return { at: h.at, zoom: h.basis === 'pin' ? 16 : h.basis === 'zip' ? 14 : 11 };
  });

  function back() {
    phase = 'idle';
    void tick().then(() => button?.focus());
  }
</script>

<div class="pin-map-button">
  {#if phase === 'idle'}
    <button type="button" class="button" bind:this={button} onclick={() => (phase = 'consent')}>{label}</button>
    <p class="small muted" aria-live="polite">
      {#if placed.length || lines}
        Placed: {[...placed, ...(lines ? [`${lines} ${lines === 1 ? 'way' : 'ways'} out`] : [])].join(', ')}.
      {:else}
        Nothing placed yet. The map is fetched from OpenStreetMap only when you press the button, after you see who gets what.
      {/if}
    </p>
  {:else if phase === 'consent'}
    <MapsConsent mode="pins" suggested={{ flood: false, surge: false, wildfire: false }} onfetch={() => (phase = 'map')} oncancel={back} headingLevel={headingLevel === 4 ? 3 : headingLevel} />
  {:else}
    <PinMap
      value={maps}
      center={start.at}
      zoom={start.zoom}
      {headingLevel}
      ondone={(next) => {
        onchange(next);
        back();
      }}
      oncancel={back}
      {...createView ? { createView } : {}}
      {...search ? { search } : {}}
    />
  {/if}
</div>
