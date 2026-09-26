/**
 * In-page links inside rendered Markdown (footnotes, "back to the text") scroll to their target
 * without touching `location.hash`, which the router owns. Use as a click handler on the container.
 */
const FOCUSABLE = 'a[href], button, input, select, textarea, summary, [tabindex]';

/**
 * Scroll to an element on this page and move keyboard focus there, without touching
 * `location.hash` (the router owns it). Every closed `<details>` around it opens first, so a card
 * folded away under "All N risks" can still be reached. `focus` picks what inside the element takes
 * focus (a card's heading); an element that cannot take focus gets `tabindex="-1"`, while links and
 * buttons keep their place in the tab order. Returns false when there is no element with that id.
 */
export function jumpTo(id: string, options: { focus?: string; block?: ScrollLogicalPosition } = {}): boolean {
  const el = document.getElementById(id);
  if (!el) return false;
  for (let d = el.closest('details'); d; d = d.parentElement?.closest('details') ?? null) {
    if (!d.open) d.open = true;
  }
  const target = (options.focus ? el.querySelector<HTMLElement>(options.focus) : null) ?? el;
  if (!target.matches(FOCUSABLE)) target.setAttribute('tabindex', '-1');
  // jsdom has no scrollIntoView; browsers all do.
  el.scrollIntoView?.({ block: options.block ?? 'start' });
  target.focus({ preventScroll: true });
  return true;
}

export function followInPageAnchor(e: MouseEvent): void {
  const link = (e.target as HTMLElement | null)?.closest('a');
  const target = link?.getAttribute('href');
  if (!link || !target || !target.startsWith('#') || target.startsWith('#/')) return;
  e.preventDefault();
  const el = document.getElementById(target.slice(1));
  if (!el) return;
  if (!el.hasAttribute('tabindex')) el.setAttribute('tabindex', '-1');
  el.scrollIntoView({ block: 'start' });
  el.focus({ preventScroll: true });
}
