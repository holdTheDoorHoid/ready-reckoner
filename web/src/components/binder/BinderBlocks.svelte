<!--
  A binder page's blocks on screen (DESIGN-DELTA-v3 §4.1, §6): headings, paragraphs, lists,
  airline steps (the "do first, from memory" ones bold on a band), labelled answers with a ruled
  line where there is none yet, tables, boxed notes that say their kind in a word, leave-or-stay
  decisions as an "If / Then" table, maps, wallet cards, blank logs to fill in by hand, and page
  breaks for paper. A page's top headings (the engine's `base` level) are `level` (the page title is
  the level above), so no heading level is skipped whichever level the engine starts a page at.
-->
<script lang="ts">
  import type { Block } from '../../engine/types';
  import { sourceDomId } from '../../lib/binder/model';
  import { CALLOUT_STYLES } from '../../lib/binder/palette';
  import { href } from '../../lib/router.svelte';
  import BinderBlocks from './BinderBlocks.svelte';
  import BinderInlines from './BinderInlines.svelte';
  import BinderMap from './BinderMap.svelte';
  import type { BinderView } from './view';

  let {
    blocks,
    view,
    level = 4,
    base = 1,
    label = '',
    sourceList = -1,
  }: {
    blocks: readonly Block[];
    view: BinderView;
    level?: number;
    /** The level of the page's top headings in the engine's tree (1 on checklists, 2 on most pages). */
    base?: number;
    label?: string;
    /** The index of the block that lists the numbered sources: its items become the citations' anchors. */
    sourceList?: number;
  } = $props();

  const tag = (l: number) => `h${Math.min(6, Math.max(2, level + l - base))}`;
  /** The page a decision branch turns to, with the words a reader sees. */
  const turnTo = (id: string | undefined) => {
    const e = id ? view.pages.get(id) : undefined;
    return e ? { id: e.page.id, text: `Tab ${e.part.tab}, ${e.page.title}` } : undefined;
  };
  // A table that may scroll sideways on a phone takes keyboard focus (tabindex 0, with a region
  // name), so it can be scrolled without a mouse; hence the svelte-ignore comments below.
  /** Each table's name for screen readers ("House fire: table 2, If this happens"), by block. */
  const tableNames = $derived.by(() => {
    const names = new Map<number, string>();
    let n = 0;
    blocks.forEach((bl, i) => {
      const header = 'table' in bl ? bl.table.header : 'log' in bl ? bl.log.columns : null;
      if (!header) return;
      n += 1;
      names.set(i, `${label ? `${label}: ` : ''}table ${n}${header[0] ? `, ${header[0]}` : ''}`);
    });
    return names;
  });
</script>

{#each blocks as block, b (b)}
  {#if 'heading' in block}
    <svelte:element this={tag(block.heading.level)} class="bh bh--{block.heading.level}">{block.heading.text}</svelte:element>
  {:else if 'para' in block}
    <p><BinderInlines inlines={block.para} {view} /></p>
  {:else if 'bullets' in block}
    <ul class="b-list">
      {#each block.bullets as item, i (i)}<li><BinderInlines inlines={item} {view} /></li>{/each}
    </ul>
  {:else if 'numbered' in block}
    {#if b === sourceList}
      <ol class="b-list b-sources">
        {#each block.numbered as item, i (i)}<li id={sourceDomId(i + 1)} tabindex="-1"><BinderInlines inlines={item} {view} /></li>{/each}
      </ol>
    {:else}
      <ol class="b-list">
        {#each block.numbered as item, i (i)}<li><BinderInlines inlines={item} {view} /></li>{/each}
      </ol>
    {/if}
  {:else if 'steps' in block}
    <ol class="steps" role="list">
      {#each block.steps as step, i (i)}
        <li class="step" class:step--memory={step.memory}>
          {#if step.memory}<span class="visually-hidden">From memory: </span>{/if}<BinderInlines inlines={step.text} {view} />
        </li>
      {/each}
    </ol>
  {:else if 'fields' in block}
    <dl class="fields">
      {#each block.fields as row, i (i)}
        <div class="field">
          <dt>{row.label}</dt>
          {#if row.value !== undefined && row.value !== ''}
            <dd>{row.value}</dd>
          {:else}
            <dd class="ruled" style:--lines={Math.max(1, row.lines)}><span class="visually-hidden">Not filled in: a line to write on</span></dd>
          {/if}
        </div>
      {/each}
    </dl>
  {:else if 'table' in block}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="table-wrap" tabindex="0" role="region" aria-label={tableNames.get(b)}>
      <table class="b-table">
        <thead>
          <tr>{#each block.table.header as h, i (i)}<th scope="col">{h}</th>{/each}</tr>
        </thead>
        <tbody>
          {#each block.table.rows as row, r (r)}
            <tr>
              {#each block.table.header.length >= row.length ? block.table.header : row as _, c (c)}
                {@const cell = row[c] ?? []}
                <td class:write-in={cell.length === 0}>{#if cell.length}<BinderInlines inlines={cell} {view} />{:else}<span class="visually-hidden">Blank, to fill in on paper</span>{/if}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if 'callout' in block}
    {@const s = CALLOUT_STYLES[block.callout.kind] ?? CALLOUT_STYLES.note}
    <div class="callout callout--{block.callout.kind}" role="note" aria-label={block.callout.title ? `${s.word}: ${block.callout.title}` : s.word}>
      <p class="callout__label"><span class="callout__word">{s.word}</span>{#if block.callout.title}<strong>{block.callout.title}</strong>{/if}</p>
      <BinderBlocks blocks={block.callout.blocks} {view} level={level + 1} {base} {label} />
    </div>
  {:else if 'decision' in block}
    <div class="decision">
      <svelte:element this={tag(base)} class="bh bh--1">{block.decision.question}</svelte:element>
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="table-wrap" tabindex="0" role="region" aria-label={`${label ? `${label}: ` : ''}${block.decision.question}`}>
        <table class="b-table decision__table">
          <thead><tr><th scope="col">If</th><th scope="col">Then</th></tr></thead>
          <tbody>
            {#each block.decision.branches as br, i (i)}
              {@const to = turnTo(br.go_to)}
              <tr>
                <td><BinderInlines inlines={br.when} {view} /></td>
                <td>
                  <BinderInlines inlines={br.then} {view} />
                  {#if to}<a class="xref turn-to" href={href('binder', to.id)} data-page={to.id}>Turn to {to.text}</a>{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else if 'map_slot' in block}
    <BinderMap slot={block.map_slot} {view} />
  {:else if 'cards' in block}
    <ul class="wallet-cards">
      {#each block.cards as card, i (i)}
        <li class="wallet-card">
          <svelte:element this={tag(base)} class="wallet-card__title">{card.title}</svelte:element>
          {#each card.lines as line, j (j)}<p><BinderInlines inlines={line} {view} /></p>{/each}
        </li>
      {/each}
    </ul>
  {:else if 'log' in block}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="table-wrap" tabindex="0" role="region" aria-label={tableNames.get(b)}>
      <table class="b-table b-log">
        <thead><tr>{#each block.log.columns as c, i (i)}<th scope="col">{c}</th>{/each}</tr></thead>
        <tbody>
          {#each Array.from({ length: Math.max(1, block.log.rows) }) as _, r (r)}
            <tr>{#each block.log.columns as _c, i (i)}<td class="write-in"><span class="visually-hidden">{r === 0 && i === 0 ? 'Blank rows to fill in on paper' : ''}</span></td>{/each}</tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if 'page_break' in block}
    <div class="binder-break" aria-hidden="true"></div>
  {/if}
{/each}

<style>
  .bh {
    margin: var(--s5) 0 var(--s2);
  }
  .bh--1 {
    font-size: var(--text-lg);
  }
  .bh--2 {
    font-size: var(--text-base);
  }
  .bh--3 {
    font-size: var(--text-base);
    font-style: italic;
  }
  .b-list {
    padding-left: 1.4em;
  }
  .b-sources {
    font-size: var(--text-sm);
    padding-left: 2.6em;
    columns: 2 22rem;
    column-gap: var(--s6);
  }
  .b-sources li {
    break-inside: avoid;
    margin-bottom: var(--s1);
    overflow-wrap: anywhere;
  }
  .b-sources li:focus {
    outline: 2px solid var(--focus);
  }
  .steps {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s4);
    counter-reset: step;
  }
  .step {
    counter-increment: step;
    position: relative;
    padding: var(--s1) var(--s3) var(--s1) 2.4em;
    margin: 0 0 2px;
    border-radius: var(--r1);
  }
  .step::before {
    content: counter(step);
    position: absolute;
    left: 0.5em;
    width: 1.4em;
    text-align: right;
    font-weight: 700;
  }
  .step--memory {
    font-weight: 700;
    background: var(--surface-3);
  }
  .fields {
    margin: 0 0 var(--s4);
  }
  .field {
    display: grid;
    grid-template-columns: minmax(8rem, 34%) 1fr;
    gap: var(--s3);
    padding: var(--s2) 0;
    border-bottom: 1px solid var(--border);
  }
  .field dt {
    font-weight: 700;
    font-size: var(--text-sm);
  }
  .field dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .ruled {
    min-height: calc(var(--lines) * 1.6em);
    background: repeating-linear-gradient(to bottom, transparent 0, transparent calc(1.6em - 1px), var(--border-strong) calc(1.6em - 1px), var(--border-strong) 1.6em);
  }
  .b-table {
    width: 100%;
  }
  .b-table td {
    vertical-align: top;
  }
  .write-in {
    height: 2rem;
  }
  .callout {
    margin: var(--s4) 0;
    padding: var(--s3) var(--s4);
    border: 2px solid var(--border-strong);
    border-radius: var(--r1);
    background: var(--surface);
  }
  .callout--stop {
    border: 3px solid var(--danger);
  }
  .callout--warning {
    border-color: var(--warn-edge);
    background: var(--warn-soft);
  }
  .callout--decision {
    border-style: dashed;
  }
  .callout--note {
    border-width: 1px;
    background: var(--note-soft);
  }
  .callout__label {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    align-items: baseline;
    margin: 0 0 var(--s2);
  }
  .callout__word {
    font-weight: 800;
    letter-spacing: 0.04em;
    font-size: var(--text-sm);
    padding: 0 var(--s2);
    border: 2px solid currentColor;
    border-radius: var(--r1);
  }
  .callout--stop .callout__word {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  .callout--warning .callout__word {
    background: var(--warn);
    border-color: var(--warn);
    color: #fff;
  }
  .callout :global(> :last-child) {
    margin-bottom: 0;
  }
  .turn-to {
    display: inline-block;
    font-weight: 700;
    margin-left: 0.25em;
  }
  .wallet-cards {
    list-style: none;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 21rem), 1fr));
    gap: var(--s4);
  }
  .wallet-card {
    border: 2px dashed var(--border-strong);
    border-radius: var(--r1);
    padding: var(--s3);
    font-size: var(--text-sm);
    break-inside: avoid;
  }
  .wallet-card p {
    margin: 0 0 var(--s1);
  }
  .wallet-card__title {
    margin: 0 0 var(--s2);
    font-size: var(--text-base);
  }
  .binder-break {
    border-top: 1px dashed var(--border);
    margin: var(--s5) 0;
  }
  @media print {
    /* Paper: black on white, the same marks as the PDF (palette.ts), shading kept where it means something. */
    .bh {
      margin: 8pt 0 3pt;
      break-after: avoid;
    }
    .bh--1 {
      font-size: 11pt;
    }
    .bh--2,
    .bh--3 {
      font-size: 10pt;
    }
    .step {
      padding: 1.5pt 4pt 1.5pt 2.2em;
      margin: 0 0 1.5pt;
      break-inside: avoid;
    }
    .step--memory {
      background: #e3e3e3 !important;
      print-color-adjust: exact;
      -webkit-print-color-adjust: exact;
    }
    .field {
      padding: 2pt 0;
      border-bottom: 0.5pt solid #8c8c8c;
      break-inside: avoid;
    }
    .ruled {
      background: repeating-linear-gradient(to bottom, transparent 0, transparent calc(1.6em - 0.6pt), #000 calc(1.6em - 0.6pt), #000 1.6em) !important;
      print-color-adjust: exact;
      -webkit-print-color-adjust: exact;
    }
    .callout {
      background: #fff !important;
      border-color: #000 !important;
      border-radius: 0;
      break-inside: avoid;
    }
    .callout--stop {
      border-width: 2.5pt !important;
    }
    .callout--warning {
      border-width: 1.5pt !important;
    }
    .callout--decision {
      border-width: 1.25pt !important;
      border-style: dashed !important;
    }
    .callout--note {
      border-width: 0.75pt !important;
    }
    .callout__word {
      print-color-adjust: exact;
      -webkit-print-color-adjust: exact;
      border-color: #000 !important;
    }
    .callout--stop .callout__word {
      background: #000 !important;
      color: #fff !important;
    }
    .callout--warning .callout__word {
      background: #4d4d4d !important;
      color: #fff !important;
    }
    .callout--note .callout__word {
      background: #e6e6e6 !important;
      color: #000 !important;
    }
    .write-in {
      height: 16pt;
    }
    /* Wallet cards: the real size, 3.5 by 2 inches, cut along the dashed edge, never split. */
    .wallet-cards {
      display: block;
    }
    .wallet-card {
      display: inline-block;
      vertical-align: top;
      box-sizing: border-box;
      width: 3.5in;
      height: 2in;
      overflow: hidden;
      margin: 0 0.15in 0.15in 0;
      padding: 6pt 7pt;
      border: 0.75pt dashed #000 !important;
      border-radius: 0;
      font-size: 8pt;
      line-height: 1.2;
      break-inside: avoid;
    }
    .wallet-card__title {
      font-size: 9pt;
      margin-bottom: 2pt;
    }
    .binder-break {
      border: 0;
      margin: 0;
      break-after: page;
    }
  }
</style>
