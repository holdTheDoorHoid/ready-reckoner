/**
 * The nearer goal on the savings track (W11). A goal of several months of expenses can be decades
 * away on a small budget, which is honest but gives nothing to aim at this year. The first goal is
 * one month of expenses; once that is saved, the next is three months (the low end of the usual
 * three-to-six-month advice the income goal rests on). Everything comes from the engine's own
 * `SavingsTrack`: expenses are `target_usd / target_months`, and the date assumes the engine's
 * suggested monthly amount, which starts once the supplies plan is done (`Plan.done_month`).
 */
import type { IsoDate, SavingsTrack } from '../engine/types';
import { addMonths } from './format';

export interface Milestone {
  /** Months of expenses: 1, then 3. */
  months: 1 | 3;
  /** The first goal (less than a month saved), rather than the next one. */
  first: boolean;
  /** In dollars, when the household gave its monthly expenses. */
  usd?: number;
  /** When the suggested saving reaches it; absent when there is no suggested amount or no end to the supplies plan. */
  by?: IsoDate;
}

const STEPS = [1, 3] as const;

/** The next goal below the full one, or null when there is none (already saved, or the full goal is that small). */
export function nextMilestone(track: SavingsTrack, planningDate?: IsoDate, doneMonth?: number): Milestone | null {
  const months = STEPS.find((m) => track.current_months < m && track.target_months > m);
  if (months === undefined) return null;
  const out: Milestone = { months, first: months === 1 };
  const expenses = track.target_months > 0 && track.target_usd > 0 ? track.target_usd / track.target_months : undefined;
  if (expenses === undefined) return out;
  out.usd = months * expenses;
  if (track.monthly_suggestion_usd > 0 && planningDate && doneMonth !== undefined) {
    const gap = (months - track.current_months) * expenses;
    out.by = addMonths(planningDate, doneMonth + Math.ceil(gap / track.monthly_suggestion_usd));
  }
  return out;
}
