<!-- A new version has downloaded in the background; it only takes over when the person chooses. -->
<script lang="ts">
  import { applyUpdate, updates } from '../lib/sw-register.svelte';
  import Icon from './Icon.svelte';
</script>

<section class="update" aria-label="Update available">
  {#if updates.ready}
    <div class="update-banner">
      <p role="status">A new version of Ready Reckoner is ready. Your plan is kept.</p>
      <button type="button" class="button button--primary button--small" onclick={applyUpdate}><Icon name="refresh" /> Reload to update</button>
    </div>
  {/if}
</section>

<style>
  .update-banner {
    position: fixed;
    left: 50%;
    bottom: var(--s4);
    transform: translateX(-50%);
    width: min(36rem, calc(100vw - 2rem));
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--r2);
    box-shadow: var(--shadow-2);
    z-index: 10;
  }
  .update-banner p {
    margin: 0;
  }
</style>
