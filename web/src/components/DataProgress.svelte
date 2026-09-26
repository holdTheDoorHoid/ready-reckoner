<!--
  A calm line under the header while the county data loads: how far it has got (measured in the
  manifest's file sizes), that it happens once, and, if it fails, why and a way to try again.
  It appears only when loading takes longer than half a second, so a visit that loads from this
  device's copy shows nothing. Screen readers hear when loading starts and when it is done, not
  every step.
-->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import Icon from './Icon.svelte';

  const app = useApp();
  const SHOW_AFTER_MS = 500;

  const core = $derived(app.data?.core);
  const loading = $derived(core?.phase === 'loading');
  const failed = $derived(core?.phase === 'failed');
  const pct = $derived(core && core.bytesTotal > 0 ? Math.min(99, Math.floor((core.bytesLoaded / core.bytesTotal) * 100)) : null);

  let visible = $state(false);
  let announcement = $state('');
  let announcedStart = false;

  $effect(() => {
    if (!loading) {
      if (visible && core?.phase === 'ready') announcement = 'The county data is ready.';
      visible = false;
      return;
    }
    const timer = setTimeout(() => {
      visible = true;
      if (!announcedStart) {
        announcedStart = true;
        announcement = 'Getting the county data ready. You can keep going; answers that need it will appear when it is in.';
      }
    }, SHOW_AFTER_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (failed) announcement = `The county data did not finish loading. ${core?.error ?? ''}`;
  });
</script>

{#if failed}
  <section class="data-progress data-progress--failed no-print" aria-label="County data">
    <div class="data-progress__inner">
      <p class="data-progress__text">
        <Icon name="alert" />
        <span><strong>The county data did not finish loading.</strong> {core?.error} Your answers are safe.</span>
      </p>
      <button type="button" class="button button--small" onclick={() => app.retryData()}><Icon name="refresh" /> Try again</button>
    </div>
  </section>
{:else if visible}
  <section class="data-progress no-print" aria-label="County data">
    <div class="data-progress__inner">
      <label class="data-progress__text" for="data-progress-bar">
        Getting the county data ready{pct === null ? '…' : `: ${pct}%`}
        <span class="small muted">A one-time download; after this the app also works offline.</span>
      </label>
      {#if pct === null}
        <progress id="data-progress-bar" max="100"></progress>
      {:else}
        <progress id="data-progress-bar" max="100" value={pct}></progress>
      {/if}
    </div>
  </section>
{/if}
<p class="visually-hidden" aria-live="polite">{announcement}</p>

<style>
  .data-progress {
    background: var(--note-soft);
    border-bottom: 1px solid var(--note-edge);
    font-size: var(--text-sm);
  }
  .data-progress--failed {
    background: var(--warn-soft);
    border-bottom-color: var(--warn-edge);
  }
  .data-progress__inner {
    max-width: var(--w-wide);
    margin: 0 auto;
    padding: var(--s2) var(--s4);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2) var(--s4);
  }
  .data-progress__text {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 var(--s3);
    margin: 0;
    font-weight: 600;
  }
  .data-progress--failed .data-progress__text {
    align-items: flex-start;
    gap: var(--s2);
    font-weight: 400;
  }
  .data-progress__text .small {
    font-weight: 400;
  }
  progress {
    flex: 0 1 16rem;
    height: 0.6rem;
    accent-color: var(--accent);
  }
</style>
