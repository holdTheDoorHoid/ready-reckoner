import { describe, expect, it } from 'vitest';

import {
  addMonths,
  band,
  dayPhrase,
  frequencySentence,
  monthsBetween,
  naturalFrequency,
  noticeRange,
  per100,
  percent,
  quantity,
  severityBand,
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
