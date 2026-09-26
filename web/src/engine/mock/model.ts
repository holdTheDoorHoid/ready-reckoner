/**
 * The mock engine's model: household facts -> hazard register -> bucket targets and coverage ->
 * requirement lines -> a month-by-month plan -> guardrail warnings. Simple deterministic rules,
 * chosen so the interface behaves like the real engine will:
 *
 * - a more cautious return period never lowers a target (the profile tables rise with the dial and
 *   every household adjustment is monotone);
 * - more people never means less water (water scales with each person);
 * - the plan buys in one fixed priority order against the cumulative budget, so more budget never
 *   buys later and spending never passes the money available by any month;
 * - a $0 budget gives free actions only; every item cites `mock_*` sources.
 */
import type {
  AgeBand,
  BucketAssessment,
  BucketId,
  Citation,
  Contribution,
  HazardId,
  HazardProfile,
  HazardTier,
  Item,
  LocationResolved,
  PlanInput,
  PlanItem,
  PlanItemKind,
  PlanMonth,
  PlanOutput,
  Relief,
  RequirementLine,
  SavingsEnvelope,
  SavingsTrack,
  ScenarioInfo,
  Target,
  TargetDays,
  TargetMonths,
  TierId,
  Warning,
} from '../types';
import { BUCKET_IDS, HAZARD_IDS, RETURN_PERIODS, TARGET_LADDER_DAYS, TIER_IDS } from '../types';
import { chanceWithin, dayPhrase, frequencySentence, monthsPhrase } from '../../lib/format';
import { citation } from './citations';
import { catalogueItem } from './items';
import { BUCKETS, hazardName } from './names';
import { buildPacket } from './packet';
import type { ByDial, DurationBucket, EvacuateSeed, HazardSeed, RegionProfile, ScenarioSeed } from './regions';
import { DURATION_BUCKETS, PROFILES, tierForDays } from './regions';

export const MOCK_ENGINE_VERSION = 'mock-0.1.0';
export const MOCK_CONTENT_VERSION = 'mock-content-2026-09-25';
export const MOCK_DATA_VERSION = 'mock-data-2026-09-25';

// ---------------------------------------------------------------------------------------------
// The day ladder
// ---------------------------------------------------------------------------------------------

const LADDER: readonly number[] = TARGET_LADDER_DAYS;

function ladderIndex(d: number): number {
  const i = LADDER.indexOf(d);
  if (i >= 0) return i;
  // Values off the ladder snap to the nearest rung at or below.
  let best = 0;
  LADDER.forEach((v, j) => {
    if (v <= d) best = j;
  });
  return best;
}

export function stepLadder(d: number, k: number): number {
  const i = Math.min(LADDER.length - 1, Math.max(0, ladderIndex(d) + k));
  return LADDER[i]!;
}

/** The largest rung at or below `x`; 0 below half a day. */
export function floorLadder(x: number): number {
  let out = 0;
  for (const v of LADDER) if (v <= x + 1e-9) out = v;
  return out;
}

/** The smallest rung at or above `x` (at most a year). */
export function ceilLadder(x: number): number {
  for (const v of LADDER) if (v >= x - 1e-9) return v;
  return LADDER[LADDER.length - 1]!;
}

function sig(x: number, digits = 4): number {
  return x === 0 ? 0 : Number(x.toPrecision(digits));
}

function cents(x: number): number {
  return Math.round(x * 100) / 100;
}

function roundHalf(x: number): number {
  return Math.round(x * 2) / 2;
}

// ---------------------------------------------------------------------------------------------
// Household facts
// ---------------------------------------------------------------------------------------------

const KCAL: Record<AgeBand, number> = { infant: 0, toddler: 1100, child: 1600, teen: 2400, adult: 2100, senior: 1900 };
const WATER_GAL = { survival: 0.8, basic: 1, comfortable: 4 } as const;
// very_stable is treated like stable here (per brief); the real ×0.5 factor lives in
// crates/rr-hazards/src/params.rs::income_stability.
const STABILITY = { very_stable: 0.7, stable: 0.7, variable: 1, seasonal: 1.4, gig: 1.5 } as const;
const DEVICE_WATTS = { cpap: 50, oxygen: 300 } as const;

export interface Facts {
  n: number;
  seniors: number;
  infants: number;
  children: number;
  vulnerable: boolean;
  kcalPerDay: number;
  /** Adults' worth of food per day (kcal / 2,100). */
  peopleEquiv: number;
  galPerDay: number;
  dailyRx: number;
  fridgeRx: boolean;
  deviceWatts: number[];
  earners: number;
  stabilityFactor: number;
  commuters: number;
  maxCommuteKm: number;
  pets: number;
  large: number;
  vehicles: number;
  fuelVehicle: boolean;
  ev: boolean;
  owner: boolean;
  /** Has its own tank water heater and room for drums. */
  house: boolean;
  attached: boolean;
  apartment: boolean;
  mobileHome: boolean;
  basement: boolean;
  floor: number;
  highRiseCoupled: boolean;
  well: boolean;
  woodHeat: boolean;
  gasHeat: boolean;
  heatNeedsPower: boolean;
  setting: PlanInput['location']['setting'];
  backup: PlanInput['housing']['backup_power'];
  alarms: PlanInput['housing']['alarms'];
  hot: boolean;
  cold: boolean;
  heatDriven: boolean;
}

export function householdFacts(input: PlanInput, profile: RegionProfile): Facts {
  const people = input.people;
  const count = (band: AgeBand) => people.filter((p) => p.age_band === band).length;
  const level = input.dials.water_level ?? 'basic';
  let gal = 0;
  let kcal = 0;
  for (const p of people) {
    gal += WATER_GAL[level] * (p.age_band === 'infant' ? 1.5 : 1) * (p.pregnant_or_nursing ? 1.25 : 1);
    kcal += KCAL[p.age_band] + (p.pregnant_or_nursing ? 400 : 0);
  }
  if (profile.hot) gal *= 1.5;
  gal += input.pets.dogs * 0.5 + input.pets.cats * 0.1 + input.pets.small * 0.05;
  const deviceWatts = people.flatMap((p) => {
    const d = p.medical.powered_device;
    if (d === 'none') return [];
    return [typeof d === 'object' ? d.other.watts : DEVICE_WATTS[d]];
  });
  const commutes = people.flatMap((p) => (p.commute && p.commute.distance_km >= 5 ? [p.commute.distance_km] : []));
  const h = input.housing;
  const apartment = h.kind === 'apartment_high_rise' || h.kind === 'apartment_low_rise';
  const seniors = count('senior');
  const infants = count('infant');
  return {
    n: people.length,
    seniors,
    infants,
    children: count('toddler') + count('child') + count('teen'),
    vulnerable:
      seniors > 0 ||
      infants > 0 ||
      deviceWatts.length > 0 ||
      people.some((p) => p.medical.mobility !== 'none' || p.pregnant_or_nursing),
    kcalPerDay: kcal,
    peopleEquiv: Math.round((kcal / 2100) * 100) / 100,
    galPerDay: Math.round(gal * 100) / 100,
    dailyRx: people.filter((p) => p.medical.daily_rx).length,
    fridgeRx: people.some((p) => p.medical.refrigerated_rx),
    deviceWatts,
    earners: people.filter((p) => p.earner).length,
    stabilityFactor: STABILITY[input.finances.income.stability],
    commuters: commutes.length,
    maxCommuteKm: commutes.length ? Math.max(...commutes) : 0,
    pets: input.pets.dogs + input.pets.cats + input.pets.small,
    large: input.pets.large_animals,
    vehicles: input.mobility.vehicles.length,
    fuelVehicle: input.mobility.vehicles.some((v) => v.fuel !== 'ev'),
    ev: input.mobility.vehicles.some((v) => v.fuel === 'ev'),
    owner: h.tenure === 'own',
    house: !apartment,
    attached: apartment || h.kind === 'rowhouse',
    apartment,
    mobileHome: h.kind === 'mobile_home',
    basement: h.basement,
    floor: h.floor,
    highRiseCoupled: h.kind === 'apartment_high_rise' && h.floor >= 7,
    well: h.water === 'well',
    woodHeat: h.heating === 'wood',
    gasHeat: h.heating === 'gas' || h.heating === 'propane' || h.heating === 'oil',
    heatNeedsPower: h.heating !== 'wood' && h.heating !== 'none',
    setting: input.location.setting,
    backup: h.backup_power,
    alarms: h.alarms,
    hot: profile.hot,
    cold: profile.cold,
    heatDriven: profile.heat_driven,
  };
}

// ---------------------------------------------------------------------------------------------
// Hazard register
// ---------------------------------------------------------------------------------------------

function hazardTier(id: HazardId): HazardTier {
  const i = HAZARD_IDS.indexOf(id);
  return i < 18 ? 'natural' : i < 27 ? 'societal' : 'personal';
}

function commonSeeds(f: Facts, loc: LocationResolved): HazardSeed[] {
  const urban = f.setting === 'urban';
  const seeds: HazardSeed[] = [
    { id: 'pandemic', rate: 0.02, spread: 2, severity: 0.55, confidence: 'prior', sources: ['mock_pandemic_stayhome', 'mock_societal_prior'], buckets: ['supplies', 'income', 'medication'], what: 'go through a pandemic with weeks of staying home' },
    { id: 'grid_failure', rate: 0.004, spread: 3, severity: 0.6, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['power', 'water_out', 'comms', 'thermal'], what: 'lose power for days in a regional grid failure' },
    { id: 'cyber_outage', rate: 0.03, spread: 3, severity: 0.25, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['comms', 'medication'], what: 'lose card payments or pharmacy systems for a day or more' },
    { id: 'civil_unrest', rate: urban ? 0.01 : 0.003, spread: 3, severity: 0.3, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['supplies', 'security'], what: 'have unrest nearby that closes stores or roads' },
    { id: 'supply_chain_disruption', rate: 0.08, spread: 2, severity: 0.2, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['supplies'], what: 'find store shelves empty for days' },
    { id: 'house_fire', rate: 0.004 * (f.attached ? 1.5 : 1) * (f.mobileHome ? 1.6 : 1) * (f.woodHeat ? 1.3 : 1), spread: 1.5, severity: 0.8, confidence: 'medium', sources: ['mock_usfa_fires'], buckets: ['fire', 'home_loss', 'evacuate'], what: 'have a fire at home, or next door, that forces them out', evac: 1 },
    { id: 'medical_emergency', rate: 0.04 * f.n + 0.06 * f.seniors, spread: 1.4, severity: 0.45, confidence: 'medium', sources: ['mock_ed_visits'], buckets: ['medical_emergency', 'income'], what: 'need emergency medical care' },
    { id: 'vehicle_stranding', rate: f.commuters > 0 ? 0.04 * f.commuters * (f.setting === 'rural' ? 1.5 : 1) : 0.01, spread: 2, severity: 0.2, confidence: 'low', sources: ['mock_crash_injury'], buckets: ['get_home'], what: 'be stranded away from home' },
    { id: 'local_utility_outage', rate: f.well ? 0.2 : 0.12, spread: 1.8, severity: 0.25, confidence: 'medium', sources: ['mock_boil_notices'], buckets: ['water_boil', 'water_out'], what: f.well ? 'have the well pump fail or the water go bad' : 'have a boil-water notice or a water main break' },
    { id: 'burglary', rate: urban ? 0.02 : f.setting === 'suburban' ? 0.012 : 0.008, spread: 1.5, severity: 0.2, confidence: 'high', sources: ['mock_burglary'], buckets: ['security'], what: 'have a burglary' },
    { id: 'extended_household_illness', rate: 0.015 * f.n, spread: 2, severity: 0.4, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['income', 'supplies', 'medication'], what: 'have someone ill for weeks' },
    { id: 'nuclear_attack', rate: 0.0003, spread: 5, severity: 1, confidence: 'prior', sources: ['mock_rare_prior', 'mock_nuclear_guidance'], buckets: ['supplies', 'power', 'water_out', 'comms'], what: 'be affected by a nuclear attack or EMP', rare: true },
    { id: 'terrorism', rate: urban ? 0.0005 : 0.0001, spread: 5, severity: 0.6, confidence: 'prior', sources: ['mock_rare_prior'], buckets: ['security', 'medical_emergency'], what: 'be caught up in a terrorist attack', rare: true },
  ];
  if (f.earners > 0) {
    seeds.push(
      { id: 'job_loss', rate: 0.083 * f.earners * f.stabilityFactor, spread: 1.4, severity: 0.6, confidence: 'medium', sources: ['mock_bls_job_loss', 'mock_ui_benefits'], buckets: ['income'], what: 'have an earner lose a job' },
      { id: 'earner_death_or_disability', rate: 0.004 * f.earners, spread: 1.6, severity: 0.85, confidence: 'medium', sources: ['mock_bls_job_loss'], buckets: ['income'], what: "lose an earner's income to death or long disability" },
    );
  }
  const hazmat = loc.facility_flags.hazmat_facilities_within_5km;
  if (hazmat > 0) {
    seeds.push({ id: 'hazmat_release', rate: 0.002 * Math.sqrt(hazmat), spread: 3, severity: 0.35, confidence: 'low', sources: ['mock_societal_prior'], buckets: ['evacuate', 'supplies'], what: 'be told to shelter indoors or leave because of a chemical release', evac: 0.5 });
  }
  if (loc.facility_flags.nuclear_plant_within_80km) {
    seeds.push({ id: 'nuclear_plant_incident', rate: loc.facility_flags.nuclear_plant_within_16km ? 0.0002 : 0.00005, spread: 5, severity: 0.8, confidence: 'prior', sources: ['mock_rare_prior', 'mock_nuclear_guidance'], buckets: ['evacuate', 'supplies'], what: 'be affected by a nuclear plant accident', rare: true, evac: 1 });
  }
  return seeds;
}

function householdModifier(seed: HazardSeed, f: Facts): number {
  let m = 1;
  if ((seed.id === 'tornado' || seed.id === 'strong_wind') && f.mobileHome) m *= 1.8;
  if (seed.id === 'riverine_flooding' || seed.id === 'coastal_flooding') {
    if (f.apartment && f.floor >= 2) m *= 0.5;
    if (f.basement) m *= 1.3;
  }
  if ((seed.id === 'winter_weather' || seed.id === 'strong_wind' || seed.id === 'ice_storm') && f.setting === 'rural') m *= 1.2;
  if (seed.id === 'drought' && !f.well) m *= 0.5;
  return m;
}

export interface RankedHazard {
  profile: HazardProfile;
  seed: HazardSeed;
  rate: number;
}

function buildRegister(input: PlanInput, f: Facts, profile: RegionProfile, loc: LocationResolved): RankedHazard[] {
  const years = input.dials.horizon_years;
  const out: RankedHazard[] = [];
  for (const seed of [...profile.hazards, ...commonSeeds(f, loc)]) {
    const climate = input.dials.climate === 'y2050' ? (seed.y2050 ?? 1) : 1;
    const rate = seed.rate * householdModifier(seed, f) * climate;
    if (!(rate > 0)) continue;
    const lo = rate / seed.spread;
    const hi = rate * seed.spread;
    const hp: HazardProfile = {
      id: seed.id,
      name: hazardName(seed.id),
      tier: hazardTier(seed.id),
      display: seed.rare ? 'rare_catastrophic' : 'ranked',
      rate_per_year: sig(rate),
      rate_range: [sig(lo), sig(hi)],
      annual_probability: sig(1 - Math.exp(-rate)),
      probability_range: [sig(1 - Math.exp(-lo)), sig(1 - Math.exp(-hi))],
      severity: seed.severity,
      climate_multiplier: climate,
      confidence: seed.confidence,
      sources: seed.sources,
      frequency_sentence: frequencySentence(chanceWithin(rate, years), seed.what, years, chanceWithin(lo, years), chanceWithin(hi, years)),
      buckets: seed.buckets,
    };
    if (seed.eal !== undefined) hp.eal_per_household_usd = seed.eal;
    out.push({ profile: hp, seed, rate });
  }
  // Ten-year chance times severity squared: likely things lead, and a rare event that hits
  // everything at once still ranks near the top instead of below everyday nuisances.
  const importance = (h: RankedHazard) => chanceWithin(h.rate, 10) * h.profile.severity ** 2;
  const ranked = out
    .filter((h) => !h.seed.rare)
    .sort((a, b) => importance(b) - importance(a) || a.seed.id.localeCompare(b.seed.id));
  const rare = out
    .filter((h) => h.seed.rare)
    .sort((a, b) => b.profile.severity - a.profile.severity || a.seed.id.localeCompare(b.seed.id));
  return [...ranked, ...rare];
}

// ---------------------------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------------------------

export interface ScenarioState {
  seed: ScenarioSeed;
  on: boolean;
}

export function scenarioStates(input: PlanInput, profile: RegionProfile): ScenarioState[] {
  const overrides = new Map((input.dials.scenario_overrides ?? []).map((t) => [t.id, t.on]));
  return profile.scenarios.map((seed) => ({ seed, on: overrides.get(seed.id) ?? seed.default_on }));
}

/** Buckets whose ranges are wider because their durations lean on expert estimates. */
const WIDE = new Set<DurationBucket>(['supplies', 'medication', 'thermal', 'comms']);
const RELIEF_BUCKETS: DurationBucket[] = ['power', 'water_boil', 'water_out', 'supplies', 'medication', 'comms'];

export interface Targets {
  days: Record<DurationBucket, TargetDays>;
  income: TargetMonths;
  evacuate: EvacuateSeed;
  relief: Partial<Record<DurationBucket, Relief>>;
}

function maxByDial(a: ByDial, b: ByDial): ByDial {
  return [Math.max(a[0], b[0]), Math.max(a[1], b[1]), Math.max(a[2], b[2]), Math.max(a[3], b[3])];
}

export function computeTargets(input: PlanInput, f: Facts, profile: RegionProfile, states: ScenarioState[]): Targets {
  const idx = RETURN_PERIODS.indexOf(input.dials.return_period);
  const raw = {} as Record<DurationBucket, number>;
  for (const b of DURATION_BUCKETS) {
    let arr = profile.targets[b];
    for (const s of states) {
      const w = s.seed.with[b];
      if (s.on && w) arr = maxByDial(arr, w);
    }
    raw[b] = arr[idx]!;
  }
  if (f.well || f.highRiseCoupled) raw.water_out = Math.max(raw.water_out, raw.power);
  if (input.dials.climate === 'y2050' && profile.heat_driven) raw.thermal = stepLadder(raw.thermal, 1);
  if (f.dailyRx === 0 && !f.fridgeRx) raw.medication = Math.min(raw.medication, 3);
  if (f.setting === 'rural') raw.supplies = stepLadder(raw.supplies, 1);

  const days = {} as Record<DurationBucket, TargetDays>;
  for (const b of DURATION_BUCKETS) {
    const v = raw[b];
    days[b] = { kind: 'days', value: v, low: stepLadder(v, -1), high: stepLadder(v, WIDE.has(b) ? 2 : 1) };
  }

  let incomeBase = profile.income;
  let evacuate = profile.evacuate;
  const relief: Partial<Record<DurationBucket, Relief>> = {};
  for (const s of states) {
    if (!s.on) continue;
    if (s.seed.income_with) incomeBase = maxByDial(incomeBase, s.seed.income_with);
    if (s.seed.evacuate_with) evacuate = s.seed.evacuate_with;
  }
  const months = incomeBase[idx]! * f.stabilityFactor * (f.earners > 0 ? 1 : 0.25);
  const iv = Math.max(0.5, roundHalf(months));
  const income: TargetMonths = {
    kind: 'months',
    value: iv,
    low: Math.min(iv, Math.max(0.5, roundHalf(months * 0.6))),
    high: Math.max(iv, roundHalf(months * 1.6)),
  };

  for (const b of RELIEF_BUCKETS) {
    const scenario = states.find((s) => s.on && s.seed.relief_with?.[b]);
    const fixed = scenario?.seed.relief_with?.[b];
    if (scenario && fixed) {
      relief[b] = {
        help_arrives_days: fixed[0],
        mostly_restored_days: fixed[1],
        sources: scenario.seed.relief_sources ?? scenario.seed.sources,
      };
    } else {
      const v = days[b].value;
      relief[b] = {
        help_arrives_days: Math.max(0.5, floorLadder(v * profile.relief.help)),
        mostly_restored_days: Math.max(v, ceilLadder(v * profile.relief.restore)),
        sources: profile.relief.sources,
      };
    }
  }
  return { days, income, evacuate, relief };
}

// ---------------------------------------------------------------------------------------------
// What the household owns
// ---------------------------------------------------------------------------------------------

export interface Owned {
  qty: Map<string, number>;
  paid: Map<string, number>;
}

/**
 * Items the mock credits as already owned when `assume_basics` is on (everyday basics: blankets,
 * a bag per person, a few days of ordinary food). Only ids that exist in the mock catalogue are
 * used; a cooking pot, a can opener and a phone have no matching catalogue item, so they carry no
 * mock credit (the flag itself is still honoured; nothing else about the mock's behaviour changes).
 */
function assumedBasicsCredit(input: PlanInput): [string, number][] {
  const people = input.people.length;
  const bagEligible = input.people.filter((p) => p.age_band !== 'infant' && p.age_band !== 'toddler').length;
  return [
    ['blankets_warm', people],
    ['go_bag', bagEligible],
    ['food_shelf_stable', people * 3],
  ];
}

export function ownedFrom(input: PlanInput): Owned {
  const qty = new Map<string, number>();
  const paid = new Map<string, number>();
  for (const o of input.existing) {
    if (!catalogueItem(o.item_id)) continue;
    qty.set(o.item_id, (qty.get(o.item_id) ?? 0) + o.qty);
    if (o.paid_usd !== undefined) paid.set(o.item_id, (paid.get(o.item_id) ?? 0) + o.paid_usd);
  }
  if (input.assume_basics ?? true) {
    for (const [id, amount] of assumedBasicsCredit(input)) {
      if (!catalogueItem(id) || amount <= 0) continue;
      qty.set(id, Math.max(qty.get(id) ?? 0, amount));
    }
  }
  return { qty, paid };
}

function has(owned: Owned, id: string, atLeast = 1): boolean {
  return (owned.qty.get(id) ?? 0) >= atLeast;
}

function q(owned: Owned, id: string): number {
  return owned.qty.get(id) ?? 0;
}

/** Gallons each water item holds. */
const GALLONS: Record<string, number> = { water_stored: 1, water_containers: 7, water_drum: 55 };
const FREE_BOTTLE_GALLONS = 6;

function heaterGallons(f: Facts): number {
  return f.house ? f.galPerDay : 0;
}

/** Days covered for each duration bucket by what is in `owned`, before snapping to the ladder. */
export function coverage(owned: Owned, f: Facts, t: Targets): Record<DurationBucket, number> {
  let gallons = 0;
  for (const [id, per] of Object.entries(GALLONS)) gallons += q(owned, id) * per;
  if (has(owned, 'water_reused_bottles')) gallons += FREE_BOTTLE_GALLONS;
  if (has(owned, 'water_boil_method')) gallons += heaterGallons(f);
  const waterDays = f.galPerDay > 0 ? gallons / f.galPerDay : 0;
  const filter = has(owned, 'water_filter') ? (f.setting === 'urban' ? 7 : 30) : 0;

  let supplies = f.peopleEquiv > 0 ? q(owned, 'food_shelf_stable') / f.peopleEquiv : t.days.supplies.value;
  if (f.infants > 0) supplies = Math.min(supplies, q(owned, 'infant_formula_reserve') / f.infants);

  let medication: number;
  if (f.dailyRx > 0) {
    medication = (has(owned, 'medication_refill_early') ? 7 : 0) + q(owned, 'medication_reserve');
    if (f.fridgeRx && !has(owned, 'cooler_ice_packs')) medication = Math.min(medication, 1);
  } else if (f.fridgeRx) {
    medication = has(owned, 'cooler_ice_packs') ? t.days.medication.value : 0;
  } else {
    medication = has(owned, 'otc_basics') ? t.days.medication.value : 0;
  }

  const lights = has(owned, 'flashlights_headlamps', f.n);
  let power = 0;
  if (lights) power += 1;
  if (lights && has(owned, 'batteries_spare')) power += 1;
  if (has(owned, 'lantern')) power += 1;
  if (has(owned, 'power_bank')) power += 0.5;
  if (has(owned, 'power_station')) power += 3;
  if (has(owned, 'generator_portable')) power += 5;
  power += { none: 0, power_station: 2, solar_battery: 3, generator: 5 }[f.backup];
  const deviceCovered =
    has(owned, 'device_battery_backup') || f.backup !== 'none' || has(owned, 'power_station') || has(owned, 'generator_portable');
  if (f.deviceWatts.length > 0 && !deviceCovered) power = 0;

  let thermal = 0;
  if (has(owned, 'temperature_plan')) thermal += 1;
  if (has(owned, 'fans_cooling') && (f.hot || f.heatDriven)) thermal += 2;
  if (has(owned, 'blankets_warm', f.n)) thermal += 3;
  if (f.woodHeat && f.cold) thermal += t.days.thermal.value;

  let comms = 0;
  if (has(owned, 'power_bank')) comms += 2;
  if (has(owned, 'radio_crank')) comms += 3;
  if (has(owned, 'cash_small_bills', 50)) comms += 1;
  if (has(owned, 'alerts_signup')) comms += 0.5;
  if (has(owned, 'plan_family_contacts')) comms += 0.5;

  const waterBoil = has(owned, 'water_bleach') || has(owned, 'water_filter') ? t.days.water_boil.value : waterDays;
  return {
    power,
    water_boil: waterBoil,
    water_out: waterDays + filter,
    supplies,
    thermal,
    medication,
    comms,
  };
}

// ---------------------------------------------------------------------------------------------
// Which items apply, and how the plan is sized
// ---------------------------------------------------------------------------------------------

interface Ctx {
  input: PlanInput;
  f: Facts;
  profile: RegionProfile;
  register: RankedHazard[];
  targets: Targets;
  owned: Owned;
}

function rateOf(ctx: Ctx, id: HazardId): number {
  return ctx.register.find((h) => h.seed.id === id)?.rate ?? 0;
}

/** Free actions that apply to this household, in plan order. */
function freeActions(ctx: Ctx): string[] {
  const { f } = ctx;
  const ids = [
    'alarms_test',
    'escape_plan',
    'plan_family_contacts',
    'documents_copies',
    'alerts_signup',
    'water_reused_bottles',
  ];
  if (f.house) ids.push('water_boil_method');
  if (f.dailyRx > 0) ids.push('medication_refill_early');
  ids.push('medication_list', 'temperature_plan', 'neighbours_numbers');
  if (f.vulnerable || f.n === 1) ids.push('check_in_agreement');
  if (f.fuelVehicle) ids.push('car_half_tank');
  if (f.ev) ids.push('ev_charge_habit');
  if (f.commuters > 0) ids.push('get_home_route');
  ids.push('go_stay_card');
  if (rateOf(ctx, 'tsunami') > 0) ids.push('tsunami_route');
  if (rateOf(ctx, 'tornado') >= 0.005) ids.push('safe_room_plan');
  if (rateOf(ctx, 'hurricane') >= 0.05) ids.push('hurricane_zone_check');
  if (f.pets > 0) ids.push('pet_plan');
  if (f.large > 0) ids.push('livestock_water_plan');
  ids.push('insurance_check', 'utility_shutoffs', 'community_group', 'firearm_safe_storage');
  return ids;
}

/** Order within a tier: higher first. Dependants' needs and life safety come first regardless. */
const PRIORITY: Record<string, number> = {
  device_battery_backup: 100,
  infant_formula_reserve: 99,
  cooler_ice_packs: 98,
  water_heater_strap: 97,
  smoke_alarm: 96,
  co_alarm: 95,
  medication_reserve: 94,
  water_stored: 92,
  food_shelf_stable: 85,
  flashlights_headlamps: 84,
  water_bleach: 80,
  first_aid_kit: 78,
  power_bank: 76,
  fans_cooling: 74,
  water_containers: 72,
  radio_crank: 70,
  batteries_spare: 66,
  otc_basics: 64,
  pet_food_reserve: 62,
  blankets_warm: 60,
  get_home_bag: 55,
  fire_extinguisher: 52,
  go_bag: 50,
  cash_small_bills: 48,
  water_filter: 46,
  lantern: 45,
  bucket_toilet: 44,
  smoke_air_filter: 42,
  n95_masks: 40,
  gas_shutoff_wrench: 38,
  water_drum: 36,
  document_bag: 35,
  generator_portable: 32,
  power_station: 30,
};

/** A share of the plan: one item, in one tier, with its size and cost. */
interface Chunk {
  item: Item;
  tier: TierId;
  qty: number;
  /** What it adds, in `resource` units, for matching against what is already owned. */
  resource: string;
  amount: number;
  unitCost: number;
  /** Price band for one unit, times `costMultiplier`. */
  bandLow: number;
  bandHigh: number;
  costMultiplier: number;
  priority: number;
  /** Days of the bucket this chunk brings the household to (duration items). */
  level?: number;
  /** Bought alongside an earlier tier because it is cheap and protects a dependant (DESIGN §4.7). */
  promotedTo?: TierId;
  done: boolean;
  paid?: number;
}

function unitCost(ctx: Ctx, item: Item): number {
  const paid = ctx.owned.paid.get(item.id);
  const owned = ctx.owned.qty.get(item.id);
  if (paid !== undefined && owned !== undefined && owned > 0) return paid / owned;
  return (item.price_band_usd.low + item.price_band_usd.high) / 2;
}

function chunk(ctx: Ctx, id: string, tier: TierId, qty: number, opts: { resource?: string; perUnit?: number; costMultiplier?: number; level?: number; priority?: number } = {}): Chunk {
  const item = catalogueItem(id);
  if (!item) throw new Error(`mock catalogue has no item ${id}`);
  const costMultiplier = opts.costMultiplier ?? 1;
  return {
    item,
    tier,
    qty,
    resource: opts.resource ?? id,
    amount: qty * (opts.perUnit ?? 1),
    unitCost: unitCost(ctx, item) * costMultiplier,
    bandLow: item.price_band_usd.low * costMultiplier,
    bandHigh: item.price_band_usd.high * costMultiplier,
    costMultiplier,
    priority: opts.priority ?? PRIORITY[id] ?? 20,
    level: opts.level,
    done: false,
  };
}

const PLAN_TIERS: { id: TierId; cap: number }[] = [
  { id: 'h72', cap: 3 },
  { id: 'w2', cap: 14 },
  { id: 'm1', cap: 30 },
  { id: 'm3', cap: 90 },
  { id: 'm6', cap: 180 },
  { id: 'y1', cap: 365 },
];

function buildChunks(ctx: Ctx): Chunk[] {
  const { f, targets } = ctx;
  const t = targets.days;
  const out: Chunk[] = [];
  const recommended = TIER_IDS.indexOf(tierRecommended(targets));
  const reaches = (tier: TierId) => TIER_IDS.indexOf(tier) <= recommended;

  // Water: bottles in the first three days, containers to two weeks, then a filter and bulk storage.
  let waterPlanned = FREE_BOTTLE_GALLONS + heaterGallons(f);
  let filterPlanned = false;
  let prevCap = 0;
  for (const { id: tier, cap } of PLAN_TIERS) {
    const level = Math.min(t.water_out.value, cap);
    if (level <= prevCap && tier !== 'h72') break;
    prevCap = cap;
    let gap = level * f.galPerDay - waterPlanned;
    if (gap <= 0.05) continue;
    if (tier === 'h72') {
      const qty = Math.ceil(gap);
      out.push(chunk(ctx, 'water_stored', tier, qty, { resource: 'water', perUnit: 1, level }));
      waterPlanned += qty;
    } else if (tier === 'w2') {
      const qty = Math.ceil(gap / 7);
      out.push(chunk(ctx, 'water_containers', tier, qty, { resource: 'water', perUnit: 7, level }));
      waterPlanned += qty * 7;
    } else {
      if (!filterPlanned) {
        out.push(chunk(ctx, 'water_filter', tier, 1, { level }));
        filterPlanned = true;
        waterPlanned += (f.setting === 'urban' ? 7 : 30) * f.galPerDay;
        gap = level * f.galPerDay - waterPlanned;
      }
      if (gap > 0.05) {
        if (f.house) {
          const qty = Math.min(3, Math.ceil(gap / 55));
          out.push(chunk(ctx, 'water_drum', tier, qty, { resource: 'water', perUnit: 55, level }));
          waterPlanned += qty * 55;
        } else {
          const qty = Math.min(6, Math.ceil(gap / 7));
          out.push(chunk(ctx, 'water_containers', tier, qty, { resource: 'water', perUnit: 7, level }));
          waterPlanned += qty * 7;
        }
      }
    }
  }

  // Food, formula, pet food and medicine, tier by tier up to each target.
  const byTier = (target: number, perDay: number, id: string, resource: string, freeDays = 0, costMultiplier = 1) => {
    let planned = freeDays * perDay;
    let prev = 0;
    for (const { id: tier, cap } of PLAN_TIERS) {
      const level = Math.min(target, cap);
      if (level <= prev && tier !== 'h72') break;
      prev = cap;
      const gap = level * perDay - planned;
      if (gap <= 0.05) continue;
      const qty = Math.ceil(gap);
      out.push(chunk(ctx, id, tier, qty, { resource, level, costMultiplier }));
      planned += qty;
    }
  };
  if (f.peopleEquiv > 0) byTier(t.supplies.value, f.peopleEquiv, 'food_shelf_stable', 'food');
  if (f.infants > 0) byTier(t.supplies.value, f.infants, 'infant_formula_reserve', 'formula');
  if (f.pets > 0) byTier(t.supplies.value, f.pets, 'pet_food_reserve', 'pet_food');
  if (f.dailyRx > 0) {
    byTier(t.medication.value, 1, 'medication_reserve', 'medicine', 7, f.dailyRx);
    // The first extra week of a daily medicine is cheap and irreplaceable: buy it with the three-day basics.
    const firstMed = out.find((c) => c.item.id === 'medication_reserve');
    if (firstMed && firstMed.tier === 'w2' && firstMed.unitCost * firstMed.qty <= 25) firstMed.promotedTo = 'h72';
  }

  // Three-day basics.
  if (f.fridgeRx) out.push(chunk(ctx, 'cooler_ice_packs', 'h72', 1));
  if (f.deviceWatts.length > 0 && f.backup === 'none') out.push(chunk(ctx, 'device_battery_backup', 'h72', f.deviceWatts.length));
  if (!f.alarms.smoke) out.push(chunk(ctx, 'smoke_alarm', 'h72', f.house ? (f.basement ? 3 : 2) : 1));
  if (!f.alarms.co) out.push(chunk(ctx, 'co_alarm', 'h72', 1));
  if (rateOf(ctx, 'earthquake') >= 0.005 && f.owner && f.house) out.push(chunk(ctx, 'water_heater_strap', 'h72', 1));
  out.push(chunk(ctx, 'flashlights_headlamps', 'h72', f.n));
  out.push(chunk(ctx, 'batteries_spare', 'h72', 1));
  out.push(chunk(ctx, 'power_bank', 'h72', Math.ceil(f.n / 2)));
  out.push(chunk(ctx, 'water_bleach', 'h72', 1));
  out.push(chunk(ctx, 'radio_crank', 'h72', 1));
  out.push(chunk(ctx, 'first_aid_kit', 'h72', 1));
  out.push(chunk(ctx, 'otc_basics', 'h72', 1));
  if (f.hot || (f.heatDriven && f.vulnerable)) {
    out.push(chunk(ctx, 'fans_cooling', 'h72', Math.ceil(f.n / 2), { priority: f.vulnerable ? 90 : undefined }));
  }
  if (f.commuters > 0) out.push(chunk(ctx, 'get_home_bag', 'h72', f.commuters));
  out.push(chunk(ctx, 'go_bag', 'h72', f.n));

  // Two weeks and beyond.
  if (reaches('w2')) {
    if (t.power.value > 3) out.push(chunk(ctx, 'lantern', 'w2', 1));
    if (f.cold && f.heatNeedsPower && t.thermal.value >= 1) {
      out.push(chunk(ctx, 'blankets_warm', 'w2', f.n, { priority: f.vulnerable ? 80 : undefined }));
    }
    out.push(chunk(ctx, 'cash_small_bills', 'w2', 100 * Math.ceil(f.n / 2)));
    if (!f.alarms.extinguisher) out.push(chunk(ctx, 'fire_extinguisher', 'w2', 1));
    out.push(chunk(ctx, 'document_bag', 'w2', 1));
    out.push(chunk(ctx, 'n95_masks', 'w2', Math.ceil(f.n / 2)));
    if (rateOf(ctx, 'wildfire') >= 0.05) out.push(chunk(ctx, 'smoke_air_filter', 'w2', 1));
    if (t.water_out.value >= 7 || f.well) out.push(chunk(ctx, 'bucket_toilet', 'w2', 1));
    if (rateOf(ctx, 'earthquake') >= 0.005 && f.gasHeat) out.push(chunk(ctx, 'gas_shutoff_wrench', 'w2', 1));
    if (f.well && f.owner && f.backup === 'none' && t.power.value >= 5) out.push(chunk(ctx, 'generator_portable', 'w2', 1));
    else if (t.power.value >= 7 && f.backup === 'none' && f.deviceWatts.length === 0) {
      out.push(chunk(ctx, 'power_station', t.power.value > 14 ? 'm1' : 'w2', 1));
    }
  }

  // Rare catastrophes: $0 by default; opted in, a single item in a late month (real allocator
  // caps this at 10% of the monthly budget, `Dials.rare_catastrophic_opt_in`).
  if (ctx.input.dials.rare_catastrophic_opt_in) {
    out.push(chunk(ctx, 'radiation_meter', 'm3', 1));
  }
  return out;
}

export function tierRecommended(targets: Targets): TierId {
  let best = 1; // h72: the three-day basics are always in the plan
  for (const b of DURATION_BUCKETS) best = Math.max(best, TIER_IDS.indexOf(tierForDays(targets.days[b].value)));
  return TIER_IDS[best]!;
}

/** Match what the household owns against the plan, in plan order, and mark what is done. */
function markDone(ctx: Ctx, chunks: Chunk[]): void {
  const pool = new Map<string, number>();
  for (const [id, qty] of ctx.owned.qty) {
    const item = catalogueItem(id);
    if (!item || item.free) continue;
    const resource = GALLONS[id] !== undefined ? 'water' : resourceOf(id);
    const per = GALLONS[id] ?? 1;
    pool.set(resource, (pool.get(resource) ?? 0) + qty * per);
  }
  for (const c of chunks) {
    const available = pool.get(c.resource) ?? 0;
    if (available <= 0) continue;
    if (available >= c.amount - 1e-9) {
      c.done = true;
      pool.set(c.resource, available - c.amount);
      const paid = ctx.owned.paid.get(c.item.id);
      const ownedQty = ctx.owned.qty.get(c.item.id);
      if (paid !== undefined && ownedQty) c.paid = cents((paid * c.qty) / ownedQty);
    } else {
      // Partly owned: buy only the rest.
      const per = c.amount / c.qty;
      const remaining = c.amount - available;
      c.qty = Math.ceil(remaining / per - 1e-9);
      c.amount = c.qty * per;
      pool.set(c.resource, 0);
    }
  }
}

function resourceOf(id: string): string {
  switch (id) {
    case 'food_shelf_stable':
      return 'food';
    case 'infant_formula_reserve':
      return 'formula';
    case 'pet_food_reserve':
      return 'pet_food';
    case 'medication_reserve':
      return 'medicine';
    default:
      return id;
  }
}

// ---------------------------------------------------------------------------------------------
// Assessment
// ---------------------------------------------------------------------------------------------

export interface ModelResult {
  output: PlanOutput;
  facts: Facts;
  profile: RegionProfile;
  register: RankedHazard[];
  targets: Targets;
  states: ScenarioState[];
  /** When each not-yet-done item is scheduled (month index), keyed `item_id:tier`. */
  schedule: Map<string, number>;
}

export function profileFor(key: string | undefined, nca: string): RegionProfile {
  return PROFILES[key ?? ''] ?? PROFILES[nca] ?? PROFILES.northeast!;
}

function bucketName(id: BucketId): string {
  return BUCKETS.find((b) => b.id === id)!.name;
}

/** Share of household-significant events that last a day or more, per bucket (placeholder). */
const DAY_SHARE: Record<DurationBucket, number> = {
  power: 0.04,
  water_boil: 0.3,
  water_out: 0.2,
  supplies: 0.3,
  thermal: 0.05,
  medication: 0.15,
  comms: 0.05,
};

const BUCKET_PHRASE: Record<DurationBucket, string> = {
  power: 'go without grid power',
  water_boil: 'need to boil or treat tap water',
  water_out: 'have no usable tap water',
  supplies: 'be unable to get to a store',
  thermal: 'face dangerous heat or cold indoors',
  medication: 'have trouble getting medicine',
  comms: 'lose phone, internet or card payments',
};

const READINESS_PHRASE: Partial<Record<BucketId, string>> = {
  evacuate: 'have to leave home quickly at least once',
  get_home: 'be stranded away from home at least once',
  medical_emergency: 'face a medical emergency at home at least once',
  fire: 'have a home fire at least once',
  security: 'have a break-in or unrest nearby at least once',
  home_loss: 'have their home damaged or made unlivable at least once',
};

const BUCKET_SOURCES: Partial<Record<BucketId, string[]>> = {
  power: ['mock_eaglei_outages', 'mock_duration_prior'],
  water_boil: ['mock_boil_notices', 'mock_duration_prior'],
  water_out: ['mock_boil_notices', 'mock_hazus_restoration', 'mock_duration_prior'],
  supplies: ['mock_pandemic_stayhome', 'mock_duration_prior'],
  thermal: ['mock_heat_projections', 'mock_duration_prior'],
  medication: ['mock_medication_reserve', 'mock_duration_prior'],
  comms: ['mock_eaglei_outages', 'mock_duration_prior'],
  evacuate: ['mock_evacuation'],
  get_home: ['mock_get_home'],
  medical_emergency: ['mock_ed_visits'],
  fire: ['mock_usfa_fires', 'mock_fire_safety'],
  security: ['mock_burglary', 'mock_social_capital'],
  income: ['mock_bls_job_loss', 'mock_ui_benefits', 'mock_savings_track'],
  home_loss: ['mock_insurance', 'mock_usfa_fires'],
};

function unique<T>(xs: T[]): T[] {
  return [...new Set(xs)];
}

/** Hazards that feed a bucket, with their shares (largest first, adding up to 1). */
function contributions(register: RankedHazard[], b: BucketId): { list: Contribution[]; rate: number } {
  const feeding = register.filter((h) => !h.seed.rare && h.seed.buckets.includes(b));
  const weights = feeding.map((h) => ({ id: h.seed.id, w: h.rate * h.seed.severity, rate: h.rate }));
  weights.sort((a, b2) => b2.w - a.w || a.id.localeCompare(b2.id));
  const top = weights.slice(0, 5);
  const total = top.reduce((s, x) => s + x.w, 0);
  if (total <= 0) return { list: [], rate: 0 };
  const list: Contribution[] = top.map((x) => ({ hazard: x.id, share: Math.round((x.w / total) * 100) / 100 }));
  const drift = Math.round((1 - list.reduce((s, c) => s + c.share, 0)) * 100) / 100;
  if (list[0]) list[0].share = Math.round((list[0].share + drift) * 100) / 100;
  return { list, rate: feeding.reduce((s, h) => s + h.rate, 0) };
}

/** Shares of events that need each readiness capability. */
function readinessRate(register: RankedHazard[], b: BucketId): number {
  let r = 0;
  for (const h of register) {
    if (h.seed.rare || !h.seed.buckets.includes(b)) continue;
    if (b === 'evacuate') r += h.rate * (h.seed.evac ?? 0.25);
    else if (b === 'home_loss') r += h.rate * (h.seed.id === 'house_fire' ? 1 : 0.2 * h.seed.severity);
    else if (b === 'get_home') r += h.rate * (h.seed.id === 'vehicle_stranding' ? 1 : 0.1);
    else r += h.rate;
  }
  return r;
}

const DIAL_RATE: Record<PlanInput['dials']['return_period'], number> = {
  one_in_10: 0.1,
  one_in_50: 0.02,
  one_in_100: 0.01,
  one_in_500: 0.002,
};

function readinessItems(ctx: Ctx, b: BucketId): { id: string; done: boolean }[] {
  const { f, owned } = ctx;
  const d = (id: string, atLeast = 1) => ({ id, done: has(owned, id, atLeast) });
  switch (b) {
    case 'get_home':
      if (f.commuters === 0) return [d('plan_family_contacts')];
      return [d('get_home_route'), d('get_home_bag', f.commuters), ...(f.fuelVehicle ? [d('car_half_tank')] : f.ev ? [d('ev_charge_habit')] : [])];
    case 'medical_emergency':
      return [d('first_aid_kit'), d('medication_list'), d('otc_basics'), d('community_group')];
    case 'fire':
      return [
        { id: 'smoke_alarm', done: f.alarms.smoke || has(owned, 'smoke_alarm') },
        { id: 'co_alarm', done: f.alarms.co || has(owned, 'co_alarm') },
        { id: 'fire_extinguisher', done: f.alarms.extinguisher || has(owned, 'fire_extinguisher') },
        d('alarms_test'),
        d('escape_plan'),
      ];
    case 'security':
      return [d('neighbours_numbers'), ...(f.vulnerable || f.n === 1 ? [d('check_in_agreement')] : []), d('community_group')];
    case 'home_loss':
      return [d('documents_copies'), d('insurance_check'), d('document_bag'), d('utility_shutoffs')];
    default:
      return [];
  }
}

export function assessModel(input: PlanInput, location: LocationResolved, profileKey: string | undefined): ModelResult {
  const profile = profileFor(profileKey, location.nca_region);
  const facts = householdFacts(input, profile);
  const register = buildRegister(input, facts, profile, location);
  const states = scenarioStates(input, profile);
  const targets = computeTargets(input, facts, profile, states);
  const owned = ownedFrom(input);
  const ctx: Ctx = { input, f: facts, profile, register, targets, owned };
  const years = input.dials.horizon_years;
  const dialRate = DIAL_RATE[input.dials.return_period];

  // Buckets. `covered_today` is what the household has now; `covered` starts the same and moves
  // to the plan's end point once the plan is scheduled (below).
  const cover = coverage(owned, facts, targets);
  const buckets: BucketAssessment[] = BUCKET_IDS.map((id) => {
    const { list, rate } = contributions(register, id);
    const sources = unique([...list.flatMap((c) => register.find((h) => h.seed.id === c.hazard)!.seed.sources), ...(BUCKET_SOURCES[id] ?? [])]);
    let target: Target;
    let covered: Target;
    let sentences: string[];
    let tierEnough: TierId;
    let relief: Relief | undefined;
    if ((DURATION_BUCKETS as string[]).includes(id)) {
      const b = id as DurationBucket;
      target = targets.days[b];
      const c = floorLadder(Math.min(cover[b], target.value));
      covered = { kind: 'days', value: c, low: c, high: c };
      tierEnough = tierForDays(target.value);
      relief = targets.relief[b];
      sentences = [];
      const dayRate = rate * DAY_SHARE[b];
      if (target.value > 1 && dayRate > dialRate) {
        sentences.push(frequencySentence(chanceWithin(dayRate, years), `${BUCKET_PHRASE[b]} for a day or more`, years));
      }
      sentences.push(frequencySentence(chanceWithin(dialRate, years), `${BUCKET_PHRASE[b]} for longer than ${dayPhrase(target.value)}`, years));
    } else if (id === 'income') {
      target = targets.income;
      const have = input.finances.emergency_fund_months;
      covered = { kind: 'months', value: have, low: have, high: have };
      tierEnough = 'm3';
      const gapRate = facts.earners > 0 ? rateOf(ctx, 'job_loss') * 0.15 + rateOf(ctx, 'earner_death_or_disability') : 0.002;
      sentences = [frequencySentence(chanceWithin(gapRate, years), 'have an income gap longer than 3 months', years)];
    } else if (id === 'evacuate') {
      const p = chanceWithin(readinessRate(register, id), 10);
      const ev = targets.evacuate;
      target = { kind: 'evacuate', p_need_10yr: sig(p), notice_hours_low: ev.notice_hours[0], notice_hours_high: ev.notice_hours[1], days_away: ev.days_away };
      covered = { ...target, days_away: has(owned, 'go_bag') ? ev.days_away : 0 };
      tierEnough = 'h72';
      sentences = [frequencySentence(p, READINESS_PHRASE.evacuate!, 10)];
    } else {
      const p = chanceWithin(readinessRate(register, id), 10);
      const items = readinessItems(ctx, id);
      const done = items.filter((i) => i.done).length;
      target = { kind: 'readiness', p_need_10yr: sig(p), done, of: items.length };
      covered = { ...target };
      tierEnough = items.every((i) => catalogueItem(i.id)?.free) ? 'now' : 'h72';
      sentences = [frequencySentence(p, READINESS_PHRASE[id] ?? 'need this at least once', 10)];
    }
    const out: BucketAssessment = {
      id,
      name: bucketName(id),
      target,
      covered,
      covered_today: { ...covered },
      tier_enough: tierEnough,
      contributions: list,
      frequency_sentences: sentences,
      sources,
    };
    if (relief) out.relief = relief;
    return out;
  });

  // Requirements
  const requirements = buildRequirements(ctx);

  // Plan
  const chunks = buildChunks(ctx);
  markDone(ctx, chunks);
  const { plan, schedule, unscheduled } = schedulePlan(ctx, chunks, buckets);
  planEndCoverage(ctx, chunks, unscheduled, buckets);

  // Tiers reached and recommended
  const recommended = tierRecommended(targets);
  const freeDone = freeActions(ctx).every((id) => has(owned, id));
  let reached: TierId = 'now';
  if (freeDone) {
    for (const tier of TIER_IDS.slice(1)) {
      if (TIER_IDS.indexOf(tier) > TIER_IDS.indexOf(recommended)) break;
      const inTier = chunks.filter((c) => TIER_IDS.indexOf(c.tier) <= TIER_IDS.indexOf(tier));
      if (inTier.every((c) => c.done)) reached = tier;
      else break;
    }
  }

  const scenarios = buildScenarios(input, facts, profile, states, targets);
  const warnings = buildWarnings(ctx, buckets, chunks, schedule, unscheduled, states);

  const provenanceIds = unique([
    ...register.flatMap((h) => h.seed.sources),
    ...buckets.flatMap((b) => [...b.sources, ...(b.relief?.sources ?? [])]),
    ...requirements.flatMap((r) => r.citations),
    ...chunks.flatMap((c) => c.item.citations),
    ...freeActions(ctx).flatMap((id) => catalogueItem(id)!.citations),
    ...scenarios.flatMap((s) => s.sources),
    'mock_savings_track',
  ]).sort();
  const provenance: Citation[] = provenanceIds.map((id) => citation(id)).filter((c): c is Citation => c !== undefined);

  const output: PlanOutput = {
    engine_version: MOCK_ENGINE_VERSION,
    api_version: 1,
    data_pack_version: MOCK_DATA_VERSION,
    content_version: MOCK_CONTENT_VERSION,
    location,
    register: register.map((h) => h.profile),
    buckets,
    scenarios,
    tier_reached: reached,
    tier_recommended: recommended,
    plan,
    requirements,
    warnings,
    packet_markdown: '',
    provenance,
  };
  const result: ModelResult = { output, facts, profile, register, targets, states, schedule };
  output.packet_markdown = buildPacket(input, result);
  return result;
}

/**
 * Moves each bucket's `covered` to where the plan takes the household once every step is done:
 * what it owns, every free step and every purchase the plan schedules (steps it cannot afford
 * within the mock's horizon are left out). `covered_today` stays what the household has now.
 */
function planEndCoverage(ctx: Ctx, chunks: Chunk[], unscheduled: Chunk[], buckets: BucketAssessment[]): void {
  const qty = new Map(ctx.owned.qty);
  const add = (id: string, n: number) => qty.set(id, (qty.get(id) ?? 0) + n);
  for (const id of freeActions(ctx)) if (!has(ctx.owned, id)) add(id, 1);
  for (const c of chunks) if (!c.done && c.qty > 0 && !unscheduled.includes(c)) add(c.item.id, c.qty);
  const end: Owned = { qty, paid: ctx.owned.paid };
  const cover = coverage(end, ctx.f, ctx.targets);
  const endCtx: Ctx = { ...ctx, owned: end };
  for (const b of buckets) {
    const t = b.target;
    const today = b.covered_today;
    if (t.kind === 'days' && today.kind === 'days') {
      const c = Math.max(today.value, floorLadder(Math.min(cover[b.id as DurationBucket], t.value)));
      b.covered = { kind: 'days', value: c, low: c, high: c };
    } else if (t.kind === 'evacuate') {
      b.covered = { ...t, days_away: has(end, 'go_bag') ? t.days_away : 0 };
    } else if (t.kind === 'readiness') {
      const items = readinessItems(endCtx, b.id);
      b.covered = { ...t, done: items.filter((i) => i.done).length, of: items.length };
    }
  }
}

// ---------------------------------------------------------------------------------------------
// Requirements
// ---------------------------------------------------------------------------------------------

function buildRequirements(ctx: Ctx): RequirementLine[] {
  const { f, targets, input } = ctx;
  const t = targets.days;
  const lines: RequirementLine[] = [];
  const add = (line: RequirementLine) => lines.push(line);
  const level = input.dials.water_level ?? 'basic';
  add({
    id: 'water_drinking',
    bucket: 'water_out',
    item_class: 'water_storage',
    quantity: Math.round(t.water_out.value * f.galPerDay * 10) / 10,
    unit: 'gallon',
    per: 'person',
    rule: 'mock_water_gallons',
    citations: ['mock_water_per_person'],
    plain: `${dayPhrase(t.water_out.value)} of water at the ${level} level: ${f.galPerDay} gallons a day for ${f.n} ${f.n === 1 ? 'person' : 'people'}${f.pets > 0 ? ' and pets' : ''}.`,
  });
  if (f.large > 0) {
    add({
      id: 'water_livestock',
      bucket: 'water_out',
      item_class: 'livestock_water_plan',
      quantity: f.large * 10 * Math.min(t.water_out.value, 3),
      unit: 'gallon',
      per: 'pet',
      rule: 'mock_livestock_water',
      citations: ['mock_water_per_person'],
      plain: `Water for ${f.large} large animals, about 10 gallons each a day. Plan a full stock tank or power for the pump rather than bottles.`,
    });
  }
  add({
    id: 'water_treatment',
    bucket: 'water_boil',
    item_class: 'water_treatment',
    quantity: 1,
    unit: 'way to treat water',
    per: 'household',
    rule: 'mock_water_treatment',
    citations: ['mock_ready_gov_kit'],
    plain: 'A way to make tap water safe during a boil-water notice: a working stove and fuel, or plain bleach.',
  });
  if (f.peopleEquiv > 0) {
    add({
      id: 'food_kcal',
      bucket: 'supplies',
      item_class: 'food_shelf_stable',
      quantity: Math.round(t.supplies.value * f.kcalPerDay),
      unit: 'kcal',
      per: 'person',
      rule: 'mock_food_kcal',
      citations: ['mock_food_kcal'],
      plain: `${dayPhrase(t.supplies.value)} of food at ${f.kcalPerDay.toLocaleString('en-US')} kcal a day for the household.`,
    });
  }
  if (f.infants > 0) {
    add({
      id: 'infant_formula',
      bucket: 'supplies',
      item_class: 'infant_formula_reserve',
      quantity: t.supplies.value * f.infants,
      unit: 'day',
      per: 'person',
      rule: 'mock_infant_formula',
      citations: ['mock_ready_gov_kit'],
      plain: `${dayPhrase(t.supplies.value)} of ready-to-feed formula for ${f.infants === 1 ? 'the baby' : `${f.infants} babies`}.`,
    });
  }
  if (f.pets > 0) {
    add({
      id: 'pet_food',
      bucket: 'supplies',
      item_class: 'pet_food_reserve',
      quantity: t.supplies.value * f.pets,
      unit: 'pet-day',
      per: 'pet',
      rule: 'mock_pet_food',
      citations: ['mock_ready_gov_kit'],
      plain: `${dayPhrase(t.supplies.value)} of food for ${f.pets} ${f.pets === 1 ? 'pet' : 'pets'}.`,
    });
  }
  if (f.dailyRx > 0) {
    add({
      id: 'medication_days',
      bucket: 'medication',
      item_class: 'medication_reserve',
      quantity: t.medication.value * f.dailyRx,
      unit: 'person-day',
      per: 'person',
      rule: 'mock_medication_days',
      citations: ['mock_medication_reserve'],
      plain: `${dayPhrase(t.medication.value)} of each daily medicine for ${f.dailyRx} ${f.dailyRx === 1 ? 'person' : 'people'}.`,
    });
  }
  if (f.fridgeRx) {
    add({
      id: 'cold_chain',
      bucket: 'medication',
      item_class: 'cooler_ice_packs',
      quantity: 1,
      unit: 'cooler',
      per: 'household',
      rule: 'mock_cold_chain',
      citations: ['mock_medication_reserve'],
      plain: 'A way to keep cold medicine cold for a day without power.',
    });
  }
  add({
    id: 'lights',
    bucket: 'power',
    item_class: 'flashlights_headlamps',
    quantity: f.n,
    unit: 'light',
    per: 'person',
    rule: 'mock_one_each',
    citations: ['mock_ready_gov_kit'],
    plain: 'One light for each person.',
  });
  if (f.deviceWatts.length > 0) {
    const wh = f.deviceWatts.reduce((s, w) => s + w, 0) * 8 * Math.min(t.power.value, 3);
    add({
      id: 'device_power',
      bucket: 'power',
      item_class: 'device_battery_backup',
      quantity: wh,
      unit: 'Wh',
      per: 'household',
      rule: 'mock_device_wh',
      citations: ['mock_ready_gov_kit'],
      plain: `Battery power for the medical ${f.deviceWatts.length === 1 ? 'device' : 'devices'}: ${f.deviceWatts.join(' + ')} watts for 8 hours a night, for up to ${dayPhrase(Math.min(t.power.value, 3))}.`,
    });
  }
  add({
    id: 'phone_power',
    bucket: 'comms',
    item_class: 'power_bank',
    quantity: Math.ceil(f.n / 2),
    unit: 'power bank',
    per: 'person',
    rule: 'mock_one_per_two',
    citations: ['mock_ready_gov_kit'],
    plain: 'A charged power bank for every two people.',
  });
  add({
    id: 'go_bags',
    bucket: 'evacuate',
    item_class: 'go_bag',
    quantity: f.n,
    unit: 'bag',
    per: 'person',
    rule: 'mock_one_each',
    citations: ['mock_evacuation'],
    plain: 'A go-bag for each person.',
  });
  if (f.commuters > 0) {
    add({
      id: 'get_home_bags',
      bucket: 'get_home',
      item_class: 'get_home_bag',
      quantity: f.commuters,
      unit: 'bag',
      per: 'commuter',
      rule: 'mock_one_each',
      citations: ['mock_get_home'],
      plain: `A get-home bag for each person who travels 3 miles or more; the longest trip is about ${Math.round(f.maxCommuteKm / 1.609)} miles, about ${Math.max(1, Math.round(f.maxCommuteKm / 1.609 / 3))} hours on foot.`,
    });
  }
  add({
    id: 'first_aid',
    bucket: 'medical_emergency',
    item_class: 'first_aid_kit',
    quantity: 1,
    unit: 'kit',
    per: 'household',
    rule: 'mock_once',
    citations: ['mock_ready_gov_kit'],
    plain: 'One first-aid kit sized for the household.',
  });
  add({
    id: 'smoke_alarms',
    bucket: 'fire',
    item_class: 'smoke_alarm',
    quantity: f.house ? (f.basement ? 3 : 2) : 1,
    unit: 'alarm',
    per: 'household',
    rule: 'mock_alarm_per_level',
    citations: ['mock_fire_safety'],
    plain: 'A working smoke alarm on every level and outside sleeping areas.',
  });
  add({
    id: 'neighbours',
    bucket: 'security',
    item_class: 'neighbours_numbers',
    quantity: 2,
    unit: 'neighbour',
    per: 'household',
    rule: 'mock_two_neighbours',
    citations: ['mock_social_capital'],
    plain: 'Numbers for at least two neighbours who agree to check on you.',
  });
  add({
    id: 'documents',
    bucket: 'home_loss',
    item_class: 'documents_copies',
    quantity: 1,
    unit: 'set',
    per: 'household',
    rule: 'mock_once',
    citations: ['mock_insurance'],
    plain: 'Copies of papers and a check of insurance cover.',
  });
  add({
    id: 'savings_months',
    bucket: 'income',
    item_class: 'savings',
    quantity: targets.income.value,
    unit: 'month',
    per: 'household',
    rule: 'mock_income_gap',
    citations: ['mock_bls_job_loss', 'mock_savings_track'],
    plain: `Savings for ${monthsPhrase(targets.income.value)} of expenses, built separately from the supplies budget.`,
  });
  return lines;
}

// ---------------------------------------------------------------------------------------------
// Scheduling
// ---------------------------------------------------------------------------------------------

const MAX_MONTHS = 36;

function hazardsFor(ctx: Ctx, item: Item): HazardId[] {
  const matching = ctx.register
    .filter((h) => !h.seed.rare && h.seed.buckets.some((b) => item.buckets.includes(b)))
    .slice(0, 3)
    .map((h) => h.seed.id);
  return unique([...matching, ...item.hazard_extras]);
}

function firstSentence(buckets: BucketAssessment[], id: BucketId): string {
  return buckets.find((b) => b.id === id)?.frequency_sentences[0] ?? '';
}

const FREE_WHY: Record<string, string> = {
  plan_family_contacts: 'Phones die and networks jam. A plan on paper means everyone knows where to go and who to call.',
  documents_copies: 'After a fire or flood, insurance and aid move faster when you can show papers. Photos cost nothing.',
  alerts_signup: 'Warnings arrive by text first. Minutes of notice change what you can do.',
  water_reused_bottles: 'Free, and about 6 gallons covers the first day or two of water for most households.',
  water_boil_method: 'Your water heater already holds drinkable water, and boiling makes tap water safe. Knowing how costs nothing.',
  medication_refill_early: 'A week of spare medicine at all times covers most short disruptions, at no extra cost.',
  medication_list: 'In an emergency room or a shelter, a written list saves time and avoids mistakes.',
  temperature_plan: 'Heat and cold are most dangerous when the power is out. Knowing where to go is free.',
  alarms_test: 'Working smoke alarms roughly halve the chance of dying in a home fire. Testing takes a minute.',
  escape_plan: 'In a fire you may have only a couple of minutes. Practising makes the way out automatic.',
  neighbours_numbers: 'Neighbours are the first help in almost every disaster. Two phone numbers are a real supply.',
  check_in_agreement: 'People who live alone or can’t get out easily are most at risk in heat and outages. A check-in plan protects them.',
  car_half_tank: 'Fuel pumps need power. Half a tank means you can leave or get home without queueing.',
  ev_charge_habit: 'A charged car is transport, and sometimes a power source. Charging before storms is free.',
  get_home_route: 'If roads or transit stop, you may walk home. A planned route saves hours.',
  go_stay_card: 'Deciding in advance removes the hardest part of leaving: the hesitation.',
  tsunami_route: 'After strong shaking near the coast, a tsunami can arrive in minutes. Walking uphill at once is the whole plan.',
  safe_room_plan: 'Tornado warnings give minutes. Knowing the spot means nobody has to think.',
  hurricane_zone_check: 'Hurricanes give days of warning. Knowing your zone and where you would go turns that into a calm departure.',
  pet_plan: 'Many people delay leaving because of pets. A pet plan removes that delay.',
  livestock_water_plan: 'Animals drink far more than people, and a stopped pump affects them first.',
  insurance_check: 'Most people find out what their policy covers after the damage. Reading it now is free.',
  utility_shutoffs: 'A gas leak or burst pipe is much less damaging if you can shut it off in seconds.',
  community_group: 'Trained neighbours reach people long before outside help can.',
  firearm_safe_storage: 'Locked storage prevents most firearm accidents and gives time in a crisis.',
};

function whyFor(ctx: Ctx, buckets: BucketAssessment[], c: Chunk): string {
  const id = c.item.id;
  if (c.item.free) return FREE_WHY[id] ?? `Costs nothing and helps with: ${c.item.buckets.map(bucketName).join(', ').toLowerCase()}.`;
  const level = c.level;
  switch (id) {
    case 'water_stored':
    case 'water_containers':
    case 'water_drum':
      return `${firstSentence(buckets, 'water_out')} This brings stored water to about ${dayPhrase(level ?? 3)} for your household.`;
    case 'water_filter':
      return `For long outages a filter goes further than more stored water: with rain or creek water it keeps going. ${firstSentence(buckets, 'water_out')}`;
    case 'food_shelf_stable':
      return `${firstSentence(buckets, 'supplies')} This brings food to about ${dayPhrase(level ?? 3)} for everyone.`;
    case 'infant_formula_reserve':
      return `A baby can't wait for stores to reopen or water to be safe. This brings formula to about ${dayPhrase(level ?? 3)}.`;
    case 'pet_food_reserve':
      return `Pets eat every day too. This brings their food to about ${dayPhrase(level ?? 3)}.`;
    case 'medication_reserve':
      return `Daily medicine is the one supply that can't be swapped for something else. This builds the extra supply to about ${dayPhrase(level ?? 7)}.`;
    case 'cooler_ice_packs':
      return `${firstSentence(buckets, 'power')} Medicine that must stay cold is at risk after about a day without power.`;
    case 'device_battery_backup':
      return `${firstSentence(buckets, 'power')} The medical device stops when the power does. A battery sized for it keeps it running through the night.`;
    case 'smoke_alarm':
    case 'co_alarm':
      return 'Working alarms are the cheapest life-saving item there is. Outages raise carbon monoxide risk from generators and grills.';
    case 'water_heater_strap':
      return 'In a strong quake an unstrapped water heater can fall, break a gas line and lose its water. Straps keep it upright.';
    case 'flashlights_headlamps':
    case 'batteries_spare':
    case 'lantern':
      return `${firstSentence(buckets, 'power')} A light for each person makes the first night easy.`;
    case 'power_bank':
    case 'radio_crank':
      return `${firstSentence(buckets, 'comms')} This keeps alerts and calls coming.`;
    case 'fans_cooling':
      return `${firstSentence(buckets, 'thermal')} Fans and wet towels help a lot when the power is out in hot weather.`;
    case 'blankets_warm':
      return `${firstSentence(buckets, 'thermal')} Your heating needs electricity, so warm bedding is the backup.`;
    case 'go_bag':
      return `${firstSentence(buckets, 'evacuate')} Start with things you already own; buy only what's missing.`;
    case 'get_home_bag':
      return `${firstSentence(buckets, 'get_home')} Good shoes and water make a long walk home safe.`;
    case 'generator_portable':
      return `Your well pump needs electricity, so every power cut is also a water cut. A generator run outdoors keeps both going.`;
    default:
      return `Helps with: ${c.item.buckets.map(bucketName).join(', ').toLowerCase()}. ${c.item.spec.split('. ')[0]?.replace(/\.$/, '')}.`;
  }
}

function toPlanItem(ctx: Ctx, buckets: BucketAssessment[], c: Chunk): PlanItem {
  const kind: PlanItemKind = c.item.free
    ? 'free_action'
    : c.item.id === 'cash_small_bills' || c.item.id === 'medication_reserve'
      ? 'reserve'
      : 'purchase';
  const est = c.done && c.paid !== undefined ? c.paid : cents(c.unitCost * c.qty);
  const out: PlanItem = {
    item_id: c.item.id,
    name: c.item.name,
    kind,
    quantity: c.qty,
    unit: c.item.unit,
    est_cost_usd: c.item.free ? 0 : est,
    price_band: { low: cents(c.bandLow * c.qty), high: cents(c.bandHigh * c.qty) },
    buckets: c.item.buckets,
    hazards: hazardsFor(ctx, c.item),
    why: whyFor(ctx, buckets, c),
    risk_reduction: Math.round(c.priority * (c.level ?? 1)),
    tier: c.tier,
  };
  if (c.done) out.done = true;
  if (c.done && c.paid !== undefined) out.paid_usd = c.paid;
  return out;
}

function schedulePlan(ctx: Ctx, chunks: Chunk[], buckets: BucketAssessment[]) {
  const monthly = Math.max(0, ctx.input.finances.monthly_budget_usd);
  const oneOff = Math.max(0, ctx.input.finances.one_off_budget_usd);
  const months = new Map<number, PlanItem[]>();
  const push = (k: number, item: PlanItem) => {
    const list = months.get(k) ?? [];
    list.push(item);
    months.set(k, list);
  };
  const schedule = new Map<string, number>();

  // Month 0: free actions (life safety first), then everything already done.
  const free = freeActions(ctx).map((id) => {
    const c = chunk(ctx, id, 'now', 1);
    c.done = has(ctx.owned, id);
    return c;
  });
  free.sort((a, b) => Number(b.item.life_safety) - Number(a.item.life_safety));
  for (const c of free) push(0, toPlanItem(ctx, buckets, c));
  for (const c of chunks.filter((x) => x.done)) push(0, toPlanItem(ctx, buckets, c));

  // Purchases: tier order, life safety first, then value for money; bought as the money allows.
  const todo = chunks
    .filter((c) => !c.done && c.qty > 0)
    .map((c) => ({ c, cost: cents(c.unitCost * c.qty) }))
    .sort(
      (a, b) =>
        TIER_IDS.indexOf(a.c.promotedTo ?? a.c.tier) - TIER_IDS.indexOf(b.c.promotedTo ?? b.c.tier) ||
        Number(b.c.item.life_safety) - Number(a.c.item.life_safety) ||
        (b.c.priority * 10) / Math.sqrt(b.cost + 4) - (a.c.priority * 10) / Math.sqrt(a.cost + 4) ||
        a.c.item.id.localeCompare(b.c.item.id),
    );
  let cumulative = 0;
  const unscheduled: Chunk[] = [];
  const envelopes: SavingsEnvelope[] = [];
  for (const { c, cost } of todo) {
    cumulative = cents(cumulative + cost);
    let k: number | undefined;
    if (cumulative <= oneOff + monthly + 1e-6) k = 0;
    else if (monthly > 0) k = Math.ceil((cumulative - oneOff) / monthly - 1e-9) - 1;
    if (k === undefined || k >= MAX_MONTHS) {
      unscheduled.push(c);
      continue;
    }
    schedule.set(`${c.item.id}:${c.tier}`, k);
    push(k, toPlanItem(ctx, buckets, c));
    if (monthly > 0 && cost > monthly && k > 0) envelopes.push({ item_id: c.item.id, saved_usd: 0, needed_usd: cost });
  }

  const planMonths: PlanMonth[] = [...months.keys()]
    .sort((a, b) => a - b)
    .map((k) => ({ index: k, budget_usd: k === 0 ? monthly + oneOff : monthly, items: months.get(k)! }));
  if (planMonths.length === 0 || planMonths[0]!.index !== 0) {
    planMonths.unshift({ index: 0, budget_usd: monthly + oneOff, items: [] });
  }
  const lastScheduled = Math.max(0, ...schedule.values());
  const plan: PlanOutput['plan'] = { months: planMonths, envelopes };
  if (unscheduled.length === 0) plan.done_month = lastScheduled;
  plan.savings_track = savingsTrack(ctx, buckets);
  return { plan, schedule, unscheduled };
}

function savingsTrack(ctx: Ctx, buckets: BucketAssessment[]): SavingsTrack {
  const { input, targets } = ctx;
  const expenses = input.finances.monthly_expenses_usd ?? 3000;
  const target = targets.income.value;
  const current = input.finances.emergency_fund_months;
  const gapUsd = Math.max(0, (target - current) * expenses);
  const suggestion = gapUsd > 0 ? Math.max(5, Math.round(gapUsd / 36 / 5) * 5) : 0;
  const sentence = firstSentence(buckets, 'income');
  const assumed = input.finances.monthly_expenses_usd === undefined ? ' This assumes $3,000 a month in expenses; add yours on the Money screen for a better figure.' : '';
  const why =
    gapUsd > 0
      ? `${sentence} Savings cover the gap that unemployment insurance leaves. Setting aside about $${suggestion} a month would get there in about three years.${assumed}`
      : `${sentence} You already have about ${monthsPhrase(current)} saved, which covers the target for your risk.${assumed}`;
  return {
    target_months: target,
    target_usd: Math.round(target * expenses),
    current_months: current,
    monthly_suggestion_usd: suggestion,
    why,
  };
}

// ---------------------------------------------------------------------------------------------
// Scenarios and warnings
// ---------------------------------------------------------------------------------------------

function buildScenarios(input: PlanInput, f: Facts, profile: RegionProfile, states: ScenarioState[], targets: Targets): ScenarioInfo[] {
  return states.map((s) => {
    const flipped = states.map((x) => (x.seed.id === s.seed.id ? { ...x, on: !x.on } : x));
    const other = computeTargets(input, f, profile, flipped);
    const withT = s.on ? targets : other;
    const withoutT = s.on ? other : targets;
    const changed = (['power', 'water_out', 'supplies', 'medication', 'comms', 'thermal'] as DurationBucket[])
      .filter((b) => withT.days[b].value !== withoutT.days[b].value)
      .slice(0, 3);
    const label: Record<DurationBucket, string> = {
      power: 'without power',
      water_boil: 'of treated water',
      water_out: 'without tap water',
      supplies: 'of food and supplies',
      thermal: 'of heat or cold',
      medication: 'of medicine',
      comms: 'without phones or payments',
    };
    const list = (t: Targets) => changed.map((b) => `about ${dayPhrase(t.days[b].value)} ${label[b]}`).join(', ');
    const effect =
      changed.length === 0
        ? 'At this setting it does not change your targets.'
        : `With it, the plan covers ${list(withT)}. Without it: ${list(withoutT)}.`;
    return {
      id: s.seed.id,
      name: s.seed.name,
      applies_because: s.seed.applies_because,
      on: s.on,
      effect_summary: effect,
      sources: s.seed.sources,
    };
  });
}

function buildWarnings(
  ctx: Ctx,
  buckets: BucketAssessment[],
  chunks: Chunk[],
  schedule: Map<string, number>,
  unscheduled: Chunk[],
  states: ScenarioState[],
): Warning[] {
  const { input, f, targets } = ctx;
  const out: Warning[] = [];
  const when = (id: string): number | undefined => {
    const c = chunks.find((x) => x.item.id === id);
    if (!c) return undefined;
    if (c.done) return -1;
    return schedule.get(`${id}:${c.tier}`) ?? Infinity;
  };
  const monthly = input.finances.monthly_budget_usd;
  const oneOff = input.finances.one_off_budget_usd;

  for (const s of states.filter((x) => x.on)) {
    const off = computeTargets(input, f, ctx.profile, states.map((x) => (x.seed.id === s.seed.id ? { ...x, on: false } : x)));
    const ratio = Math.max(
      ...DURATION_BUCKETS.map((b) => targets.days[b].value / Math.max(0.5, off.days[b].value)),
    );
    if (ratio >= 3) {
      const b = DURATION_BUCKETS.reduce((best, x) =>
        targets.days[x].value / Math.max(0.5, off.days[x].value) > targets.days[best].value / Math.max(0.5, off.days[best].value) ? x : best,
      );
      out.push({
        id: `cliff_${s.seed.id}`,
        severity: 'warn',
        message: `Your plan depends mostly on one event: ${s.seed.name.replace(/^Plan for /, '')}.`,
        why: `At this setting, planning for it raises "${bucketName(b).toLowerCase()}" from about ${dayPhrase(off.days[b].value)} to about ${dayPhrase(targets.days[b].value)}. That is a reasonable choice where official guidance covers it. To see the plan without it, switch it off under "Named scenarios" on the risks screen.`,
        related: [s.seed.id, s.seed.hazard, b],
      });
    }
  }
  if (monthly <= 0 && oneOff <= 0) {
    out.push({
      id: 'zero_budget',
      severity: 'note',
      message: 'Your plan is free steps only, because the budget is $0.',
      why: 'That is a fine place to start. The free steps still cover the first days of water, a household plan and your papers. Even $10 a month would add lights and food over the next few months.',
      related: [],
    });
  }
  if (f.deviceWatts.length > 0 && f.backup === 'none') {
    const k = when('device_battery_backup');
    if (k === undefined || k > 3) {
      out.push({
        id: 'powered_device_no_backup',
        severity: 'warn',
        message: 'The medical device has no backup power in the first three months of this plan.',
        why: `${firstSentence(buckets, 'power')} A battery sized for the device keeps it running. You could move it earlier, or ask the device supplier about a loaner battery.`,
        related: ['power', 'device_battery_backup'],
      });
    }
  }
  if (f.fridgeRx) {
    const k = when('cooler_ice_packs');
    if (k === undefined || k > 1) {
      out.push({
        id: 'refrigerated_no_cooling',
        severity: 'warn',
        message: 'The medicine that must stay cold has no cooling plan yet.',
        why: `${firstSentence(buckets, 'power')} A cooler with gel packs keeps cold medicine safe for about a day.`,
        related: ['medication', 'cooler_ice_packs'],
      });
    }
  }
  const firstWater = Math.min(
    ...chunks.filter((c) => c.resource === 'water').map((c) => (c.done ? -1 : (schedule.get(`${c.item.id}:${c.tier}`) ?? Infinity))),
  );
  const freeWaterDays = (FREE_BOTTLE_GALLONS + heaterGallons(f)) / Math.max(0.1, f.galPerDay);
  if (freeWaterDays < 1 && firstWater > 1) {
    out.push({
      id: 'no_water_after_month1',
      severity: 'warn',
      message: 'Stored water comes late in this plan.',
      why: `${firstSentence(buckets, 'water_out')} Filling clean bottles you already have is free and covers the first day.`,
      related: ['water_out', 'water_reused_bottles'],
    });
  }
  const evac = buckets.find((b) => b.id === 'evacuate')!.target;
  if (evac.kind === 'evacuate' && evac.p_need_10yr >= 0.1) {
    const k = when('go_bag');
    if (k === undefined || k > 6) {
      out.push({
        id: 'evac_no_go_bag',
        severity: 'warn',
        message: 'Leaving quickly is fairly likely here, but the go-bags come late in this plan.',
        why: `${firstSentence(buckets, 'evacuate')} You could pack a bag now with things you already own, and fill the gaps later.`,
        related: ['evacuate', 'go_bag'],
      });
    }
  }
  if (f.owner && !input.finances.insurance.flood) {
    const floodRate = ctx.register
      .filter((h) => ['riverine_flooding', 'coastal_flooding', 'hurricane'].includes(h.seed.id))
      .reduce((s, h) => s + h.rate, 0);
    if (floodRate >= 0.05) {
      out.push({
        id: 'insurance_gap_flood',
        severity: 'note',
        message: 'Your home has no flood insurance.',
        why: 'Standard home insurance usually leaves out flood damage, and flood policies often take 30 days to start. Worth deciding before storm season.',
        related: ['home_loss', 'riverine_flooding', 'insurance_check'],
      });
    }
  }
  if (f.owner && !input.finances.insurance.earthquake && (ctx.register.find((h) => h.seed.id === 'earthquake')?.rate ?? 0) >= 0.005) {
    out.push({
      id: 'insurance_gap_quake',
      severity: 'note',
      message: 'Your home has no earthquake insurance.',
      why: 'Home insurance usually leaves out earthquake damage. Earthquake policies have high deductibles, so compare the cost with what you could cover yourself.',
      related: ['home_loss', 'earthquake', 'insurance_check'],
    });
  }
  if (!f.owner && !input.finances.insurance.home_or_renters) {
    out.push({
      id: 'no_renters_insurance',
      severity: 'note',
      message: 'You have no renters insurance.',
      why: 'It usually pays for belongings and a place to stay after a fire or flood. Ask for a quote; it is often cheaper than people expect.',
      related: ['home_loss', 'insurance_check'],
    });
  }
  if (unscheduled.length > 0 && monthly > 0) {
    out.push({
      id: 'plan_longer_than_three_years',
      severity: 'note',
      message: `At this budget, ${unscheduled.length} ${unscheduled.length === 1 ? 'item falls' : 'items fall'} beyond three years.`,
      why: 'The plan covers the most likely disruptions first, so the early months do the most good. A less cautious setting, or a little more each month, brings the rest closer.',
      related: [],
    });
  }
  return out;
}

