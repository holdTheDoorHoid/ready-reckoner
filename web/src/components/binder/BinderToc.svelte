<!--
  The binder's table of contents beside it: each tab and the pages behind it, every entry a link
  to its page (`#/binder/<page-id>`), the page on screen marked. On a wide screen it stays in view
  as the binder scrolls; on a phone it folds away under "Contents".
-->
<script lang="ts">
  import type { Binder } from '../../engine/types';
  import { tableOfContents } from '../../lib/binder/model';
  import { href } from '../../lib/router.svelte';

  let { binder, current = '', onjump }: { binder: Binder; current?: string; onjump?: (id: string, e: MouseEvent) => void } = $props();

  const toc = $derived(tableOfContents(binder));
</script>

<nav class="binder-toc" aria-label="Binder contents">
  <details open>
    <summary class="binder-toc__summary">Contents</summary>
    <ol class="binder-toc__parts">
      {#each toc as part (part.id)}
        <li class="binder-toc__part">
          {#if part.pages[0]}
            <a class="binder-toc__partlink" href={href('binder', part.pages[0].id)} onclick={(e) => onjump?.(part.pages[0]!.id, e)}>
              <span class="binder-toc__tab">{part.tab}</span>
              {part.title}
            </a>
          {/if}
          <ol class="binder-toc__pages">
            {#each part.pages as page (page.id)}
              <li>
                <a href={href('binder', page.id)} aria-current={current === page.id ? 'location' : undefined} onclick={(e) => onjump?.(page.id, e)}>
                  {page.kind === 'cover' ? 'Cover' : page.title}
                </a>
              </li>
            {/each}
          </ol>
        </li>
      {/each}
    </ol>
  </details>
</nav>

<style>
  .binder-toc {
    font-size: var(--text-sm);
  }
  .binder-toc__summary {
    font-weight: 700;
    cursor: pointer;
    padding: var(--s2) 0;
  }
  .binder-toc__parts,
  .binder-toc__pages {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .binder-toc__part {
    margin: 0 0 var(--s3);
  }
  .binder-toc__partlink {
    display: flex;
    gap: var(--s2);
    align-items: baseline;
    font-weight: 700;
    color: var(--text);
    text-decoration: none;
    padding: var(--s1) 0;
  }
  .binder-toc__partlink:hover,
  .binder-toc__pages a:hover {
    text-decoration: underline;
  }
  .binder-toc__tab {
    display: inline-block;
    min-width: 1.6em;
    padding: 0 0.3em;
    text-align: center;
    background: var(--text);
    color: var(--bg);
    border-radius: 2px;
    font-size: 0.85em;
  }
  .binder-toc__pages a {
    display: block;
    padding: 0.2rem 0 0.2rem 2.1em;
    color: var(--text-muted);
    text-decoration: none;
    line-height: 1.3;
  }
  .binder-toc__pages a[aria-current='location'] {
    color: var(--text);
    font-weight: 700;
    box-shadow: inset 3px 0 0 var(--accent);
  }
</style>
