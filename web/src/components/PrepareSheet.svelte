<!--
  The Prepare sheet on paper (DESIGN-DELTA-v3 §6): the engine's `prepare_markdown` (the plan month
  by month, what to buy, the documents and money decisions, the upkeep calendar) as a clean
  printed sheet. A line at the top says what it is and when it was made; the sections follow on
  from each other, never leaving a heading at the foot of a page; the sources are set in two
  columns of small type; the last line says which app made it. The Markdown goes through the
  site's sanitising renderer, as everywhere else.
-->
<script lang="ts">
  import type { IsoDate } from '../engine/types';
  import { formatDate } from '../lib/format';
  import { packetSections, renderMarkdown } from '../lib/markdown';

  let { markdown, planDate, appVersion }: { markdown: string; planDate: IsoDate; appVersion: string } = $props();

  const sections = $derived.by(() => {
    let tables = 0;
    return packetSections(markdown).map((s) => {
      const html = renderMarkdown(s.markdown, { idPrefix: `prep-${s.slug}`, headingOffset: 1, tableStart: tables, notesLabel: 'Notes: your preparation plan' });
      tables += html.split('class="table-wrap"').length - 1;
      return { slug: s.slug, html };
    });
  });
</script>

<article class="prepare-sheet" aria-label="Your preparation plan">
  <p class="prepare-sheet__kicker">Ready Reckoner · Prepare sheet · plan date {formatDate(planDate)}</p>
  {#each sections as s (s.slug)}
    <div class="prepare-sheet__section prepare-sheet__section--{s.slug}">{@html s.html}</div>
  {/each}
  <p class="prepare-sheet__made">Made with Ready Reckoner {appVersion}. Keep it with your binder; print it again when the plan changes.</p>
</article>

<style>
  .prepare-sheet {
    font-size: 10pt;
    line-height: 1.35;
  }
  .prepare-sheet__kicker {
    font-size: 8.5pt;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    border-bottom: 1.5pt solid #000;
    padding-bottom: 3pt;
    margin: 0 0 8pt;
  }
  .prepare-sheet :global(h2) {
    font-size: 18pt;
    margin: 0 0 6pt;
  }
  .prepare-sheet :global(h3) {
    font-size: 13pt;
    margin: 14pt 0 5pt;
    padding-bottom: 2pt;
    border-bottom: 1pt solid #000;
    break-after: avoid;
  }
  .prepare-sheet :global(h4),
  .prepare-sheet :global(h5) {
    font-size: 10.5pt;
    margin: 9pt 0 3pt;
    break-after: avoid;
  }
  .prepare-sheet :global(p),
  .prepare-sheet :global(ul),
  .prepare-sheet :global(ol) {
    margin: 0 0 5pt;
  }
  .prepare-sheet :global(blockquote) {
    border: 1pt solid #000;
    padding: 4pt 8pt;
    margin: 6pt 0;
  }
  .prepare-sheet :global(li:has(> label > input[type='checkbox'])) {
    list-style: none;
    margin-left: -1.2em;
  }
  .prepare-sheet :global(table) {
    font-size: 9pt;
  }
  .prepare-sheet :global(tr) {
    break-inside: avoid;
  }
  /* Sources: two columns of small type, as the packet printed them. */
  .prepare-sheet__section--sources {
    font-size: 7.5pt;
    line-height: 1.3;
    columns: 2;
    column-gap: 6mm;
  }
  .prepare-sheet__section--sources :global(h3) {
    column-span: all;
    font-size: 13pt;
  }
  .prepare-sheet__section--sources :global(p),
  .prepare-sheet__section--sources :global(li) {
    break-inside: avoid;
    overflow-wrap: anywhere;
  }
  .prepare-sheet__made {
    margin-top: 10pt;
    padding-top: 3pt;
    border-top: 0.75pt solid #000;
    font-size: 8pt;
  }
</style>
