/**
 * The maintenance calendar (docs/UI.md screen 9): what to rotate, check and practise, and when,
 * computed from the dates the household recorded. The app never sends reminders (it never
 * contacts anyone); the calendar can be downloaded as an .ics file for the household's own
 * calendar. The file holds item names and dates only, never the address or household details.
 */
import type { Catalogue, IsoDate, Item } from '../engine/types';
import { addMonths } from './format';
import type { SavedPlan } from './persistence';

export type TaskKind = 'rotate' | 'check' | 'drill' | 'review';

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

function titleFor(kind: TaskKind, item: Item): string {
  // Free steps are already phrased as actions ("Test smoke alarms..."), so they keep their names.
  if (item.free) return kind === 'rotate' ? `${item.name} (swap it for fresh)` : item.name;
  // Only the first letter is lowered, so "CO alarm" stays "CO alarm".
  const name = item.name.charAt(0).toLowerCase() + item.name.slice(1);
  if (kind === 'rotate') return `Use and replace: ${name}`;
  if (kind === 'drill') return `Practise: ${name}`;
  return `Check: ${name}`;
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
