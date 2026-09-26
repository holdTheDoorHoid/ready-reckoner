<!--
  A reviewed plan block from content/guidance (plan_communication, plan_shelter) as help for one
  part of the family plan: folded under its own summary, kept or trimmed for this household the way
  the packet trims it (a tornado paragraph only where tornadoes are likely, the high-rise line only
  in a tall building), with each note linked to its source. Before the block exists in a build,
  a short fallback shows instead.
-->
<script lang="ts">
  import { withSourceLinks } from '../learn/articles';
  import { followInPageAnchor } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import { householdFacts } from '../lib/conditions';
  import { blockText, guidanceBlock } from '../lib/guidance';
  import { renderMarkdown } from '../lib/markdown';

  let { id, summary, fallback }: { id: string; summary: string; fallback: string } = $props();
  const app = useApp();

  const block = $derived(guidanceBlock(id));
  const html = $derived.by(() => {
    if (!block) return '';
    const text = blockText(block, householdFacts(app.engineInput, app.result.output));
    return renderMarkdown(withSourceLinks(text, app.catalogue?.citations), { idPrefix: `guide-${id}`, headingOffset: 2 });
  });
</script>

<details class="plan-guide">
  <summary>{summary}</summary>
  {#if block}
    <!-- The click handler only follows the notes' in-page links, which keyboards reach themselves. -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="prose plan-guide__body" onclick={followInPageAnchor}>
      {@html html}
    </div>
  {:else}
    <p class="plan-guide__body">{fallback}</p>
  {/if}
</details>

<style>
  .plan-guide {
    margin: 0 0 var(--s4);
    padding: var(--s2) var(--s4);
    border: 1px solid var(--border);
    border-radius: var(--r2);
    background: var(--surface-2);
  }
  .plan-guide summary {
    color: var(--accent);
  }
  .plan-guide__body {
    padding: var(--s2) 0 var(--s1);
    font-size: var(--text-sm);
  }
  .plan-guide__body :global(p) {
    margin: 0 0 var(--s3);
  }
  .plan-guide__body :global(ol),
  .plan-guide__body :global(ul) {
    margin: 0 0 var(--s3);
  }
  .plan-guide__body :global(.footnotes) {
    margin-top: var(--s3);
    padding-top: var(--s2);
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }
  .plan-guide__body :global(.footnotes ol) {
    margin-bottom: 0;
  }
  .plan-guide__body :global(sup a) {
    text-decoration: none;
    padding: 0 0.15em;
  }
</style>
