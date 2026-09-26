/**
 * What sits around a duration target (model review Part 3.3–3.4): the stress line, the confidence
 * badge and the driver bars. The stress events are the data pack's own rows
 * (`core/outage_stress.csv` in agent/data-model: Buncombe's Helene curve, Philadelphia's March 2018
 * windstorm), so the words are checked against real curves.
 */
import { describe, expect, it } from 'vitest';

import type { BucketAssessment, HazardProfile, StressTest } from '../engine/types';
import { CONFIDENCE_BADGE, confidenceOf, drivers, shareOutAfter, shareWords, stressLine } from './targets';

/** A power event from `outage_stress.csv`: share of all customers still out at 1, 3, 7, 14, 30 days (peak × share of peak). */
function power(event: string, date: string, region: string, peak: number, shares: number[], covered: boolean): StressTest {
  return {
    event,
    date,
    region,
    share_out_at_days: [1, 3, 7, 14, 30].map((d, i) => [d, peak * shares[i]!] as [number, number]),
    covered_by_target: covered,
    sources: ['ornl_eagle_i_outages'],
  };
}

// Buncombe County NC: Hurricane Helene, recorded in the county, 100% out at the peak.
const HELENE = power('Hurricane Helene', '2024-09-25', "your county's records", 1, [0.9222, 0.7782, 0.6938, 0.2213, 0], false);
// Philadelphia: a March 2018 windstorm recorded 65 km away (Hunterdon NJ), 46.9% out at the peak.
const MARCH_2018 = power(
  'Wind and thunderstorms, March 2018',
  '2018-03-02',
  "your region's records (where it was worst, about 40 miles away)",
  0.469,
  [0.6649, 0.3138, 0.1223, 0, 0],
  false,
);

describe('the stress line', () => {
  it('reads the share still out the way rr-consequence decides covered_by_target', () => {
    const pts = HELENE.share_out_at_days;
    // From everyone out at the peak to the first mark.
    expect(shareOutAfter(pts, 0.5)).toBeCloseTo(1 - (1 - 0.9222) * 0.5, 10);
    expect(shareOutAfter(pts, 7)).toBeCloseTo(0.6938, 10);
    // Straight in log-time between marks: halfway between 7 and 14 days in log terms is √98 days.
    expect(shareOutAfter(pts, Math.sqrt(7 * 14))).toBeCloseTo((0.6938 + 0.2213) / 2, 10);
    expect(shareOutAfter(pts, 45)).toBe(0);
    // Helene at a 2-week target: 22 in 100 still out, so not covered (the backtest's rule: 1 in 10).
    expect(shareOutAfter(pts, 14)).toBeGreaterThan(0.1);
  });

  it('says how long the worst event lasted and what the target would have done (Helene, 2 weeks)', () => {
    expect(stressLine(HELENE, 'power', 14)).toEqual({
      text: "In the worst power cut in your county's records (Hurricane Helene, 2024), some homes were without power for up to 1 month. A target of 2 weeks would have left about 20 in 100 homes there still waiting.",
      covered: false,
    });
  });

  it('gives the region its own sentence, and never repeats the year', () => {
    const line = stressLine(MARCH_2018, 'power', 3);
    expect(line.text).toBe(
      "In the worst power cut in your region's records (Wind and thunderstorms, March 2018), some homes were without power for up to 2 weeks. It was worst about 40 miles away. A target of 3 days would have left about 15 in 100 homes there still waiting.",
    );
    expect(line.text).not.toMatch(/2018, 2018/);
  });

  it('says so plainly when the target would have covered it', () => {
    const covered = { ...MARCH_2018, covered_by_target: true };
    expect(stressLine(covered, 'power', 14).text).toMatch(/A target of 2 weeks would have covered at least 9 in 10 homes there\.$/);
    expect(stressLine(covered, 'power', 14).covered).toBe(true);
  });

  it('words a water event by its typical and longest waits (rr-consequence water events)', () => {
    const jackson: StressTest = {
      event: 'the Jackson water crisis',
      date: '2022-08-29',
      region: 'Jackson, Mississippi',
      share_out_at_days: [
        [7, 0.5],
        [10, 0.1],
      ],
      covered_by_target: false,
      sources: ['npr_jackson_water_restored_2022'],
    };
    expect(stressLine(jackson, 'water_out', 7).text).toBe(
      'In the worst loss of tap water in Jackson, Mississippi (the Jackson water crisis, 2022), it lasted about 7 days for most homes and up to 10 days for some. A target of 7 days would have fallen short for some homes there.',
    );
    const uri: StressTest = {
      event: 'Winter Storm Uri',
      date: '2021-02-17',
      region: 'Austin, Texas',
      share_out_at_days: [[6, 1]],
      covered_by_target: true,
      sources: ['kut_austin_boil_2021'],
    };
    expect(stressLine(uri, 'water_boil', 7).text).toBe(
      'In the worst boil-water notice in Austin, Texas (Winter Storm Uri, 2021), it lasted about 6 days. A target of 7 days would have covered at least 9 in 10 homes there.',
    );
    // An event named with its year keeps it once.
    const blackout = { ...uri, event: 'the August 2003 Northeast blackout', date: '2003-08-14', region: 'Cleveland, Ohio', share_out_at_days: [[3, 1]] as [number, number][] };
    expect(stressLine(blackout, 'water_boil', 5).text).toMatch(/^In the worst boil-water notice in Cleveland, Ohio \(the August 2003 Northeast blackout\), it lasted about 3 days\./);
  });
});

function hazard(id: HazardProfile['id'], confidence: HazardProfile['confidence']): HazardProfile {
  return {
    id,
    name: id,
    tier: 'natural',
    display: 'ranked',
    rate_per_year: 0.1,
    rate_range: [0.05, 0.2],
    annual_probability: 0.095,
    probability_range: [0.049, 0.18],
    severity: 0.5,
    climate_multiplier: 1,
    confidence,
    sources: [],
    frequency_sentence: '',
    buckets: ['power'],
  };
}

function bucket(contributions: BucketAssessment['contributions']): BucketAssessment {
  const t = { kind: 'days' as const, value: 3, low: 2, high: 5 };
  return { id: 'power', name: 'No grid power at home', target: t, covered: t, covered_today: t, tier_enough: 'h72', contributions, frequency_sentences: [], sources: [] };
}

describe('the confidence badge and the driver bars', () => {
  const register = [hazard('hurricane', 'medium'), hazard('strong_wind', 'low'), hazard('grid_failure', 'prior'), hazard('cyber_outage', 'prior')];

  it('lists what drives a target, largest first, each from records or an estimate', () => {
    // Philadelphia's power target in the v0.1 golden: hurricanes 65, windstorms 14, grid failure 5 in 100.
    const d = drivers(
      bucket([
        { hazard: 'hurricane', share: 0.65 },
        { hazard: 'strong_wind', share: 0.14 },
        { hazard: 'grid_failure', share: 0.05 },
      ]),
      register,
      null,
    );
    expect(d.map((x) => [x.hazard, x.evidence])).toEqual([
      ['hurricane', 'records'],
      ['strong_wind', 'records'],
      ['grid_failure', 'estimate'],
    ]);
    expect(d.map((x) => shareWords(x.share))).toEqual(['65 in 100', '14 in 100', '5 in 100']);
    expect(confidenceOf(d)).toBe('records');
  });

  it('says "Partly estimates" between a quarter and three quarters, "Estimates" above', () => {
    const mixed = drivers(bucket([{ hazard: 'hurricane', share: 0.5 }, { hazard: 'cyber_outage', share: 0.5 }]), register, null);
    expect(confidenceOf(mixed)).toBe('partly');
    const guessed = drivers(bucket([{ hazard: 'cyber_outage', share: 0.8 }, { hazard: 'hurricane', share: 0.2 }]), register, null);
    expect(confidenceOf(guessed)).toBe('estimates');
    expect(confidenceOf([])).toBeNull();
    expect(Object.values(CONFIDENCE_BADGE)).toEqual(['From records', 'Partly estimates', 'Estimates']);
    expect(shareWords(0.004)).toBe('fewer than 1 in 100');
  });
});
