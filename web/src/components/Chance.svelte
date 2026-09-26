<!--
  A chance as text, with "1 in 2,000" and a range like "(63–98)" each kept on one line, so a narrow
  column reads "between 1 in 200 / and 1 in 41", never "between 1 in / 200". On the smallest
  screens "1 in N" may wrap again, so a four-column table still fits.
-->
<script lang="ts">
  import { chancePieces } from '../lib/format';

  let { text }: { text: string } = $props();
</script>

{#each chancePieces(text) as piece, k (k)}{#if piece.kind}<span class="keep keep--{piece.kind}">{piece.text}</span>{:else}{piece.text}{/if}{/each}

<style>
  .keep {
    white-space: nowrap;
  }
  @media (max-width: 23rem) {
    .keep--one_in {
      white-space: normal;
    }
  }
</style>
