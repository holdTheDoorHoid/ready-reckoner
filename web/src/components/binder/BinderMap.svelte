<!--
  A `map_slot` in the binder (DESIGN-DELTA-v3 §4.1, §9): the household's map for the slot, drawn
  by web-maps' MapFigure (the image, its numbered legend, pattern keys, what is missing and the
  dated credits), or, when this device holds no map for the plan, one line saying so with "Add
  maps" beside it. Nothing is fetched here: "Add maps" opens the maps panel, whose consent screen
  comes first on every press. On paper the missing map is that one line, never an empty frame.
-->
<script lang="ts">
  import type { MapSlot } from '../../engine/types';
  import MapFigure from '../maps/MapFigure.svelte';
  import type { BinderView } from './view';

  let { slot, view }: { slot: MapSlot; view: BinderView } = $props();

  const record = $derived(view.maps.records[slot.kind] ?? null);
  const WHAT: Record<MapSlot['kind'], string> = {
    neighbourhood: 'your neighborhood',
    area: 'your city or county',
    region: 'your region and the ways out',
  };
  const why = $derived(
    view.maps.status === 'stale'
      ? 'the maps need refreshing.'
      : view.maps.status === 'unavailable'
        ? 'this browser does not keep maps.'
        : 'no map added.',
  );
</script>

{#if record}
  <div class="binder-map">
    <MapFigure slot={slot.kind} caption={slot.caption} {record} status="ready" />
  </div>
{:else}
  <p class="binder-map--missing">
    <strong>Map of {WHAT[slot.kind]}:</strong>
    {why}
    {#if view.addMaps}
      <button type="button" class="button button--small no-print" onclick={() => view.addMaps?.()}>{view.maps.status === 'stale' ? 'Refresh maps' : 'Add maps'}</button>
    {/if}
    <span class="print-only">Add maps on the Binder screen of the app, then print this page again.</span>
  </p>
{/if}

<style>
  .binder-map {
    margin: var(--s4) 0;
  }
  .binder-map--missing {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    align-items: baseline;
    padding: var(--s2) var(--s3);
    border-left: 3px solid var(--border-strong);
    background: var(--surface-2);
    border-radius: var(--r1);
  }
  @media print {
    .binder-map--missing {
      display: block;
      padding: 0;
      border: 0;
      background: none;
      font-size: 8.5pt;
    }
  }
</style>
