<!--
  Says plainly, on every screen, when the numbers are not the full engine on the national data: the
  stand-in engine, or the real engine running on its seven built-in sample counties because no data
  pack is loaded (engine_info lists no packs).
-->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import { href } from '../lib/router.svelte';
  const app = useApp();
  // The real engine on its built-in sample counties: the site has no data files at all. (While
  // the data is still loading, the progress line says so instead.)
  const sampleCounties = $derived(
    app.source?.kind === 'wasm' && (app.data ? app.data.core.phase === 'none' : app.info !== null && app.info.packs_loaded.length === 0),
  );
</script>

{#if app.source?.kind === 'mock'}
  <section class="mock-banner" aria-label="About these numbers">
    <p>
      <strong>Sample numbers.</strong>
      This preview runs on a stand-in engine while the real one is built, so the risks, prices and sources are placeholders.
      <a href={href('about')}>What this means</a>
    </p>
  </section>
{:else if sampleCounties}
  <section class="mock-banner" aria-label="About these numbers">
    <p>
      <strong>Sample counties only.</strong>
      This copy of the site has no national data, so the planner knows just seven sample counties and cannot plan anywhere else.
      <a href={href('about')}>What this means</a>
    </p>
  </section>
{/if}

<style>
  .mock-banner {
    background: var(--mock-bg);
    color: var(--mock-ink);
    border-bottom: 1px solid var(--mock-edge);
    font-size: var(--text-sm);
  }
  .mock-banner p {
    max-width: var(--w-wide);
    margin: 0 auto;
    padding: var(--s2) var(--s4);
  }
  .mock-banner a {
    color: inherit;
    font-weight: 650;
  }
</style>
