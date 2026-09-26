<!--
  The sources behind a number: a small disclosure listing each citation with its publisher, year,
  link and the date it was checked. Expert estimates are labelled as such.
-->
<script lang="ts">
  import type { Citation } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { formatDate } from '../lib/format';

  let { ids, label = 'Sources' }: { ids: readonly string[]; label?: string } = $props();
  const app = useApp();

  const citations = $derived.by(() => {
    const known = new Map<string, Citation>();
    for (const c of app.catalogue?.citations ?? []) known.set(c.id, c);
    for (const c of app.result.output?.provenance ?? []) known.set(c.id, c);
    return [...new Set(ids)].map((id) => known.get(id) ?? { id, title: id, publisher: '', url: '', retrieved: '', license: '' });
  });
</script>

{#if citations.length}
  <details class="sources">
    <summary>{label} ({citations.length})</summary>
    <ol>
      {#each citations as c (c.id)}
        <li>
          {#if c.url}
            <a href={c.url} target="_blank" rel="noopener noreferrer">{c.title}<span class="visually-hidden"> (opens in a new tab)</span></a>
          {:else}
            <span>{c.title}</span>
          {/if}
          {#if c.publisher}<span class="meta">. {c.publisher}{c.year ? `, ${c.year}` : ''}.</span>{/if}
          {#if c.retrieved}<span class="meta"> Checked {formatDate(c.retrieved)}.</span>{/if}
          {#if c.prior}<span class="chip">Expert estimate</span>{/if}
        </li>
      {/each}
    </ol>
  </details>
{/if}

<style>
  .sources {
    font-size: var(--text-sm);
  }
  .sources summary {
    min-height: 36px;
    color: var(--accent);
  }
  .sources ol {
    margin: var(--s2) 0 var(--s2);
    padding-left: 1.5em;
  }
  .meta {
    color: var(--text-muted);
  }
  .chip {
    margin-left: var(--s1);
  }
</style>
