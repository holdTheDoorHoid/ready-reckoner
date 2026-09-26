/**
 * Conditional spans in guidance blocks, ported from rr-content (`policy.rs`: `Condition`,
 * `apply_conditions`, `apply_conditions_for`), so a block reads the same on screen as in the
 * packet. Six kinds of condition:
 *
 * - `{if:tornado}…{/if}`: kept when that hazard is likely enough for the household (a ten-year
 *   chance of at least 1 in 100); an id that names no hazard keeps its text.
 * - `{if:home:apartment_high_rise|…}…{/if}` and `{if:not_home:…}…{/if}`: the kind of home.
 * - `{if:need:hearing|vision}…{/if}`: anyone in the household has one of these access needs.
 * - `{if:has:<item id>|…}…{/if}`: the household owns the item or the plan includes it.
 * - `{if:benefit:snap_wic|…}…{/if}`: the household relies on one of these benefits.
 *
 * Spans do not nest and stay within a paragraph. A malformed condition keeps its text (the
 * content validator rejects it before release), and a view with no household keeps every span.
 */
import type { PlanInput, PlanOutput } from '../engine/types';
import { ACCESS_NEEDS, BENEFITS, HAZARD_IDS, HOUSING_KINDS } from '../engine/types';

export const CONDITION_OPEN = '{if:';
export const CONDITION_CLOSE = '{/if}';

export type Condition =
  | { kind: 'hazard'; id: string }
  | { kind: 'home'; kinds: string[] }
  | { kind: 'not_home'; kinds: string[] }
  | { kind: 'need'; values: string[] }
  | { kind: 'has'; values: string[] }
  | { kind: 'benefit'; values: string[] };

/** What a household looks like to the conditions (rr-content's `HouseholdFacts`). */
export interface HouseholdFacts {
  hazardRelevant(hazard: string): boolean;
  home(): string;
  hasAccessNeed(need: string): boolean;
  hasItem(item: string): boolean;
  hasBenefit(benefit: string): boolean;
}

const WELL_FORMED_ID = /^[a-z][a-z0-9_]{0,63}$/;

/** `a|b|c` split into its parts; null for an empty or repeated part. */
function splitValues(values: string): string[] | null {
  const out: string[] = [];
  for (const raw of values.split('|')) {
    const v = raw.trim();
    if (v === '' || out.includes(v)) return null;
    out.push(v);
  }
  return out;
}

function listed(values: string, allowed: readonly string[]): string[] | null {
  const parts = splitValues(values);
  return parts && parts.every((v) => allowed.includes(v)) ? parts : null;
}

/** Reads a condition (the text between `{if:` and `}`); null when it is malformed. */
export function parseCondition(text: string): Condition | null {
  const id = text.trim();
  if (id.startsWith('need:')) {
    const values = listed(id.slice(5), ACCESS_NEEDS);
    return values ? { kind: 'need', values } : null;
  }
  if (id.startsWith('benefit:')) {
    const values = listed(id.slice(8), BENEFITS);
    return values ? { kind: 'benefit', values } : null;
  }
  if (id.startsWith('has:')) {
    const values = splitValues(id.slice(4));
    return values && values.every((v) => WELL_FORMED_ID.test(v)) ? { kind: 'has', values } : null;
  }
  const home = id.startsWith('home:') ? id.slice(5) : undefined;
  const notHome = id.startsWith('not_home:') ? id.slice(9) : undefined;
  const kindsText = home ?? notHome;
  if (kindsText === undefined) return { kind: 'hazard', id };
  const kinds = listed(kindsText, HOUSING_KINDS);
  if (!kinds) return null;
  return home !== undefined ? { kind: 'home', kinds } : { kind: 'not_home', kinds };
}

export function conditionHolds(c: Condition, h: HouseholdFacts): boolean {
  switch (c.kind) {
    case 'hazard':
      return h.hazardRelevant(c.id);
    case 'home':
      return c.kinds.includes(h.home());
    case 'not_home':
      return !c.kinds.includes(h.home());
    case 'need':
      return c.values.some((v) => h.hasAccessNeed(v));
    case 'has':
      return c.values.some((v) => h.hasItem(v));
    case 'benefit':
      return c.values.some((v) => h.hasBenefit(v));
  }
}

/**
 * `text` with the spans `keep` rejects removed and the markers of the others dropped. A span
 * removed from between two words takes one of the two spaces around it; one at the start of a
 * line takes the space after it. Malformed markers are left as they are.
 */
export function applyConditions(text: string, keep: (id: string) => boolean): string {
  let out = '';
  let rest = text;
  for (;;) {
    const start = rest.indexOf(CONDITION_OPEN);
    if (start < 0) break;
    const afterOpen = rest.slice(start + CONDITION_OPEN.length);
    const idEnd = afterOpen.indexOf('}');
    const close = afterOpen.indexOf(CONDITION_CLOSE);
    if (idEnd < 0 || close < 0 || close < idEnd) break;
    const id = afterOpen.slice(0, idEnd).trim();
    const inner = afterOpen.slice(idEnd + 1, close);
    out += rest.slice(0, start);
    let tail = afterOpen.slice(close + CONDITION_CLOSE.length);
    if (keep(id)) {
      out += inner;
    } else {
      const atLineStart = out === '' || out.endsWith('\n');
      if ((atLineStart || out.endsWith(' ')) && tail.startsWith(' ')) tail = tail.slice(1);
    }
    rest = tail;
  }
  return out + rest;
}

/** `text` with each span kept only when its condition holds for this household. */
export function applyConditionsFor(text: string, household: HouseholdFacts | null): string {
  if (!household) return applyConditions(text, () => true);
  return applyConditions(text, (id) => {
    const c = parseCondition(id);
    return c ? conditionHolds(c, household) : true;
  });
}

/** The ten-year chance that makes a hazard's advice worth keeping (the packet's cut). */
export const HAZARD_RELEVANT_TEN_YEAR_CHANCE = 0.01;

/**
 * The household as the conditions see it, from its answers and its latest assessment: a hazard is
 * relevant when its ten-year chance is at least 1 in 100 (every hazard counts before there is an
 * assessment); an item counts when it is owned (`input.existing` as the engine sees it) or in the
 * plan. Null when there is no household yet, which keeps every span.
 */
export function householdFacts(input: PlanInput | null | undefined, output: PlanOutput | undefined): HouseholdFacts | null {
  if (!input) return null;
  const hazards = new Set<string>(HAZARD_IDS);
  const planned = new Set<string>([
    ...(output?.plan.months.flatMap((m) => m.items.map((i) => i.item_id)) ?? []),
    ...(output?.plan.long_horizon?.map((i) => i.item_id) ?? []),
  ]);
  return {
    hazardRelevant(id) {
      if (!output || !hazards.has(id)) return true;
      const row = output.register.find((h) => h.id === id);
      return !!row && 1 - Math.exp(-10 * row.rate_per_year) >= HAZARD_RELEVANT_TEN_YEAR_CHANCE;
    },
    home: () => input.housing.kind,
    hasAccessNeed: (need) => input.people.some((p) => ((p.access_needs ?? []) as readonly string[]).includes(need)),
    hasItem: (item) => input.existing.some((o) => o.item_id === item && o.qty > 0) || planned.has(item),
    hasBenefit: (b) => ((input.finances.benefits ?? []) as readonly string[]).includes(b),
  };
}
