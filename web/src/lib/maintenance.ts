/**
 * The maintenance calendar (docs/UI.md screen 9): what to rotate, check, test and practise, and
 * when, computed from the dates the household recorded. The app never sends reminders (it never
 * contacts anyone); the calendar can be downloaded as an .ics file for the household's own
 * calendar. The file holds item names and dates only, never the address or household details.
 *
 * Contract v2 adds two kinds: **tests** for items with `Item.test_interval_months` (a jump pack, a
 * generator, a key safe: try it and record the day, `Owned.tested_on`, kept by persistence's
 * `testedOn` / `setTestedOn` as on the Have screen), and **seasonal anchors**
 * for items with `Item.season` (have it before the season starts, or check it then; meteorological
 * seasons, so summer starts on 1 June with the hurricane season).
 */
import type { Catalogue, IsoDate, Item, Season } from '../engine/types';
import { SEASONS } from '../engine/types';
import { addMonths } from './format';
import type { SavedPlan } from './persistence';
import { testedOn } from './persistence';

export type TaskKind = 'rotate' | 'check' | 'drill' | 'review' | 'test' | 'season';

export interface Task {
  /** Stable key into `SavedPlan.done_dates`. */
  key: string;
  kind: TaskKind;
  item_id?: string;
  title: string;
  interval_months: number;
  /** When it was last done, if known. */
  last?: IsoDate;
  due: IsoDate;
  /** The household had it before the plan, so its age is unknown. */
  from_inventory: boolean;
}

function lastPurchase(plan: SavedPlan, itemId: string): IsoDate | undefined {
  const dates = plan.purchases.filter((p) => p.item_id === itemId).map((p) => p.date);
  return dates.length ? dates.sort().at(-1) : undefined;
}

function every(months: number): string {
  if (months === 1) return 'every month';
  if (months === 12) return 'every year';
  if (months % 12 === 0) return `every ${months / 12} years`;
  return `every ${months} months`;
}

export function intervalLabel(months: number): string {
  const s = every(months);
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** The month each season starts (meteorological seasons). */
export const SEASON_MONTH: Record<Season, number> = { spring: 3, summer: 6, fall: 9, winter: 12 };

/** "summer" as a reader says it, with when it starts: "summer (June)". */
export const SEASON_WORDS: Record<Season, { name: string; month: string }> = {
  spring: { name: 'spring', month: 'March' },
  summer: { name: 'summer', month: 'June' },
  fall: { name: 'fall', month: 'September' },
  winter: { name: 'winter', month: 'December' },
};

function titleFor(kind: TaskKind, item: Item): string {
  // Only the first letter is lowered, so "CO alarm" stays "CO alarm".
  const name = item.name.charAt(0).toLowerCase() + item.name.slice(1);
  if (kind === 'test') return `Test: ${name}`;
  if (kind === 'season') return `Before ${item.season ? SEASON_WORDS[item.season].name : 'the season'}: ${item.free ? name : `check ${name}`}`;
  // Free steps are already phrased as actions ("Test smoke alarms..."), so they keep their names.
  if (item.free) return kind === 'rotate' ? `${item.name} (swap it for fresh)` : item.name;
  if (kind === 'rotate') return `Use and replace: ${name}`;
  if (kind === 'drill') return `Practice: ${name}`;
  return `Check: ${name}`;
}

/** The first day of `season` on or after `from` (strictly after, when `after` is set). */
export function nextSeasonStart(season: Season, from: IsoDate, after = false): IsoDate {
  const [y, m, d] = from.split('-').map(Number) as [number, number, number];
  const month = SEASON_MONTH[season];
  const start = (year: number) => `${year}-${String(month).padStart(2, '0')}-01`;
  const thisYear = start(y);
  const passed = after ? thisYear <= from : m > month || (m === month && d > 1);
  return passed ? start(y + 1) : thisYear;
}

/** Every task for what the household has, soonest first. */
export function maintenanceTasks(plan: SavedPlan, catalogue: Catalogue): Task[] {
  const owned = new Set<string>();
  for (const o of plan.input.existing) if (o.qty > 0) owned.add(o.item_id);
  for (const p of plan.purchases) if (p.qty > 0) owned.add(p.item_id);
  const tasks: Task[] = [];
  for (const id of owned) {
    const item = catalogue.items.find((i) => i.id === id);
    const m = item?.maintenance;
    if (!item || !m) continue;
    const bought = lastPurchase(plan, id);
    const fromInventory = bought === undefined;
    const start = bought ?? plan.input.planning_date;
    if (m.rotate_months) {
      const key = `rotate:${id}`;
      const last = plan.done_dates[key] ?? bought;
      tasks.push({ key, kind: 'rotate', item_id: id, title: titleFor('rotate', item), interval_months: m.rotate_months, due: addMonths(last ?? start, m.rotate_months), from_inventory: fromInventory, ...(last ? { last } : {}) });
    }
    if (m.check_months) {
      const kind: TaskKind = item.unit === 'drill' ? 'drill' : 'check';
      const key = `check:${id}`;
      const last = plan.done_dates[key] ?? bought;
      tasks.push({ key, kind, item_id: id, title: titleFor(kind, item), interval_months: m.check_months, due: addMonths(last ?? start, m.check_months), from_inventory: fromInventory, ...(last ? { last } : {}) });
    }
  }
  // Contract v2: tests and seasonal anchors, for what the household has.
  for (const id of owned) {
    const item = catalogue.items.find((i) => i.id === id);
    if (!item) continue;
    const bought = lastPurchase(plan, id);
    const fromInventory = bought === undefined;
    if (item.test_interval_months) {
      const key = `test:${id}`;
      const last = testedOn(plan, id);
      const months = item.test_interval_months;
      tasks.push({ key, kind: 'test', item_id: id, title: titleFor('test', item), interval_months: months, due: addMonths(last ?? bought ?? plan.input.planning_date, months), from_inventory: fromInventory, ...(last ? { last } : {}) });
    }
    if (item.season) {
      const key = `season:${id}`;
      const last = plan.done_dates[key];
      const due = last ? nextSeasonStart(item.season, last, true) : nextSeasonStart(item.season, plan.input.planning_date);
      tasks.push({ key, kind: 'season', item_id: id, title: titleFor('season', item), interval_months: 12, due, from_inventory: fromInventory, ...(last ? { last } : {}) });
    }
  }
  const reviewed = plan.reviewed_on ?? plan.done_dates.review;
  tasks.push({
    key: 'review',
    kind: 'review',
    title: 'Review your plan and your answers',
    interval_months: 12,
    due: addMonths(reviewed ?? plan.input.planning_date, 12),
    from_inventory: false,
    ...(reviewed ? { last: reviewed } : {}),
  });
  return tasks.sort((a, b) => a.due.localeCompare(b.due) || a.title.localeCompare(b.title));
}

/** One season's anchors for the "Through the year" view: the items to have ready or check when it starts. */
export interface SeasonAnchors {
  season: Season;
  items: Item[];
}

/**
 * The seasonal anchors of what the household has and what its plan still holds, season by season
 * from spring; seasons with nothing are left out.
 */
export function seasonalAnchors(catalogue: Catalogue, itemIds: ReadonlySet<string>): SeasonAnchors[] {
  return SEASONS.map((season) => ({
    season,
    items: catalogue.items.filter((i) => i.season === season && itemIds.has(i.id)),
  })).filter((s) => s.items.length > 0);
}

/** Drills in the catalogue that apply to this plan (free actions practised on a schedule). */
export function drillItems(catalogue: Catalogue, planItemIds: ReadonlySet<string>): Item[] {
  return catalogue.items.filter((i) => i.free && i.unit === 'drill' && planItemIds.has(i.id));
}

// ---------------------------------------------------------------------------------------------
// iCalendar export (RFC 5545)
// ---------------------------------------------------------------------------------------------

function icsEscape(text: string): string {
  return text.replace(/\\/g, '\\\\').replace(/;/g, '\\;').replace(/,/g, '\\,').replace(/\r?\n/g, '\\n');
}

/** Fold lines longer than 75 octets, as the format requires. */
function fold(line: string): string {
  const bytes = new TextEncoder().encode(line);
  if (bytes.length <= 75) return line;
  const out: string[] = [];
  let current = '';
  let size = 0;
  for (const ch of line) {
    const n = new TextEncoder().encode(ch).length;
    if (size + n > (out.length === 0 ? 75 : 74)) {
      out.push(current);
      current = '';
      size = 0;
    }
    current += ch;
    size += n;
  }
  out.push(current);
  return out.join('\r\n ');
}

const compact = (iso: IsoDate) => iso.replace(/-/g, '');

/** A calendar file with one repeating all-day event per task. */
export function calendarFile(tasks: readonly Task[], today: IsoDate): string {
  const lines = ['BEGIN:VCALENDAR', 'VERSION:2.0', 'PRODID:-//Ready Reckoner//Maintenance calendar//EN', 'CALSCALE:GREGORIAN', 'METHOD:PUBLISH'];
  for (const t of tasks) {
    const next = addMonths(t.due, 0);
    const end = new Date(`${next}T00:00:00Z`);
    end.setUTCDate(end.getUTCDate() + 1);
    lines.push(
      'BEGIN:VEVENT',
      `UID:${t.key.replace(/[^A-Za-z0-9_-]/g, '-')}@ready-reckoner.local`,
      `DTSTAMP:${compact(today)}T000000Z`,
      `DTSTART;VALUE=DATE:${compact(next)}`,
      `DTEND;VALUE=DATE:${compact(end.toISOString().slice(0, 10))}`,
      `RRULE:FREQ=MONTHLY;INTERVAL=${t.interval_months}`,
      `SUMMARY:${icsEscape(t.title)}`,
      `DESCRIPTION:${icsEscape(`${intervalLabel(t.interval_months)}. From your Ready Reckoner plan.`)}`,
      'END:VEVENT',
    );
  }
  lines.push('END:VCALENDAR');
  return `${lines.map(fold).join('\r\n')}\r\n`;
}
