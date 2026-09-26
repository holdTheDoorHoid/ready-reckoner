/**
 * Validation for the mock engine, mirroring `PlanInput::validate` in rr-types (ENGINE-API.md,
 * "Validation"): a structural pass (types, enums, unknown fields) reported as `schema` problems,
 * then the semantic checks in a fixed order. Messages are plain language, fit to show beside the
 * field the `field` path names.
 *
 * Contract v2: every new input is optional (absent means "not asked", or its default), and the one
 * new check is `rare_opt_in`, whose entries must be family ids or `all` (`unknown_id`). Nothing in
 * the family plan is ever a problem beyond its types: it is only tidied (`tidyFamilyPlan`).
 */
import type { PlanInput, Problem, ProblemCode } from '../types';
import {
  ACCESS_NEEDS,
  AGE_BANDS,
  BACKUP_POWER_KINDS,
  BENEFITS,
  CLIMATE_HORIZONS,
  COMMUTE_MODES,
  COOKING_FUELS,
  COOLING_KINDS,
  FUELS,
  HEATING_KINDS,
  HOLDS,
  HOUSING_KINDS,
  INCOME_STABILITIES,
  MOBILITY_LEVELS,
  RARE_HAZARD_IDS,
  RAW_WATER_SOURCES,
  RETURN_PERIODS,
  SETTINGS,
  SIMPLE_POWERED_DEVICES,
  STAGES,
  TENURES,
  WASTEWATER_KINDS,
  WATER_LEVELS,
  WATER_SOURCES,
  WATER_SYSTEM_RECORDS,
} from '../types';

type Schema =
  | { t: 'string' }
  | { t: 'date' }
  | { t: 'number'; int?: 'u8' | 'i8' }
  | { t: 'bool' }
  | { t: 'enum'; values: readonly string[] }
  | { t: 'array'; of: Schema }
  | { t: 'object'; fields: Record<string, Schema>; optional?: readonly string[] }
  | { t: 'device' };

const str: Schema = { t: 'string' };
const num: Schema = { t: 'number' };
const u8: Schema = { t: 'number', int: 'u8' };
const bool: Schema = { t: 'bool' };
const en = (values: readonly string[]): Schema => ({ t: 'enum', values });
const obj = (fields: Record<string, Schema>, optional: readonly string[] = []): Schema => ({ t: 'object', fields, optional });
const list = (of: Schema): Schema => ({ t: 'array', of });
/** An object whose every field may be left out (the family plan and its parts). */
const loose = (fields: Record<string, Schema>): Schema => obj(fields, Object.keys(fields));

const CONTACT: Schema = loose({ name: str, phone: str });

/** `FamilyPlan` (contract v2): free text only, every field optional. */
const FAMILY_PLAN: Schema = loose({
  meeting_place_near: str,
  meeting_place_far: str,
  out_of_area_contact: CONTACT,
  school_pickup: str,
  work_plans: str,
  shelter_spot_home: str,
  shelter_spot_work: str,
  where_we_would_go: str,
  routes: list(str),
  neighbours_who_check: str,
  who_takes_animals: str,
  shutoff_gas: str,
  shutoff_water: str,
  shutoff_electric: str,
  trusted_circle: list(loose({ name: str, phone: str, holds: list(en(HOLDS)) })),
  lawyer: CONTACT,
  roadside_assistance: str,
  numbers_by_heart: list(str),
});

const PLAN_INPUT: Schema = obj(
  {
    planning_date: { t: 'date' },
    location: obj({ country: str, zip: str, county_fips: str, setting: en(SETTINGS) }, ['zip', 'county_fips']),
    housing: obj({
      kind: en(HOUSING_KINDS),
      tenure: en(TENURES),
      floor: { t: 'number', int: 'i8' },
      basement: bool,
      water: en(WATER_SOURCES),
      sewer: en(WASTEWATER_KINDS),
      heating: en(HEATING_KINDS),
      cooling: en(COOLING_KINDS),
      backup_power: en(BACKUP_POWER_KINDS),
      alarms: obj({ smoke: bool, co: bool, extinguisher: bool }),
      below_grade_bedroom: bool,
      cooking: en(COOKING_FUELS),
      raw_water_source: en(RAW_WATER_SOURCES),
      water_system_record: en(WATER_SYSTEM_RECORDS),
    }, ['below_grade_bedroom', 'cooking', 'raw_water_source', 'water_system_record']),
    people: {
      t: 'array',
      of: obj(
        {
          age_band: en(AGE_BANDS),
          pregnant_or_nursing: bool,
          medical: obj({
            daily_rx: bool,
            refrigerated_rx: bool,
            powered_device: { t: 'device' },
            mobility: en(MOBILITY_LEVELS),
            dietary: { t: 'array', of: str },
            epinephrine: bool,
          }),
          earner: bool,
          commute: obj({ distance_km: num, mode: en(COMMUTE_MODES), remote_possible: bool }),
          access_needs: list(en(ACCESS_NEEDS)),
        },
        ['commute', 'access_needs'],
      ),
    },
    pets: obj({ dogs: u8, cats: u8, small: u8, large_animals: u8 }),
    mobility: obj({ vehicles: { t: 'array', of: obj({ fuel: en(FUELS) }) } }),
    finances: obj(
      {
        monthly_budget_usd: num,
        one_off_budget_usd: num,
        emergency_fund_months: num,
        monthly_expenses_usd: num,
        income: obj({ earners: u8, stability: en(INCOME_STABILITIES) }),
        insurance: obj(
          { home_or_renters: bool, flood: bool, earthquake: bool, sewer_backup: bool, life_or_disability: bool },
          ['sewer_backup', 'life_or_disability'],
        ),
        benefits: list(en(BENEFITS)),
      },
      ['monthly_expenses_usd', 'benefits'],
    ),
    existing: { t: 'array', of: obj({ item_id: str, qty: num, paid_usd: num, tested_on: { t: 'date' } }, ['paid_usd', 'tested_on']) },
    assume_basics: bool,
    dials: obj(
      {
        return_period: en(RETURN_PERIODS),
        climate: en(CLIMATE_HORIZONS),
        horizon_years: u8,
        water_level: en(WATER_LEVELS),
        scenario_overrides: { t: 'array', of: obj({ id: str, on: bool }) },
        rare_catastrophic_opt_in: bool,
        rare_opt_in: list(str),
        minimum_kit: bool,
        long_horizon: bool,
      },
      ['water_level', 'scenario_overrides', 'rare_catastrophic_opt_in', 'rare_opt_in', 'minimum_kit', 'long_horizon'],
    ),
    stage: en(STAGES),
    confidence_1to5: u8,
    family_plan: FAMILY_PLAN,
  },
  ['stage', 'confidence_1to5', 'assume_basics', 'family_plan'],
);

function problem(code: ProblemCode, field: string, message: string): Problem {
  return { code, field, message };
}

function describe(value: unknown): string {
  if (Array.isArray(value)) return 'a list';
  if (value === null) return 'empty';
  return typeof value === 'object' ? 'a group of fields' : `"${String(value)}"`;
}

function walk(value: unknown, schema: Schema, path: string, out: Problem[]): void {
  switch (schema.t) {
    case 'string':
      if (typeof value !== 'string') out.push(problem('schema', path, 'This should be text.'));
      return;
    case 'date':
      if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value) || Number.isNaN(Date.parse(value))) {
        out.push(problem('schema', path, 'This should be a date written YYYY-MM-DD.'));
      }
      return;
    case 'bool':
      if (typeof value !== 'boolean') out.push(problem('schema', path, 'This should be yes or no.'));
      return;
    case 'number':
      if (typeof value !== 'number') {
        out.push(problem('schema', path, 'This should be a number.'));
      } else if (schema.int && Number.isFinite(value)) {
        const [lo, hi] = schema.int === 'u8' ? [0, 255] : [-128, 127];
        if (!Number.isInteger(value) || value < lo || value > hi) {
          out.push(problem('schema', path, `This should be a whole number from ${lo} to ${hi}.`));
        }
      }
      return;
    case 'enum':
      if (typeof value !== 'string' || !schema.values.includes(value)) {
        out.push(problem('schema', path, `${describe(value)} is not one of the choices.`));
      }
      return;
    case 'device':
      if (typeof value === 'string') {
        if (!(SIMPLE_POWERED_DEVICES as readonly string[]).includes(value)) {
          out.push(problem('schema', path, `${describe(value)} is not one of the choices.`));
        }
      } else if (
        typeof value !== 'object' ||
        value === null ||
        Object.keys(value).length !== 1 ||
        !('other' in value) ||
        typeof value.other !== 'object' ||
        value.other === null ||
        Object.keys(value.other).join() !== 'watts' ||
        typeof (value.other as { watts: unknown }).watts !== 'number'
      ) {
        out.push(problem('schema', path, 'A device is "none", "cpap", "oxygen", or another device with its watts.'));
      }
      return;
    case 'array':
      if (!Array.isArray(value)) {
        out.push(problem('schema', path, 'This should be a list.'));
        return;
      }
      value.forEach((v, i) => walk(v, schema.of, `${path}[${i}]`, out));
      return;
    case 'object': {
      if (typeof value !== 'object' || value === null || Array.isArray(value)) {
        out.push(problem('schema', path, 'This should be a group of fields.'));
        return;
      }
      const record = value as Record<string, unknown>;
      const optional = schema.optional ?? [];
      for (const [key, sub] of Object.entries(schema.fields)) {
        const sp = path ? `${path}.${key}` : key;
        if (!(key in record) || record[key] === undefined) {
          if (!optional.includes(key)) out.push(problem('schema', sp, 'This field is missing.'));
        } else {
          walk(record[key], sub, sp, out);
        }
      }
      for (const key of Object.keys(record)) {
        if (!(key in schema.fields)) {
          out.push(problem('schema', path ? `${path}.${key}` : key, 'This field is not part of a plan.'));
        }
      }
      return;
    }
  }
}

const SNAKE = /^[a-z][a-z0-9]*(_[a-z0-9]+)*$/;

/** Every problem with the input, schema problems first; empty when the input is valid. */
export function validatePlanInput(input: unknown): Problem[] {
  const out: Problem[] = [];
  walk(input, PLAN_INPUT, '', out);
  if (out.length > 0) return out;
  const p = input as PlanInput;

  const finite = (value: number, field: string, allowNegative = false): void => {
    if (!Number.isFinite(value)) out.push(problem('not_finite', field, 'Enter a number.'));
    else if (!allowNegative && value < 0) out.push(problem('negative_value', field, "This can't be less than zero."));
  };

  if (p.location.country !== 'US') {
    out.push(problem('unsupported_country', 'location.country', 'Only places in the United States are supported so far.'));
  }
  if (p.location.zip === undefined && p.location.county_fips === undefined) {
    out.push(problem('location_missing', 'location', 'Enter a ZIP code or choose a county.'));
  }
  if (p.location.zip !== undefined && !/^\d{5}$/.test(p.location.zip)) {
    out.push(problem('zip_format', 'location.zip', 'A ZIP code is five digits, like 19147.'));
  }
  if (p.location.county_fips !== undefined && !/^\d{5}$/.test(p.location.county_fips)) {
    out.push(problem('county_fips_format', 'location.county_fips', 'A county code is five digits, like 42101.'));
  }
  if (p.people.length === 0) out.push(problem('no_people', 'people', 'Add at least one person.'));
  p.people.forEach((person, i) => {
    const device = person.medical.powered_device;
    if (typeof device === 'object') {
      const field = `people[${i}].medical.powered_device.other.watts`;
      if (!Number.isFinite(device.other.watts)) out.push(problem('not_finite', field, 'Enter a number.'));
      else if (device.other.watts <= 0) {
        out.push(problem('out_of_range', field, "Enter the device's power in watts, more than 0. It is on the label or charger."));
      }
    }
    if (person.commute) finite(person.commute.distance_km, `people[${i}].commute.distance_km`);
  });
  finite(p.finances.monthly_budget_usd, 'finances.monthly_budget_usd');
  finite(p.finances.one_off_budget_usd, 'finances.one_off_budget_usd');
  finite(p.finances.emergency_fund_months, 'finances.emergency_fund_months');
  if (p.finances.monthly_expenses_usd !== undefined) {
    finite(p.finances.monthly_expenses_usd, 'finances.monthly_expenses_usd');
  }
  p.existing.forEach((owned, i) => {
    if (!SNAKE.test(owned.item_id)) {
      out.push(problem('id_format', `existing[${i}].item_id`, 'Item ids use lowercase letters, numbers and underscores.'));
    }
    finite(owned.qty, `existing[${i}].qty`);
    if (owned.paid_usd !== undefined) finite(owned.paid_usd, `existing[${i}].paid_usd`);
  });
  if (p.dials.horizon_years < 1 || p.dials.horizon_years > 50) {
    out.push(problem('out_of_range', 'dials.horizon_years', 'Choose between 1 and 50 years.'));
  }
  if (p.confidence_1to5 !== undefined && (p.confidence_1to5 < 1 || p.confidence_1to5 > 5)) {
    out.push(problem('out_of_range', 'confidence_1to5', 'Choose a number from 1 to 5.'));
  }
  const earners = p.people.filter((person) => person.earner).length;
  if (p.finances.income.earners !== earners) {
    out.push(
      problem(
        'earners_mismatch',
        'finances.income.earners',
        `The plan says ${p.finances.income.earners} ${p.finances.income.earners === 1 ? 'person earns' : 'people earn'}, but ${earners} ${earners === 1 ? 'person is' : 'people are'} marked as earning.`,
      ),
    );
  }
  const seen = new Set<string>();
  (p.dials.scenario_overrides ?? []).forEach((toggle, i) => {
    if (!SNAKE.test(toggle.id)) {
      out.push(problem('id_format', `dials.scenario_overrides[${i}].id`, 'Scenario ids use lowercase letters, numbers and underscores.'));
    } else if (seen.has(toggle.id)) {
      out.push(problem('duplicate_id', `dials.scenario_overrides[${i}].id`, 'This scenario is listed twice.'));
    }
    seen.add(toggle.id);
  });
  (p.dials.rare_opt_in ?? []).forEach((family, i) => {
    const field = `dials.rare_opt_in[${i}]`;
    if (!SNAKE.test(family)) {
      out.push(problem('id_format', field, `"${family}" is not a family code. Family codes use lowercase letters, numbers and underscores, like nuclear_attack.`));
    } else if (family !== 'all' && !(RARE_HAZARD_IDS as readonly string[]).includes(family)) {
      out.push(problem('unknown_id', field, `"${family}" is not a rare-event family. Use ${RARE_HAZARD_IDS.join(', ')} or all.`));
    }
  });
  return out;
}
