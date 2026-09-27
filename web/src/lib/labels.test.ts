import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { FIXTURE_NAMES, FIXTURES } from '../engine/fixtures';
import { RETURN_PERIODS } from '../engine/types';
import { golden, repoRoot } from '../test/real';
import { dialJointSentence, dialSentence, engineDialSentence, NUCLEAR_NOTE, returnPeriodHelp, STATUS_LINE } from './labels';
import { lowerFirst } from './lookup';

/** The sentence without the engine's figure (the stand-in engine, or while a new setting is worked out). */
const NUMBERLESS_DIAL =
  'For any one need, something worse than its target comes in about 1 of every 10 ten-year stretches. Across all your needs together, the chance that at least one runs out is higher. That is why the plan also gives you ways to cope when a target runs out.';
const CANONICAL_STATUS =
  'Ready Reckoner is an independent, open-source planning aid. It is not official emergency guidance, and not medical, legal or financial advice. Follow instructions from your local officials first.';

describe('the dial sentence (M-04)', () => {
  it('without the engine’s figure, says what each need’s target promises and puts no number on all needs together', () => {
    expect(dialSentence('one_in_100')).toBe(NUMBERLESS_DIAL);
    expect(dialSentence('one_in_100', '# A packet with no targets section')).toBe(NUMBERLESS_DIAL);
  });

  it('is the engine’s own sentence for the household, read from its packet, on every fixture (R3-25)', () => {
    // The share that meets at least one longer disruption differs by household: 2, 3, 4 and 5 in 10.
    const joint = new Set<string>();
    for (const name of FIXTURE_NAMES) {
      const out = golden(name);
      const rp = FIXTURES[name].dials.return_period;
      const md = readFileSync(join(repoRoot(), 'fixtures', 'golden', `${name}.md`), 'utf8');
      const printed = /^How long to be ready for each kind of disruption at the 1-in-\d+ setting\. (.*)\[[\d, ]+\]$/m.exec(md)![1]!;
      const sentence = engineDialSentence(out.prepare_markdown, rp);
      expect(sentence, name).toBe(printed);
      expect(dialSentence(rp, out.prepare_markdown), name).toBe(printed);
      expect(sentence, name).toMatch(/^At this setting, about \d+ in 10+ households like yours will face a longer disruption of any one kind in the next 10 years; about \d+ in 10+ will face at least one kind that runs past its target\. That is why the plan also gives you ways to cope when a target runs out\.$/);
      joint.add(/; about (\d+ in 10+) will face/.exec(sentence!)![1]!);
      // A packet worked out for another setting is not this setting's sentence.
      const other = RETURN_PERIODS.find((r) => r !== rp)!;
      expect(engineDialSentence(out.prepare_markdown, other), name).toBeUndefined();
    }
    expect([...joint].sort()).toEqual(['2 in 10', '3 in 10', '3 in 100', '4 in 10', '5 in 10']);
  });

  it('says what each setting promises for any one need, with the right count', () => {
    expect(returnPeriodHelp('one_in_10')).toBe('For any one need, something worse than its target comes in about 6 of every 10 ten-year stretches.');
    expect(returnPeriodHelp('one_in_50')).toBe('For any one need, something worse than its target comes in about 2 of every 10 ten-year stretches.');
    expect(returnPeriodHelp('one_in_100')).toBe('For any one need, something worse than its target comes in about 1 of every 10 ten-year stretches.');
    expect(returnPeriodHelp('one_in_500')).toBe('For any one need, something worse than its target comes in about 2 of every 100 ten-year stretches.');
  });

  it('never puts a fixed number on all needs together (the old "roughly 1 in 3" was one household’s figure)', () => {
    const joint = dialJointSentence();
    expect(joint).toBe('Across all your needs together, the chance that at least one runs out is higher. That is why the plan also gives you ways to cope when a target runs out.');
    for (const rp of RETURN_PERIODS) expect(dialSentence(rp)).not.toMatch(/\d+ in \d+\./);
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
      for (const old of ['of every 100 ten-year stretches', 'Something longer reaches', 'Something worse than these targets', 'would see a longer one in ten years', 'roughly 1 in 3']) {
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
