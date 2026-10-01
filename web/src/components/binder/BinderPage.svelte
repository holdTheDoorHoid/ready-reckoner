<!--
  One page of the binder on screen, as it prints: the tab it sits behind, its title (the cover
  shows the binder's own title), its blocks, and on the Sources page the numbered sources (each an
  anchor the citation numbers jump to) and the data credits. "Print this page" prints it alone, so
  a page can be reprinted when something changes.
-->
<script lang="ts">
  import Icon from '../Icon.svelte';
  import { creditsNotShown, pageDomId, sourceDomId, sourceListIndex, type PageEntry } from '../../lib/binder/model';
  import BinderBlocks from './BinderBlocks.svelte';
  import type { BinderView } from './view';

  let { entry, view, onprint }: { entry: PageEntry; view: BinderView; onprint?: (id: string) => void } = $props();

  const page = $derived(entry.page);
  const titleId = $derived(`${pageDomId(page.id)}-title`);
  const title = $derived(page.kind === 'cover' ? view.binder.title : page.title);
  /** The engine's own list of the numbered sources on this page, or -1. */
  const listAt = $derived(sourceListIndex(page, view.binder));
  /** Credits the page does not print itself. */
  const credits = $derived(page.kind === 'sources' ? creditsNotShown(page, view.binder) : []);
</script>

<article class="binder-page binder-page--{page.kind} fit-{page.fit}" id={pageDomId(page.id)} aria-labelledby={titleId} data-page={page.id}>
  <header class="binder-page__head">
    <p class="binder-page__tab"><span class="binder-page__tabno">{entry.part.tab}</span> {entry.part.short_title}</p>
    <h3 id={titleId} class="binder-page__title" tabindex="-1">{title}</h3>
    {#if onprint}
      <button type="button" class="button button--small button--quiet no-print" onclick={() => onprint(page.id)}>
        <Icon name="print" /> Print this page<span class="visually-hidden">: {title}</span>
      </button>
    {/if}
  </header>
  <BinderBlocks blocks={page.blocks} {view} label={title} sourceList={listAt} />
  {#if page.kind === 'sources'}
    {#if view.binder.sources.length && listAt < 0}
      <h4 class="binder-sources__title">Numbered sources</h4>
      <ol class="binder-sources">
        {#each view.binder.sources as s (s.n)}
          <li id={sourceDomId(s.n)} value={s.n} tabindex="-1">
            {s.publisher}, {s.title}{s.year ? ` (${s.year})` : ''}{s.expert ? ', an expert estimate' : ''}.
            {#if s.url}<a class="bare-url" href={s.url} rel="noopener noreferrer" target="_blank">{s.url}<span class="visually-hidden"> (opens in a new tab)</span></a>{/if}
          </li>
        {/each}
      </ol>
    {/if}
    {#if credits.length}
      <h4 class="binder-sources__title">Data credits</h4>
      <ul class="binder-credits">
        {#each credits as c, i (i)}<li>{c}</li>{/each}
      </ul>
    {/if}
  {/if}
</article>

<style>
  .binder-page {
    padding: var(--s5);
    margin: 0 0 var(--s5);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r2);
    scroll-margin-top: var(--s4);
  }
  .binder-page__head {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0 var(--s3);
    align-items: start;
    margin-bottom: var(--s3);
    padding-bottom: var(--s2);
    border-bottom: 2px solid var(--text);
  }
  .binder-page__tab {
    grid-column: 1 / -1;
    margin: 0 0 var(--s1);
    font-size: var(--text-sm);
    font-weight: 700;
    color: var(--text-muted);
  }
  .binder-page__tabno {
    display: inline-block;
    min-width: 1.6em;
    padding: 0 0.3em;
    text-align: center;
    background: var(--text);
    color: var(--bg);
    border-radius: 2px;
  }
  .binder-page__title {
    margin: 0;
    font-size: var(--text-xl);
  }
  .binder-page__title:focus {
    outline: none;
  }
  .binder-page__head .button {
    align-self: center;
  }
  .binder-page--cover .binder-page__title {
    font-size: var(--text-2xl);
  }
  .binder-sources__title {
    margin: var(--s5) 0 var(--s2);
  }
  .binder-sources {
    font-size: var(--text-sm);
    padding-left: 2.6em;
    columns: 2 22rem;
    column-gap: var(--s6);
  }
  .binder-sources li {
    break-inside: avoid;
    margin-bottom: var(--s1);
    overflow-wrap: anywhere;
  }
  .binder-sources li:focus {
    outline: 2px solid var(--focus);
  }
  .binder-credits {
    font-size: var(--text-sm);
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  @media print {
    .binder-page__head {
      border-bottom: 1.5pt solid #000;
      margin-bottom: 6pt;
      padding-bottom: 3pt;
    }
    .binder-page__tab {
      color: #000;
      font-size: 8.5pt;
    }
    .binder-page__tabno {
      background: #000 !important;
      color: #fff !important;
      print-color-adjust: exact;
      -webkit-print-color-adjust: exact;
    }
    .binder-page__title {
      font-size: 15pt;
    }
    .binder-page--cover .binder-page__title {
      font-size: 24pt;
      margin-top: 1in;
    }
    .binder-page--sources {
      font-size: 7.5pt;
    }
    .binder-page--sources :global(.b-sources),
    .binder-sources {
      font-size: 7pt;
      column-gap: 6mm;
    }
  }
</style>
