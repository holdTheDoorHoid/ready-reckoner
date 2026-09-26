<!--
  A set of radio buttons drawn as tappable cards, inside a fieldset with a legend. Native radios
  keep arrow-key movement and screen-reader grouping; the card shows the checked state with a
  border and a filled radio, never with colour alone. With no value yet (a question not answered,
  such as the optional contract v2 questions), no card is checked.
-->
<script lang="ts" generics="T extends string">
  import Icon from './Icon.svelte';

  let {
    legend,
    name,
    options,
    value = $bindable(),
    help,
    error,
    columns = 1,
    onchange,
  }: {
    legend: string;
    name: string;
    options: { value: T; label: string; help?: string }[];
    value: T | undefined;
    help?: string;
    error?: string;
    columns?: 1 | 2 | 3 | 4;
    onchange?: (value: T) => void;
  } = $props();

  const uid = $props.id();
</script>

<fieldset class="choice-group" aria-describedby={[help ? `${uid}-help` : '', error ? `${uid}-error` : ''].filter(Boolean).join(' ') || undefined}>
  <legend>{legend}</legend>
  {#if help}<p class="help" id="{uid}-help">{help}</p>{/if}
  <div class="choices choices--{columns}">
    {#each options as option (option.value)}
      <label class="choice">
        <input
          type="radio"
          {name}
          value={option.value}
          checked={value === option.value}
          onchange={() => {
            value = option.value;
            onchange?.(option.value);
          }}
        />
        <span class="choice__text">
          <span>{option.label}</span>
          {#if option.help}<span class="choice__help">{option.help}</span>{/if}
        </span>
      </label>
    {/each}
  </div>
  {#if error}
    <p class="error-text" id="{uid}-error"><Icon name="alert" /><span><span class="visually-hidden">Problem: </span>{error}</span></p>
  {/if}
</fieldset>

<style>
  .choice-group {
    margin-bottom: var(--s5);
  }
</style>
