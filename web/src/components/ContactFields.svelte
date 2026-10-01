<!--
  Someone to call, on the optional steps (a doctor, a utility, the hospital, a vet): a fieldset
  with a name and a phone number side by side and, where the binder prints one, an address. Each
  part is saved as it is typed and tidied when left, at the engine's limits (`CONTACT_MAX`). The
  same labels repeat, so `context` ("of the electric company") tells screen readers whose they are.
  Anything that belongs with the contact (a policy number, a shut-off) goes in `children`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Contact } from '../engine/types';
  import { CONTACT_MAX } from '../lib/tidy';
  import TextField from './TextField.svelte';

  type Part = 'name' | 'phone' | 'address';

  let {
    id,
    legend,
    help,
    value,
    context,
    labels = {},
    address = false,
    oninput,
    onleave,
    children,
  }: {
    id: string;
    legend: string;
    help?: string;
    value: Contact | undefined;
    context: string;
    labels?: Partial<Record<Part, string>>;
    /** Ask for an address too. */
    address?: boolean;
    oninput: (part: Part, text: string) => void;
    onleave: (part: Part, max: number) => void;
    children?: Snippet;
  } = $props();
</script>

<fieldset class="contact-fields" aria-describedby={help ? `${id}-help` : undefined}>
  <legend>{legend}</legend>
  {#if help}<p class="help" id="{id}-help">{help}</p>{/if}
  <div class="contact-fields__pair">
    <TextField
      id="{id}-name"
      label={labels.name ?? 'Name'}
      {context}
      value={value?.name}
      maxlength={CONTACT_MAX.name}
      oninput={(t) => oninput('name', t)}
      onleave={() => onleave('name', CONTACT_MAX.name)}
    />
    <TextField
      id="{id}-phone"
      label={labels.phone ?? 'Phone'}
      {context}
      value={value?.phone}
      maxlength={CONTACT_MAX.phone}
      inputmode="tel"
      oninput={(t) => oninput('phone', t)}
      onleave={() => onleave('phone', CONTACT_MAX.phone)}
    />
  </div>
  {#if address}
    <TextField
      id="{id}-address"
      label={labels.address ?? 'Address'}
      {context}
      value={value?.address}
      maxlength={CONTACT_MAX.address}
      oninput={(t) => oninput('address', t)}
      onleave={() => onleave('address', CONTACT_MAX.address)}
    />
  {/if}
  {@render children?.()}
</fieldset>

<style>
  .contact-fields {
    margin-bottom: var(--s5);
  }
  /* A group's title reads as a small heading above the answers it groups. */
  .contact-fields > legend {
    font-size: var(--text-lg);
  }
  .contact-fields > :global(.field:last-child) {
    margin-bottom: 0;
  }
  /* A name and a phone side by side when there is room; the grid gap spaces them either way. */
  .contact-fields__pair {
    display: grid;
    gap: var(--s3) var(--s4);
    margin-bottom: var(--s4);
    align-items: end;
  }
  .contact-fields__pair :global(.field) {
    margin-bottom: 0;
  }
  .contact-fields__pair:last-child {
    margin-bottom: 0;
  }
  @media (min-width: 36rem) {
    .contact-fields__pair {
      grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
    }
  }
</style>
