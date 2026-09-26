import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { RETURN_PERIODS } from '../engine/types';
import { dialJointSentence, dialSentence, NUCLEAR_NOTE, returnPeriodHelp, STATUS_LINE } from './labels';
import { lowerFirst } from './lookup';

/** COMMON-P1, "The canonical sentences": these exact words. */
const CANONICAL_DIAL =
  'For any one need, something worse than its target comes in about 1 of every 10 ten-year stretches. Across all your needs together, the chance that at least one runs out is higher, roughly 1 in 3. That is why the plan also gives you ways to cope when a target runs out.';
const CANONICAL_STATUS =
  'Ready Reckoner is an independent, open-source planning aid. It is not official emergency guidance, and not medical, legal or financial advice. Follow instructions from your local officials first.';

describe('the dial sentence (M-04)', () => {
  it('is the canonical sentence, word for word, at the usual 1-in-100 setting', () => {
    expect(dialSentence('one_in_100')).toBe(CANONICAL_DIAL);
  });

  it('says what each setting promises for any one need, with the right count', () => {
    expect(returnPeriodHelp('one_in_10')).toBe('For any one need, something worse than its target comes in about 6 of every 10 ten-year stretches.');
    expect(returnPeriodHelp('one_in_50')).toBe('For any one need, something worse than its target comes in about 2 of every 10 ten-year stretches.');
    expect(returnPeriodHelp('one_in_100')).toBe('For any one need, something worse than its target comes in about 1 of every 10 ten-year stretches.');
    expect(returnPeriodHelp('one_in_500')).toBe('For any one need, something worse than its target comes in about 2 of every 100 ten-year stretches.');
  });

  it('never puts a number on all needs together except where one was worked out (1-in-100)', () => {
    for (const rp of RETURN_PERIODS) {
      const joint = dialJointSentence(rp);
      expect(joint).toContain('Across all your needs together, the chance that at least one runs out is higher');
      expect(joint).toContain('That is why the plan also gives you ways to cope when a target runs out.');
      expect(joint.includes('1 in 3')).toBe(rp === 'one_in_100');
    }
  });

  it('the old all-needs wording is gone from every web source file', () => {
    const files: string[] = [];
    const walk = (dir: string) => {
      for (const f of readdirSync(dir)) {
        const p = join(dir, f);
        if (statSync(p).isDirectory()) walk(p);
        else if (/\.(ts|svelte)$/.test(f) && !f.endsWith('.test.ts')) files.push(p);
      }
    };
    walk(join(process.cwd(), 'src'));
    expect(files.length).toBeGreaterThan(50);
    for (const f of files) {
      const text = readFileSync(f, 'utf8');
      for (const old of ['of every 100 ten-year stretches', 'Something longer reaches', 'Something worse than these targets', 'would see a longer one in ten years']) {
        expect(text.includes(old), `${f}: "${old}"`).toBe(false);
      }
    }
  });
});

describe('fixed wording', () => {
  it('the status line is the canonical one (C1)', () => {
    expect(STATUS_LINE).toBe(CANONICAL_STATUS);
  });

  it('the nuclear note says the figure is worldwide (H-02)', () => {
    expect(NUCLEAR_NOTE).toBe(
      'The nuclear figure is the chance of a nuclear catastrophe anywhere in the world, not for your county; a location-aware version is coming.',
    );
  });

  it('names inside a sentence keep a leading acronym (W6)', () => {
    expect(lowerFirst('N95 respirators')).toBe('N95 respirators');
    expect(lowerFirst('NOAA Weather Radio with a tone alert')).toBe('NOAA Weather Radio with a tone alert');
    expect(lowerFirst('Family first-aid kit')).toBe('family first-aid kit');
    expect(lowerFirst('A warm blanket for each person')).toBe('a warm blanket for each person');
    expect(lowerFirst('Heat wave')).toBe('heat wave');
  });
});
