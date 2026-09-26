<!--
  The return-period dial: four labelled settings, each with the plain phrase, the jargon in
  brackets, and a one-line explanation of the ten-year chance.
-->
<script lang="ts">
  import type { ReturnPeriod } from '../engine/types';
  import { RETURN_PERIODS } from '../engine/types';
  import { RETURN_PERIOD, returnPeriodHelp } from '../lib/labels';

  let { value, onchange }: { value: ReturnPeriod; onchange: (rp: ReturnPeriod) => void } = $props();
  const uid = $props.id();
</script>

<fieldset class="dial" aria-describedby="{uid}-help">
  <legend>How rare an event to be ready for</legend>
  <p class="help" id="{uid}-help">
    A more cautious setting plans for longer, rarer disruptions. "1-in-100" means an event that size has about a 1 in 100 chance each year.
  </p>
  <div class="dial__options">
    {#each RETURN_PERIODS as rp, i (rp)}
      <label class="dial__option">
        <input type="radio" name="{uid}-rp" value={rp} checked={value === rp} onchange={() => onchange(rp)} />
        <span class="dial__step" aria-hidden="true">{i + 1}</span>
        <span class="dial__text">
          <span class="dial__label">{RETURN_PERIOD[rp].label} <span class="jargon">({RETURN_PERIOD[rp].jargon})</span></span>
          {#if rp === 'one_in_100'}<span class="chip chip--accent">Usual starting point</span>{/if}
          <span class="dial__help">{returnPeriodHelp(rp)}</span>
        </span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
  .dial {
    margin-bottom: var(--s5);
  }
  .dial__options {
    display: grid;
    gap: var(--s2);
  }
  @media (min-width: 56rem) {
    .dial__options {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  .dial__option {
    position: relative;
    display: flex;
    gap: var(--s3);
    align-items: flex-start;
    padding: var(--s3);
    border: 1px solid var(--border);
    border-radius: var(--r2);
    background: var(--surface);
    cursor: pointer;
    font-weight: 500;
  }
  .dial__option:has(input:checked) {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
    background: var(--accent-soft);
  }
  .dial__option:has(input:focus-visible) {
    outline: 3px solid var(--focus);
    outline-offset: 2px;
  }
  .dial__option input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .dial__step {
    flex: none;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 2px solid var(--border-strong);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .dial__option:has(input:checked) .dial__step {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .dial__text {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    align-items: flex-start;
  }
  .dial__label {
    font-weight: 650;
  }
  .jargon {
    color: var(--text-muted);
    font-weight: 500;
  }
  .dial__help {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>
