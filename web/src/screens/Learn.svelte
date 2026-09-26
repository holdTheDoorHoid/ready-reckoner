<!--
  Screen 10, Learn: short articles (the reviewed topic blocks in content/guidance); one at a time at
  #/learn/<slug>, each note linked to its source. The blocks' own headings start at "##", which
  follows the page's h1 as h2.
-->
<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { ARTICLES, article, withSourceLinks } from '../learn/articles';
  import { followInPageAnchor } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import { renderMarkdown } from '../lib/markdown';
  import { href, useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();
  const current = $derived(router.current.param ? article(router.current.param) : undefined);
</script>

<div class="page page--narrow">
  {#if router.current.param && current}
    <p class="no-print"><a href={href('learn')}><Icon name="chevron-left" /> All articles</a></p>
    <h1 id="page-title" tabindex="-1">{current.title}</h1>
    {#if current.draft}<p class="chip draft">Draft text, still being reviewed</p>{/if}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <article class="prose" onclick={followInPageAnchor}>
      {@html renderMarkdown(withSourceLinks(current.body, app.catalogue?.citations), { idPrefix: `learn-${current.slug}`, headingOffset: 0 })}
    </article>
  {:else}
    <h1 id="page-title" tabindex="-1">Learn</h1>
    {#if router.current.param}<p class="card">That article could not be found. Here are the others.</p>{/if}
    <p class="lead">Short reads on how the plan thinks, and what research says about how disasters really go.</p>
    <ul class="articles">
      {#each ARTICLES as a (a.slug)}
        <li class="card">
          <h2><a href={href('learn', a.slug)}>{a.title}</a></h2>
          <p>{a.summary}</p>
          {#if a.draft}<p class="small muted">Draft text, still being reviewed.</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .articles {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--s3);
  }
  .articles li + li {
    margin-top: 0;
  }
  .articles h2 {
    margin: 0 0 var(--s2);
    font-size: var(--text-lg);
  }
  .articles p {
    margin: 0;
  }
  .draft {
    margin-bottom: var(--s4);
  }
  .prose :global(li + li) {
    margin-top: var(--s2);
  }
  .prose :global(.footnotes) {
    margin-top: var(--s5);
    padding-top: var(--s3);
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }
</style>
