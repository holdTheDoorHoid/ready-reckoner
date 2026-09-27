<!--
  A repeating group on the optional steps (medicines, animals, vehicles, accounts, policies): the
  trusted-circle pattern. Each row is a fieldset with its own legend ("Medicine 2", or what the row
  names once typed) and a Remove button; "Add …" adds an empty row and moves focus into it, until
  the group is full. No row is ever required: an empty group is simply left out of the plan.
  Rows keep their places while being edited; the engine drops empty ones.
-->
<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    id,
    count,
    max,
    legend,
    addLabel,
    fullText,
    onadd,
    onremove,
    onstatus,
    row,
  }: {
    /** Prefix for the rows' ids: row `i` is `{id}-{i}`, the add button `{id}-add`. */
    id: string;
    count: number;
    max: number;
    /** The row's legend, for example "Medicine 2" or the name typed in it. */
    legend: (index: number) => string;
    addLabel: string;
    /** Said in place of the add button once the group is full. */
    fullText: string;
    /** Add an empty row; false when there was no room. */
    onadd: () => boolean;
    onremove: (index: number) => void;
    /** A short message for the page's live region ("Medicine 3 added."). */
    onstatus?: (text: string) => void;
    row: Snippet<[number]>;
  } = $props();

  async function add() {
    // The new row's place, read before adding (afterwards `count` may already include it).
    const index = count;
    if (!onadd()) return;
    await tick();
    onstatus?.(`${legend(index)} added.`);
    document.getElementById(`${id}-${index}`)?.querySelector<HTMLElement>('input, textarea, select')?.focus();
  }

  async function remove(index: number) {
    const name = legend(index);
    onremove(index);
    await tick();
    onstatus?.(`${name} removed.`);
    document.getElementById(`${id}-add`)?.focus();
  }
</script>

{#if count > 0}
  <ol class="rows">
    {#each { length: count } as _, i (i)}
      <li>
        <fieldset class="row" id="{id}-{i}">
          <legend class="row__legend">{legend(i)}</legend>
          <button type="button" class="button button--quiet button--small row__remove" onclick={() => remove(i)}>
            <Icon name="trash" /> Remove<span class="visually-hidden">{' '}{legend(i)}</span>
          </button>
          <div class="row__body">{@render row(i)}</div>
        </fieldset>
      </li>
    {/each}
  </ol>
{/if}
<div class="rows__add">
  {#if count < max}
    <button id="{id}-add" type="button" class="button button--small" onclick={add}><Icon name="plus" /> {addLabel}</button>
  {:else}
    <p class="small muted" id="{id}-add" tabindex="-1">{fullText}</p>
  {/if}
</div>

<style>
  .rows {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s3);
    display: grid;
    gap: var(--s3);
  }
  .rows li + li {
    margin-top: 0;
  }
  .row {
    position: relative;
    padding: var(--s3) var(--s4) var(--s1);
    border: 1px solid var(--border);
    border-radius: var(--r2);
    background: var(--surface);
  }
  /* Floated so it flows inside the row's border (a legend otherwise sits on the border line). */
  .row__legend {
    float: left;
    width: 100%;
    padding-right: 7.5rem;
    margin-bottom: var(--s3);
    overflow-wrap: anywhere;
    font-size: var(--text-lg);
  }
  /* Groups inside a row (a vet, an insurer) title below the row's own title. */
  .row .row__body :global(.contact-fields > legend) {
    font-size: var(--text-base);
  }
  /*
   * Everything else starts below the legend (nothing may flow beside the float), across the whole
   * row: without the explicit width Chrome shrinks a row to its content inside the grid of rows.
   */
  .row__body {
    clear: both;
    width: 100%;
  }
  .row__remove {
    position: absolute;
    top: var(--s2);
    right: var(--s2);
  }
  .rows__add {
    margin-bottom: var(--s5);
  }
  /* Last in its group: the group's own spacing follows. */
  .rows__add:last-child {
    margin-bottom: 0;
  }
  .rows__add p {
    margin: 0;
  }
</style>
