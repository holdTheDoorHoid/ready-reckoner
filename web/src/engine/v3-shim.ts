/**
 * awaiting: types3 — this whole file is a stand-in, deleted when `v0.3` brings contract v3 into
 * `web/src/engine/types.ts` (types3 owns that file). It declares the contract v3 input names from
 * docs/DESIGN-DELTA-v3 §3.1–3.2 verbatim and adds them to the v2 interfaces by module augmentation,
 * so every screen already reads and writes `person.profile`, `family_plan.home` and so on under
 * their final names. At the merge: delete this file, import the types below from `./types`, and
 * replace `prepareMarkdownOf(output)` with `output.prepare_markdown`.
 *
 * One name differs from the delta, as it does in types3's `types.ts`: §3.1 calls a person's health
 * insurance `Insurance`, which `types.ts` already uses for what the household's finances hold
 * (home, flood, earthquake), so it is `HealthInsurance`. The JSON field is `profile.insurance`.
 * Every name and cap here matches agent/types3 at 85b21e2.
 */
import type { Contact, PlanOutput } from './types';

// The field caps and list limits, named and valued as types3's `types.ts` exports them (checked
// against agent/types3 at 85b21e2). Characters after trimming; use them as the forms' maxlength.

/** AccountInfo.last4 keeps at most this many characters, and only digits. */
export const LAST4_LEN = 4;
/** PersonProfile.blood_type. */
export const BLOOD_TYPE_MAX = 8;
/** VehicleInfo.plate. */
export const PLATE_MAX = 20;
/** Dates of birth, the v3 phone fields, the kinds of pet, account and policy. */
export const SHORT_TEXT_MAX = 40;
/** Names of people and animals, doses, member IDs, group, policy and microchip numbers. */
export const MEDIUM_TEXT_MAX = 60;
/** Emails, institutions, insurance carriers and plan names, insurers, a medication's name, schedule and purpose. */
export const LABEL_TEXT_MAX = 80;
/** A place's name, an animal or a vehicle described. */
export const DESCRIPTION_MAX = 120;
/** Addresses and "where it is" answers, allergies, pick-up rules, safe spots, ID notes, alerts, pet medicines, what stays in the car. */
export const LONG_TEXT_MAX = 200;
/** Medical conditions, notes, a place's own emergency plan. */
export const NOTE_MAX = 400;
/** Most medications per person. */
export const MEDICATIONS_MAX = 12;
/** Most animals in FamilyPlan.pets. */
export const PETS_MAX = 8;
/** Most vehicles in FamilyPlan.vehicles. */
export const VEHICLES_MAX = 4;
/** Most accounts in DocumentsInfo.accounts. */
export const ACCOUNTS_MAX = 12;
/** Most policies in DocumentsInfo.policies. */
export const POLICIES_MAX = 8;

/** Where a person spends the day (DESIGN-DELTA-v3 §3.1 `Place.kind`). */
export const PLACE_KINDS = ['work', 'school', 'childcare', 'other'] as const;
export type PlaceKind = (typeof PLACE_KINDS)[number];

/** Everything step 6 asks about one person; every field optional and echo-only (§2.1, §3.1). */
export interface PersonProfile {
  name?: string;
  date_of_birth?: string;
  phone?: string;
  email?: string;
  place?: Place;
  doctor?: Contact;
  pharmacy?: Contact;
  conditions?: string;
  /** At most 12 (`MEDICATIONS_MAX`); empty rows are dropped. */
  medications?: Medication[];
  allergies?: string;
  blood_type?: string;
  insurance?: HealthInsurance;
  id_notes?: string;
  notes?: string;
}

export interface Place {
  kind: PlaceKind;
  name?: string;
  address?: string;
  phone?: string;
  plan?: string;
  pickup?: string;
  safest_spot?: string;
}

export interface Medication {
  name?: string;
  dose?: string;
  schedule?: string;
  purpose?: string;
}

/** §3.1 `Insurance` (a person's health insurance); renamed here, see the file comment. */
export interface HealthInsurance {
  carrier?: string;
  plan_name?: string;
  member_id?: string;
  group_number?: string;
  phone?: string;
}

export interface HomeInfo {
  address?: string;
  electric_utility?: Contact;
  gas_utility?: Contact;
  water_utility?: Contact;
  insurer?: Contact;
  policy_number?: string;
  landlord_or_mortgage?: Contact;
  where_kit?: string;
  where_documents?: string;
  where_cash?: string;
  where_keys?: string;
}

export interface Neighbourhood {
  hospital?: Contact;
  urgent_care?: Contact;
  pharmacy?: Contact;
  shelter?: Contact;
  county_emergency_office?: Contact;
  alerts?: string;
}

export interface PetInfo {
  name?: string;
  kind?: string;
  description?: string;
  medications?: string;
  vet?: Contact;
  microchip?: string;
  records_where?: string;
}

export interface VehicleInfo {
  description?: string;
  plate?: string;
  insurer?: Contact;
  policy_number?: string;
  kept_in_car?: string;
}

export interface DocumentsInfo {
  /** At most 12 (`ACCOUNTS_MAX`). */
  accounts?: AccountInfo[];
  /** At most 8 (`POLICIES_MAX`). */
  policies?: PolicyInfo[];
  where_originals?: string;
  where_copies?: string;
  digital_backup?: string;
}

export interface AccountInfo {
  institution?: string;
  kind?: string;
  phone?: string;
  /** Digits only, at most four: the engine keeps the last four digits of whatever was typed. */
  last4?: string;
}

export interface PolicyInfo {
  insurer?: string;
  kind?: string;
  policy_number?: string;
  phone?: string;
}

declare module './types' {
  interface Person {
    /** Step 6's answers about this person (contract v3); omitted when empty. */
    profile?: PersonProfile;
  }
  interface Contact {
    /** Contract v3: an optional address (200 characters). */
    address?: string;
  }
  interface FamilyPlan {
    home?: HomeInfo;
    neighbourhood?: Neighbourhood;
    /** At most 8 (`PETS_MAX`). */
    pets?: PetInfo[];
    /** At most 4 (`VEHICLES_MAX`). */
    vehicles?: VehicleInfo[];
    documents?: DocumentsInfo;
  }
}

/**
 * The Prepare sheet's Markdown (contract v3 `PlanOutput.prepare_markdown`, DESIGN-DELTA-v3 §4).
 * Until types3 lands, the v2 engine sends the whole packet instead, which is what types3's
 * transitional `prepare_markdown` holds anyway.
 */
export function prepareMarkdownOf(output: PlanOutput): string {
  const v3 = (output as PlanOutput & { prepare_markdown?: string }).prepare_markdown;
  return v3 ?? output.packet_markdown;
}
