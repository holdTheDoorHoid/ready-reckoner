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
          {@render row(i)}
        </fieldset>
      </li>
    {/each}
  </ol>
{/if}
{#if count < max}
  <button id="{id}-add" type="button" class="button button--small" onclick={add}><Icon name="plus" /> {addLabel}</button>
{:else}
  <p class="small muted" id="{id}-add" tabindex="-1">{fullText}</p>
{/if}

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
  .row__legend {
    float: left;
    width: 100%;
    padding-right: 7.5rem;
    margin-bottom: var(--s3);
    overflow-wrap: anywhere;
  }
  .row__legend + * {
    clear: both;
  }
  .row__remove {
    position: absolute;
    top: var(--s2);
    right: var(--s2);
  }
</style>
