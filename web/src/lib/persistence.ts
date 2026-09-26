/**
 * What the app keeps in the browser (docs/UI.md "Persistence"): one `localStorage` entry,
 * `rr.plan.v1`, holding the household and dials, what the household already had, the check-offs
 * and paid amounts recorded on the plan, and small bits of progress. The same object is what
 * "Export" saves as `ready-reckoner-plan.json` and "Import" reads back. Display preferences
 * (theme, expert view) live separately in `rr.prefs.v1` and are not exported.
 *
 * Contract v2 answers live inside `input` like every other answer (the household's access needs,
 * the new home and money questions, the dials, the family plan), so they are saved, exported and
 * imported with no change to the file's version. The one v2 answer outside `input` is when an item
 * that needs testing was last tried (`Owned.tested_on`): it is kept on the item's entry in
 * `input.existing` when the household had it before the plan, otherwise on its latest check-off
 * (`Purchase.tested_on`), and `engineInput` hands the engine the latest date for each item.
 *
 * Nothing here ever leaves the browser.
 */
import type { IsoDate, Owned, PlanInput, TierId } from '../engine/types';
import { TIER_IDS } from '../engine/types';

export const STORAGE_KEY = 'rr.plan.v1';
export const PREFS_KEY = 'rr.prefs.v1';
export const EXPORT_FORMAT = 'ready-reckoner-plan';
export const EXPORT_FILENAME = 'ready-reckoner-plan.json';

/** The interview steps, in order. */
export const STEP_IDS = ['where', 'who', 'travel', 'money', 'have'] as const;
export type StepId = (typeof STEP_IDS)[number];

/** Something bought or done from the plan, with the day it was recorded (for the maintenance calendar). */
export interface Purchase {
  item_id: string;
  /** The plan tier it was checked off in; an item can appear once per tier. */
  tier?: TierId;
  qty: number;
  paid_usd?: number;
  date: IsoDate;
  /** When it was last tried and worked (items that need testing; contract v2 `Owned.tested_on`). */
  tested_on?: IsoDate;
}

export interface SavedPlan {
  format: typeof EXPORT_FORMAT;
  version: 1;
  /** When the file was exported; informational. */
  saved_at?: string;
  /** The household and dials. `existing` holds only what the household had before the plan (the "What you already have" screen). */
  input: PlanInput;
  purchases: Purchase[];
  progress: { completed: StepId[]; last?: string };
  confidence: { before?: number; after?: number };
  /** Guardrail warnings the household chose to keep anyway. */
  dismissed_warnings: string[];
  /** Maintenance: key -> the last day it was done (rotations, checks, drills). */
  done_dates: Record<string, IsoDate>;
  reviewed_on?: IsoDate;
}

export interface Prefs {
  theme: 'system' | 'light' | 'dark';
  expert: boolean;
}

export const DEFAULT_PREFS: Prefs = { theme: 'system', expert: false };

export function newPlan(input: PlanInput): SavedPlan {
  return {
    format: EXPORT_FORMAT,
    version: 1,
    input,
    purchases: [],
    progress: { completed: [] },
    confidence: {},
    dismissed_warnings: [],
    done_dates: {},
  };
}

// ---------------------------------------------------------------------------------------------
// The engine's view of a saved plan
// ---------------------------------------------------------------------------------------------

/**
 * One `Owned` entry per item: the starting inventory plus everything recorded on the plan. A paid
 * amount is sent only when every unit of the item has one, so a partial record never turns into a
 * misleading unit price. The latest tested-on date of the item, wherever it was recorded, goes
 * with it.
 */
export function mergeOwned(baseline: readonly Owned[], purchases: readonly Purchase[]): Owned[] {
  const order: string[] = [];
  const totals = new Map<string, { qty: number; paid: number; allPaid: boolean; tested?: IsoDate }>();
  const add = (id: string, qty: number, paid: number | undefined, tested: IsoDate | undefined) => {
    let t = totals.get(id);
    if (!t) {
      t = { qty: 0, paid: 0, allPaid: true };
      totals.set(id, t);
      order.push(id);
    }
    t.qty += qty;
    if (paid === undefined) t.allPaid = false;
    else t.paid += paid;
    if (tested !== undefined && (t.tested === undefined || tested > t.tested)) t.tested = tested;
  };
  for (const o of baseline) add(o.item_id, o.qty, o.paid_usd, o.tested_on);
  for (const p of purchases) add(p.item_id, p.qty, p.paid_usd, p.tested_on);
  return order.map((id) => {
    const t = totals.get(id)!;
    const out: Owned = { item_id: id, qty: Math.round(t.qty * 1000) / 1000 };
    if (t.allPaid) out.paid_usd = Math.round(t.paid * 100) / 100;
    if (t.tested !== undefined) out.tested_on = t.tested;
    return out;
  });
}

// ---------------------------------------------------------------------------------------------
// When an item that needs testing was last tried (contract v2 `Owned.tested_on`)
// ---------------------------------------------------------------------------------------------

/** How much of an item the household has: before the plan, plus what it checked off. */
export function heldQuantity(plan: SavedPlan, itemId: string): number {
  const before = plan.input.existing.filter((o) => o.item_id === itemId).reduce((s, o) => s + o.qty, 0);
  const bought = plan.purchases.filter((p) => p.item_id === itemId).reduce((s, p) => s + p.qty, 0);
  return before + bought;
}

/** The latest day the item was tried, wherever it was recorded. */
export function testedOn(plan: SavedPlan, itemId: string): IsoDate | undefined {
  let latest: IsoDate | undefined;
  for (const d of [
    ...plan.input.existing.filter((o) => o.item_id === itemId).map((o) => o.tested_on),
    ...plan.purchases.filter((p) => p.item_id === itemId).map((p) => p.tested_on),
  ]) {
    if (d !== undefined && (latest === undefined || d > latest)) latest = d;
  }
  return latest;
}

/**
 * Record (or clear) the day the item was last tried: on its entry in what the household had before
 * the plan, or else on its latest check-off. Every other copy of the date is cleared, so the date
 * shown is the one kept. False when the household has none of the item.
 */
export function setTestedOn(plan: SavedPlan, itemId: string, date: IsoDate | undefined): boolean {
  const owned = plan.input.existing.find((o) => o.item_id === itemId && o.qty > 0);
  const purchases = plan.purchases.filter((p) => p.item_id === itemId);
  const latest = purchases.reduce<Purchase | undefined>((best, p) => (!best || p.date >= best.date ? p : best), undefined);
  const home = owned ?? latest;
  if (!home) return false;
  for (const o of plan.input.existing) if (o.item_id === itemId) delete o.tested_on;
  for (const p of purchases) delete p.tested_on;
  if (date !== undefined) home.tested_on = date;
  return true;
}

/** The `PlanInput` the engine sees: inventory merged with check-offs, earners kept consistent, confidence attached. */
export function engineInput(plan: SavedPlan): PlanInput {
  const input: PlanInput = JSON.parse(JSON.stringify(plan.input)) as PlanInput;
  input.existing = mergeOwned(plan.input.existing, plan.purchases);
  input.finances.income.earners = input.people.filter((p) => p.earner).length;
  const confidence = plan.confidence.after ?? plan.confidence.before;
  if (confidence !== undefined) input.confidence_1to5 = confidence;
  else delete input.confidence_1to5;
  return input;
}

// ---------------------------------------------------------------------------------------------
// Checking what comes back from storage or a file
// ---------------------------------------------------------------------------------------------

type Check = { ok: true; plan: SavedPlan } | { ok: false; reason: string };

const isObject = (x: unknown): x is Record<string, unknown> => typeof x === 'object' && x !== null && !Array.isArray(x);
const isDate = (x: unknown): x is IsoDate => typeof x === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(x);
const isNum = (x: unknown): x is number => typeof x === 'number' && Number.isFinite(x);

/** Looks enough like a `PlanInput` for the engine to judge the rest (it rejects anything malformed). */
function looksLikeInput(x: unknown): x is PlanInput {
  return (
    isObject(x) &&
    isDate(x.planning_date) &&
    isObject(x.location) &&
    isObject(x.housing) &&
    Array.isArray(x.people) &&
    isObject(x.pets) &&
    isObject(x.mobility) &&
    isObject(x.finances) &&
    Array.isArray(x.existing) &&
    isObject(x.dials)
  );
}

export function checkSavedPlan(x: unknown): Check {
  if (!isObject(x)) return { ok: false, reason: 'The file is not a saved plan.' };
  if (x.format !== EXPORT_FORMAT) return { ok: false, reason: 'The file is not a Ready Reckoner plan.' };
  if (x.version !== 1) return { ok: false, reason: 'The plan was saved by a newer version of Ready Reckoner. Reload the page to update, then try again.' };
  if (!looksLikeInput(x.input)) return { ok: false, reason: 'The household details in the file are incomplete.' };
  const purchases = x.purchases ?? [];
  if (
    !Array.isArray(purchases) ||
    !purchases.every(
      (p) =>
        isObject(p) &&
        typeof p.item_id === 'string' &&
        isNum(p.qty) &&
        p.qty >= 0 &&
        isDate(p.date) &&
        (p.paid_usd === undefined || (isNum(p.paid_usd) && p.paid_usd >= 0)) &&
        (p.tested_on === undefined || isDate(p.tested_on)) &&
        (p.tier === undefined || (TIER_IDS as readonly unknown[]).includes(p.tier)),
    )
  ) {
    return { ok: false, reason: 'The check-offs in the file are damaged.' };
  }
  const progress = isObject(x.progress) ? x.progress : {};
  const completed = Array.isArray(progress.completed)
    ? progress.completed.filter((s): s is StepId => (STEP_IDS as readonly unknown[]).includes(s))
    : [];
  const confidence = isObject(x.confidence) ? x.confidence : {};
  const inRange = (v: unknown) => (isNum(v) && v >= 1 && v <= 5 ? v : undefined);
  const doneDates: Record<string, IsoDate> = {};
  if (isObject(x.done_dates)) for (const [k, v] of Object.entries(x.done_dates)) if (isDate(v)) doneDates[k] = v;
  const plan: SavedPlan = {
    format: EXPORT_FORMAT,
    version: 1,
    input: x.input,
    purchases: purchases as Purchase[],
    progress: { completed, ...(typeof progress.last === 'string' ? { last: progress.last } : {}) },
    confidence: {},
    dismissed_warnings: Array.isArray(x.dismissed_warnings) ? x.dismissed_warnings.filter((w): w is string => typeof w === 'string') : [],
    done_dates: doneDates,
  };
  const before = inRange(confidence.before);
  const after = inRange(confidence.after);
  if (before !== undefined) plan.confidence.before = before;
  if (after !== undefined) plan.confidence.after = after;
  if (isDate(x.reviewed_on)) plan.reviewed_on = x.reviewed_on;
  return { ok: true, plan };
}

/**
 * Read an imported file: a Ready Reckoner export, or a bare household (`PlanInput`, such as a
 * fixture file). Structural problems are reported here; the engine judges the household itself.
 */
export function parseImport(text: string): Check {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, reason: 'The file could not be read. Choose the .json file you exported from Ready Reckoner.' };
  }
  if (isObject(data) && data.format === undefined && looksLikeInput(data)) {
    return { ok: true, plan: newPlan(data) };
  }
  return checkSavedPlan(data);
}

export function exportText(plan: SavedPlan, now: Date = new Date()): string {
  return `${JSON.stringify({ ...plan, saved_at: now.toISOString() }, null, 2)}\n`;
}

// ---------------------------------------------------------------------------------------------
// Browser storage (every access guarded: private windows and blocked storage throw)
// ---------------------------------------------------------------------------------------------

export function browserStorage(): Storage | null {
  try {
    const s = window.localStorage;
    const probe = 'rr.probe';
    s.setItem(probe, '1');
    s.removeItem(probe);
    return s;
  } catch {
    return null;
  }
}

export function loadPlan(storage: Storage | null): { plan: SavedPlan | null; damaged: boolean } {
  if (!storage) return { plan: null, damaged: false };
  try {
    const raw = storage.getItem(STORAGE_KEY);
    if (raw === null) return { plan: null, damaged: false };
    const check = checkSavedPlan(JSON.parse(raw));
    return check.ok ? { plan: check.plan, damaged: false } : { plan: null, damaged: true };
  } catch {
    return { plan: null, damaged: true };
  }
}

export function savePlan(storage: Storage | null, plan: SavedPlan | null): boolean {
  if (!storage) return false;
  try {
    if (plan === null) storage.removeItem(STORAGE_KEY);
    else storage.setItem(STORAGE_KEY, JSON.stringify(plan));
    return true;
  } catch {
    return false;
  }
}

export function loadPrefs(storage: Storage | null): Prefs {
  if (!storage) return { ...DEFAULT_PREFS };
  try {
    const raw = JSON.parse(storage.getItem(PREFS_KEY) ?? '{}') as Partial<Prefs>;
    return {
      theme: raw.theme === 'light' || raw.theme === 'dark' ? raw.theme : 'system',
      expert: raw.expert === true,
    };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

/** Saves preferences; the defaults are stored as nothing, so "forget everything" leaves no trace. */
export function savePrefs(storage: Storage | null, prefs: Prefs): void {
  if (!storage) return;
  try {
    if (prefs.theme === DEFAULT_PREFS.theme && prefs.expert === DEFAULT_PREFS.expert) storage.removeItem(PREFS_KEY);
    else storage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // Preferences are a convenience; losing them is harmless.
  }
}

/** "Forget everything": remove every key this app wrote. Returns how many were removed. */
export function forgetEverything(storage: Storage | null): number {
  if (!storage) return 0;
  let removed = 0;
  try {
    const keys: string[] = [];
    for (let i = 0; i < storage.length; i++) {
      const key = storage.key(i);
      if (key && key.startsWith('rr.')) keys.push(key);
    }
    for (const key of keys) {
      storage.removeItem(key);
      removed += 1;
    }
  } catch {
    // Nothing more can be done; the screen says what happened.
  }
  return removed;
}
