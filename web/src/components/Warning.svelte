<!--
  A guardrail. It says what looks off and why, and offers "Keep anyway". It never stops anything:
  keeping folds it away (and it can be brought back).
-->
<script lang="ts">
  import type { Warning } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import Icon from './Icon.svelte';

  let { warning }: { warning: Warning } = $props();
  const app = useApp();
  const dismissed = $derived(app.isDismissed(warning.id));
  const uid = $props.id();
</script>

{#if dismissed}
  <div class="warning warning--kept">
    <p>
      <Icon name="check" />
      <span>You chose to keep your plan as it is: {warning.message.replace(/\.$/, '')}.</span>
      <button type="button" class="button button--quiet button--small" onclick={() => app.setDismissed(warning.id, false)}>Show again</button>
    </p>
  </div>
{:else}
  <div class="warning warning--{warning.severity}" role="note" aria-labelledby="{uid}-msg">
    <div class="warning__icon"><Icon name={warning.severity === 'warn' ? 'alert' : 'info'} size="1.4em" /></div>
    <div class="warning__body">
      <p class="warning__kind">{warning.severity === 'warn' ? 'Worth a look' : 'Note'}</p>
      <p class="warning__msg" id="{uid}-msg">{warning.message}</p>
      <p class="warning__why">{warning.why}</p>
      <button type="button" class="button button--small" onclick={() => app.setDismissed(warning.id, true)}>
        Keep my plan as it is
      </button>
    </div>
  </div>
{/if}

<style>
  .warning {
    display: flex;
    gap: var(--s3);
    padding: var(--s4);
    border-radius: var(--r2);
    border: 1px solid var(--border);
    border-left-width: 6px;
    margin-bottom: var(--s3);
    background: var(--surface);
  }
  .warning--warn {
    background: var(--warn-soft);
    border-color: var(--warn-edge);
  }
  .warning--warn .warning__icon,
  .warning--warn .warning__kind {
    color: var(--warn);
  }
  .warning--note {
    background: var(--note-soft);
    border-color: var(--note-edge);
  }
  .warning--note .warning__icon,
  .warning--note .warning__kind {
    color: var(--note);
  }
  .warning__kind {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .warning__msg {
    margin: 0 0 var(--s1);
    font-weight: 650;
  }
  .warning__why {
    margin: 0 0 var(--s3);
  }
  .warning--kept {
    border-left-width: 1px;
    padding: var(--s2) var(--s3);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .warning--kept p {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s2);
  }
</style>
