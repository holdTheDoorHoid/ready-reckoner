/**
 * The binder's ink on paper: the greys the PDF draws with (and the print stylesheet copies), and
 * the contrast arithmetic the tests hold them to. The binder is printed on office printers, very
 * often in black and white, so nothing here means anything by colour: each callout kind says its
 * kind in a word and differs from the others in the weight of its border and the shade of its
 * label, and the "do first, from memory" steps are bold on a grey band, so a greyscale copy keeps
 * every distinction (DESIGN-DELTA-v3 §6, "Black and white").
 */
import type { CalloutKind } from '../../engine/types';

/** Body text. */
export const INK = '#000000';
/** Secondary text (sources, credits, footers): 12.6 : 1 on paper. */
export const MUTED = '#333333';
/** Paper. */
export const PAPER = '#ffffff';
/** Hairlines between rows of answers. */
export const HAIRLINE = '#8c8c8c';
/** Table header rows. */
export const HEADER_FILL = '#e6e6e6';
/** The band behind a "do first" step. */
export const MEMORY_FILL = '#e3e3e3';

/** How a callout of each kind is drawn. */
export interface CalloutStyle {
  /** The word printed in its label: the kind, never only a colour. */
  word: string;
  /** Label background and text. */
  labelFill: string;
  labelInk: string;
  /** The box around it. */
  border: number;
  dash: boolean;
}

export const CALLOUT_STYLES: Record<CalloutKind, CalloutStyle> = {
  stop: { word: 'STOP', labelFill: '#000000', labelInk: '#ffffff', border: 2.5, dash: false },
  warning: { word: 'WARNING', labelFill: '#4d4d4d', labelInk: '#ffffff', border: 1.5, dash: false },
  decision: { word: 'DECIDE', labelFill: '#ffffff', labelInk: '#000000', border: 1.25, dash: true },
  note: { word: 'NOTE', labelFill: '#e6e6e6', labelInk: '#000000', border: 0.75, dash: false },
};

/** How a step is drawn: bold on a band for the memory items, plain otherwise. */
export const STEP_STYLES = {
  memory: { bold: true, fill: MEMORY_FILL, ink: INK },
  plain: { bold: false, fill: PAPER, ink: INK },
} as const;

// ---------------------------------------------------------------------------------------------
// Greyscale arithmetic (WCAG 2.x relative luminance)
// ---------------------------------------------------------------------------------------------

function channel(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

/** Relative luminance of a `#rrggbb` colour: what a greyscale print keeps of it. */
export function luminance(hex: string): number {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) throw new Error(`not a #rrggbb colour: ${hex}`);
  const [r, g, b] = [m[1]!, m[2]!, m[3]!].map((x) => channel(parseInt(x, 16)));
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}

/** The grey a colour prints as in black and white (`#rrggbb`, equal channels). */
export function greyscale(hex: string): string {
  const l = luminance(hex);
  const s = l <= 0.0031308 ? 12.92 * l : 1.055 * l ** (1 / 2.4) - 0.055;
  const v = Math.round(Math.min(1, Math.max(0, s)) * 255)
    .toString(16)
    .padStart(2, '0');
  return `#${v}${v}${v}`;
}

/** Contrast ratio of two colours, 1 to 21 (WCAG 2.x). */
export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}
