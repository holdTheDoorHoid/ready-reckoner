/**
 * The optional steps' answers (DESIGN-DELTA-v3 §2–3): their length limits, the engine's tidy, and
 * editing them in the saved plan by path.
 *
 * Every one of these answers is echo-only free text: the engine trims it, cuts it at a length and
 * prints it, and never computes with it or checks it. §3 gives the limits ("length caps are
 * characters after trimming; tidy() cuts and drops empties as FamilyPlan::tidy does today"). They
 * are written here once, as data: the forms use them as `maxlength` (so nothing typed is ever cut
 * later), and `tidyValue` applies them exactly as the engine will, so the mock engine echoes what
 * the real one prints. Blank text becomes absent, an empty row or group is dropped, a list keeps
 * its first N entries, and an account's `last4` keeps only the last four digits typed (a pasted
 * full account number never reaches the file).
 *
 * The named caps are the contract's (`types.ts`, mirrored from rr-types); the tables below say
 * which field uses which, as docs/ENGINE-API.md does.
 */
import {
  ACCOUNTS_MAX,
  BLOOD_TYPE_MAX,
  DESCRIPTION_MAX,
  FAMILY_PLAN_SHORT_MAX,
  LABEL_TEXT_MAX,
  LONG_TEXT_MAX,
  MEDICATIONS_MAX,
  MEDIUM_TEXT_MAX,
  NOTE_MAX,
  PETS_MAX,
  PLACE_KINDS,
  PLATE_MAX,
  POLICIES_MAX,
  SHORT_TEXT_MAX,
  VEHICLES_MAX,
} from '../engine/types';

export { ACCOUNTS_MAX, MEDICATIONS_MAX, PETS_MAX, POLICIES_MAX, VEHICLES_MAX };

// ---------------------------------------------------------------------------------------------
// Text, as the engine tidies it
// ---------------------------------------------------------------------------------------------

/**
 * `text` trimmed and at most `max` characters long (counted as the engine counts them: Unicode
 * characters, not UTF-16 units), trimmed again at the end; undefined when nothing is left.
 */
export function tidyText(text: string | undefined, max: number): string | undefined {
  if (typeof text !== 'string') return undefined;
  const kept = Array.from(text.trim()).slice(0, max).join('').trimEnd();
  return kept === '' ? undefined : kept;
}

/** The last four digits of whatever was typed, or undefined when it holds no digit (§3.2 `last4`). */
export function lastFour(text: string | undefined): string | undefined {
  if (typeof text !== 'string') return undefined;
  const digits = text.replace(/\D/g, '');
  return digits === '' ? undefined : digits.slice(-4);
}

// ---------------------------------------------------------------------------------------------
// The limits (DESIGN-DELTA-v3 §3.1–3.2), in the engine's field order
// ---------------------------------------------------------------------------------------------

/** `Contact`: v2's name and phone (80 each) and v3's address. */
export const CONTACT_MAX = { name: FAMILY_PLAN_SHORT_MAX, phone: FAMILY_PLAN_SHORT_MAX, address: LONG_TEXT_MAX } as const;
export const PROFILE_MAX = {
  name: MEDIUM_TEXT_MAX,
  date_of_birth: SHORT_TEXT_MAX,
  phone: SHORT_TEXT_MAX,
  email: LABEL_TEXT_MAX,
  conditions: NOTE_MAX,
  allergies: LONG_TEXT_MAX,
  blood_type: BLOOD_TYPE_MAX,
  id_notes: LONG_TEXT_MAX,
  notes: NOTE_MAX,
} as const;
export const PLACE_MAX = {
  name: DESCRIPTION_MAX,
  address: LONG_TEXT_MAX,
  phone: SHORT_TEXT_MAX,
  plan: NOTE_MAX,
  pickup: LONG_TEXT_MAX,
  safest_spot: LONG_TEXT_MAX,
} as const;
export const MEDICATION_MAX = { name: LABEL_TEXT_MAX, dose: MEDIUM_TEXT_MAX, schedule: LABEL_TEXT_MAX, purpose: LABEL_TEXT_MAX } as const;
export const HEALTH_INSURANCE_MAX = {
  carrier: LABEL_TEXT_MAX,
  plan_name: LABEL_TEXT_MAX,
  member_id: MEDIUM_TEXT_MAX,
  group_number: MEDIUM_TEXT_MAX,
  phone: SHORT_TEXT_MAX,
} as const;
export const HOME_MAX = {
  address: LONG_TEXT_MAX,
  policy_number: MEDIUM_TEXT_MAX,
  where_kit: LONG_TEXT_MAX,
  where_documents: LONG_TEXT_MAX,
  where_cash: LONG_TEXT_MAX,
  where_keys: LONG_TEXT_MAX,
} as const;
export const NEIGHBOURHOOD_MAX = { alerts: LONG_TEXT_MAX } as const;
export const PET_MAX = {
  name: MEDIUM_TEXT_MAX,
  kind: SHORT_TEXT_MAX,
  description: DESCRIPTION_MAX,
  medications: LONG_TEXT_MAX,
  microchip: MEDIUM_TEXT_MAX,
  records_where: LONG_TEXT_MAX,
} as const;
export const VEHICLE_MAX = { description: DESCRIPTION_MAX, plate: PLATE_MAX, policy_number: MEDIUM_TEXT_MAX, kept_in_car: LONG_TEXT_MAX } as const;
export const DOCUMENTS_MAX = { where_originals: LONG_TEXT_MAX, where_copies: LONG_TEXT_MAX, digital_backup: LONG_TEXT_MAX } as const;
export const ACCOUNT_MAX = { institution: LABEL_TEXT_MAX, kind: SHORT_TEXT_MAX, phone: SHORT_TEXT_MAX } as const;
export const POLICY_MAX = { insurer: LABEL_TEXT_MAX, kind: SHORT_TEXT_MAX, policy_number: MEDIUM_TEXT_MAX, phone: SHORT_TEXT_MAX } as const;

// ---------------------------------------------------------------------------------------------
// The shapes, as data
// ---------------------------------------------------------------------------------------------

export type Spec =
  | { t: 'text'; max: number }
  | { t: 'last4' }
  /** A required choice (`Place.kind`): kept when valid, but on its own it is not an answer. */
  | { t: 'enum'; values: readonly string[] }
  | { t: 'obj'; fields: Readonly<Record<string, Spec>> }
  | { t: 'list'; of: Spec; max: number };

const text = (max: number): Spec => ({ t: 'text', max });
const obj = (fields: Record<string, Spec>): Spec => ({ t: 'obj', fields });
const list = (of: Spec, max: number): Spec => ({ t: 'list', of, max });
const texts = (caps: Readonly<Record<string, number>>): Record<string, Spec> =>
  Object.fromEntries(Object.entries(caps).map(([k, max]) => [k, text(max)]));

export const CONTACT_SPEC: Spec = obj(texts(CONTACT_MAX));

export const PLACE_SPEC: Spec = obj({ kind: { t: 'enum', values: PLACE_KINDS }, ...texts(PLACE_MAX) });

export const PROFILE_SPEC: Spec = obj({
  name: text(PROFILE_MAX.name),
  date_of_birth: text(PROFILE_MAX.date_of_birth),
  phone: text(PROFILE_MAX.phone),
  email: text(PROFILE_MAX.email),
  place: PLACE_SPEC,
  doctor: CONTACT_SPEC,
  pharmacy: CONTACT_SPEC,
  conditions: text(PROFILE_MAX.conditions),
  medications: list(obj(texts(MEDICATION_MAX)), MEDICATIONS_MAX),
  allergies: text(PROFILE_MAX.allergies),
  blood_type: text(PROFILE_MAX.blood_type),
  insurance: obj(texts(HEALTH_INSURANCE_MAX)),
  id_notes: text(PROFILE_MAX.id_notes),
  notes: text(PROFILE_MAX.notes),
});

export const HOME_SPEC: Spec = obj({
  address: text(HOME_MAX.address),
  electric_utility: CONTACT_SPEC,
  gas_utility: CONTACT_SPEC,
  water_utility: CONTACT_SPEC,
  insurer: CONTACT_SPEC,
  policy_number: text(HOME_MAX.policy_number),
  landlord_or_mortgage: CONTACT_SPEC,
  where_kit: text(HOME_MAX.where_kit),
  where_documents: text(HOME_MAX.where_documents),
  where_cash: text(HOME_MAX.where_cash),
  where_keys: text(HOME_MAX.where_keys),
});

export const NEIGHBOURHOOD_SPEC: Spec = obj({
  hospital: CONTACT_SPEC,
  urgent_care: CONTACT_SPEC,
  pharmacy: CONTACT_SPEC,
  shelter: CONTACT_SPEC,
  county_emergency_office: CONTACT_SPEC,
  alerts: text(NEIGHBOURHOOD_MAX.alerts),
});

export const PET_SPEC: Spec = obj({
  name: text(PET_MAX.name),
  kind: text(PET_MAX.kind),
  description: text(PET_MAX.description),
  medications: text(PET_MAX.medications),
  vet: CONTACT_SPEC,
  microchip: text(PET_MAX.microchip),
  records_where: text(PET_MAX.records_where),
});

export const VEHICLE_SPEC: Spec = obj({
  description: text(VEHICLE_MAX.description),
  plate: text(VEHICLE_MAX.plate),
  insurer: CONTACT_SPEC,
  policy_number: text(VEHICLE_MAX.policy_number),
  kept_in_car: text(VEHICLE_MAX.kept_in_car),
});

export const DOCUMENTS_SPEC: Spec = obj({
  accounts: list(obj({ ...texts(ACCOUNT_MAX), last4: { t: 'last4' } }), ACCOUNTS_MAX),
  policies: list(obj(texts(POLICY_MAX)), POLICIES_MAX),
  ...texts(DOCUMENTS_MAX),
});

// ---------------------------------------------------------------------------------------------
// Tidying by shape
// ---------------------------------------------------------------------------------------------

const isRecord = (x: unknown): x is Record<string, unknown> => typeof x === 'object' && x !== null && !Array.isArray(x);

/**
 * `value` as the engine keeps it under `spec`: a new value with every text trimmed and cut, empty
 * rows and groups dropped and lists cut to their limit, fields in the engine's order; undefined
 * when nothing is left. Fields the shape does not know are left out. The argument is not changed.
 */
export function tidyValue(value: unknown, spec: Spec): unknown {
  switch (spec.t) {
    case 'text':
      return tidyText(typeof value === 'string' ? value : undefined, spec.max);
    case 'last4':
      return lastFour(typeof value === 'string' ? value : undefined);
    case 'enum':
      return typeof value === 'string' && spec.values.includes(value) ? value : undefined;
    case 'list': {
      if (!Array.isArray(value)) return undefined;
      const kept = value.map((v) => tidyValue(v, spec.of)).filter((v) => v !== undefined).slice(0, spec.max);
      return kept.length ? kept : undefined;
    }
    case 'obj': {
      if (!isRecord(value)) return undefined;
      const out: Record<string, unknown> = {};
      let answered = false;
      for (const [key, sub] of Object.entries(spec.fields)) {
        const kept = tidyValue(value[key], sub);
        if (kept === undefined) continue;
        out[key] = kept;
        if (sub.t !== 'enum') answered = true;
      }
      return answered ? out : undefined;
    }
  }
}

// ---------------------------------------------------------------------------------------------
// Editing by path (the forms)
// ---------------------------------------------------------------------------------------------

/** Keys from a root object down to one answer: `['profile', 'medications', 2, 'dose']`. */
export type Path = readonly (string | number)[];

type Node = Record<string | number, unknown>;

/** The node at `path` (an object or a list), or undefined. */
function nodeAt(root: unknown, path: Path): unknown {
  let node = root;
  for (const key of path) {
    if (typeof node !== 'object' || node === null) return undefined;
    node = (node as Node)[key];
  }
  return node;
}

/** The text at `path`, or undefined. */
export function textAt(root: unknown, path: Path): string | undefined {
  const v = nodeAt(root, path);
  return typeof v === 'string' ? v : undefined;
}

/**
 * The object or list at `path`, created (with any on the way) when missing. In the app the plan is
 * reactive state: each step reads the child back after creating it, so the returned node is the
 * watched one, never the plain object just assigned.
 */
export function ensureAt(root: object, path: Path, list = false): Node {
  let node = root as Node;
  path.forEach((key, i) => {
    const wantList = i === path.length - 1 ? list : typeof path[i + 1] === 'number';
    const child = node[key];
    if (typeof child !== 'object' || child === null) node[key] = wantList ? [] : {};
    node = node[key] as Node;
  });
  return node;
}

/**
 * Remove every object left empty along `path`, deepest first, up to (not including) the root. A
 * row of a list is never removed this way (rows keep their places while being edited), and neither
 * is anything above it.
 */
export function pruneAlong(root: object, path: Path): void {
  for (let depth = path.length; depth > 0; depth--) {
    const parent = nodeAt(root, path.slice(0, depth - 1));
    const key = path[depth - 1]!;
    if (Array.isArray(parent) || typeof parent !== 'object' || parent === null) return;
    const child = (parent as Node)[key];
    const empty = Array.isArray(child) ? child.length === 0 : isRecord(child) && Object.keys(child).length === 0;
    if (child !== undefined && !empty) return;
    delete (parent as Node)[key];
  }
}

/** Set the text at `path` as typed, creating the groups on the way; blank removes it (and any group left empty). */
export function setTextAt(root: object, path: Path, value: string): void {
  const key = path[path.length - 1]!;
  if (value === '') {
    const parent = nodeAt(root, path.slice(0, -1));
    if (typeof parent === 'object' && parent !== null) delete (parent as Node)[key];
    pruneAlong(root, path.slice(0, -1));
    return;
  }
  ensureAt(root, path.slice(0, -1))[key] = value;
}

/** Trim and cut the text at `path` when the person leaves the field, as the engine will; nothing left removes it. */
export function tidyTextAt(root: object, path: Path, max: number): void {
  const current = textAt(root, path);
  if (current === undefined) return;
  const kept = tidyText(current, max);
  if (kept !== current) setTextAt(root, path, kept ?? '');
}

/** How many rows the list at `path` has. */
export function rowCount(root: unknown, path: Path): number {
  const v = nodeAt(root, path);
  return Array.isArray(v) ? v.length : 0;
}

/** Add an empty row to the list at `path` (created when missing); false when it already has `max` rows. */
export function addRow(root: object, path: Path, max: number): boolean {
  if (rowCount(root, path) >= max) return false;
  (ensureAt(root, path, true) as unknown as unknown[]).push({});
  return true;
}

/** Remove row `index` of the list at `path`; an emptied list goes, and any group left empty. */
export function removeRow(root: object, path: Path, index: number): void {
  const rows = nodeAt(root, path);
  if (!Array.isArray(rows) || index < 0 || index >= rows.length) return;
  rows.splice(index, 1);
  pruneAlong(root, path);
}
