/**
 * The rare-but-severe box (REVIEW §2.4; hazard-expansion Deliverable C): nine families, each one
 * line when collapsed, sorted by how likely they are where the household lives and never by
 * expected loss. How likely is a range only; "why here" comes from the family's location factor;
 * "what it changes" is the engine's own words, with a check mark when it is "nothing beyond your
 * basics". The household may let the plan spend up to a tenth of its monthly budget on things made
 * for the families it ticks (`Dials.rare_opt_in`, read and written by lib/dials.ts), and the box says
 * what that bought.
 *
 * Also here: the "Also checked" line (every hazard and rare sub-row checked for this household and
 * found under 1 in 100,000 a year), which the engine writes into the packet's notes.
 */
import type { Catalogue, Dials, HazardProfile, PlanItem, PlanOutput } from '../engine/types';
import { RARE_HAZARD_IDS } from '../engine/types';
import { rareFamilies } from './dials';
import { catalogueItem } from './lookup';

/** The engine's share of the monthly budget the rare allowance may use (DESIGN §4.7). */
export const RARE_ALLOWANCE_SHARE = 0.1;

/**
 * The first sentence of a text: up to a full stop, question or exclamation mark that follows a
 * lower-case letter, a digit, a closing bracket or a degree sign, and is followed by a space and
 * a capital. "U.S. Air Force" is never cut after "U.S.".
 */
export function firstSentence(text: string): string {
  const m = /^(.+?[a-z0-9)°%][.!?])\s+(?=[A-Z"“])/.exec(text.trim());
  return m ? m[1]! : text.trim();
}

/** "Why here" in a few words for the table: the location factor's first sentence, or "the same everywhere". */
export function whyHereShort(h: HazardProfile): string {
  if (!h.location_factor?.label) return 'The same everywhere: this chance does not depend on where you live.';
  return firstSentence(h.location_factor.label);
}

/** "Nothing beyond your basics", "Nothing new: …": the engine's words say the plan needs nothing more. */
export function changesNothing(h: HazardProfile): boolean {
  return /^nothing\b/i.test((h.what_it_changes ?? '').trim());
}

/** What the family changes in the plan; the engine always sends it for a rare row. */
export function whatItChanges(h: HazardProfile): string {
  return h.what_it_changes ?? 'Nothing beyond your basics.';
}

export interface AllowanceLine {
  item: PlanItem;
  /** Plan month, from 0. */
  month: number;
  /** The rare families the item is made for. */
  families: string[];
}

export interface Allowance {
  /** Families ticked. */
  families: string[];
  /** The most a month the allowance may use, in dollars. */
  monthly_usd: number;
  /** What the plan buys with it, soonest first. */
  bought: AllowanceLine[];
}

/** The rare families an item is made for: its catalogue `hazard_extras` and the plan's `hazards`. */
function itemFamilies(cat: Catalogue | null, item: PlanItem): string[] {
  const extras = catalogueItem(cat, item.item_id)?.hazard_extras ?? [];
  const rare = new Set<string>(RARE_HAZARD_IDS);
  return [...new Set([...extras, ...item.hazards].filter((h) => rare.has(h)))];
}

/** What the rare allowance buys in this plan: the catalogue's rare-catastrophe items the plan schedules. */
export function allowance(output: PlanOutput, cat: Catalogue | null, dials: Dials, monthlyBudget: number): Allowance {
  const bought: AllowanceLine[] = [];
  for (const m of output.plan.months) {
    for (const item of m.items) {
      if (!catalogueItem(cat, item.item_id)?.rare_catastrophic) continue;
      bought.push({ item, month: m.index, families: itemFamilies(cat, item) });
    }
  }
  return {
    families: rareFamilies(dials),
    monthly_usd: Math.max(0, monthlyBudget) * RARE_ALLOWANCE_SHARE,
    bought,
  };
}

// ---------------------------------------------------------------------------------------------
// Also checked
// ---------------------------------------------------------------------------------------------

export interface AlsoChecked {
  /** The engine's lead, for example "Also checked, and under 1 in 100,000 a year here". */
  lead: string;
  /** Each hazard or sub-row, with its rate in words when the engine gives one. */
  items: { name: string; rate?: string }[];
}

/** Split "a (x), b (y) and c (z)" at the top level: commas and a final " and ", never inside brackets. */
function splitList(list: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let current = '';
  for (let i = 0; i < list.length; i++) {
    const c = list[i]!;
    if (c === '(') depth++;
    if (c === ')') depth = Math.max(0, depth - 1);
    if (depth === 0 && c === ',') {
      parts.push(current);
      current = '';
      continue;
    }
    if (depth === 0 && list.startsWith(' and ', i)) {
      parts.push(current);
      current = '';
      i += 4;
      continue;
    }
    current += c;
  }
  parts.push(current);
  return parts.map((p) => p.trim()).filter(Boolean);
}

/**
 * The "Also checked" note from the packet's notes (rr-hazards writes it; rr-plan prints each note
 * as a bullet). Two forms are read: v0.2's "Also checked, and under 1 in 100,000 a year here:
 * dust storms (none recorded here), … and a Yellowstone super-eruption (about 1 in 730,000 a
 * year)." and v0.1's "Also checked, and too rare here to list (under 1 in 100,000 a year):
 * landslides and wildfires." Markdown escapes are removed. Null when the packet has no such note.
 */
export function parseAlsoChecked(packet: string | undefined): AlsoChecked | null {
  if (!packet) return null;
  const line = packet.split('\n').find((l) => /^\s*(?:[-*]\s+)?Also checked\b/.test(l));
  if (!line) return null;
  const text = line
    .replace(/^\s*(?:[-*]\s+)?/, '')
    .replace(/\\([\\`*_[\]{}()#+\-.!|])/g, '$1')
    .trim()
    .replace(/\.$/, '');
  // The list starts after the first colon that is outside brackets.
  let depth = 0;
  let colon = -1;
  for (let i = 0; i < text.length; i++) {
    if (text[i] === '(') depth++;
    else if (text[i] === ')') depth = Math.max(0, depth - 1);
    else if (text[i] === ':' && depth === 0) {
      colon = i;
      break;
    }
  }
  if (colon < 0) return { lead: text, items: [] };
  const items = splitList(text.slice(colon + 1)).map((part) => {
    const m = /^(.*?)\s*\(([^()]*)\)$/.exec(part);
    return m ? { name: m[1]!, rate: m[2]! } : { name: part };
  });
  return { lead: text.slice(0, colon).trim(), items };
}

/**
 * The "Also checked" list for the Risks screen: the engine's note when the packet carries one;
 * otherwise every hazard the engine can check that is not on this household's list, by name only.
 */
export function alsoCheckedFor(output: PlanOutput, cat: Catalogue | null): AlsoChecked | null {
  const note = parseAlsoChecked(output.packet_markdown);
  if (note) return note;
  const listed = new Set<string>(output.register.map((h) => h.id));
  const rest = (cat?.hazards ?? []).filter((h) => !listed.has(h.id));
  if (rest.length === 0) return null;
  return { lead: 'Also checked, and not on your list', items: rest.map((h) => ({ name: h.name })) };
}

/** "6 in 10", or "3 in 100" below one in ten. */
function shareOf(x: number, per100: boolean): string {
  return per100 ? `${Math.max(1, Math.round(x * 100))}` : `${Math.max(1, Math.round(x * 10))}`;
}

/**
 * The location term as a link in the chain, where it has a plain meaning of its own: for the
 * nuclear family, the chance that the county would be in a blast or dangerous-fallout zone if a
 * large attack happened (its strategic-site group's factor). Null for the other families, whose
 * "why here" sentence already says what their factor does.
 */
export function locationChain(h: HazardProfile): string | null {
  const f = h.location_factor;
  if (!f || h.id !== 'nuclear_attack') return null;
  const [lo, mid, hi] = f.multiplier;
  if (!(mid > 0 && hi >= lo)) return null;
  const per100 = lo < 0.095;
  const unit = per100 ? 'in 100' : 'in 10';
  return `If a large attack on the country happened, the chance that your county would be in a blast or dangerous-fallout zone: about ${shareOf(mid, per100)} ${unit} (${shareOf(lo, per100)} to ${shareOf(hi, per100)} ${unit}).`;
}
