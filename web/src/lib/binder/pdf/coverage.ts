/**
 * Which characters the PDF's typeface can draw: the runs `scripts/subset-fonts.mjs` recorded from
 * the four Noto Sans faces themselves (`fonts/coverage.json`, `covered`). A character outside them
 * prints as an empty box, so the screen says which ones before the household relies on the PDF;
 * the browser's own Print uses the device's typefaces and draws them.
 */
import coverage from './fonts/coverage.json';

const RUNS: [number, number][] = (coverage.covered as string[][]).map(([a, b]) => [parseInt(a!, 16), parseInt(b!, 16)]);

/** Whether the PDF's typeface has this character (line breaks and tabs count as drawn). */
export function covered(cp: number): boolean {
  if (cp === 0x0a || cp === 0x0d || cp === 0x09) return true;
  let lo = 0;
  let hi = RUNS.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const [a, b] = RUNS[mid]!;
    if (cp < a) hi = mid - 1;
    else if (cp > b) lo = mid + 1;
    else return true;
  }
  return false;
}

/** The distinct characters in these texts that the PDF cannot draw, in order of first use. */
export function missingCharacters(texts: readonly string[]): string[] {
  const out = new Set<string>();
  for (const t of texts) {
    for (const ch of t.normalize('NFC')) {
      const cp = ch.codePointAt(0)!;
      if (!covered(cp)) out.add(ch);
    }
  }
  return [...out];
}
