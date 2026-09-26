<!--
  A readiness bucket as a have / not-yet list: things you either have ready or don't. The list
  comes from the plan items for this bucket; each line says "Have" or "Not yet" in words.
-->
<script lang="ts">
  import type { BucketAssessment, PlanItem } from '../engine/types';
  import { dayPhrase, naturalFrequency, noticeRange } from '../lib/format';
  import ExplainButton from './ExplainButton.svelte';
  import Icon from './Icon.svelte';
  import Sources from './Sources.svelte';

  let { bucket, items }: { bucket: BucketAssessment; items: PlanItem[] } = $props();
  const uid = $props.id();

</script>

<article class="readiness card" aria-labelledby="{uid}-name">
  <h3 id="{uid}-name">{bucket.name}</h3>
  {#if bucket.target.kind === 'evacuate'}
    <p class="small">
      {naturalFrequency(bucket.target.p_need_10yr).replace(/^a/, 'A')} households like yours have to leave quickly in ten years.
      Notice could be {noticeRange(bucket.target.notice_hours_low, bucket.target.notice_hours_high)}; plan to be away about {dayPhrase(bucket.target.days_away)}.
    </p>
  {:else if bucket.target.kind === 'readiness'}
    <p class="small">{bucket.frequency_sentences[0]}</p>
  {/if}
  {#if items.length}
    {@const ready = items.filter((i) => i.done)}
    {@const todo = items.filter((i) => !i.done)}
    <p class="count"><strong>{ready.length} of {items.length}</strong> ready</p>
    {#if ready.length}
      <p class="list-title">Ready</p>
      <ul class="have-list">
        {#each ready as item (item.item_id + item.tier)}
          <li class="has"><Icon name="check" /><span>{item.name}</span></li>
        {/each}
      </ul>
    {/if}
    {#if todo.length}
      <p class="list-title">Still to do</p>
      <ul class="have-list">
        {#each todo as item (item.item_id + item.tier)}
          <li><Icon name="circle" /><span>{item.name}</span></li>
        {/each}
      </ul>
    {/if}
  {/if}
  <footer class="foot">
    <ExplainButton kind="bucket" id={bucket.id} />
    <Sources ids={bucket.sources} />
  </footer>
</article>

<style>
  .readiness {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .readiness h3 {
    font-size: var(--text-base);
    margin: 0;
  }
  .have-list {
    list-style: none;
    padding: 0;
    margin: 0;
    font-size: var(--text-sm);
  }
  .have-list li {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    color: var(--text-muted);
  }
  .have-list li.has {
    color: var(--good);
    font-weight: 600;
  }
  .list-title {
    margin: var(--s2) 0 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .count {
    margin: 0;
    font-size: var(--text-sm);
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
    margin-top: auto;
  }
</style>
