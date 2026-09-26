/**
 * How numbers read on screen and in the packet (docs/CONTENT_STANDARDS.md §6, risk-model §7):
 * natural frequencies rounded to whole numbers up to 10 and to the nearest 5 above; days on the
 * target ladder in the unit a person would use; whole dollars with bands as "$30–45".
 */
import type { DataConfidence, IsoDate } from '../engine/types';

// ---------------------------------------------------------------------------------------------
// Natural frequencies
// ---------------------------------------------------------------------------------------------

export type Per100 =
  | { kind: 'fewer' }
  | { kind: 'number'; n: number }
  | { kind: 'almost_all' };

/** A probability as households out of 100: under 1, 1–10 whole, above 10 to the nearest 5. */
export function per100(p: number): Per100 {
  const n = 100 * Math.min(1, Math.max(0, p));
  if (n < 1) return { kind: 'fewer' };
  if (n >= 97.5) return { kind: 'almost_all' };
  if (n <= 10) return { kind: 'number', n: Math.max(1, Math.round(n)) };
  return { kind: 'number', n: Math.round(n / 5) * 5 };
}

function per100Text(v: Per100): string {
  if (v.kind === 'fewer') return 'fewer than 1';
  if (v.kind === 'almost_all') return 'almost all';
  return String(v.n);
}

/** The bracketed range after a natural frequency, or '' when it adds nothing. */
function rangeText(main: Per100, lo?: number, hi?: number): string {
  if (lo === undefined || hi === undefined) return '';
  const l = per100(lo);
  const h = per100(hi);
  const lt = per100Text(l);
  const ht = per100Text(h);
  if (lt === ht || (lt === per100Text(main) && ht === per100Text(main))) return '';
  if (l.kind === 'fewer' && h.kind === 'almost_all') return '';
  if (l.kind === 'fewer') return ` (up to ${ht})`;
  if (h.kind === 'almost_all') return ` (${lt} or more)`;
  return ` (${lt}–${ht})`;
}

/** "in the next 10 years" / "in the next year". */
export function inYears(years: number): string {
  return years === 1 ? 'in the next year' : `in the next ${years} years`;
}

/** The chance of at least one event in `years` years at `ratePerYear`. */
export function chanceWithin(ratePerYear: number, years: number): number {
  return 1 - Math.exp(-ratePerYear * years);
}

/**
 * "About 25 (15–40) of 100 households like yours will lose power in a windstorm in the next 10 years."
 * `p`, `lo` and `hi` are chances over the whole horizon.
 */
export function frequencySentence(p: number, what: string, years: number, lo?: number, hi?: number): string {
  const main = per100(p);
  const tail = `will ${what} ${inYears(years)}.`;
  if (main.kind === 'almost_all') return `Almost all households like yours ${tail}`;
  if (main.kind === 'fewer') return `Fewer than 1 in 100 households like yours ${tail}`;
  return `About ${main.n}${rangeText(main, lo, hi)} of 100 households like yours ${tail}`;
}

/** "about 10 of 100", for short labels. */
export function naturalFrequency(p: number): string {
  const v = per100(p);
  if (v.kind === 'almost_all') return 'almost all';
  if (v.kind === 'fewer') return 'fewer than 1 of 100';
  return `about ${v.n} of 100`;
}

/** The expert-view percentage: at most two significant figures. */
export function percent(p: number): string {
  const v = p * 100;
  if (v === 0) return '0%';
  if (v < 0.1) return 'under 0.1%';
  if (v >= 10) return `${Math.round(v)}%`;
  if (v >= 1) return `${Math.round(v * 10) / 10}%`;
  return `${Math.round(v * 100) / 100}%`;
}

// ---------------------------------------------------------------------------------------------
// Days
// ---------------------------------------------------------------------------------------------

interface DayUnit {
  unit: 'day' | 'week' | 'month' | 'year';
  n: number;
}

function unitOf(d: number): DayUnit {
  if (d === 365) return { unit: 'year', n: 1 };
  if (d === 14 || d === 21) return { unit: 'week', n: d / 7 };
  if (d === 45) return { unit: 'month', n: 1.5 };
  if (d >= 30 && d % 30 === 0) return { unit: 'month', n: d / 30 };
  if (d > 30 && d < 365) return { unit: 'month', n: Math.round(d / 30) };
  if (d > 365) return { unit: 'year', n: Math.round((d / 365) * 10) / 10 };
  return { unit: 'day', n: d };
}

function nText(n: number): string {
  if (n === 0.5) return '½';
  if (Number.isInteger(n)) return String(n);
  if (n % 1 === 0.5) return `${Math.floor(n)}½`;
  return String(Math.round(n * 10) / 10);
}

/** "half a day", "1 day", "3 days", "2 weeks", "1 month", "3 months", "1 year". */
export function dayPhrase(d: number): string {
  if (d === 0) return '0 days';
  if (d === 0.5) return 'half a day';
  const u = unitOf(d);
  return `${nText(u.n)} ${u.unit}${u.n === 1 ? '' : 's'}`;
}

/** A target with its range: "about 3 days (2–5)", "about 2 weeks (10 days–3 weeks)". */
export function targetDays(value: number, low: number, high: number): string {
  const main = `about ${dayPhrase(value)}`;
  if (low === value && high === value) return main;
  if (low === value) return `${main} (up to ${dayPhrase(high)})`;
  const [ul, uv, uh] = [unitOf(low), unitOf(value), unitOf(high)];
  if (ul.unit === uv.unit && uv.unit === uh.unit && value !== 0.5 && low !== 0.5) {
    return `${main} (${nText(ul.n)}–${nText(uh.n)})`;
  }
  return `${main} (${dayPhrase(low)}–${dayPhrase(high)})`;
}

/** Months of income: "about 4 months (2–6)". */
export function targetMonths(value: number, low: number, high: number): string {
  const m = (v: number) => (v === 0.5 ? 'half a month' : `${nText(v)} month${v === 1 ? '' : 's'}`);
  if (low === value && high === value) return `about ${m(value)}`;
  return `about ${m(value)} (${nText(low)}–${nText(high)})`;
}

export function monthsPhrase(v: number): string {
  if (v === 0) return 'no months';
  if (v === 0.5) return 'half a month';
  return `${nText(v)} month${v === 1 ? '' : 's'}`;
}

/** Notice time: "15 minutes to 12 hours", "1 to 3 days". */
export function noticeRange(lowHours: number, highHours: number): string {
  const one = (h: number): string => {
    if (h < 1) return `${Math.round(h * 60)} minutes`;
    if (h < 48) return `${nText(Math.round(h * 10) / 10)} hour${h === 1 ? '' : 's'}`;
    const d = Math.round(h / 24);
    return `${d} day${d === 1 ? '' : 's'}`;
  };
  return lowHours === highHours ? one(lowHours) : `${one(lowHours)} to ${one(highHours)}`;
}

// ---------------------------------------------------------------------------------------------
// Money and quantities
// ---------------------------------------------------------------------------------------------

const usd0 = new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 });

/** Whole dollars: "$1,200". */
export function usd(n: number): string {
  return `$${usd0.format(Math.round(n))}`;
}

/** A price band: "$30–45"; "$30" when both ends match; "Free" at $0. */
export function band(low: number, high: number): string {
  const l = Math.round(low);
  const h = Math.round(high);
  if (l === 0 && h === 0) return 'Free';
  if (l === h) return usd(l);
  return `${usd(l)}–${usd0.format(h)}`;
}

const IRREGULAR: Record<string, string> = {
  each: 'each',
  box: 'boxes',
  pouch: 'pouches',
  'person-day': 'person-days',
  'pet-day': 'pet-days',
  'day per person': 'days per person',
};

export function plural(unit: string, qty: number): string {
  if (qty === 1) return unit;
  const irregular = IRREGULAR[unit];
  if (irregular) return irregular;
  if (/[^aeiou]y$/.test(unit)) return `${unit.slice(0, -1)}ies`;
  if (/(s|x|ch|sh)$/.test(unit)) return `${unit}es`;
  return `${unit}s`;
}

/** "12 gallons", "1 kit", "$100" for a quantity of dollars. */
export function quantity(qty: number, unit: string): string {
  const q = Number.isInteger(qty) ? usd0.format(qty) : String(Math.round(qty * 10) / 10);
  if (unit === 'dollar') return usd(qty);
  if (unit === 'each') return q;
  return `${q} ${plural(unit, qty)}`;
}

// ---------------------------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------------------------

/** Severity as a word; the UI pairs it with a pattern so colour is never the only signal. */
export function severityBand(s: number): { level: 1 | 2 | 3 | 4 | 5; label: string } {
  if (s < 0.2) return { level: 1, label: 'Minor' };
  if (s < 0.4) return { level: 2, label: 'Moderate' };
  if (s < 0.6) return { level: 3, label: 'Serious' };
  if (s < 0.8) return { level: 4, label: 'Severe' };
  return { level: 5, label: 'Very severe' };
}

export const CONFIDENCE_LABELS: Record<DataConfidence, string> = {
  high: 'Based on data',
  medium: 'Mostly data',
  low: 'Rough data',
  prior: 'Expert estimate',
};

// ---------------------------------------------------------------------------------------------
// Dates (calendar dates in UTC, so a date never shifts with the viewer's time zone)
// ---------------------------------------------------------------------------------------------

const dateFmt = new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric', year: 'numeric', timeZone: 'UTC' });
const monthFmt = new Intl.DateTimeFormat('en-US', { month: 'long', year: 'numeric', timeZone: 'UTC' });

function parse(iso: IsoDate): Date {
  return new Date(`${iso}T00:00:00Z`);
}

export function isoDate(d: Date): IsoDate {
  return d.toISOString().slice(0, 10);
}

/** Today's calendar date where the viewer is. */
export function localToday(now = new Date()): IsoDate {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, '0');
  const d = String(now.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/** "Oct 1, 2026". */
export function formatDate(iso: IsoDate): string {
  return dateFmt.format(parse(iso));
}

/** "October 2026". */
export function formatMonth(iso: IsoDate): string {
  return monthFmt.format(parse(iso));
}

/** The same day `months` later, clamped to the month's last day. */
export function addMonths(iso: IsoDate, months: number): IsoDate {
  const d = parse(iso);
  const day = d.getUTCDate();
  const target = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + months, 1));
  const last = new Date(Date.UTC(target.getUTCFullYear(), target.getUTCMonth() + 1, 0)).getUTCDate();
  target.setUTCDate(Math.min(day, last));
  return isoDate(target);
}

/** Whole months from `from` to `to` (negative when `to` is earlier). */
export function monthsBetween(from: IsoDate, to: IsoDate): number {
  const a = parse(from);
  const b = parse(to);
  let months = (b.getUTCFullYear() - a.getUTCFullYear()) * 12 + (b.getUTCMonth() - a.getUTCMonth());
  if (b.getUTCDate() < a.getUTCDate()) months -= 1;
  return months;
}

export function daysBetween(from: IsoDate, to: IsoDate): number {
  return Math.round((parse(to).getTime() - parse(from).getTime()) / 86_400_000);
}
