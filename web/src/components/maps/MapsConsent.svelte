<!--
  The consent screen for maps (DESIGN-DELTA-v3 §9.1, §9.4; DESIGN.md §10, hard rule 3). It names
  every outside service the maps would use, says in one sentence what each receives, and offers a
  box per layer: the street map and nearby places ticked, flood zones and wildfire hazard ticked
  when §9.2's rule says they matter here (the caller passes that in). Nothing is sent until "Fetch
  maps" is pressed, and it is shown on every press: nothing about the choice is remembered.

  `mode="pins"` is the short form for the pin map on its own (step 7): only the street map is
  fetched there, so it names only that service and has no boxes.
-->
<script lang="ts">
  import { untrack } from 'svelte';

  import type { SuggestedLayers } from '../../lib/maps/slots';
  import { EVERY_RECIPIENT_SEES, RECIPIENTS, type RecipientId, SURGE_NOTE } from '../../lib/maps/sources';
  import type { LayerChoice } from '../../lib/maps/compose';

  let {
    mode = 'maps',
    suggested,
    hasHomePin = false,
    offline = false,
    onfetch,
    oncancel,
    headingLevel = 2,
  }: {
    mode?: 'maps' | 'pins';
    /** The §9.2 rule for this household (flood, surge, wildfire). */
    suggested: SuggestedLayers;
    /** Whether a home pin is already placed (without a street map there is nothing to place one on). */
    hasHomePin?: boolean;
    /** The browser says it is offline: warn, do not block. */
    offline?: boolean;
    onfetch: (layers: LayerChoice) => void;
    oncancel: () => void;
    headingLevel?: 2 | 3;
  } = $props();

  const uid = $props.id();

  // Every press starts from the same defaults (this screen is made afresh each time): nothing
  // about an earlier choice is kept.
  let chosen = $state<Record<RecipientId, boolean>>(
    untrack(() => ({ base: true, places: true, flood: suggested.flood, wildfire: suggested.wildfire })),
  );

  const shown = $derived(mode === 'pins' ? RECIPIENTS.filter((r) => r.id === 'base') : RECIPIENTS);
  const anything = $derived(mode === 'pins' || Object.values(chosen).some(Boolean));

  function hint(id: RecipientId): string {
    if (id === 'flood') return suggested.flood ? 'Suggested: flooding is likely enough here to matter.' : 'Not suggested for your area, but you can add it.';
    if (id === 'wildfire') return suggested.wildfire ? 'Suggested: wildfire is likely enough here to matter.' : 'Not suggested for your area, but you can add it.';
    return '';
  }

  function fetchMaps() {
    if (mode === 'pins') onfetch({ base: true, places: false, flood: false, surge: false, wildfire: false });
    else onfetch({ base: chosen.base, places: chosen.places, flood: chosen.flood, surge: false, wildfire: chosen.wildfire });
  }
</script>

<section class="consent card" aria-labelledby="{uid}-title">
  <svelte:element this={`h${headingLevel}`} id="{uid}-title" class="consent__title">
    {mode === 'pins' ? 'Before the map opens' : 'Before we fetch your maps'}
  </svelte:element>
  <p>
    {#if mode === 'pins'}
      To place your pins, this app shows a street map from an outside service. Nothing has been sent yet.
    {:else}
      Your maps are drawn from outside services. Nothing has been sent yet. Choose what to include: each service below gets only
      what it says.
    {/if}
  </p>

  {#if mode === 'pins'}
    {#each shown as r (r.id)}
      <div class="recipient recipient--only">
        <p class="recipient__who"><strong>{r.layer}:</strong> {r.who}</p>
        <p class="recipient__gets">They receive: which parts of the map you look at, which shows roughly where you are looking.</p>
      </div>
    {/each}
  {:else}
    <fieldset>
      <legend>What to include, and who receives what</legend>
      <ul class="recipients">
        {#each shown as r (r.id)}
          <li>
            <label class="choice">
              <input type="checkbox" bind:checked={chosen[r.id]} aria-describedby="{uid}-{r.id}-who {uid}-{r.id}-gets" />
              <span class="choice__text">
                <span class="recipient__layer">{r.layer}</span>
                <span class="choice__help" id="{uid}-{r.id}-who">{r.who}</span>
                <span class="choice__help recipient__gets" id="{uid}-{r.id}-gets">They receive: {r.receives.charAt(0).toLowerCase()}{r.receives.slice(1)}</span>
                {#if hint(r.id)}<span class="choice__help recipient__hint">{hint(r.id)}</span>{/if}
              </span>
            </label>
          </li>
        {/each}
      </ul>
    </fieldset>
    {#if suggested.surge}
      <p class="note">{SURGE_NOTE} To find your evacuation zone, use your state or county emergency office’s map.</p>
    {/if}
    {#if !chosen.base && !hasHomePin}
      <p class="note" role="status">
        Without the street map there is nothing to place your pins on, so the maps will centre on the middle of your ZIP code and will
        not mark your home.
      </p>
    {/if}
    {#if !anything}
      <p class="note" role="status">With nothing ticked, the maps show only your county line and anything you already placed.</p>
    {/if}
  {/if}

  <p class="small">{EVERY_RECIPIENT_SEES}</p>
  <p class="small">
    “Type an address instead” on the pin map sends what you type to OpenStreetMap’s search service, run by the OpenStreetMap Foundation
    (UK). You will be asked first.
  </p>
  {#if offline}
    <p class="note" role="status">This device seems to be offline. The maps need an internet connection; you can still try.</p>
  {/if}
  <p class="small muted">You will see this every time. Nothing about your choice is remembered.</p>

  <div class="button-row">
    <button type="button" class="button button--primary" onclick={fetchMaps}>{mode === 'pins' ? 'Show the map' : 'Fetch maps'}</button>
    <button type="button" class="button" onclick={oncancel}>Not now</button>
  </div>
</section>

<style>
  .consent {
    max-width: var(--w-text, 44rem);
  }
  .consent__title {
    margin-top: 0;
  }
  .recipients {
    display: grid;
    gap: var(--s2);
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s4);
  }
  .recipients li + li {
    margin-top: 0;
  }
  .recipient__layer {
    font-weight: 650;
  }
  .recipient__gets {
    color: var(--text);
  }
  .recipient__hint {
    font-style: italic;
  }
  .recipient--only {
    padding: var(--s3) var(--s4);
    border: 1px solid var(--border);
    border-radius: var(--r2);
    margin-bottom: var(--s4);
  }
  .recipient--only p {
    margin: 0;
  }
  .recipient--only p + p {
    margin-top: var(--s1);
  }
  .note {
    padding: var(--s3) var(--s4);
    border-left: 4px solid var(--note-edge);
    background: var(--note-soft);
    border-radius: var(--r1);
  }
</style>
