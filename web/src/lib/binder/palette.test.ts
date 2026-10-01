/**
 * The binder in black and white (DESIGN-DELTA-v3 §6): nothing means anything by colour alone.
 * Every callout kind keeps its meaning in greyscale (a word of its own, a label that reads at
 * WCAG AA or better against its own shade, a box that differs from the other kinds' by more than
 * hue), the "do first, from memory" steps stand apart from plain steps by weight and by a band a
 * greyscale copy keeps, and every text colour reads on its background.
 */
import { describe, expect, it } from 'vitest';

import { CALLOUT_KINDS } from '../../engine/types';
import { CALLOUT_STYLES, contrast, greyscale, HAIRLINE, HEADER_FILL, INK, luminance, MEMORY_FILL, MUTED, PAPER, SECTION_FILL, STEP_STYLES } from './palette';

describe('the greyscale arithmetic', () => {
  it('matches the WCAG reference values', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 5);
    expect(contrast('#767676', '#ffffff')).toBeCloseTo(4.54, 2);
    expect(luminance('#ffffff')).toBe(1);
    expect(greyscale('#ff0000')).toBe('#7f7f7f');
    expect(greyscale('#4d4d4d')).toBe('#4d4d4d');
  });
});

describe('the binder reads the same in black and white', () => {
  it('every colour the PDF draws with is already a grey', () => {
    const colours = [INK, MUTED, PAPER, HAIRLINE, HEADER_FILL, MEMORY_FILL, SECTION_FILL, ...Object.values(CALLOUT_STYLES).flatMap((s) => [s.labelFill, s.labelInk])];
    for (const c of colours) expect(greyscale(c), c).toBe(c.toLowerCase());
  });

  it('text reads on its background: body and sources text, table headers', () => {
    expect(contrast(INK, PAPER)).toBeGreaterThanOrEqual(7);
    expect(contrast(MUTED, PAPER)).toBeGreaterThanOrEqual(7);
    expect(contrast(INK, HEADER_FILL)).toBeGreaterThanOrEqual(7);
    expect(contrast(INK, SECTION_FILL)).toBeGreaterThanOrEqual(7);
    // A register's section band shows against the paper, and apart from the column header's.
    expect(contrast(SECTION_FILL, PAPER)).toBeGreaterThan(1.05);
    expect(contrast(HEADER_FILL, SECTION_FILL)).toBeGreaterThan(1.05);
  });

  it('each callout kind says what it is in a word, readable in greyscale, and looks different from the others', () => {
    const words = CALLOUT_KINDS.map((k) => CALLOUT_STYLES[k].word);
    expect(new Set(words).size).toBe(CALLOUT_KINDS.length);
    for (const k of CALLOUT_KINDS) {
      const s = CALLOUT_STYLES[k];
      expect(s.word, k).toMatch(/^[A-Z]{3,}$/);
      // The label's word against its own shade, and the box's text against the paper.
      expect(contrast(greyscale(s.labelInk), greyscale(s.labelFill)), k).toBeGreaterThanOrEqual(4.5);
      expect(contrast(INK, PAPER), k).toBeGreaterThanOrEqual(4.5);
    }
    // Two kinds never look alike: their label shades differ clearly, or their boxes do (weight or dashes).
    for (let i = 0; i < CALLOUT_KINDS.length; i++) {
      for (let j = i + 1; j < CALLOUT_KINDS.length; j++) {
        const a = CALLOUT_STYLES[CALLOUT_KINDS[i]!];
        const b = CALLOUT_STYLES[CALLOUT_KINDS[j]!];
        const shades = contrast(greyscale(a.labelFill), greyscale(b.labelFill));
        const boxes = a.border !== b.border || a.dash !== b.dash;
        expect(shades >= 1.5 || boxes, `${CALLOUT_KINDS[i]} and ${CALLOUT_KINDS[j]}`).toBe(true);
      }
    }
    // The strongest warning is the darkest label and the heaviest box.
    expect(luminance(CALLOUT_STYLES.stop.labelFill)).toBeLessThan(luminance(CALLOUT_STYLES.warning.labelFill));
    expect(CALLOUT_STYLES.stop.border).toBeGreaterThan(Math.max(CALLOUT_STYLES.warning.border, CALLOUT_STYLES.note.border, CALLOUT_STYLES.decision.border));
  });

  it('memory steps stand apart from plain text by weight and a band, and their text reads on the band', () => {
    const memory = STEP_STYLES.memory;
    const plain = STEP_STYLES.plain;
    expect(memory.bold).toBe(true);
    expect(plain.bold).toBe(false);
    // The band shows in greyscale (it is not white), and black on it reads at the AAA level.
    expect(contrast(greyscale(memory.fill), greyscale(plain.fill))).toBeGreaterThanOrEqual(1.15);
    expect(contrast(greyscale(memory.ink), greyscale(memory.fill))).toBeGreaterThanOrEqual(7);
    expect(contrast(greyscale(plain.ink), greyscale(plain.fill))).toBeGreaterThanOrEqual(7);
  });
});
