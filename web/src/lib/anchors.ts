/**
 * In-page links inside rendered Markdown (footnotes, "back to the text") scroll to their target
 * without touching `location.hash`, which the router owns. Use as a click handler on the container.
 */
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
