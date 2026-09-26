<!--
  On a plan step whose answer is written down in the family plan ("Make a household plan", the
  trusted circle, legal readiness, leaving, the shut-offs): a link to that part of the
  "Your family plan" screen. Nothing for any other step.
-->
<script lang="ts">
  import { familySectionFor, type FamilySection } from '../lib/family';
  import { href } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  let { itemId }: { itemId: string } = $props();

  const LABELS: Record<FamilySection, string> = {
    contact: 'Fill in your household plan here',
    children: 'Write down who picks up the children',
    shelter: 'Write down where you would shelter',
    leave: 'Write down where you would go, and how',
    home: 'Write down where the shut-offs are',
    circle: 'Write down your trusted circle',
    lawyer: "Write down your lawyer's number",
  };

  const section = $derived(familySectionFor(itemId));
</script>

{#if section}
  <p class="family-link no-print">
    <a href={href('family', section === 'contact' ? undefined : section)}>{LABELS[section]} <Icon name="chevron-right" /></a>
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
