<!-- The privacy promise, the display settings, and the version. -->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import { href } from '../lib/router.svelte';
  import Icon from './Icon.svelte';

  const app = useApp();
</script>

<footer class="site-footer">
  <div class="site-footer__inner">
    <p class="promise"><Icon name="lock" /> Your answers stay in this browser. Nothing is sent anywhere, and it works offline.</p>
    <div class="settings">
      <div class="setting">
        <label for="theme-select">Appearance</label>
        <select id="theme-select" class="input" bind:value={app.prefs.theme}>
          <option value="system">Match this device</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>
      <label class="setting setting--check">
        <input type="checkbox" bind:checked={app.prefs.expert} />
        <span>Show the numbers behind it (expert view)</span>
      </label>
    </div>
    <p class="small muted">
      Ready Reckoner {__RR_APP_VERSION__}. Free software (GPLv3); text CC BY-SA 4.0.
      <a href={href('about')}>About and sources</a>
    </p>
  </div>
</footer>

<style>
  .site-footer {
    border-top: 1px solid var(--border);
    background: var(--surface);
    margin-top: var(--s8);
  }
  .site-footer__inner {
    max-width: var(--w-wide);
    margin: 0 auto;
    padding: var(--s5) var(--s4);
  }
  .promise {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    font-weight: 600;
  }
  .settings {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s4) var(--s6);
    align-items: flex-end;
    margin-bottom: var(--s4);
  }
  .setting label {
    display: block;
    font-size: var(--text-sm);
    margin-bottom: var(--s1);
  }
  .setting select {
    min-width: 14rem;
  }
  .setting--check {
    display: flex;
    gap: var(--s2);
    align-items: center;
    min-height: var(--tap);
    font-weight: 500;
  }
</style>
