<!--
  "Why?": opens a short explanation from the engine, fetched only when opened. The arithmetic is
  shown in the expert view.
-->
<script lang="ts">
  import type { ExplainKind, Explanation } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import Sources from './Sources.svelte';

  let { kind, id, label = 'Why?' }: { kind: ExplainKind; id: string; label?: string } = $props();
  const app = useApp();

  let explanation = $state<Explanation | null>(null);
  let failed = $state('');
  let loading = $state(false);

  async function load() {
    if (!app.engine || !app.engineInput || loading) return;
    loading = true;
    const r = await app.engine.explain({ kind, id, input: app.engineInput });
    loading = false;
    if (r.ok) {
      explanation = r.value;
      failed = '';
    } else {
      failed = r.error.message;
    }
  }
</script>

<details
  class="explain"
  ontoggle={(e) => {
    if ((e.currentTarget as HTMLDetailsElement).open) void load();
  }}
>
  <summary>{label}</summary>
  <div class="explain__body" aria-live="polite" aria-busy={loading}>
    {#if loading && !explanation}
      <p class="muted">Working it out…</p>
    {:else if failed}
      <p>{failed}</p>
    {:else if explanation}
      <p class="explain__title">{explanation.title}</p>
      {#each explanation.plain as para, i (i)}<p>{para}</p>{/each}
      {#if app.prefs.expert && explanation.math?.length}
        <ul class="explain__math">
          {#each explanation.math as step, i (i)}<li>{step}</li>{/each}
        </ul>
      {/if}
      <Sources ids={explanation.sources.map((s) => s.id)} />
    {/if}
  </div>
</details>

<style>
  .explain summary {
    color: var(--accent);
    min-height: 36px;
    font-size: var(--text-sm);
  }
  .explain__body {
    border-left: 3px solid var(--accent-soft);
    padding: var(--s1) 0 var(--s1) var(--s3);
    margin: var(--s1) 0 var(--s3);
    font-size: var(--text-sm);
  }
  .explain__title {
    font-weight: 650;
  }
  .explain__math {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
    background: var(--surface-2);
    border-radius: var(--r1);
    padding: var(--s2) var(--s2) var(--s2) 1.8em;
  }
</style>
