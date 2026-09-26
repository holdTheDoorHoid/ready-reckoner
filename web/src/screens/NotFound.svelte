<script lang="ts">
  import { href } from '../lib/router.svelte';
</script>

<div class="page page--narrow">
  <h1 id="page-title" tabindex="-1">Page not found</h1>
  <p class="lead">That address doesn't match anything in Ready Reckoner. Your plan is safe.</p>
  <p class="button-row">
    <a class="button button--primary" href={href('start')}>Go to the start</a>
    <a class="button" href={href('plan')}>Your plan</a>
  </p>
</div>
