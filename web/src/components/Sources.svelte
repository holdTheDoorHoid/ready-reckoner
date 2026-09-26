<!--
  The sources behind a number (docs/UI.md, SourceLink): a disclosure listing each citation with
  its title (linked, with the site's name), publisher, year, the date it was checked, the quoted
  passage the number rests on, and "Expert estimate" when the source is a judgement rather than
  data. Details come from the plan's own provenance (then the catalogue), so they match the
  numbers on screen.

  Two sizes: `footer` ("Sources (3)") under a card, and `inline` (a small "sources" after one
  number in a list). The list is only built when opened, so long plans stay light.
-->
<script lang="ts">
  import type { Citation } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { formatDate } from '../lib/format';

  let {
    ids,
    label = 'Sources',
    variant = 'footer',
    what,
  }: {
    ids: readonly string[];
    label?: string;
    variant?: 'footer' | 'inline';
    /** What the numbers are, for screen readers when the visible label is short ("sources"). */
    what?: string;
  } = $props();
  const app = useApp();
  let open = $state(false);

  const unique = $derived([...new Set(ids)]);

  const citations = $derived.by((): Citation[] => {
    if (!open) return [];
    const known = new Map<string, Citation>();
    for (const c of app.catalogue?.citations ?? []) known.set(c.id, c);
    for (const c of app.result.output?.provenance ?? []) known.set(c.id, c);
    return unique.map((id) => known.get(id) ?? { id, title: id, publisher: '', url: '', retrieved: '', license: '' });
  });

  function site(url: string): string {
    try {
      return new URL(url).hostname.replace(/^www\./, '');
    } catch {
      return '';
    }
  }
</script>

{#if unique.length}
  <details class="sources sources--{variant}" bind:open>
    <summary>
      {#if variant === 'inline'}
        sources<span class="visually-hidden"> for {what ?? 'this number'} ({unique.length})</span>
      {:else}
        {label} ({unique.length}){#if what}<span class="visually-hidden"> for {what}</span>{/if}
      {/if}
    </summary>
    {#if open}
      <ol class="sources__list">
        {#each citations as c (c.id)}
          <li>
            <p class="sources__title">
              {#if c.url}
                <a href={c.url} target="_blank" rel="noopener noreferrer">{c.title}<span class="visually-hidden"> (opens in a new tab)</span></a>
                {#if site(c.url)}<span class="meta"> ({site(c.url)})</span>{/if}
              {:else}
                <span>{c.title}</span>
              {/if}
              {#if c.prior}<span class="chip">Expert estimate</span>{/if}
            </p>
            {#if c.publisher || c.retrieved}
              <p class="meta">
                {#if c.publisher}{c.publisher}{c.year ? `, ${c.year}` : ''}.{/if}
                {#if c.retrieved}Checked {formatDate(c.retrieved)}.{/if}
              </p>
            {:else}
              <p class="meta">Details for this source appear when the full planner is running.</p>
            {/if}
            {#if c.quote}<blockquote class="sources__quote">“{c.quote}”</blockquote>{/if}
          </li>
        {/each}
      </ol>
    {/if}
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
  .sources--inline {
    display: inline-block;
    vertical-align: baseline;
  }
  .sources--inline summary {
    min-height: 0;
    padding: 0 var(--s1);
    font-size: 0.8125rem;
    text-decoration: underline dotted;
    text-underline-offset: 0.2em;
  }
  .sources--inline summary::marker,
  .sources--inline summary::-webkit-details-marker {
    content: '';
    display: none;
  }
  .sources--inline[open] {
    display: block;
    margin: var(--s1) 0 var(--s2);
  }
  .sources__list {
    margin: var(--s2) 0 var(--s2);
    padding-left: 1.5em;
  }
  .sources__list li + li {
    margin-top: var(--s2);
  }
  .sources__list p {
    margin: 0;
  }
  .sources__title a {
    font-weight: 600;
  }
  .meta {
    color: var(--text-muted);
  }
  .chip {
    margin-left: var(--s1);
  }
  .sources__quote {
    margin: var(--s1) 0 0;
    padding: var(--s1) var(--s3);
    border-left: 3px solid var(--border-strong);
    color: var(--text-muted);
    font-style: italic;
  }
</style>
