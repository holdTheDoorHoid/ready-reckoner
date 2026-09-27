<!--
  The pin map (DESIGN-DELTA-v3 §9.4): an interactive street map (Leaflet, loaded on first use)
  where the household places its pins (home, the meeting place near home, the one outside the
  neighbourhood, where it would go) and draws its two ways out as click-to-add lines.

  Pick what to place, then click the map. Every action also works without dragging or a mouse
  (WCAG 2.2, 2.5.7 "Dragging movements"): move the map with the arrow keys or the zoom buttons
  until the cross in the middle sits on the spot, then press "Put it at the cross". Pins can also
  be dragged. "Type an address instead" (D6) finds a place by its address after a warning.

  It fetches tiles, so it only ever opens after the consent screen. "Done" hands the new pins and
  lines back to the caller; "Cancel" drops them.
-->
<script lang="ts">
  import { onMount } from 'svelte';

  import type { LatLon } from '../../engine/types';
  import type { AddressMatch, AddressSearch as Search } from '../../lib/maps/nominatim';
  import { clearRoute, type Draft, fromDraft, movePin, PIN_MARKS, place, removePin, toDraft, type Tool, TOOL_LABELS, toolState, TOOLS, undoPoint } from '../../lib/maps/pins';
  import { FIX_THE_MAP_URL } from '../../lib/maps/sources';
  import type { MapsState, PinId } from '../../lib/maps/state';
  import { type CreateView, createLeafletView, type MapView, type ViewPin } from '../../lib/maps/view';
  import AddressSearch from './AddressSearch.svelte';

  let {
    value,
    center,
    zoom,
    ondone,
    oncancel,
    headingLevel = 3,
    createView = createLeafletView,
    search,
  }: {
    value: MapsState | undefined;
    center: LatLon;
    zoom: number;
    ondone: (next: MapsState) => void;
    oncancel: () => void;
    headingLevel?: 2 | 3 | 4;
    /** The map library; tests pass a stub. */
    createView?: CreateView;
    /** The address search service; tests pass a stub. */
    search?: Search;
  } = $props();

  const uid = $props.id();
  let draft = $state<Draft>(toDraft(undefined));
  let tool = $state<Tool>('home');
  let phase = $state<'loading' | 'ready' | 'failed'>('loading');
  let fallback = $state(false);
  let said = $state('');
  let element: HTMLDivElement | undefined = $state();
  let view: MapView | null = null;

  const isRoute = $derived(tool === 'route1' || tool === 'route2');

  function pinsOf(d: Draft): ViewPin[] {
    return (Object.keys(d.pins) as PinId[]).map((id) => ({ id, at: d.pins[id]!, mark: PIN_MARKS[id], label: `${TOOL_LABELS[id]} pin` }));
  }

  function sync() {
    if (!view) return;
    const snap = $state.snapshot(draft) as Draft;
    view.setPins(pinsOf(snap));
    view.setRoutes(snap.routes);
  }

  function say(text: string) {
    said = text;
  }

  function apply(at: LatLon) {
    draft = place(draft, tool, at);
    sync();
    say(`${TOOL_LABELS[tool]}: ${toolState(draft, tool).toLowerCase()}.`);
  }

  function atCross() {
    if (view) apply(view.getCenter());
  }

  function remove() {
    if (isRoute) return;
    draft = removePin(draft, tool as PinId);
    sync();
    say(`${TOOL_LABELS[tool]} removed.`);
  }

  function undo() {
    if (!isRoute) return;
    draft = undoPoint(draft, tool as 'route1' | 'route2');
    sync();
    say(`${TOOL_LABELS[tool]}: ${toolState(draft, tool).toLowerCase()}.`);
  }

  function clear() {
    if (!isRoute) return;
    draft = clearRoute(draft, tool as 'route1' | 'route2');
    sync();
    say(`${TOOL_LABELS[tool]} cleared.`);
  }

  function chooseAddress(match: AddressMatch) {
    const target: Tool = isRoute ? 'home' : tool;
    draft = place(draft, target, match.at);
    view?.setView(match.at, 16);
    sync();
    say(`${TOOL_LABELS[target]} placed at ${match.label}.`);
  }

  function done() {
    ondone(fromDraft($state.snapshot(draft) as Draft, value));
  }

  onMount(() => {
    draft = toDraft(value);
    tool = value?.home ? 'meeting_near' : 'home';
    let live = true;
    createView(element!, {
      center,
      zoom,
      onClick: (at) => apply(at),
      onPinMoved: (id, at) => {
        draft = movePin(draft, id, at);
        sync();
        say(`${TOOL_LABELS[id]} moved.`);
      },
      onFallback: () => (fallback = true),
    })
      .then((v) => {
        if (!live) {
          v.destroy();
          return;
        }
        view = v;
        phase = 'ready';
        sync();
      })
      .catch(() => {
        if (live) phase = 'failed';
      });
    return () => {
      live = false;
      view?.destroy();
      view = null;
    };
  });
</script>

<section class="pin-map" aria-labelledby="{uid}-title">
  <svelte:element this={`h${headingLevel}`} id="{uid}-title">Place your pins and ways out</svelte:element>
  <p class="small">
    Pick what to place, then click the map. Or move the map (arrow keys, or the + and − buttons) until the cross in the middle is on the
    spot, and press “Put it at the cross”. You can drag a pin to move it.
  </p>

  <div class="pin-map__layout">
    <fieldset class="pin-map__tools">
      <legend>What to place</legend>
      {#each TOOLS as t (t)}
        <label class="choice pin-map__tool">
          <input type="radio" name="{uid}-tool" value={t} bind:group={tool} />
          <span class="choice__text">
            <span>{#if t in PIN_MARKS}<span class="pin-map__mark" aria-hidden="true">{PIN_MARKS[t as PinId]}</span>{/if}{TOOL_LABELS[t]}</span>
            <span class="choice__help">{toolState(draft, t)}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <div class="pin-map__side">
      <div class="pin-map__frame">
        <div class="pin-map__map" bind:this={element} role="application" aria-label="Street map. Click to place the chosen pin or line point." aria-describedby="{uid}-said"></div>
        <div class="pin-map__cross" aria-hidden="true"></div>
        {#if phase === 'loading'}
          <p class="pin-map__over">Loading the map…</p>
        {:else if phase === 'failed'}
          <p class="pin-map__over">The map could not be loaded. Check your connection, or type an address instead.</p>
        {/if}
      </div>
      {#if fallback}
        <p class="small" role="status">The OpenStreetMap map did not answer, so this is the U.S. Census Bureau’s street map.</p>
      {/if}

      <div class="button-row pin-map__actions">
        <button type="button" class="button" onclick={atCross} disabled={phase !== 'ready'}>Put it at the cross</button>
        {#if isRoute}
          <button type="button" class="button button--quiet" onclick={undo}>Undo last point</button>
          <button type="button" class="button button--quiet" onclick={clear}>Clear this line</button>
        {:else}
          <button type="button" class="button button--quiet" onclick={remove} disabled={!draft.pins[tool as PinId]}>Remove this pin</button>
        {/if}
      </div>
      <p class="visually-hidden" id="{uid}-said" aria-live="polite">{said}</p>

      <AddressSearch onchoose={chooseAddress} target={isRoute ? TOOL_LABELS.home : TOOL_LABELS[tool]} {...search ? { search } : {}} />

      <p class="small muted pin-map__credit">
        Map © <a href="https://www.openstreetmap.org/copyright" rel="noopener">OpenStreetMap</a> contributors.
        <a href={FIX_THE_MAP_URL} rel="noopener">Report a mistake on the map</a>.
      </p>
    </div>
  </div>

  <div class="button-row">
    <button type="button" class="button button--primary" onclick={done}>Done</button>
    <button type="button" class="button" onclick={oncancel}>Cancel</button>
  </div>
</section>

<style>
  .pin-map__layout {
    display: grid;
    gap: var(--s4);
    margin-bottom: var(--s4);
  }
  @media (min-width: 52rem) {
    .pin-map__layout {
      grid-template-columns: 16rem minmax(0, 1fr);
    }
  }
  .pin-map__tools {
    display: grid;
    gap: var(--s2);
    align-content: start;
  }
  .pin-map__tool {
    padding: var(--s2) var(--s3);
  }
  .pin-map__mark {
    display: inline-block;
    min-width: 1.9em;
    margin-right: 0.4em;
    padding: 0 0.3em;
    background: #111;
    color: #fff;
    font-size: var(--text-sm);
    font-weight: 700;
    text-align: center;
    border-radius: 2px;
  }
  .pin-map__frame {
    position: relative;
  }
  .pin-map__map {
    height: min(60vh, 26rem);
    min-height: 18rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--r2);
    background: var(--surface-2);
  }
  .pin-map__cross {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 28px;
    height: 28px;
    margin: -14px 0 0 -14px;
    pointer-events: none;
    z-index: 500;
    background:
      linear-gradient(#111, #111) center / 2px 100% no-repeat,
      linear-gradient(#111, #111) center / 100% 2px no-repeat;
    filter: drop-shadow(0 0 1px #fff) drop-shadow(0 0 1px #fff);
  }
  .pin-map__over {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0;
    padding: var(--s4);
    text-align: center;
    background: var(--surface-2);
    border-radius: var(--r2);
    z-index: 600;
  }
  .pin-map__actions {
    margin-top: var(--s3);
  }
  .pin-map__credit {
    margin-top: var(--s3);
  }
  /* The pins Leaflet draws (outside this component's markup, so global). */
  :global(.rr-pin) {
    display: flex;
    align-items: center;
    justify-content: center;
    background: #111;
    color: #fff;
    border: 2px solid #fff;
    border-radius: 3px;
    box-shadow: 0 0 0 1px #111;
    font: 700 13px/1 var(--font);
  }
  :global(.rr-pin:focus-visible) {
    outline: 3px solid var(--focus);
    outline-offset: 2px;
  }
</style>
