/**
 * What sits around a duration target (model review Part 3.3 and 3.4): one number per need, one
 * anchor from a real event, one badge.
 *
 * - The **stress line**: the worst event in the region's record for this need
 *   (`BucketAssessment.stress_test`), how long homes there went without it, and whether a target
 *   this long would have covered nine in ten of them.
 * - The **confidence badge**: "From records", "Partly estimates" or "Estimates", from how much of
 *   what drives the target rests on recorded events rather than expert judgement.
 * - The **driver bars**: the hazards behind the target (`contributions`), each with its share and
 *   whether its chance comes from records or from an estimate.
 */
import type { BucketAssessment, BucketId, Catalogue, HazardProfile, StressTest } from '../engine/types';
import { dayPhrase } from './format';
import { hazardName } from './lookup';

// ---------------------------------------------------------------------------------------------
// The stress line
// ---------------------------------------------------------------------------------------------

/** A share below this counts as everyone back (half a customer in a hundred). */
const BACK = 0.005;

/**
 * The share still out after `days`, read off the event's points exactly as rr-consequence reads
 * them (`share_out_after`, which decides `covered_by_target`): from everyone out down to the first
 * point, then straight lines in log-time between points, then the last share.
 */
export function shareOutAfter(points: readonly (readonly [number, number])[], days: number): number {
  const pts = points
    .map(([d, s]) => [d, Math.min(1, Math.max(0, s))] as const)
    .filter(([d]) => Number.isFinite(d) && d > 0)
    .sort((a, b) => a[0] - b[0]);
  if (pts.length === 0) return 0;
  const [d0, s0] = pts[0]!;
  if (days <= d0) return 1 - (1 - s0) * Math.min(1, Math.max(0, days / d0));
  for (let i = 1; i < pts.length; i++) {
    const [a, sa] = pts[i - 1]!;
    const [b, sb] = pts[i]!;
    if (days <= b) {
      const t = (Math.log(days) - Math.log(a)) / (Math.log(b) - Math.log(a));
      return sa + (sb - sa) * t;
    }
  }
  return pts[pts.length - 1]![1];
}

/** "about 22 in 100": a share of homes as a natural frequency (whole numbers, never "0 in 100"). */
function inHundred(share: number): string {
  const n = share * 100;
  if (n < 0.5) return 'fewer than 1 in 100';
  if (n >= 99.5) return 'nearly all';
  return `about ${n < 10 ? Math.round(n) : Math.round(n / 5) * 5} in 100`;
}

const EVENT_NOUN: Partial<Record<BucketId, string>> = {
  power: 'power cut',
  water_out: 'loss of tap water',
  water_boil: 'boil-water notice',
};

const WITHOUT: Partial<Record<BucketId, string>> = {
  power: 'without power',
  water_out: 'without tap water',
  water_boil: 'boiling their water',
};

export interface StressLine {
  /** The whole line, in plain words. */
  text: string;
  /** Whether a target this long would have covered nine in ten of the homes there. */
  covered: boolean;
}

/**
 * The stress line under a target, for example: "In the worst power cut in your region's records
 * (Hurricane Helene, 2024), some homes were without power for up to 1 month. A target of 2 weeks
 * would have left about 20 in 100 homes there still waiting."
 */
export function stressLine(st: StressTest, bucket: BucketId, targetDays: number): StressLine {
  const noun = EVENT_NOUN[bucket] ?? 'disruption';
  const without = WITHOUT[bucket] ?? 'without it';
  const year = st.date.slice(0, 4);
  const event = /\b\d{4}\b/.test(st.event) ? st.event : `${st.event}, ${year}`;
  // "your region's records (where it was worst, about 40 miles away)": keep brackets unnested.
  const m = /^(.*?)\s*\((.*)\)\s*$/.exec(st.region);
  const region = m ? m[1]! : st.region;
  const aside = m ? `; ${m[2]!}` : '';
  const lead = `In the worst ${noun} in ${region} (${event}${aside})`;

  const pts = [...st.share_out_at_days].sort((a, b) => a[0] - b[0]);
  let howLong: string;
  const lastOut = pts.filter(([, s]) => s >= BACK).at(-1);
  if (!lastOut) {
    howLong = `nearly every home had it back within ${dayPhrase(pts[0]?.[0] ?? 1)}`;
  } else {
    const after = pts.find(([d, s]) => d > lastOut[0] && s < BACK);
    howLong = after
      ? `some homes were ${without} for up to ${dayPhrase(after[0])}`
      : `some homes were still ${without} after ${dayPhrase(lastOut[0])}`;
  }

  const target = dayPhrase(targetDays);
  const left = shareOutAfter(pts, targetDays);
  const verdict = st.covered_by_target
    ? `A target of ${target} would have covered at least 9 in 10 homes there.`
    : left >= BACK
      ? `A target of ${target} would have left ${inHundred(left)} homes there still waiting.`
      : `A target of ${target} would have fallen short for some homes there.`;
  return { text: `${lead}, ${howLong}. ${verdict}`, covered: st.covered_by_target };
}

// ---------------------------------------------------------------------------------------------
// The confidence badge and the driver bars
// ---------------------------------------------------------------------------------------------

export type Evidence = 'records' | 'estimate';

export interface Driver {
  hazard: string;
  name: string;
  /** 0 to 1; the drivers of one target add up to 1. */
  share: number;
  evidence: Evidence;
}

/** A hazard's chance comes from records unless the engine marks it an expert estimate (`prior`). */
function evidenceOf(register: readonly HazardProfile[], hazard: string): Evidence {
  return register.find((h) => h.id === hazard)?.confidence === 'prior' ? 'estimate' : 'records';
}

/** The hazards behind a target, largest share first, each with where its chance comes from. */
export function drivers(bucket: BucketAssessment, register: readonly HazardProfile[], cat: Catalogue | null): Driver[] {
  return bucket.contributions
    .filter((c) => c.share > 0)
    .map((c) => ({ hazard: c.hazard, name: hazardName(cat, c.hazard), share: c.share, evidence: evidenceOf(register, c.hazard) }));
}

export type Confidence = 'records' | 'partly' | 'estimates';

export const CONFIDENCE_BADGE: Record<Confidence, string> = {
  records: 'From records',
  partly: 'Partly estimates',
  estimates: 'Estimates',
};

/**
 * The badge for a target: "From records" when estimates drive less than a quarter of it,
 * "Estimates" when they drive more than three quarters, "Partly estimates" between. Null when
 * nothing drives it (a target of 0, or a bucket with no events).
 */
export function confidenceOf(list: readonly Driver[]): Confidence | null {
  const total = list.reduce((s, d) => s + d.share, 0);
  if (!(total > 0)) return null;
  const estimate = list.filter((d) => d.evidence === 'estimate').reduce((s, d) => s + d.share, 0) / total;
  if (estimate < 0.25) return 'records';
  if (estimate > 0.75) return 'estimates';
  return 'partly';
}

/** "65 in 100" for a driver's share (whole numbers; "fewer than 1 in 100" below half a percent). */
export function shareWords(share: number): string {
  const n = Math.round(share * 100);
  return n < 1 ? 'fewer than 1 in 100' : `${n} in 100`;
}
