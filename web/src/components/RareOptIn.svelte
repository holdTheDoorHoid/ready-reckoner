<!--
  The rare-catastrophe allowance, family by family (contract v2 `Dials.rare_opt_in`). Off by
  default: the plan never spends on these unless the household ticks one, and then at most 10% of
  the monthly budget. "All of them" writes ["all"], which also covers families added later; each
  box below it turns one family on or off. The v1 switch (`rare_catastrophic_opt_in`) reads as
  "all of them" and is retired the first time anything here changes (lib/dials.ts).

  Used in the Risks screen's settings; the rare box (web-risks) may place it beside its rows.
-->
<script lang="ts">
  import type { Dials, RareHazardId } from '../engine/types';
  import { RARE_HAZARD_IDS } from '../engine/types';
  import { allowsEveryRareFamily, rareFamilies, setEveryRareFamily, setRareFamily } from '../lib/dials';

  let {
    dials,
    names,
    order,
    onchange,
  }: {
    /** The saved plan's dials (changed in place). */
    dials: Dials;
    /** Plain names by family id, from `catalogue().hazards`; the id is never shown. */
    names: ReadonlyMap<string, string>;
    /** The families in the order to list them (defaults to `RARE_HAZARD_IDS`). */
    order?: readonly RareHazardId[];
    /** Called after a change with a sentence for screen readers. */
    onchange?: (announcement: string) => void;
  } = $props();

  const uid = $props.id();
  const chosen = $derived(new Set<string>(rareFamilies(dials)));
  const every = $derived(allowsEveryRareFamily(dials));
  const some = $derived(!every && chosen.size > 0);
  const listed = $derived<readonly RareHazardId[]>([
    ...(order ?? []).filter((f) => (RARE_HAZARD_IDS as readonly string[]).includes(f)),
    ...RARE_HAZARD_IDS.filter((f) => !(order ?? []).includes(f)),
  ]);

  // "Some but not all" shows as a half-ticked box; it is a property, not an attribute.
  let allBox: HTMLInputElement | undefined = $state();
  $effect(() => {
    if (allBox) allBox.indeterminate = some;
  });

  function nameOf(id: string): string {
    return names.get(id) ?? id.replace(/_/g, ' ');
  }

  function setAll(on: boolean) {
    setEveryRareFamily(dials, on);
    onchange?.(on ? 'Rare-catastrophe allowance on for all of them.' : 'Rare-catastrophe allowance switched off.');
  }

  function setOne(id: RareHazardId, on: boolean) {
    setRareFamily(dials, id, on);
    onchange?.(`Rare-catastrophe allowance ${on ? 'on' : 'off'} for ${nameOf(id).toLowerCase()}.`);
  }
</script>

<fieldset class="rare-opt-in" aria-describedby="{uid}-help">
  <legend>Allow up to 10% of my budget for rare catastrophes (off by default)</legend>
  <p class="help" id="{uid}-help">
    Tick the ones you want covered. The plan never spends on these unless you do. It could then add items such as a radiation meter, or
    potassium iodide only on official instruction. See "Rare but severe" below for how likely each one is.
  </p>
  <label class="choice rare-opt-in__all">
    <input bind:this={allBox} type="checkbox" checked={every} onchange={(e) => setAll((e.currentTarget as HTMLInputElement).checked)} />
    <span class="choice__text">
      <span>All of them</span>
      <span class="choice__help">Also covers any added later.</span>
    </span>
  </label>
  <ul class="rare-opt-in__list" aria-label="Rare catastrophes, one by one">
    {#each listed as id (id)}
      <li>
        <label class="choice">
          <input type="checkbox" checked={chosen.has(id)} onchange={(e) => setOne(id, (e.currentTarget as HTMLInputElement).checked)} />
          <span class="choice__text"><span>{nameOf(id)}</span></span>
        </label>
      </li>
    {/each}
  </ul>
</fieldset>

<style>
  .rare-opt-in {
    margin-bottom: var(--s5);
  }
  .rare-opt-in__all {
    margin-bottom: var(--s2);
  }
  .rare-opt-in__list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--s2);
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 15rem), 1fr));
  }
  .rare-opt-in__list li + li {
    margin-top: 0;
  }
  .rare-opt-in__list .choice {
    padding: var(--s2) var(--s3);
    height: 100%;
  }
</style>
