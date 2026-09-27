<!--
  On a plan step whose answer is written down in the optional interview steps ("Make a household
  plan", the trusted circle, legal readiness, leaving, the shut-offs): a link to where that answer
  goes. The household-plan step opens step 6 ("Add your people and places for the binder"); the
  others open the card on step 7 or 8 that holds their questions (DESIGN-DELTA-v3 §1). Nothing for
  any other step.
-->
<script lang="ts">
  import { familySectionFor, type FamilySection } from '../lib/family';
  import { FAMILY_REDIRECTS, href, type Route } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  let { itemId }: { itemId: string } = $props();

  const LABELS: Record<FamilySection, string> = {
    contact: 'Add your people and places for the binder',
    children: 'Write down who picks up the children',
    shelter: 'Write down where you would shelter',
    leave: 'Write down where you would go, and how',
    home: 'Write down where the shut-offs are',
    circle: 'Write down your trusted circle',
    lawyer: "Write down your lawyer's number",
  };

  const section = $derived(familySectionFor(itemId));
  const target = $derived.by((): Route | undefined => {
    if (!section) return undefined;
    return section === 'contact' ? { id: 'people' } : FAMILY_REDIRECTS[section];
  });
</script>

{#if section && target}
  <p class="family-link no-print">
    <a href={href(target.id, target.param)}>{LABELS[section]} <Icon name="chevron-right" /></a>
  </p>
{/if}

<style>
  .family-link {
    margin: 0 0 var(--s2);
    font-size: var(--text-sm);
  }
  .family-link a {
    display: inline-flex;
    align-items: center;
    gap: var(--s1);
    min-height: 36px;
    font-weight: 600;
  }
</style>
