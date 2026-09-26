/**
 * The backtest behind the public validation page (`#/validation`; model review Part 3.1): 22 real
 * disasters, the household used for each, what happened, the target the planner gives that
 * household today, and a plain verdict, with the misses kept on the page.
 *
 * This is the web's copy of the frozen test in `docs/VALIDATION.md` (rr-consequence's backtest,
 * `cargo test -p rr-consequence --test backtest`), rewritten for readers. The engine carries only
 * the tally (`EngineInfo.validation`); the rows live here. `validation.test.ts` checks every
 * verdict and in-sample flag against `docs/VALIDATION.md` once that file is in the repository
 * (awaiting: consequence, which adds it), and the tally against the mock engine's summary.
 */
import type { CitationId, ValidationSummary } from '../engine/types';

export type Verdict = 'covered' | 'partial' | 'short' | 'not_modelled';

export const VERDICTS: readonly Verdict[] = ['covered', 'partial', 'short', 'not_modelled'];

/** The scoring rule's words (docs/VALIDATION.md "The scoring rule"), as the page says them. */
export const VERDICT_WORDS: Record<Verdict, { label: string; means: string }> = {
  covered: {
    label: 'Covered',
    means: 'The target was long enough for about 9 in 10 of the homes it hit (or more than three times what happened).',
  },
  partial: {
    label: 'Partly covered',
    means: 'The target was long enough for a typical home it hit, but not for the longest waits.',
  },
  short: { label: 'Short', means: 'The target was shorter than what a typical home it hit went through.' },
  not_modelled: { label: 'Not modelled', means: 'The planner has no way to show this kind of event yet.' },
};

export interface ValidationEvent {
  /** The row number in docs/VALIDATION.md. */
  n: number;
  event: string;
  place: string;
  /** Month and year, as words. */
  when: string;
  /** Who we planned for, in plain words. */
  household: string;
  /** What happened, for the needs scored. */
  happened: string;
  /** What the planner tells that household today. */
  target: string;
  verdict: Verdict;
  /**
   * The verdict before this round's model changes (docs/VALIDATION.md "Before": the engine as
   * merged at the start of v0.2, `3c46b9b`, scored by the same rule). Its tally, 6 / 5 / 10 / 1,
   * is also the first version's as the round-2 review scored it.
   */
  before: Verdict;
  /** The event is inside the records the model learned from. */
  in_sample: boolean;
  /** What we changed because of it, or what the miss still needs. */
  changed: string;
  /** Sources in the citation list; the rest are named in docs/VALIDATION.md. */
  sources: CitationId[];
}

/** When and against what these verdicts were recorded (docs/VALIDATION.md "Results"). */
export const VALIDATION_RUN = {
  /** The run the rows report: this version, with the data pack v2 tables and the v2 answers. */
  label: 'this version, with the data pack v2 tables and the households’ answers to the new questions',
  recorded: '2026-09-26',
  /** The tally before this round's changes (and of the first version), for comparison. */
  first: { covered: 6, partial: 5, short: 10, not_modelled: 1 },
} as const;

/** The public copy of the frozen test (repository documents are published under CC BY-SA). */
export const VALIDATION_DOC_URL = 'https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/docs/VALIDATION.md';

export const VALIDATION_EVENTS: readonly ValidationEvent[] = [
  {
    n: 1,
    event: 'Winter Storm Uri',
    place: 'Austin, Texas',
    when: 'February 2021',
    household: 'The usual family (two working adults, a child and a dog) in a house with a heat pump',
    happened: 'Power: half of homes back in about 2 days, 9 in 10 in about 4. Cold indoors for as long. A boil-water notice for 6 days.',
    target: 'Power 5 days, heat or cold 3 days, boil water 3 weeks.',
    verdict: 'partial',
    before: 'short',
    in_sample: true,
    changed: 'A grid emergency in extreme cold is now its own kind of event, using the region’s record of such emergencies.',
    sources: ['kut_austin_boil_2021'],
  },
  {
    n: 2,
    event: 'Winter Storm Uri',
    place: 'Houston, Texas',
    when: 'February 2021',
    household: 'The usual family in a house with gas heat',
    happened: 'Power: half of homes back in about 2 days, 9 in 10 in about 4. Cold indoors for as long.',
    target: 'Power 10 days.',
    verdict: 'covered',
    before: 'covered',
    in_sample: true,
    changed: 'Power cuts after hurricanes now last as long as the region’s own record says.',
    sources: [],
  },
  {
    n: 3,
    event: 'Hurricane Helene',
    place: 'Asheville, North Carolina (city water)',
    when: 'September 2024',
    household: 'The usual family in a house on city water',
    happened: 'No tap water: 3 in 4 homes back by day 19, 9 in 10 by day 21. The boil-water notice lasted 52 days. Power: 9 in 10 back in about 2 weeks.',
    target: 'No tap water 3 weeks (covered), boil water 45 days (short), power 10 days (partly).',
    verdict: 'short',
    before: 'short',
    in_sample: true,
    changed: 'Floods that shut the water plant, long boil notices after a system failure, and the water system’s record now raise the water targets.',
    sources: ['avl_watchdog_water_2024', 'nchn_asheville_water_2024', 'epa_asheville_boil_notice_2024'],
  },
  {
    n: 4,
    event: 'Hurricane Helene',
    place: 'Rural Buncombe County, North Carolina (well)',
    when: 'September 2024',
    household: 'Two working adults in a rural house on a well',
    happened: 'No tap water while the power was out: 1 to 3 weeks.',
    target: 'No tap water 45 days.',
    verdict: 'covered',
    before: 'covered',
    in_sample: true,
    changed: 'Nothing needed.',
    sources: [],
  },
  {
    n: 5,
    event: 'Hurricane Ida',
    place: 'Jefferson Parish, Louisiana',
    when: 'August 2021',
    household: 'The usual family in a house',
    happened: 'Power: about 2 in 100 of the parish still out on day 18.',
    target: 'Power 10 days.',
    verdict: 'partial',
    before: 'partial',
    in_sample: true,
    changed: 'Hurricanes are no longer counted twice.',
    sources: [],
  },
  {
    n: 6,
    event: 'Remnants of Hurricane Ida',
    place: 'Queens, New York (basement flat)',
    when: 'September 2021',
    household: 'Two working adults renting a flat below street level, no car, $40 a month',
    happened: 'A flash flood with about 15 minutes’ warning; 11 people drowned in basement homes across the city. Out of the home about a week.',
    target: 'Leave at once when warned (3 minutes’ warning planned for); away 3 days.',
    verdict: 'partial',
    before: 'short',
    in_sample: false,
    changed: 'A question about sleeping below street level, and a flash-flood row for those homes. The flood chance itself is still the county’s.',
    sources: [],
  },
  {
    n: 7,
    event: 'Camp Fire',
    place: 'Paradise, California',
    when: 'November 2018',
    household: 'A retired couple on daily prescriptions, one who needs help getting around',
    happened: 'About an hour and a half from the fire starting to the first order to leave; 95 in 100 of the town burned. Away for months or years.',
    target: 'Leave when warned; away 3 days.',
    verdict: 'short',
    before: 'short',
    in_sample: false,
    changed: 'Still short. Time away after a wildfire needs data on homes at the edge of wild land.',
    sources: [],
  },
  {
    n: 8,
    event: 'Lahaina fire',
    place: 'Maui, Hawaii',
    when: 'August 2023',
    household: 'The usual family, renting, with no renters insurance',
    happened: 'Minutes of warning; away for months.',
    target: 'Leave when warned; away 2 days.',
    verdict: 'short',
    before: 'short',
    in_sample: false,
    changed: 'Still short, for the same reason as Paradise.',
    sources: [],
  },
  {
    n: 9,
    event: 'Jackson water crisis',
    place: 'Jackson, Mississippi',
    when: 'August–September 2022',
    household: 'The usual family in a house on city water',
    happened: 'No tap water for about 7 days. The boil-water notice lasted about 7 weeks.',
    target: 'No tap water 3 weeks (covered), boil water 30 days (short).',
    verdict: 'short',
    before: 'short',
    in_sample: true,
    changed: 'As for Asheville: the water system’s record now raises the water targets. A notice this long is beyond the usual setting; a stove, bleach or a filter covers a notice of any length.',
    sources: ['npr_jackson_water_restored_2022', 'npr_jackson_boil_2022'],
  },
  {
    n: 10,
    event: 'East Palestine derailment',
    place: 'East Palestine, Ohio',
    when: 'February 2023',
    household: 'The usual family in a house',
    happened: 'An order to leave after a chemical release, with about an hour’s warning. Away 5 days.',
    target: 'Leave when told; away 2 days.',
    verdict: 'partial',
    before: 'partial',
    in_sample: false,
    changed: 'Nothing yet.',
    sources: [],
  },
  {
    n: 11,
    event: 'Colonial Pipeline shutdown',
    place: 'Gwinnett County, Georgia and Mecklenburg County, North Carolina',
    when: 'May 2021',
    household: 'The usual family, with two gasoline cars',
    happened: 'Gas stations ran dry for days.',
    target: 'The planner has no fuel target.',
    verdict: 'not_modelled',
    before: 'not_modelled',
    in_sample: false,
    changed: 'Still a gap. The free step of keeping half a tank is the answer for now.',
    sources: [],
  },
  {
    n: 12,
    event: 'Change Healthcare and CrowdStrike outages',
    place: 'Franklin County, Ohio',
    when: 'February and July 2024',
    household: 'A working adult and an older adult, both on daily prescriptions (one kept cold)',
    happened: 'Pharmacy claims and e-prescribing were down: about a week for most, 15 days in all.',
    target: 'Medicine 30 days.',
    verdict: 'covered',
    before: 'partial',
    in_sample: false,
    changed: 'A medicine-shortage row.',
    sources: [],
  },
  {
    n: 13,
    event: 'Hurricane Maria',
    place: 'San Juan, Puerto Rico',
    when: 'September 2017',
    household: 'The usual family in a house on public water, with window air conditioning',
    happened: 'Households went about 84 days without power and 68 without tap water on average; 9 in 10 had power back after about 170 days.',
    target: 'Power 90 days (partly), no tap water 180 days (covered).',
    verdict: 'partial',
    before: 'short',
    in_sample: true,
    changed: 'Maria’s own restoration record for the island’s grid, and public water that fails with long power cuts.',
    sources: ['doe_maria_situation_reports', 'kishore_2018_maria'],
  },
  {
    n: 14,
    event: 'Hurricane Maria',
    place: 'Utuado, Puerto Rico',
    when: 'September 2017',
    household: 'The usual family in a rural house on public water',
    happened: 'As in San Juan, and among the last places to get power back.',
    target: 'Power 45 days, no tap water 60 days.',
    verdict: 'short',
    before: 'short',
    in_sample: true,
    changed: 'Still short: inland counties need their own share of major hurricanes (about 17 in 100 here), which is not passed through yet.',
    sources: ['doe_maria_situation_reports', 'kishore_2018_maria'],
  },
  {
    n: 15,
    event: 'SNAP benefits stopped',
    place: 'Philadelphia, Pennsylvania',
    when: 'November 2025',
    household: 'A single working parent with a child and a toddler, renting, on SNAP, $10 a month',
    happened: 'Food benefits stopped for 12 days.',
    target: 'Food 3 weeks.',
    verdict: 'covered',
    before: 'short',
    in_sample: false,
    changed: 'A row for pay or benefits stopping, for households that rely on them.',
    sources: [],
  },
  {
    n: 16,
    event: 'Superstorm Sandy',
    place: 'Staten Island, New York',
    when: 'October 2012',
    household: 'The usual family in a house with a basement',
    happened: 'Power: half of homes back in about 5 days; the last about 2 weeks after.',
    target: 'Power 5 days.',
    verdict: 'partial',
    before: 'partial',
    in_sample: false,
    changed: 'Nothing yet.',
    sources: [],
  },
  {
    n: 17,
    event: 'Superstorm Sandy',
    place: 'Long Beach, New York',
    when: 'October 2012',
    household: 'A working adult and an older adult on daily prescriptions',
    happened: 'Power: 9 in 10 back after 13 days. Water and sewers back on day 13.',
    target: 'Power 5 days, no tap water 5 days.',
    verdict: 'short',
    before: 'short',
    in_sample: false,
    changed: 'Still short: a barrier-island city is averaged with its county. It needs storm-surge data for each ZIP code.',
    sources: ['cbs_long_beach_water_2012'],
  },
  {
    n: 18,
    event: 'Northeast blackout',
    place: 'Cleveland, Ohio',
    when: 'August 2003',
    household: 'The usual family in a house',
    happened: 'Power back in about 2 days; a boil-water advisory for 3 days.',
    target: 'Power and boil-water targets both long enough.',
    verdict: 'covered',
    before: 'covered',
    in_sample: false,
    changed: 'Nothing needed.',
    sources: ['cleveland19_blackout_2003'],
  },
  {
    n: 19,
    event: 'Northeast blackout',
    place: 'Manhattan, New York (20th floor)',
    when: 'August 2003',
    household: 'A working adult and an older adult on daily prescriptions, renting on the 20th floor',
    happened: 'Power and tap water out for about 29 hours; up to 4 days in some neighbourhoods.',
    target: 'Power and tap-water targets both long enough.',
    verdict: 'covered',
    before: 'covered',
    in_sample: false,
    changed: 'Nothing needed.',
    sources: [],
  },
  {
    n: 20,
    event: 'Ice storm',
    place: 'Oklahoma City, Oklahoma',
    when: 'October 2020',
    household: 'The usual family in a house',
    happened: 'Power: about 40,000 of 370,000 homes still out on day 10.',
    target: 'Power 5 days.',
    verdict: 'partial',
    before: 'covered',
    in_sample: true,
    changed: 'Outage records are now pooled across the region, so this storm no longer sets the county’s target alone.',
    sources: [],
  },
  {
    n: 21,
    event: 'Ice storm',
    place: 'Austin, Texas',
    when: 'February 2023',
    household: 'The usual family in a house with a heat pump',
    happened: 'Power: 3 in 10 homes out at the peak; about a week for the last.',
    target: 'Power 5 days.',
    verdict: 'partial',
    before: 'partial',
    in_sample: true,
    changed: 'Nothing yet.',
    sources: [],
  },
  {
    n: 22,
    event: 'Derecho',
    place: 'Linn County, Iowa',
    when: 'August 2020',
    household: 'The usual family in a house',
    happened: 'Power: about 9 in 10 back in 8 days, all by day 17 or 18.',
    target: 'Power 5 days.',
    verdict: 'partial',
    before: 'covered',
    in_sample: true,
    changed: 'Outage records are now pooled across the region: without its own derecho, the county looks like its neighbours.',
    sources: [],
  },
];

export type Tally = Record<Verdict, number>;

/** How many events got each verdict. */
export function tally(rows: readonly Pick<ValidationEvent, 'verdict'>[], key: 'verdict' | 'before' = 'verdict'): Tally {
  const out: Tally = { covered: 0, partial: 0, short: 0, not_modelled: 0 };
  for (const r of rows) out[(r as ValidationEvent)[key] ?? r.verdict] += 1;
  return out;
}

/** The engine's summary agrees with the rows here. */
export function agrees(summary: ValidationSummary, rows: readonly ValidationEvent[]): boolean {
  const t = tally(rows);
  return (
    summary.events_tested === rows.length &&
    summary.covered === t.covered &&
    summary.partial === t.partial &&
    summary.short === t.short &&
    summary.not_modelled === t.not_modelled
  );
}
