<!--
  "Type an address instead" (DESIGN-DELTA-v3 D6): a plain warning first (who receives the
  address, and that they ask for no personal details), then a field and a "Search" button. Each
  press sends one request (never as the person types), and up to three matches are offered to
  place the pin. Warn, do not block: placing the pin by hand is always there instead.
-->
<script lang="ts">
  import { tick } from 'svelte';

  import { type AddressMatch, AddressSearch, addressSearch, SearchError } from '../../lib/maps/nominatim';
  import { NOMINATIM, SEARCH_RECIPIENT } from '../../lib/maps/sources';

  let {
    onchoose,
    target = 'Home',
    search = addressSearch,
  }: {
    onchoose: (match: AddressMatch) => void;
    /** What the chosen match will place ("Home", "Where you would go"…). */
    target?: string;
    search?: AddressSearch;
  } = $props();

  const uid = $props.id();
  let stage = $state<'closed' | 'warning' | 'open'>('closed');
  let text = $state('');
  let busy = $state(false);
  let matches = $state<AddressMatch[] | null>(null);
  let problem = $state('');
  let field: HTMLInputElement | undefined = $state();

  async function run(e?: SubmitEvent) {
    e?.preventDefault();
    if (busy) return;
    problem = '';
    busy = true;
    try {
      matches = await search.search(text);
      if (matches.length === 0) problem = text.trim() ? 'No match was found. Check the spelling, add the town, or place the pin by hand.' : 'Type a street address and town first.';
    } catch (err) {
      matches = null;
      problem = err instanceof SearchError ? err.message : 'The address search did not work. Place the pin by hand instead.';
    } finally {
      busy = false;
    }
  }

  function agree() {
    stage = 'open';
    void tick().then(() => field?.focus());
  }
</script>

<div class="address">
  {#if stage === 'closed'}
    <button type="button" class="button button--quiet" onclick={() => (stage = 'warning')}>Type an address instead</button>
  {:else if stage === 'warning'}
    <div class="address__warning" role="group" aria-labelledby="{uid}-warn-title">
      <p id="{uid}-warn-title"><strong>Before you type an address</strong></p>
      <p>
        To find an address, this app sends what you type to {SEARCH_RECIPIENT.who.replace(/^The /, 'the ').replace(/\.$/, '')}. They receive the
        address, your internet (IP) address and this site’s address. Their rules ask that no one sends personal details, so type only the
        street address and town, with no names.
      </p>
      <p>If you would rather not, close this and place the pin by hand on the map.</p>
      <div class="button-row">
        <button type="button" class="button" onclick={agree}>Continue</button>
        <button type="button" class="button button--quiet" onclick={() => (stage = 'closed')}>Close</button>
      </div>
    </div>
  {:else}
    <form class="address__form" onsubmit={run}>
      <label for="{uid}-q">Street address and town</label>
      <span class="help" id="{uid}-help">Only the address, no names. Each press of Search sends it once.</span>
      <div class="address__row">
        <input
          id="{uid}-q"
          class="input"
          type="text"
          bind:this={field}
          bind:value={text}
          maxlength="200"
          autocomplete="street-address"
          aria-describedby="{uid}-help"
        />
        <button type="submit" class="button" disabled={busy}>{busy ? 'Searching…' : 'Search'}</button>
      </div>
    </form>
    <div aria-live="polite">
      {#if problem}
        <p class="error-text">{problem}</p>
      {:else if matches && matches.length > 0}
        <p class="small">Choose the right one to place “{target}” there:</p>
        <ul class="address__matches">
          {#each matches as m (m.label)}
            <li>
              <button type="button" class="button button--small address__match" onclick={() => onchoose(m)}>{m.label}</button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <p class="small muted">{NOMINATIM.credit}</p>
    <button type="button" class="button button--quiet button--small" onclick={() => (stage = 'closed')}>Close the address search</button>
  {/if}
</div>

<style>
  .address {
    margin-top: var(--s3);
  }
  .address__warning {
    padding: var(--s3) var(--s4);
    border-left: 4px solid var(--warn-edge);
    background: var(--warn-soft);
    border-radius: var(--r1);
  }
  .address__row {
    display: flex;
    gap: var(--s2);
    align-items: stretch;
  }
  .address__row .input {
    flex: 1;
  }
  .address__matches {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s2);
    display: grid;
    gap: var(--s2);
  }
  .address__matches li + li {
    margin-top: 0;
  }
  .address__match {
    text-align: left;
    justify-content: flex-start;
    font-weight: 500;
  }
</style>
