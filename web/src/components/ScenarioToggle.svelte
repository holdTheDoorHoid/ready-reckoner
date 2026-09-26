<!--
  A named scenario the engine offers here (for example a Cascadia earthquake): a switch, why it
  applies, and the plan with and without it side by side. Turning it off is a choice, not an error.
-->
<script lang="ts">
  import type { ScenarioInfo } from '../engine/types';
  import type { ComparisonRow } from '../lib/ui-types';
  import Sources from './Sources.svelte';

  let {
    scenario,
    comparison,
    onchange,
  }: { scenario: ScenarioInfo; comparison: ComparisonRow[] | null; onchange: (on: boolean) => void } = $props();
  const uid = $props.id();
</script>

<div class="scenario card">
  <label class="scenario__switch">
    <input
      type="checkbox"
      role="switch"
      checked={scenario.on}
      aria-describedby="{uid}-why"
      onchange={(e) => onchange((e.currentTarget as HTMLInputElement).checked)}
    />
    <span class="scenario__name">{scenario.name}</span>
    <span class="scenario__state">{scenario.on ? 'On' : 'Off'}</span>
  </label>
  <p class="small" id="{uid}-why">{scenario.applies_because}</p>
  {#if !comparison?.length}<p class="small">{scenario.effect_summary}</p>{/if}
  {#if comparison?.length}
    <!-- Wide tables scroll sideways on phones; a focusable, labelled region lets keyboard users scroll it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="table-wrap" tabindex="0" role="region" aria-label="With and without this scenario">
      <table>
        <caption class="visually-hidden">Targets with and without: {scenario.name}</caption>
        <thead>
          <tr><th scope="col">Be ready for</th><th scope="col">With it</th><th scope="col">Without it</th></tr>
        </thead>
        <tbody>
          {#each comparison as row (row.bucket)}
            <tr><th scope="row">{row.bucket}</th><td>{row.with}</td><td>{row.without}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  <Sources ids={scenario.sources} />
</div>

<style>
  .scenario__switch {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-height: var(--tap);
    cursor: pointer;
  }
  .scenario__switch input {
    appearance: none;
    -webkit-appearance: none;
    width: 3rem;
    height: 1.75rem;
    border-radius: 999px;
    border: 2px solid var(--border-strong);
    background: var(--surface-3);
    position: relative;
    margin: 0;
    flex: none;
    cursor: pointer;
  }
  .scenario__switch input::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 1.2rem;
    height: 1.2rem;
    border-radius: 50%;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    transition: transform 0.15s ease;
  }
  .scenario__switch input:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  .scenario__switch input:checked::after {
    transform: translateX(1.25rem);
  }
  .scenario__name {
    font-weight: 650;
    flex: 1;
  }
  .scenario__state {
    font-size: var(--text-sm);
    font-weight: 700;
    padding: 0.1em 0.6em;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
  }
</style>
