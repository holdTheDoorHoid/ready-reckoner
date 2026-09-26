<!-- Says plainly, on every screen, when the numbers come from the stand-in engine. -->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import { href } from '../lib/router.svelte';
  const app = useApp();
</script>

{#if app.source?.kind === 'mock'}
  <section class="mock-banner" aria-label="About these numbers">
    <p>
      <strong>Sample numbers.</strong>
      This preview runs on a stand-in engine while the real one is built, so the risks, prices and sources are placeholders.
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
