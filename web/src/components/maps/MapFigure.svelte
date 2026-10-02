<!--
  One map in its slot (DESIGN-DELTA-v3 §4.1 `map_slot`, §9.2): the composed image (with its scale
  bar, north arrow and credit drawn in), then the numbered legend table (name, kind, address or
  phone when OpenStreetMap has them), the patterns explained with a swatch each (so nothing rests
  on colour), a line for every layer that is missing and why, the notes, and the credit lines with
  their dates. Without an image it draws the dashed placeholder the printed binder uses, saying
  "Maps need refreshing" when the plan has pins but this device has no matching images.

  web-binder renders one per `map_slot` block; the maps panel uses it for its three maps. Place
  names and addresses are OpenStreetMap's data, shown as text (never as markup).
-->
<script lang="ts">
  import type { LegendKey } from '../../lib/maps/compose';
  import type { MapSlotKind } from '../../lib/maps/slots';
  import { FIX_THE_MAP_URL } from '../../lib/maps/sources';
  import type { MapRecord, MapsStatus } from '../../lib/maps/store';

  let {
    slot,
    caption,
    record = null,
    status = 'none',
  }: {
    slot: MapSlotKind;
    /** The slot's caption from the binder ("Your neighbourhood"). */
    caption: string;
    record?: MapRecord | null;
    /** Why there is no image, when there is none. */
    status?: MapsStatus;
  } = $props();

  const uid = $props.id();

  const PLACEHOLDER: Record<MapSlotKind, string> = {
    neighbourhood: 'your neighborhood',
    area: 'your city or county',
    region: 'your region and the ways out',
  };

  const places = $derived(record ? record.legend.filter((r) => !r.own) : []);
  const alt = $derived.by(() => {
    if (!record) return '';
    const own = record.legend.filter((r) => r.own && !r.offMap).map((r) => `${r.mark} ${r.name.toLowerCase()}`);
    const parts = [`Map of ${PLACEHOLDER[slot]}`];
    if (own.length) parts.push(`showing ${own.join(', ')}`);
    if (places.length) parts.push(`${places.length} places numbered as in the table below`);
    return `${parts.join(', ')}. ${record.scale}`;
  });

  function swatch(k: LegendKey): string {
    return k.pattern;
  }
</script>

<figure class="map-figure map-figure--{slot}" aria-labelledby="{uid}-cap">
  {#if record}
    <img class="map-figure__img" src={record.image} width={record.width} height={record.height} {alt} />
    <figcaption id="{uid}-cap" class="map-figure__caption"><strong>{caption}</strong></figcaption>
    {#if record.legend.length > 0}
      <table class="map-figure__legend">
        <caption class="visually-hidden">What the marks on the map of {PLACEHOLDER[slot]} show</caption>
        <thead>
          <tr>
            <th scope="col">Mark</th>
            <th scope="col">Name</th>
            <th scope="col">What it is</th>
            <th scope="col">Address or phone</th>
          </tr>
        </thead>
        <tbody>
          {#each record.legend as row (row.mark)}
            <tr>
              <td><span class="map-figure__mark" class:own={row.own}>{row.mark}</span></td>
              <td>{row.name}{#if row.offMap}<span class="small muted"> (beyond the edge of this map)</span>{/if}</td>
              <td>{row.kind}</td>
              <td>{[row.address, row.phone].filter(Boolean).join(' · ') || '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if record.keys.length > 0}
      <ul class="map-figure__keys">
        {#each record.keys as k (k.pattern)}
          <li>
            <svg class="map-figure__swatch" viewBox="0 0 28 16" aria-hidden="true" focusable="false">
              <defs>
                <pattern id="{uid}-{k.pattern}" width="6" height="6" patternUnits="userSpaceOnUse">
                  {#if swatch(k) === 'stripes'}<path d="M-1 7L7 -1M5 7L7 5M-1 1L1 -1" class="ink ink--flood" />{/if}
                  {#if swatch(k) === 'dots'}<circle cx="1.5" cy="1.5" r="1" class="dot dot--flood" /><circle cx="4.5" cy="4.5" r="1" class="dot dot--flood" />{/if}
                  {#if swatch(k) === 'stripes-back'}<path d="M-1 -1L7 7M5 -1L7 1M-1 5L1 7" class="ink ink--fire" />{/if}
                  {#if swatch(k) === 'cross'}<path d="M-1 7L7 -1M-1 -1L7 7" class="ink ink--fire" />{/if}
                </pattern>
              </defs>
              {#if k.pattern === 'route-1'}
                <line x1="1" y1="8" x2="27" y2="8" class="line line--solid" />
              {:else if k.pattern === 'route-2'}
                <line x1="1" y1="8" x2="27" y2="8" class="line line--dashed" />
              {:else if k.pattern === 'county'}
                <line x1="1" y1="8" x2="27" y2="8" class="line line--county" />
              {:else}
                <rect x="0.5" y="0.5" width="27" height="15" fill="url(#{uid}-{k.pattern})" class="box" />
              {/if}
            </svg>
            <span>{k.text}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if record.statuses.length > 0 || record.notes.length > 0}
      <ul class="map-figure__notes small">
        {#each record.statuses as s (s.layer)}<li>{s.text}</li>{/each}
        {#each record.notes as n (n)}<li>{n}</li>{/each}
      </ul>
    {/if}
    <p class="map-figure__credits small">
      {#each record.credits as c, i (c)}{i > 0 ? ' · ' : ''}{c}{/each}.
      <span class="no-print"><a href={FIX_THE_MAP_URL} rel="noopener">Report a mistake on the map</a>.</span>
    </p>
  {:else}
    <div class="map-figure__placeholder">
      <p><strong>Map: {PLACEHOLDER[slot]}.</strong></p>
      {#if status === 'stale'}
        <p>Maps need refreshing: this device has no maps for these pins yet.</p>
      {:else if status === 'unavailable'}
        <p>This browser does not keep maps. Add them in the app on another browser, or paste a printed map here.</p>
      {:else}
        <p>Add it in the app, or paste a printed map here.</p>
      {/if}
    </div>
    <figcaption id="{uid}-cap" class="map-figure__caption"><strong>{caption}</strong></figcaption>
  {/if}
</figure>

<style>
  .map-figure {
    margin: 0 0 var(--s6);
    break-inside: avoid;
  }
  .map-figure__img {
    display: block;
    width: 100%;
    max-width: 50rem;
    height: auto;
    border: 1px solid var(--border-strong);
  }
  .map-figure__caption {
    margin: var(--s2) 0;
  }
  .map-figure__legend {
    margin: var(--s2) 0 var(--s3);
    max-width: 50rem;
  }
  .map-figure__legend th,
  .map-figure__legend td {
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }
  .map-figure__mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.7em;
    height: 1.7em;
    padding: 0 0.25em;
    border: 2px solid #111;
    border-radius: 999px;
    font-weight: 700;
    font-size: var(--text-sm);
    background: #fff;
    color: #111;
  }
  .map-figure__mark.own {
    border-radius: 2px;
    background: #111;
    color: #fff;
  }
  .map-figure__keys,
  .map-figure__notes {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s3);
    max-width: 50rem;
  }
  .map-figure__keys li {
    display: flex;
    gap: var(--s2);
    align-items: center;
    margin: 0 0 var(--s1);
  }
  .map-figure__notes li {
    margin: 0 0 var(--s1);
  }
  .map-figure__swatch {
    flex: none;
    width: 28px;
    height: 16px;
  }
  .map-figure__swatch .box {
    stroke: #111;
    stroke-width: 1;
  }
  .map-figure__swatch .ink {
    stroke-width: 1.2;
    fill: none;
  }
  .map-figure__swatch .ink--flood,
  .map-figure__swatch .dot--flood {
    stroke: #15437d;
    fill: #15437d;
  }
  .map-figure__swatch .ink--fire {
    stroke: #7a2508;
  }
  .map-figure__swatch .line {
    stroke: #111;
    stroke-width: 3;
  }
  .map-figure__swatch .line--dashed {
    stroke-dasharray: 6 4;
  }
  .map-figure__swatch .line--county {
    stroke-width: 2;
    stroke-dasharray: 5 3;
  }
  .map-figure__credits {
    color: var(--text-muted);
    max-width: 50rem;
  }
  .map-figure__placeholder {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 12rem;
    max-width: 50rem;
    aspect-ratio: 10 / 7;
    padding: var(--s5);
    border: 2px dashed var(--border-strong);
    border-radius: var(--r2);
    color: var(--text-muted);
    text-align: center;
  }
  .map-figure__placeholder p {
    margin: 0 0 var(--s2);
  }
  @media print {
    .map-figure__img {
      max-width: 100%;
      border-color: #000;
    }
    .map-figure__legend,
    .map-figure__keys,
    .map-figure__notes,
    .map-figure__credits {
      font-size: 8.5pt;
    }
    .map-figure__credits {
      color: #000;
    }
    .map-figure__placeholder {
      border-color: #000;
      color: #000;
    }
  }
</style>
