import { describe, expect, it } from 'vitest';

import { golden } from '../test/real';
import {
  addMonths,
  band,
  chancePieces,
  chanceShort,
  chanceWithin,
  dayPhrase,
  frequencySentence,
  monthsBetween,
  naturalFrequency,
  noticeRange,
  per100,
  percent,
  perYearWords,
  quantity,
  rangeOnly,
  roundSig2,
  severityBand,
  sig2,
  targetDays,
  targetMonths,
  usd,
} from './format';

describe('natural frequencies', () => {
  it('rounds to whole numbers up to 10 and to the nearest 5 above, with words at the ends', () => {
    expect(per100(0.004)).toEqual({ kind: 'fewer' });
    expect(per100(0.012)).toEqual({ kind: 'number', n: 1 });
    expect(per100(0.094)).toEqual({ kind: 'number', n: 9 });
    expect(per100(0.1)).toEqual({ kind: 'number', n: 10 });
    expect(per100(0.123)).toEqual({ kind: 'number', n: 10 });
    expect(per100(0.26)).toEqual({ kind: 'number', n: 25 });
    expect(per100(0.81)).toEqual({ kind: 'number', n: 80 });
    expect(per100(0.99)).toEqual({ kind: 'almost_all' });
  });

  it('writes the sentence with its range, and says "almost all" or "fewer than 1" plainly', () => {
    expect(frequencySentence(0.26, 'lose power for a day or more', 10, 0.15, 0.4)).toBe(
      'About 25 (15–40) of 100 households like yours will lose power for a day or more in the next 10 years.',
    );
    expect(frequencySentence(0.26, 'lose power', 1)).toBe('About 25 of 100 households like yours will lose power in the next year.');
    expect(frequencySentence(0.05, 'x', 10, 0.004, 0.2)).toBe('About 5 (up to 20) of 100 households like yours will x in the next 10 years.');
    expect(frequencySentence(0.6, 'x', 10, 0.4, 0.99)).toBe('About 60 (40 or more) of 100 households like yours will x in the next 10 years.');
    expect(frequencySentence(0.995, 'x', 10)).toBe('Almost all households like yours will x in the next 10 years.');
    expect(frequencySentence(0.001, 'x', 10)).toBe('Fewer than 1 in 100 households like yours will x in the next 10 years.');
    expect(naturalFrequency(0.095)).toBe('about 10 of 100');
  });

  it('shows percentages with at most two significant figures (expert view only)', () => {
    expect(percent(0.2634)).toBe('26%');
    expect(percent(0.0234)).toBe('2.3%');
    expect(percent(0.00234)).toBe('0.23%');
    expect(percent(0.0000234)).toBe('under 0.1%');
  });
});

describe('days and months', () => {
  it('names ladder days the way a person would', () => {
    expect([0.5, 1, 3, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365].map(dayPhrase)).toEqual([
      'half a day',
      '1 day',
      '3 days',
      '7 days',
      '10 days',
      '2 weeks',
      '3 weeks',
      '1 month',
      '1½ months',
      '2 months',
      '3 months',
      '6 months',
      '1 year',
    ]);
  });

  it('writes every target as "about N (low–high)"', () => {
    expect(targetDays(3, 2, 5)).toBe('about 3 days (2–5)');
    expect(targetDays(0.5, 0.5, 1)).toBe('about half a day (up to 1 day)');
    expect(targetDays(14, 10, 21)).toBe('about 2 weeks (10 days–3 weeks)');
    expect(targetDays(90, 60, 365)).toBe('about 3 months (2 months–1 year)');
    expect(targetDays(7, 7, 7)).toBe('about 7 days');
    expect(targetMonths(4, 2, 6)).toBe('about 4 months (2–6)');
    expect(targetMonths(1.5, 1, 2)).toBe('about 1½ months (1–2)');
  });

  it('describes warning times in minutes, hours or days', () => {
    expect(noticeRange(0.25, 12)).toBe('15 minutes to 12 hours');
    expect(noticeRange(24, 72)).toBe('24 hours to 3 days');
  });

  it('says "1 minute", "1 hour" and "1 day" in the singular, after rounding (W1)', () => {
    // Every fixture's evacuate target starts at 0.02 hours (1.2 minutes): "1 minutes" before v0.1.1.
    expect(noticeRange(0.02, 72)).toBe('1 minute to 3 days');
    expect(noticeRange(0.02, 2)).toBe('1 minute to 2 hours');
    expect(noticeRange(0.999, 30)).toBe('1 hour to 30 hours');
    expect(noticeRange(1.04, 1.04)).toBe('1 hour');
    expect(noticeRange(1, 1.02)).toBe('1 hour');
    expect(noticeRange(1.5, 24)).toBe('1½ hours to 24 hours');
    expect(noticeRange(0.5, 60)).toBe('30 minutes to 3 days');
    expect(noticeRange(20, 36)).toBe('20 hours to 36 hours');
    expect(noticeRange(48, 48)).toBe('2 days');
    expect(noticeRange(0, 0.02)).toBe('1 minute');
  });
});

describe('money and quantities', () => {
  it('uses whole dollars and "$30–45" bands', () => {
    expect(usd(1234.4)).toBe('$1,234');
    expect(band(30, 45)).toBe('$30–45');
    expect(band(0, 0)).toBe('Free');
    expect(band(25, 25)).toBe('$25');
    expect(quantity(12, 'gallon')).toBe('12 gallons');
    expect(quantity(1, 'kit')).toBe('1 kit');
    expect(quantity(2, 'box')).toBe('2 boxes');
    expect(quantity(3, 'pet-day')).toBe('3 pet-days');
    expect(quantity(100, 'dollar')).toBe('$100');
  });

  it('labels severity in words', () => {
    expect(severityBand(0.1).label).toBe('Minor');
    expect(severityBand(0.45).label).toBe('Serious');
    expect(severityBand(1).label).toBe('Very severe');
  });
});

describe('calendar dates', () => {
  it('adds months without spilling into the next month', () => {
    expect(addMonths('2026-10-01', 1)).toBe('2026-11-01');
    expect(addMonths('2027-01-31', 1)).toBe('2027-02-28');
    expect(addMonths('2026-10-01', 12)).toBe('2027-10-01');
  });

  it('counts whole months between dates', () => {
    expect(monthsBetween('2026-10-01', '2026-10-31')).toBe(0);
    expect(monthsBetween('2026-10-01', '2026-11-01')).toBe(1);
    expect(monthsBetween('2026-10-15', '2027-01-14')).toBe(2);
    expect(monthsBetween('2026-10-01', '2026-09-25')).toBe(-1);
  });
});

describe('short chances for tables, worded as the engine words them', () => {
  it('rounds to two significant figures like rr-hazards', () => {
    expect(roundSig2(86.37)).toBe(86);
    expect(roundSig2(1234)).toBe(1200);
    expect(roundSig2(200.5)).toBe(200);
    expect(roundSig2(40.5)).toBe(41);
    expect(roundSig2(1000)).toBe(1000);
    expect(roundSig2(0)).toBe(0);
    expect(sig2(4.47)).toBe('4.5');
    expect(sig2(1.89)).toBe('1.9');
    expect(sig2(12_500)).toBe('13,000');
  });

  it('gives the same number, and the same range, as the card sentence for every ranked hazard in every golden packet', () => {
    const names = ['philadelphia-renters-4', 'miami-condo-retiree-1', 'coos-bay-well-owner-2', 'hays-kansas-farm-5', 'chicago-student-zero-budget-1', 'phoenix-apartment-cpap-1', 'sugar-land-ev-household-3'];
    let checked = 0;
    for (const name of names) {
      for (const h of golden(name).register.filter((x) => x.display === 'ranked')) {
        const s = h.frequency_sentence;
        const p = chanceWithin(h.rate_per_year, 10);
        const lo = chanceWithin(h.rate_range[0], 10);
        const hi = chanceWithin(h.rate_range[1], 10);
        let want: string | undefined;
        let range: string | undefined;
        let m: RegExpExecArray | null;
        if (s.startsWith('Nearly every household')) want = 'nearly every household';
        else if ((m = /^Of 100 households like yours, about (\S+?|fewer than 1|nearly all)(?: \((.+?)\))? will/.exec(s))) {
          range = m[2];
          want = `about ${m[1]}${range ? ` (${range})` : ''} of 100`;
        } else if ((m = /^About (\S+?|fewer than 1)(?: \((.+?)\))? in 1,000 households/.exec(s))) {
          range = m[2];
          want = `about ${m[1]}${range ? ` (${range})` : ''} in 1,000`;
        } else if ((m = /^About (1 in [\d,]+)(?: \((.+?)\))? households/.exec(s))) {
          range = m[2];
          want = `about ${m[1]}${range ? ` (${range})` : ''}`;
        }
        if (!want) continue; // a hand-written sentence (none among ranked hazards today)
        const got = range ? chanceShort(p, lo, hi) : chanceShort(p);
        expect(got, `${name} ${h.id}: ${s}`).toBe(want);
        checked += 1;
      }
    }
    expect(checked).toBeGreaterThan(150);
  });

  it('says how often a year: times a year, once a year, or the chance in any one year', () => {
    expect(perYearWords(4.47, 1 - Math.exp(-4.47))).toBe('about 4.5 times a year');
    expect(perYearWords(1.03, 1 - Math.exp(-1.03))).toBe('about once a year');
    expect(perYearWords(0.2, 1 - Math.exp(-0.2))).toBe('about 1 in 6 a year');
    expect(perYearWords(0.00524, 1 - Math.exp(-0.00524))).toBe('about 1 in 190 a year');
    expect(perYearWords(0.00005, 1 - Math.exp(-0.00005))).toBe('about 1 in 20,000 a year');
  });

  it('marks "1 in N" and short numeric ranges so a narrow column keeps them on one line', () => {
    expect(chancePieces('between 1 in 200 and 1 in 41')).toEqual([
      { text: 'between ', kind: '' },
      { text: '1 in 200', kind: 'one_in' },
      { text: ' and ', kind: '' },
      { text: '1 in 41', kind: 'one_in' },
    ]);
    expect(chancePieces('about 86 (63–98) of 100')).toEqual([
      { text: 'about 86 ', kind: '' },
      { text: '(63–98)', kind: 'range' },
      { text: ' of 100', kind: '' },
    ]);
    expect(chancePieces('nearly every household')).toEqual([{ text: 'nearly every household', kind: '' }]);
    expect(chancePieces('about 1 in 20,000 a year').map((p) => p.text).join('')).toBe('about 1 in 20,000 a year');
  });

  it('shows a rare catastrophe as a range only, in words when it spans more than 1,000 times (H-02)', () => {
    // Philadelphia's two rare rows (golden packet), over ten years and over one.
    expect(rangeOnly(0.0005, 0.0025, 10)).toBe('between 1 in 200 and 1 in 41');
    expect(rangeOnly(0.0001, 0.001, 10)).toBe('between 1 in 1,000 and 1 in 100');
    expect(rangeOnly(0.0005, 0.0025, 1)).toBe('between 1 in 2,000 and 1 in 400');
    expect(rangeOnly(1e-7, 1e-3, 10)).toBe('very unlikely: less than 1 in 100');
    expect(rangeOnly(0.001, 0.001, 10)).toBe('about 1 in 100');
    expect(rangeOnly(0.0005, 0.0025, 10)).not.toMatch(/of 100/);
  });
});
