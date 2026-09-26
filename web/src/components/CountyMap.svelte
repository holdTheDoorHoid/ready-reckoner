<!--
  The county map thumbnail: the county marked inside its state (a hatched fill and a heavy
  outline, so it reads in black and white and without colour), or, for a ZIP code that spans
  several counties, the area around them with each candidate numbered to match the list. Plain
  inline SVG from the lazy `geo` pack; nothing is fetched from anywhere else, and it prints.
-->
<script lang="ts">
  import type { LocationResolved } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { areaView, type CountyShapes, MAP_HEIGHT, MAP_WIDTH, type MapView, stateView } from '../lib/geo';

  let {
    location,
    candidates = [],
    selected,
    caption = true,
    size = 'normal',
  }: {
    location?: LocationResolved | null;
    /** Several counties to choose between (an ambiguous ZIP code), numbered in this order. */
    candidates?: readonly LocationResolved[];
    selected?: string;
    caption?: boolean;
    size?: 'small' | 'normal';
  } = $props();

  const app = useApp();
  const uid = $props.id();

  let shapes = $state.raw<CountyShapes | null | undefined>(undefined);
  let failed = $state(false);

  $effect(() => {
    let live = true;
    app
      .countyShapes()
      .then((s) => {
        if (live) shapes = s;
      })
      .catch(() => {
        if (live) failed = true;
      });
    return () => {
      live = false;
    };
  });

  const choosing = $derived(candidates.length > 1);
  const view = $derived.by((): MapView | null => {
    if (!shapes) return null;
    if (choosing) return areaView(shapes, candidates.map((c) => c.county_fips), selected);
    return location ? stateView(shapes, location.county_fips) : null;
  });

  const place = $derived(location ? `${location.county_name}, ${location.state_name}` : '');
  const label = $derived.by(() => {
    if (choosing) {
      const names = candidates.map((c, i) => `${i + 1}: ${c.county_name}, ${c.state_abbr}`).join('; ');
      return `Map of the counties this ZIP code spans, numbered as in the list (${names}).`;
    }
    return location ? `Map of ${location.state_name} with ${location.county_name} marked.` : 'County map';
  });
</script>

<figure class="county-map" class:small={size === 'small'}>
  {#if view}
    <svg
      class="county-map__svg"
      viewBox="0 0 {MAP_WIDTH} {MAP_HEIGHT}"
      role="img"
      aria-labelledby="{uid}-title"
      preserveAspectRatio="xMidYMid meet"
    >
      <title id="{uid}-title">{label}</title>
      <defs>
        <pattern id="{uid}-hatch" patternUnits="userSpaceOnUse" width="4" height="4" patternTransform="rotate(45)">
          <rect width="4" height="4" class="hatch-bg" />
          <line x1="0" y1="0" x2="0" y2="4" class="hatch-line" />
        </pattern>
        <pattern id="{uid}-dots" patternUnits="userSpaceOnUse" width="4" height="4">
          <rect width="4" height="4" class="dots-bg" />
          <circle cx="2" cy="2" r="0.9" class="dots-dot" />
        </pattern>
      </defs>
      {#each view.paths as p (p.fips)}
        <path
          d={p.d}
          class="county county--{p.role}"
          fill-rule="evenodd"
          fill={p.role === 'focus' ? `url(#${uid}-hatch)` : p.role === 'candidate' ? `url(#${uid}-dots)` : undefined}
        />
      {/each}
      {#each view.markers as m (m.fips)}
        {#if m.label}
          <g class="marker marker--numbered">
            <circle cx={m.x} cy={m.y} r="10.5" />
            <text x={m.x} y={m.y} dy="0.35em" text-anchor="middle">{m.label}</text>
          </g>
        {:else}
          <circle class="marker marker--ring" cx={m.x} cy={m.y} r={m.r ?? 8} />
        {/if}
      {/each}
    </svg>
  {:else if shapes === undefined && !failed}
    <div class="county-map__placeholder" aria-busy="true"><span>Loading the map…</span></div>
  {:else}
    <div class="county-map__placeholder"><span>{failed ? 'The map could not be loaded. It will try again next time.' : 'No map is available for this place.'}</span></div>
  {/if}
  {#if caption && (place || choosing)}
    <figcaption class="small muted">
      {#if choosing}
        The numbers match the list. The dotted areas are the counties this ZIP code covers{selected ? '; the striped one is your choice' : ''}.
      {:else}
        <strong class="county-map__place">{place}</strong> (striped).
        {location?.data_note ?? 'Your risks are worked out for the whole county.'}
      {/if}
    </figcaption>
  {/if}
</figure>

<style>
  .county-map {
    margin: 0;
  }
  .county-map__svg,
  .county-map__placeholder {
    display: block;
    width: 100%;
    aspect-ratio: 3 / 2;
    max-height: 13rem;
    border: 1px solid var(--border);
    border-radius: var(--r2);
    background: var(--map-water);
  }
  .small .county-map__svg,
  .small .county-map__placeholder {
    max-height: 9rem;
    max-width: 13.5rem;
  }
  .county-map__placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--s3);
    color: var(--text-muted);
    font-size: var(--text-sm);
    text-align: center;
  }
  .county {
    stroke: var(--map-line);
    stroke-width: 0.6;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
  .county--context {
    fill: var(--map-land);
  }
  .county--focus,
  .county--candidate {
    stroke: var(--map-focus-line);
    stroke-width: 2;
  }
  .county--focus {
    stroke-width: 2.5;
  }
  .hatch-bg,
  .dots-bg {
    fill: var(--map-focus-bg);
  }
  .hatch-line {
    stroke: var(--map-focus-line);
    stroke-width: 1.6;
  }
  .dots-dot {
    fill: var(--map-focus-line);
  }
  .marker--ring {
    fill: none;
    stroke: var(--map-focus-line);
    stroke-width: 2;
  }
  .marker--numbered circle {
    fill: var(--surface);
    stroke: var(--map-focus-line);
    stroke-width: 1.5;
  }
  .marker--numbered text {
    fill: var(--text);
    font: 700 12.5px var(--font);
  }
  figcaption {
    margin-top: var(--s2);
  }
  .county-map__place {
    color: var(--text);
  }
  @media print {
    .county-map {
      break-inside: avoid;
      max-width: 11cm;
    }
    .county-map__svg {
      max-height: 6.5cm;
      border-color: #999;
      background: #fff;
    }
  }
</style>
