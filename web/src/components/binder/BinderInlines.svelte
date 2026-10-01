<!--
  A run of binder text (DESIGN-DELTA-v3 §4.1 `Inline`): plain and bold words, citation numbers
  that jump to the Sources page, links to other pages of the binder ("(Tab 3, Home)", or the link
  alone when it is all the run holds), and blanks drawn as a line to write on. Every string is
  shown as text, never read as markup or Markdown: the household's own words print exactly as
  they typed them.
-->
<script lang="ts">
  import type { Inline } from '../../engine/types';
  import { sourceDomId } from '../../lib/binder/model';
  import { href } from '../../lib/router.svelte';
  import type { BinderView } from './view';

  let { inlines, view }: { inlines: readonly Inline[]; view: BinderView } = $props();

  /** A link with nothing but citations beside it stands alone, without brackets. */
  const alone = (k: number) => inlines.every((x, j) => j === k || 'cite' in x);
  /** Whether what comes before ends in a space (so a bracket needs none of its own). */
  function spaced(k: number): boolean {
    const prev = inlines[k - 1];
    if (!prev) return true;
    const text = 't' in prev ? prev.t : 'b' in prev ? prev.b : '';
    return text === '' || /\s$/.test(text);
  }
  const sources = $derived(view.binder.sources.length);
</script>

{#each inlines as i, k (k)}{#if 't' in i}{i.t}{:else if 'b' in i}<strong>{i.b}</strong>{:else if 'blank' in i}<span class="blank" style:--chars={Math.max(3, Math.min(40, i.blank))}><span class="visually-hidden">(blank, to fill in on paper)</span></span>{:else if 'cite' in i}{@const ns = i.cite.filter((n) => n >= 1 && n <= sources)}{#if ns.length}<span class="cite">{spaced(k) ? '' : '\u2009'}[{#each ns as n, j (n)}{j ? ', ' : ''}<a href="#{sourceDomId(n)}" aria-label="Source {n}">{n}</a>{/each}]</span>{/if}{:else if 'link' in i}{@const known = view.pages.has(i.link.to)}{#if alone(k)}{#if known}<a class="xref" href={href('binder', i.link.to)} data-page={i.link.to}>{i.link.text}</a>{:else}{i.link.text}{/if}{:else}{spaced(k) ? '' : ' '}({#if known}<a class="xref" href={href('binder', i.link.to)} data-page={i.link.to}>{i.link.text}</a>{:else}{i.link.text}{/if}){/if}{/if}{/each}

<style>
  .blank {
    display: inline-block;
    min-width: calc(var(--chars) * 0.5em);
    max-width: 100%;
    height: 1em;
    vertical-align: -0.2em;
    border-bottom: 1px solid currentColor;
  }
  .cite {
    font-size: 0.75em;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .cite a {
    color: inherit;
  }
</style>
