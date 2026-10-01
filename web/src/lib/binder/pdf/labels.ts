/**
 * The tab label sheet, the PDF's last page (DESIGN-DELTA-v3 §6): the ten tabs' numbers and short
 * titles as labels to cut out, with dashed cut lines, two of each (one for each side of a tab), and
 * the same titles listed for writing on the tabs by hand.
 *
 * **Size: 1 inch by ½ inch (72 × 36 points).** A ten-tab divider set for letter or A4 binders
 * staggers its ten tabs along the long edge of the page, about ten inches of it, so each tab is
 * about an inch long; the tabs stand about half an inch out from the page. A label that size covers
 * the face of one tab, slides into the paper insert of an insertable tab, or tapes onto a plain one.
 * The short title (at most 14 characters, "Happening now") takes one or two lines at 7 points beside
 * the tab number, which is printed large so the tab can be found at a glance.
 */
import type { Part } from '../../../engine/types';
import { INK } from '../palette';

type Node = Record<string, unknown>;

/** The label's size and type, for the tests and the report. */
export const LABEL_SPEC = {
  widthIn: 1,
  heightIn: 0.5,
  width: 72,
  height: 36,
  numberSize: 15,
  titleSize: 7,
  copies: 2,
} as const;

/** The labels, the cut lines around them, and the hand-writing note. */
export function labelSheet(parts: readonly Part[], width: number): Node[] {
  const { width: W, height: H, numberSize, titleSize, copies } = LABEL_SPEC;
  const gap = 18;
  const perRow = Math.max(1, Math.floor((width + gap) / (W * copies + gap)));
  const groups: Node[] = [];
  const tabs = parts.slice(0, 10);
  for (let i = 0; i < tabs.length; i += perRow) {
    const row = tabs.slice(i, i + perRow);
    groups.push({
      columns: row.map((p) => ({
        width: W * copies,
        table: {
          widths: Array.from({ length: copies }, () => W - 6),
          heights: [H - 1],
          body: [Array.from({ length: copies }, () => labelCell(p, numberSize, titleSize))],
        },
        layout: {
          hLineWidth: () => 0.6,
          vLineWidth: () => 0.6,
          hLineColor: () => INK,
          vLineColor: () => INK,
          hLineStyle: () => ({ dash: { length: 3, space: 2 } }),
          vLineStyle: () => ({ dash: { length: 3, space: 2 } }),
          paddingLeft: () => 3,
          paddingRight: () => 3,
          paddingTop: () => 2,
          paddingBottom: () => 1,
        },
      })),
      columnGap: gap,
      margin: [0, 0, 0, 12],
    });
  }
  return [
    {
      text: 'Cut along the dashed lines. Each label is 1 inch by ½ inch, the face of one tab on a standard ten-tab divider set. There are two of each: one for each side of the tab. Slide them into the tab inserts, or tape them on.',
      margin: [0, 0, 0, 12],
    },
    ...groups,
    { text: 'Or write the titles on the tabs by hand', bold: true, margin: [0, 8, 0, 4] },
    {
      columns: [0, 1].map((col) => ({
        width: '*',
        stack: tabs
          .filter((_, i) => i % 2 === col)
          .map((p) => ({ text: [{ text: `Tab ${p.tab}: `, bold: true }, p.short_title.normalize('NFC'), ` (${p.title.normalize('NFC')})`], margin: [0, 0, 0, 3] })),
      })),
      columnGap: 18,
    },
  ];
}

function labelCell(p: Part, numberSize: number, titleSize: number): Node {
  return {
    columns: [
      { text: `${p.tab}`, bold: true, fontSize: numberSize, width: 20, alignment: 'center', margin: [0, 3, 0, 0] },
      { text: p.short_title.normalize('NFC'), bold: true, fontSize: titleSize, width: '*', margin: [0, 9, 0, 0] },
    ],
    columnGap: 2,
  };
}
