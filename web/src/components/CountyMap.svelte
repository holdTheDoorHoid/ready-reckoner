<!-- Placeholder for the county map thumbnail (the map workstream draws the real one). -->
<script lang="ts">
  import type { LocationResolved } from '../engine/types';
  let { location }: { location: LocationResolved } = $props();
</script>

<figure class="county-map">
  <div class="county-map__box" aria-hidden="true">
    <svg viewBox="0 0 120 80" width="100%" height="100%">
      <path d="M18 14 L62 8 L96 18 L104 44 L88 68 L44 72 L16 56 Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-dasharray="4 3" />
      <circle cx="60" cy="40" r="3" fill="currentColor" />
    </svg>
    <span class="county-map__label">County map</span>
  </div>
  <figcaption class="small muted">
    {location.county_name}, {location.state_abbr}. A map of your county will appear here in a later version.
    {#if location.data_note}<br />{location.data_note}{/if}
  </figcaption>
</figure>

<style>
  .county-map {
    margin: 0;
  }
  .county-map__box {
    position: relative;
    aspect-ratio: 3 / 2;
    max-height: 11rem;
    border: 2px dashed var(--border-strong);
    border-radius: var(--r2);
    background: var(--surface-2);
    color: var(--text-muted);
    padding: var(--s3);
  }
  .county-map__label {
    position: absolute;
    left: var(--s3);
    bottom: var(--s2);
    font-weight: 650;
    font-size: var(--text-sm);
  }
  figcaption {
    margin-top: var(--s2);
  }
</style>
