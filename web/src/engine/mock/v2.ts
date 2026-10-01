/**
 * The mock engine's contract v2 outputs (docs/ENGINE-API.md "Changes from v1"), shaped like the
 * real engine's so the screens can be built before the WebAssembly engine carries them:
 *
 * - the nine **rare families** with their sub-causes, location factor ("why here"), range only,
 *   the anchor sentence, "if it reaches you" and "what it changes" (rr-hazards `rare.rs`, whose
 *   numbers are copied here: `docs/RISK_MODEL.md` "The rare families");
 * - the **ten new ranked hazards** and arrests where they apply, and named **sub-causes** on the
 *   ranked cards (`subcauses.rs`);
 * - the place's **exposure** for the fixture counties (data pack v2 columns);
 * - the **stress test** on the power and water targets (rr-consequence, the data pack's worst
 *   events: `core/outage_stress.csv` for power, documented water failures by state);
 * - the **"Also checked"** note, the **first savings milestone**, the **validation summary**.
 *
 * Every source is a `mock_*` placeholder, as in the rest of the mock. The app never depends on
 * anything here that the real engine could not send.
 */
import type {
  BucketId,
  DataConfidence,
  Exposure,
  HazardId,
  HazardProfile,
  LocationFactor,
  LocationResolved,
  PlanInput,
  SavingsMilestone,
  SavingsTrack,
  StressTest,
  SubCause,
  ValidationSummary,
} from '../types';
import { chanceWithin, oneIn, roundSig2, thousands } from '../../lib/format';
import type { RankedHazard } from './model';
import { hazardName, hazardTier } from './names';
import type { HazardSeed } from './regions';

// ---------------------------------------------------------------------------------------------
// Sentences, worded as rr-hazards words them (crates/rr-hazards/src/sentence.rs)
// ---------------------------------------------------------------------------------------------

const NUMBER_WORDS: Record<number, string> = { 2: 'two', 3: 'three', 4: 'four', 5: 'five', 6: 'six', 7: 'seven', 8: 'eight', 9: 'nine', 10: 'ten' };

function horizonPhrase(years: number): string {
  if (years <= 1) return 'in the next year';
  return `in the next ${NUMBER_WORDS[years] ?? years} years`;
}

/** "1 in 2,500", "fewer than 1 in 1,000,000", as rr-hazards' `one_in`. */
function oneInEngine(p: number): string {
  if (p < 0.95e-6) return 'fewer than 1 in 1,000,000';
  return `1 in ${thousands(roundSig2(1 / p))}`;
}

/** The range-only sentence of a rare row (`sentence::range_sentence`). */
export function rangeSentence(verb: string, low: number, high: number, years: number): string {
  const when = horizonPhrase(years);
  const pl = chanceWithin(low, years);
  const ph = chanceWithin(high, years);
  if (ph <= 0) return `No household like yours is expected to ${verb} ${when}.`;
  if (low <= 0 || high / low > 1000) {
    return `Expert estimates for this span more than a thousandfold. At most about ${oneInEngine(ph)} households like yours would ${verb} ${when}.`;
  }
  if (pl < 0.95e-6) return `At most about ${oneInEngine(ph)} households like yours would ${verb} ${when}, and perhaps fewer than 1 in 1,000,000.`;
  return `Between ${oneInEngine(pl)} and ${oneInEngine(ph)} households like yours would ${verb} ${when}.`;
}

function perWord(n: number, of: number): string {
  if (n < 0.5) return 'fewer than 1';
  if (n < 10) return String(Math.round(n));
  return of === 100 ? String(roundSig2(n)) : thousands(roundSig2(n));
}

/** "about 5 in 100", "about 3 in 1,000", "about 1 in 2,500" (`sentence::chance_words`). */
function chanceWords(p: number): string {
  if (p >= 0.995) return 'nearly certain';
  if (p * 100 >= 0.95) return `about ${perWord(p * 100, 100)} in 100`;
  if (p * 1000 >= 0.95) return `about ${perWord(p * 1000, 1000)} in 1,000`;
  if (p >= 1e-6) return `about ${oneIn(p)}`;
  return 'fewer than 1 in 1,000,000';
}

/** How the anchor names a ranked hazard: "less likely than {phrase}" (rr-hazards `anchor_phrase`). */
const ANCHOR_PHRASE: Partial<Record<HazardId, string>> = {
  avalanche: 'an avalanche reaching your home or road',
  coastal_flooding: 'coastal flooding reaching your home',
  cold_wave: 'a cold wave',
  drought: 'a drought that limits your water',
  earthquake: 'an earthquake strong enough to knock things off shelves',
  hail: 'hail damage',
  heat_wave: 'a heat wave',
  hurricane: 'a hurricane or tropical storm',
  ice_storm: 'an ice storm',
  landslide: 'a landslide',
  lightning: 'lightning damaging your home',
  riverine_flooding: 'flood water reaching your home',
  strong_wind: 'a windstorm',
  tornado: 'a tornado',
  tsunami: 'a tsunami warning',
  volcanic_activity: 'ash or mudflows from a volcano',
  wildfire: 'a wildfire',
  winter_weather: 'a winter storm',
  wildfire_smoke: 'days of wildfire smoke',
  dust_storm: 'a dust storm',
  sinkhole: 'a sinkhole',
  pandemic: 'a pandemic that changes daily life',
  grid_failure: 'a regional blackout',
  cyber_outage: 'a computer outage that stops services',
  civil_unrest: 'a curfew',
  supply_chain_disruption: 'empty store shelves',
  hazmat_release: 'a chemical spill order',
  nuclear_plant_incident: 'a nuclear plant accident',
  dam_failure: 'a dam or levee failure',
  network_outage: 'a phone or internet outage',
  drug_shortage: 'a medicine shortage',
  benefit_interruption: 'pay or benefits stopping',
  attack_disruption: 'an attack or threat closing your area',
  job_loss: 'a job loss',
  house_fire: 'a house fire',
  medical_emergency: 'a medical emergency',
  vehicle_stranding: 'being stranded in a vehicle',
  local_utility_outage: 'a water main break or boil-water notice',
  burglary: 'a break-in',
  earner_death_or_disability: 'the death or disability of an earner',
  extended_household_illness: 'a long illness at home',
  water_damage: 'a burst pipe or leak',
  eviction: 'an eviction',
  arrest_or_detention: 'an arrest in the household',
};

/**
 * The anchor of a rare row (REVIEW §2.4): the household's own ranked hazard with the smallest rate
 * still above the row's upper bound. None when no ranked hazard is that likely.
 */
export function anchorFor(high: number, ranked: readonly RankedHazard[], years: number): string | undefined {
  const above = ranked.filter((h) => h.rate > high).sort((a, b) => a.rate - b.rate)[0];
  if (!above) return undefined;
  const phrase = ANCHOR_PHRASE[above.seed.id] ?? hazardName(above.seed.id).toLowerCase();
  return `Less likely than ${phrase} (${chanceWords(chanceWithin(above.rate, years))} for you ${horizonPhrase(years)}).`;
}

// ---------------------------------------------------------------------------------------------
// Where: the fixture counties' exposure (data pack v2 columns, placeholder values)
// ---------------------------------------------------------------------------------------------

type StrategicClass = 'A' | 'B' | 'C1' | 'C2' | 'D' | 'E';

interface Place {
  strategic: StrategicClass;
  /** Words for the place in "why here". */
  metro?: string;
  /** B and D: what the fallout would drift from, and how far. */
  upwind?: string;
  geomag: number;
  lat: number;
  /** The FEMA urban area's share of UASI money, or 0 outside the funded areas. */
  uasi: number;
  uasiArea?: string;
  smokeDays: number;
  dustStorms?: number;
  karst?: number;
  landslide?: number;
  leveed?: number;
  waterFlag?: number;
  eviction?: number;
}

const PLACES: Record<string, Place> = {
  '42101': { strategic: 'C1', metro: 'the Philadelphia metro area', geomag: 0.29, lat: 40, uasi: 0.028, uasiArea: 'Philadelphia', smokeDays: 2.1, karst: 0.02, leveed: 0.01, waterFlag: 0.04, eviction: 0.061 },
  '41011': { strategic: 'E', geomag: 0.31, lat: 43, uasi: 0, smokeDays: 6.4, landslide: 0.42, waterFlag: 0.12, eviction: 0.021 },
  '12086': { strategic: 'C1', metro: 'the Miami metro area', geomag: 0.1, lat: 26, uasi: 0.026, uasiArea: 'Miami/Fort Lauderdale', smokeDays: 0.8, karst: 0.9, leveed: 0.06, waterFlag: 0.08, eviction: 0.048 },
  '20051': { strategic: 'B', upwind: 'the nuclear missile fields in Wyoming, Nebraska and Colorado, about 290 miles to your northwest', geomag: 0.27, lat: 39, uasi: 0, smokeDays: 1.6, dustStorms: 0.05, karst: 0.1, waterFlag: 0.03, eviction: 0.014 },
  '04013': { strategic: 'C1', metro: 'the Phoenix metro area', geomag: 0.15, lat: 33, uasi: 0.017, uasiArea: 'Phoenix', smokeDays: 3.2, dustStorms: 0.4, waterFlag: 0.05, eviction: 0.07 },
  '48157': { strategic: 'C1', metro: 'the Houston metro area', geomag: 0.12, lat: 30, uasi: 0.031, uasiArea: 'Houston', smokeDays: 1.1, leveed: 0.04, waterFlag: 0.06, eviction: 0.052 },
  '17031': { strategic: 'C1', metro: 'the Chicago metro area', geomag: 0.36, lat: 42, uasi: 0.055, uasiArea: 'Chicago', smokeDays: 2.6, leveed: 0.02, waterFlag: 0.03, eviction: 0.035 },
};

/** Same-metro counties share their core county's place. */
const SAME_PLACE: Record<string, string> = {
  '42045': '42101', '42029': '42101', '42091': '42101', '42017': '42101',
  '41015': '41011', '41019': '41011', '12011': '12086', '48201': '48157', '17043': '17031',
};

function placeOf(fips: string): Place | undefined {
  return PLACES[fips] ?? PLACES[SAME_PLACE[fips] ?? ''];
}

const src = (id: string) => id;

/** `LocationResolved.exposure` for a mock county, or undefined when nothing is known (every sample county). */
export function exposureFor(fips: string): Exposure | undefined {
  const p = placeOf(fips);
  if (!p) return undefined;
  const e: Exposure = {
    strategic_class: { value: p.strategic, source: src('mock_strategic_sites') },
    smoke_days_35: { value: p.smokeDays, source: src('mock_smoke_days') },
    geomag_factor: { value: p.geomag, source: src('mock_solar_storms') },
    uasi_share: { value: p.uasi, source: src('mock_uasi') },
  };
  if (p.karst !== undefined) e.karst_share = { value: p.karst, source: 'mock_nri_county' };
  if (p.landslide !== undefined) e.landslide_susceptible_share = { value: p.landslide, source: 'mock_nri_county' };
  if (p.leveed !== undefined) e.leveed_pop_share = { value: p.leveed, source: 'mock_nri_county' };
  if (p.waterFlag !== undefined) e.water_system_flag = { value: p.waterFlag, source: 'mock_boil_notices' };
  if (p.eviction !== undefined) e.eviction_rate = { value: p.eviction, source: 'mock_eviction' };
  return e;
}

// ---------------------------------------------------------------------------------------------
// The nine rare families (rr-hazards rare.rs; the numbers of docs/RISK_MODEL.md)
// ---------------------------------------------------------------------------------------------

/** [middle, low, high] a year. */
type Triple = [number, number, number];

/** Low times low, high times high, middle times middle (`Estimate::times_span`). */
function times(a: Triple, b: Triple): Triple {
  return [a[0] * b[0], a[1] * b[1], a[2] * b[2]];
}

function plus(a: Triple, b: Triple): Triple {
  return [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
}

function scaled(a: Triple, k: number): Triple {
  return [a[0] * k, a[1] * k, a[2] * k];
}

const LAMBDA_S: Triple = [4e-4, 1e-4, 4e-3];
const LAMBDA_L: Triple = [1e-4, 2e-5, 5e-4];
const LAMBDA_I: Triple = [5e-5, 1e-5, 2.5e-4];
const F_S: Record<StrategicClass | 'unknown', Triple> = {
  A: [0.9, 0.8, 1],
  B: [0.5, 0.2, 0.8],
  C1: [0.6, 0.3, 0.9],
  C2: [0.3, 0.1, 0.5],
  D: [0.15, 0.05, 0.3],
  E: [0.03, 0.01, 0.1],
  unknown: [0.314, 0.01, 0.99],
};
const HEMP_GIVEN_STRATEGIC: Triple = [0.5, 0.2, 0.8];
const CARRINGTON: Triple = [3e-3, 5e-4, 1.3e-2];
const GMD_OUTAGE: Triple = [0.09, 0.06, 0.12];
const GMD_ALPHA_MEAN = 0.2285;
const WAR: Triple = [5e-3, 2e-3, 1e-2];
const WAR_HOMELAND: Triple = [0.5, 0.2, 0.8];
const CBRN_US: Triple = [0.03, 0.01, 0.1];
const CBRN_METRO_SHARE: Triple = [0.05, 0.01, 0.2];
const CBRN_NON_UASI: Triple = [1.5e-7, 1e-8, 2e-6];

interface RareRow {
  id: HazardId;
  est: Triple;
  verb: string;
  severity: number;
  confidence: DataConfidence;
  sources: string[];
  sub_causes: SubCause[];
  location_factor?: LocationFactor;
  if_it_reaches_you: string;
  what_it_changes: string;
}

function sub(id: string, name: string, note: string, range: [number, number] | undefined, sources: string[]): SubCause {
  const s: SubCause = { id, name, note, sources };
  if (range) s.rate_range = range;
  return s;
}

function whyHere(cls: StrategicClass | 'unknown', p: Place | undefined, county: string): string {
  switch (cls) {
    case 'A':
      return 'Your county has, or is close to, a strategic military site. In a large nuclear war, places like this are treated as likely targets. That does not mean an attack is likely. It means that if one happened, this area would face more danger than most.';
    case 'B':
      return `You live downwind of ${p?.upwind ?? 'the nuclear missile fields of the northern Great Plains'}. Winds here usually blow from the west. If those missile sites were ever attacked, radioactive fallout could drift over your area within a day or two. That is the main reason your chance is higher than in most places.`;
    case 'C1':
      return `You live in ${p?.metro ?? county}. Because of its size and importance, a place like this is treated as a likely target in a large nuclear war. That does not mean an attack is likely, only that the danger here would be greater than in most places.`;
    case 'C2':
      return `You live in ${p?.metro ?? county}: a large metro area. Places like this could be targets in a large nuclear war, but they are less likely targets than the biggest cities and military sites.`;
    case 'D':
      return `You live downwind of ${p?.upwind ?? 'a likely target'}. If that place were attacked, fallout could drift here, so your chance of serious effects is higher than in most rural areas, though still lower than near the target itself.`;
    case 'E':
      return 'None of the places treated as likely targets, and no missile-field fallout path, is close to you. In a large nuclear war, the main effects here would be shortages, power cuts and lost income, not blast or heavy fallout.';
    default:
      return 'The data for where you live does not include its strategic-site group yet, so this uses the average over every county.';
  }
}

function nuclear(p: Place | undefined, county: string, state: string): RareRow {
  const cls: StrategicClass | 'unknown' = p?.strategic ?? 'unknown';
  const f = F_S[cls];
  let local = times(LAMBDA_S, f);
  if (cls === 'A' || state === 'HI' || state === 'GU') local = plus(local, scaled(LAMBDA_L, 0.3 * 0.3));
  if (p && p.uasi > 0) local = plus(local, scaled(LAMBDA_I, p.uasi * 0.1));
  const hemp = times(LAMBDA_S, HEMP_GIVEN_STRATEGIC);
  const national = plus(LAMBDA_S, scaled(LAMBDA_L, 0.5));
  const [severity, reaches] =
    cls === 'A' || cls === 'C1'
      ? [1, 'Life-threatening: blast or heavy fallout near likely targets.']
      : cls === 'E'
        ? [0.3, 'Shortages, power cuts and lost income, not blast or heavy fallout.']
        : cls === 'unknown'
          ? [0.5, 'Life-threatening near likely targets; elsewhere serious disruption, shortages and power cuts.']
          : [0.5, 'Serious disruption: sheltering inside for a day or more against fallout, then shortages and outages.'];
  return {
    id: 'nuclear_attack',
    est: local,
    verb: 'be in a blast zone or under dangerous fallout',
    severity,
    confidence: 'prior',
    sources: ['mock_forecasts', 'mock_strategic_sites', 'mock_nuclear_guidance', 'mock_rare_prior'],
    sub_causes: [
      sub('limited_strike', 'Limited strike on US territory', 'One or a few weapons on US soil, for example a strike on Guam, Hawaii, Alaska or a West Coast base. It adds to your chance only where your county is a plausible target.', [LAMBDA_L[1], LAMBDA_L[2]], ['mock_forecasts', 'mock_rare_prior']),
      sub('nuclear_terrorism', 'A crude nuclear device in a city', 'Heavy damage within about a mile, and everyone within 50 miles told to get inside. It adds to your chance by your metro area’s share of FEMA’s terrorism-preparedness money.', [LAMBDA_I[1], LAMBDA_I[2]], ['mock_forecasts', 'mock_nuclear_guidance', 'mock_rare_prior']),
      sub('emp', 'Electromagnetic pulse (EMP) from a high-altitude burst', 'Only as part of a nuclear attack: a burst high above the country could cut power and phones over a wide area and damage some cars and electronics. A 2019 study found months-long nationwide blackouts unlikely; the claim that most Americans would die is not a published model.', [hemp[1], hemp[2]], ['mock_rare_prior']),
      sub('national_disruption', 'Disruption across the country', 'Shortages, power cuts and lost income even far from any target. Shown for everyone and never added to your local chance.', [national[1], national[2]], ['mock_forecasts', 'mock_rare_prior']),
      sub('use_abroad', 'A nuclear weapon used elsewhere in the world', 'At home: market shocks, shortages and worry, with no fallout of concern. Never added to your local chance.', [1e-3, 2e-2], ['mock_forecasts', 'mock_rare_prior']),
    ],
    location_factor: {
      class: cls,
      label: whyHere(cls, p, county),
      multiplier: [f[1], f[0], f[2]],
      sources: ['mock_strategic_sites', 'mock_rare_prior'],
    },
    if_it_reaches_you: reaches,
    what_it_changes:
      cls === 'E'
        ? 'Nothing beyond your basics.'
        : 'One free step: pick your shelter spot at home and at work (a basement, or the middle of the building away from windows).',
  };
}

function geomagnetic(p: Place | undefined): { row: RareRow; est: Triple } {
  const ratio = p ? p.geomag / GMD_ALPHA_MEAN : 1;
  const conditional: Triple = [Math.min(0.5, GMD_OUTAGE[0] * ratio), Math.min(0.5, GMD_OUTAGE[1] * ratio), Math.min(0.5, GMD_OUTAGE[2] * ratio)];
  const est = times(CARRINGTON, conditional);
  let label = 'The data for where you live does not include its geomagnetic latitude yet, so this uses the national average.';
  let cls = 'unknown';
  if (p) {
    cls = p.geomag >= 0.4 ? 'high' : p.geomag > 0.15 ? 'middle' : 'low';
    const times_ = ratio >= 1.05 ? `about ${String(Number(ratio.toPrecision(2)))} times the national average` : ratio <= 0.95 ? `about ${String(Number(ratio.toPrecision(2)))} of the national average` : 'about the national average';
    label = `Your county is at a ${cls} geomagnetic latitude (about ${p.lat}°). Solar storms drive the strongest currents into long power lines nearer the magnetic pole, so the chance of a long outage here is ${times_}. The ground’s conductivity also matters and is not counted.`;
  }
  return {
    est,
    row: {
      id: 'geomagnetic_storm',
      est,
      verb: 'lose power for days to a severe solar storm',
      severity: 0.5,
      confidence: 'prior',
      sources: ['mock_solar_storms', 'mock_rare_prior'],
      sub_causes: [],
      location_factor: { class: cls, label, multiplier: [ratio, ratio, ratio], sources: ['mock_solar_storms'] },
      if_it_reaches_you: 'Power out for days, and longer where large transformers fail.',
      what_it_changes: 'Nothing beyond your power plan: a solar storm harms long power lines, not phones or radios.',
    },
  };
}

function war(p: Place | undefined): { row: RareRow; est: Triple } {
  const near = p ? ['A', 'C1', 'C2'].includes(p.strategic) : undefined;
  const k: Triple = near === undefined ? [0.71, 0.3, 1] : near ? [1, 1, 1] : [0.3, 0.3, 0.3];
  const est = times(times(WAR, WAR_HOMELAND), k);
  const label =
    near === undefined
      ? 'The data for where you live does not say how near it is to military sites and infrastructure, so this uses the national average.'
      : near
        ? 'Your county has military sites, a big city, a port or a refinery nearby: the kinds of places attacks on infrastructure aim at.'
        : 'Your county is far from military sites, big cities, ports and refineries, so wartime attacks on infrastructure are less likely to reach it.';
  return {
    est,
    row: {
      id: 'war_infrastructure',
      est,
      verb: 'lose power, water or phone service for days to attacks in a war',
      severity: 0.5,
      confidence: 'prior',
      sources: ['mock_forecasts', 'mock_rare_prior'],
      sub_causes: [],
      location_factor: { class: near === undefined ? 'unknown' : near ? 'near' : 'far', label, multiplier: [k[1], k[0], k[2]], sources: ['mock_strategic_sites', 'mock_rare_prior'] },
      if_it_reaches_you: 'Outages of power, water or phones for days, and shortages across the country.',
      what_it_changes: 'Nothing beyond your basics.',
    },
  };
}

function multiMonth(gmd: Triple, warEst: Triple, lower48: boolean, ownRecord: Triple): RareRow {
  const gmdPart = times(gmd, [0.1, 0.02, 0.3]);
  const hempPart: Triple = lower48 ? times(times(LAMBDA_S, HEMP_GIVEN_STRATEGIC), [0.1, 0.02, 0.3]) : [0, 0, 0];
  const warPart = times(warEst, [0.02, 0.005, 0.1]);
  const est = plus(plus(plus(gmdPart, hempPart), warPart), ownRecord);
  return {
    id: 'multi_month_blackout',
    est,
    verb: 'be without power for two months or more',
    severity: 0.8,
    confidence: 'medium',
    sources: ['mock_solar_storms', 'mock_eaglei_outages', 'mock_rare_prior'],
    sub_causes: [
      sub('solar_storm', 'A severe solar storm', 'Long outages where large transformers fail; most places would be back within days.', [gmdPart[1], gmdPart[2]], ['mock_solar_storms', 'mock_rare_prior']),
      sub('emp', 'EMP from a nuclear attack', 'Only as part of a nuclear attack; a 2019 study found months-long nationwide blackouts unlikely.', [hempPart[1], hempPart[2]], ['mock_rare_prior']),
      sub('war', 'Attacks on the grid in a war', 'Cyberattacks, sabotage or missiles on power plants and lines.', [warPart[1], warPart[2]], ['mock_forecasts', 'mock_rare_prior']),
      sub('own_record', 'Your county’s own outage record', 'Storms and failures like the ones in your county’s outage records, with the long tail the records cannot show yet.', [ownRecord[1], ownRecord[2]], ['mock_eaglei_outages']),
    ],
    if_it_reaches_you: 'No power for months: water, heat, medicine and money all affected. Puerto Rico waited 328 days after Hurricane Maria.',
    what_it_changes: 'Nothing to stockpile for months. The long-horizon section lists what helps instead: a water filter with a water source, a way to cook, sanitation.',
  };
}

function cbrn(p: Place | undefined, county: string): RareRow {
  const funded = p && p.uasi > 0;
  const est: Triple = funded ? times(scaled(CBRN_US, p.uasi), CBRN_METRO_SHARE) : p ? CBRN_NON_UASI : [CBRN_NON_UASI[0], CBRN_NON_UASI[1], CBRN_US[2] * 0.05 * CBRN_METRO_SHARE[2]];
  const money = p && p.uasi * 100 >= 0.95 ? `about ${String(roundSig2(p.uasi * 100))} in 100` : `about ${String(roundSig2((p?.uasi ?? 0) * 1000))} in 1,000`;
  const label = funded
    ? `The ${p.uasiArea ?? county} urban area is one of the 44 urban areas FEMA funds for terrorism preparedness, chosen by the risk the Department of Homeland Security assigns them. It gets ${money} of the money, which is how this estimate weighs your area.`
    : p
      ? `${county} is outside the 44 urban areas FEMA funds for terrorism preparedness, so an attack that closes your area is much less likely than in the big cities.`
      : 'The data for where you live does not say whether it is in one of the 44 urban areas FEMA funds for terrorism preparedness, so the range runs from a small town to a large city.';
  const base = CBRN_US[0] * CBRN_METRO_SHARE[0];
  return {
    id: 'cbrn_attack',
    est,
    verb: 'be told to stay inside, or to collect medicine at a public site, after a chemical, biological or radiological attack',
    severity: 0.5,
    confidence: 'prior',
    sources: ['mock_uasi', 'mock_rare_prior'],
    sub_causes: [
      sub('chemical', 'Chemical', 'About 3 in 4 such incidents worldwide involve chemicals: shelter inside for hours, or leave the immediate area if told to.', undefined, ['mock_rare_prior']),
      sub('biological', 'Biological', 'A germ spread on purpose (the anthrax letters of 2001): stay home, and collect preventive medicine at a public site if told to.', undefined, ['mock_rare_prior']),
      sub('radiological', 'Radiological ("dirty bomb")', 'Shelter inside for hours; the area may stay closed for weeks. None has caused mass casualties anywhere.', undefined, ['mock_rare_prior']),
    ],
    location_factor: {
      class: funded ? (p.uasi >= 0.05 ? 'uasi_top' : p.uasi >= 0.02 ? 'uasi_large' : 'uasi') : p ? 'not_funded' : 'unknown',
      label,
      multiplier: funded ? [p.uasi, p.uasi, p.uasi] : [est[1] / base, est[0] / base, est[2] / base],
      sources: ['mock_uasi', 'mock_rare_prior'],
    },
    if_it_reaches_you: 'An order to stay inside for hours, closed buildings, or medicine handed out at public sites.',
    what_it_changes: 'Nothing beyond your basics: your three-day supplies cover sheltering inside.',
  };
}

const WORLDWIDE: RareRow[] = [
  {
    id: 'severe_pandemic',
    est: [1.5e-3, 5e-4, 5e-3],
    verb: 'live through a pandemic far deadlier than COVID-19',
    severity: 0.8,
    confidence: 'prior',
    sources: ['mock_pandemic_stayhome', 'mock_rare_prior'],
    sub_causes: [
      sub('natural_1918_class', 'A natural pandemic as deadly as 1918', 'The 1918 flu killed about 675,000 Americans. Records of pandemics over four centuries give the chance.', undefined, ['mock_pandemic_stayhome']),
      sub('engineered', 'An engineered germ', 'A germ made or changed on purpose. There is no record to count; it is part of the expert range.', undefined, ['mock_rare_prior']),
    ],
    if_it_reaches_you: 'Months of disruption, strained hospitals and lost income.',
    what_it_changes: 'Nothing new: the pandemic row already sizes your food and medicine.',
  },
  {
    id: 'vei7_eruption',
    est: [1.8e-3, 8e-4, 4e-3],
    verb: 'live through a year or two of higher food prices after a very large eruption somewhere in the world',
    severity: 0.3,
    confidence: 'medium',
    sources: ['mock_rare_prior'],
    sub_causes: [
      sub('yellowstone', 'A Yellowstone super-eruption', 'Devastation nearby and ash across much of the country. The US Geological Survey puts it at about 1 in 730,000 a year.', [5e-7, 3e-6], ['mock_rare_prior']),
    ],
    if_it_reaches_you: 'A year or two of higher food prices and some shortages.',
    what_it_changes: 'Nothing beyond a two-week pantry.',
  },
  {
    id: 'financial_crisis',
    est: [2e-3, 5e-4, 1e-2],
    verb: 'have banks close for three days or more in a financial crisis',
    severity: 0.3,
    confidence: 'prior',
    sources: ['mock_rare_prior'],
    sub_causes: [
      sub('bank_failure', 'Your own bank fails', 'Insured deposits (up to $250,000 per person at each bank) move to another bank, usually within days. About 23 banks fail in a typical year, most of them small.', undefined, ['mock_rare_prior']),
    ],
    if_it_reaches_you: 'Cards and bank transfers stop for days.',
    what_it_changes: 'Keep some cash in small bills (already in your plan) and a second account at another bank (free).',
  },
];

const RARE_BUCKETS: Record<string, BucketId[]> = {
  geomagnetic_storm: ['power', 'water_out', 'medication', 'comms'],
  vei7_eruption: ['supplies', 'income'],
  nuclear_attack: ['power', 'supplies', 'medication', 'comms', 'evacuate', 'income'],
  multi_month_blackout: ['power', 'water_out', 'medication'],
  war_infrastructure: ['power', 'water_out', 'supplies', 'comms'],
  cbrn_attack: ['supplies', 'medication', 'comms', 'evacuate'],
  severe_pandemic: ['supplies', 'medication', 'income'],
  financial_crisis: ['comms', 'income'],
  mass_violence: ['medical_emergency', 'security'],
};

function sig(x: number): number {
  return x === 0 ? 0 : Number(x.toPrecision(4));
}

/**
 * The nine rare families for this household, most likely here first (by the middle of the range,
 * never shown), each with its anchor against the household's own ranked list.
 */
export function rareFamilies(input: PlanInput, loc: LocationResolved, ranked: readonly RankedHazard[], people: number): RankedHazard[] {
  const years = input.dials.horizon_years;
  const p = placeOf(loc.county_fips);
  const county = `${loc.county_name}, ${loc.state_abbr}`;
  const lower48 = !['AK', 'HI', 'PR', 'GU', 'VI', 'AS', 'MP'].includes(loc.state_abbr);
  const gmd = geomagnetic(p);
  const w = war(p);
  // The county's own chance of a power cut of two months or more (rr-consequence's curve at 60 days).
  const own: Triple = [1e-5, 1e-6, 6e-5];
  const rows: RareRow[] = [
    gmd.row,
    WORLDWIDE[1]!,
    nuclear(p, county, loc.state_abbr),
    multiMonth(gmd.est, w.est, lower48, own),
    w.row,
    cbrn(p, county),
    WORLDWIDE[0]!,
    WORLDWIDE[2]!,
    {
      id: 'mass_violence',
      est: scaled([3e-7, 1e-7, 1e-6], Math.max(1, people)),
      verb: 'have someone hurt in a mass shooting or bombing',
      severity: 0.9,
      confidence: 'medium',
      sources: ['mock_active_shooter', 'mock_rare_prior'],
      sub_causes: [],
      location_factor: {
        class: 'not_modelled',
        label: 'The data are too sparse to say where this is more likely, so the chance is the same everywhere.',
        multiplier: [1, 1, 1],
        sources: ['mock_active_shooter'],
      },
      if_it_reaches_you: 'Injury or death.',
      what_it_changes: 'Nothing to buy. Two free steps: know "run, hide, fight", and learn to stop bleeding.',
    },
  ];
  const out = rows.map((r): RankedHazard => {
    const [mid, lo, hi] = r.est;
    const seed: HazardSeed = { id: r.id, rate: mid, spread: 1, severity: r.severity, confidence: r.confidence, sources: r.sources, buckets: RARE_BUCKETS[r.id] ?? [], what: r.verb, rare: true };
    const profile: HazardProfile = {
      id: r.id,
      name: hazardName(r.id),
      tier: hazardTier(r.id),
      display: 'rare_catastrophic',
      rate_per_year: sig(mid),
      rate_range: [sig(lo), sig(hi)],
      annual_probability: sig(1 - Math.exp(-mid)),
      probability_range: [sig(1 - Math.exp(-lo)), sig(1 - Math.exp(-hi))],
      severity: r.severity,
      climate_multiplier: 1,
      confidence: r.confidence,
      sources: r.sources,
      frequency_sentence: rangeSentence(r.verb, lo, hi, years),
      buckets: RARE_BUCKETS[r.id] ?? [],
      family: r.id,
      range_only: true,
      if_it_reaches_you: r.if_it_reaches_you,
      what_it_changes: r.what_it_changes,
    };
    if (r.sub_causes.length) profile.sub_causes = r.sub_causes;
    if (r.location_factor) profile.location_factor = r.location_factor;
    const anchor = anchorFor(hi, ranked, years);
    if (anchor) profile.anchor_sentence = anchor;
    return { profile, seed, rate: mid };
  });
  return out.sort((a, b) => b.rate - a.rate || a.seed.id.localeCompare(b.seed.id));
}

// ---------------------------------------------------------------------------------------------
// The new ranked hazards (contract v2) and the sub-causes on ranked cards
// ---------------------------------------------------------------------------------------------

/** Ranked hazards whose rate rests on stacked expert judgement: shown as a range only. */
export const RANGE_ONLY_RANKED: ReadonlySet<HazardId> = new Set(['attack_disruption']);

interface HouseholdFacts {
  n: number;
  dailyRx: number;
  owner: boolean;
  basement: boolean;
  setting: PlanInput['location']['setting'];
  adults: number;
}

/** The v2 ranked hazards that apply to this household, as mock seeds (rr-hazards personal.rs, societal.rs, natural.rs). */
export function v2Seeds(input: PlanInput, f: HouseholdFacts, loc: LocationResolved): HazardSeed[] {
  const p = placeOf(loc.county_fips);
  const seeds: HazardSeed[] = [
    { id: 'water_damage', rate: 0.015 * (f.basement ? 1.3 : 1), spread: 1.5, severity: 0.25, confidence: 'medium', sources: ['mock_insurance'], buckets: ['water_out', 'home_loss'], what: 'have a burst pipe or leak that damages the home' },
    { id: 'wildfire_smoke', rate: 0.17 * (p?.smokeDays ?? 2), spread: 1.6, severity: 0.2, confidence: 'medium', sources: ['mock_smoke_days'], buckets: ['medical_emergency', 'clean_air'], what: 'have days of wildfire smoke thick enough to keep windows shut' },
    { id: 'network_outage', rate: 0.3, spread: 2, severity: 0.1, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['comms', 'medical_emergency'], what: 'lose phone or internet service for a day or more' },
    { id: 'arrest_or_detention', rate: 0.023 * Math.max(1, f.adults), spread: 2, severity: 0.4, confidence: 'medium', sources: ['mock_arrests'], buckets: ['income', 'home_loss'], what: 'have a household member arrested or detained' },
  ];
  if (f.dailyRx > 0) {
    seeds.push({ id: 'drug_shortage', rate: 0.05 * f.dailyRx, spread: 2, severity: 0.3, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['medication'], what: 'find a daily medicine short at the pharmacy' });
  }
  if (!f.owner) {
    seeds.push({ id: 'eviction', rate: p?.eviction !== undefined ? p.eviction * 0.4 : 0.023, spread: 1.6, severity: 0.7, confidence: 'medium', sources: ['mock_eviction'], buckets: ['evacuate', 'income', 'home_loss'], what: 'face an eviction judgment' });
  }
  if ((input.finances.benefits ?? []).length > 0) {
    seeds.push({ id: 'benefit_interruption', rate: 0.077, spread: 1.5, severity: 0.4, confidence: 'prior', sources: ['mock_societal_prior'], buckets: ['supplies', 'income'], what: 'have government pay or benefits stop for a week or more' });
  }
  if (p && p.uasi > 0) {
    seeds.push({ id: 'attack_disruption', rate: 8.5e-4 * (p.uasi / 0.028), spread: 4, severity: 0.3, confidence: 'prior', sources: ['mock_uasi', 'mock_societal_prior'], buckets: ['supplies', 'comms', 'get_home', 'security'], what: 'have an attack or threat close their area for a day or more' });
  }
  if (p?.dustStorms) {
    seeds.push({ id: 'dust_storm', rate: p.dustStorms, spread: 1.8, severity: 0.2, confidence: 'medium', sources: ['mock_nri_county'], buckets: ['supplies', 'get_home', 'clean_air'], what: 'be caught in a dust storm that closes roads' });
  }
  if (p?.karst && p.karst >= 0.5) {
    seeds.push({ id: 'sinkhole', rate: 4e-4, spread: 3, severity: 0.4, confidence: 'low', sources: ['mock_nri_county'], buckets: ['evacuate', 'home_loss'], what: 'have a sinkhole damage the home or street' });
  }
  return seeds;
}

/** Named sub-causes on ranked cards (rr-hazards subcauses.rs): notes, with a range where one is known. */
const SUB_CAUSES: Partial<Record<HazardId, SubCause[]>> = {
  riverine_flooding: [
    sub('flash_flood', 'Flash flood', 'Water rises in minutes on roads and near creeks; most flood deaths are people driving into it. Counted in the flood rate; turn around, don’t drown.', [0.001, 0.05], ['mock_nri_county', 'mock_rare_prior']),
    sub('urban_basement_flood', 'Basement or street flooding in heavy rain', 'Drains overflow and water or sewage comes up into basements outside the mapped flood zone. It is the outside-the-zone part of the flood rate, higher with a basement.', [0.0005, 0.005], ['mock_insurance', 'mock_rare_prior']),
    sub('levee_failure', 'Levee failure or overtopping', 'Counted on the dam and levee card, not here: leveed land is mapped outside the high-risk flood zone.', undefined, ['mock_nri_county']),
  ],
  strong_wind: [sub('derecho', 'Derecho', 'A fast line of storms with hurricane-force gusts over hundreds of miles; outages can last days. Counted in the windstorm rate.', undefined, ['mock_nri_county'])],
  winter_weather: [
    sub('blizzard_lake_effect', 'Blizzard or lake-effect snow', 'Whiteouts or very deep snow close roads for days. Counted in the winter-storm rate.', undefined, ['mock_nri_county']),
    sub('snow_load_collapse', 'Roof damage under snow', 'Heavy snow or ice loads a roof until it sags or fails. Counted in the winter-storm rate; clear flat roofs early.', undefined, ['mock_ready_gov_kit']),
  ],
  hurricane: [sub('inland_tropical_flood', 'Inland flooding from a tropical storm', 'A weakening hurricane can drop a foot or more of rain far inland (Helene in western North Carolina, 2024), cutting water service for weeks.', undefined, ['mock_hurricane_climatology'])],
  heat_wave: [sub('heat_blackout_compound', 'A blackout during a heat wave', 'When the power fails in extreme heat, air conditioning fails for everyone at once. It is the most dangerous combination; where it matters most it is a named scenario.', undefined, ['mock_heat_projections'])],
  cold_wave: [sub('cold_gas_curtailment', 'Rolling blackouts in extreme cold', 'Gas supply and power plants fail in deep cold and the grid cuts power in rotation or for days (Texas, February 2021). Already in the county’s outage record.', undefined, ['mock_eaglei_outages'])],
  grid_failure: [
    sub('grid_physical_attack', 'Attack on power substations', 'Gunfire or sabotage at substations can cut power to a county for days (Moore County, North Carolina, 2022). Most reported attacks cause no outage.', undefined, ['mock_societal_prior']),
    sub('grid_cyberattack', 'Cyberattack on the grid', 'Hackers switched off parts of Ukraine’s grid in 2015; no US case has cut power.', undefined, ['mock_societal_prior']),
  ],
  house_fire: [
    sub('co_poisoning', 'Carbon monoxide poisoning', 'Carbon monoxide from a furnace, generator or car builds up indoors, most often after storms when generators run too close to the house. Shown beside fires, not added to them; a CO alarm is the defence.', [7e-5, 3e-4], ['mock_fire_safety']),
    sub('battery_fire', 'Lithium battery fire', 'E-bike, scooter and power-bank batteries can catch fire while charging; charge them where you can see them.', undefined, ['mock_fire_safety']),
  ],
  medical_emergency: [sub('falls_older_adults', 'Falls', 'Falls are a leading reason older adults need emergency care; clear floors and light the way at night.', undefined, ['mock_ed_visits'])],
  water_damage: [sub('sewer_backup', 'Sewer backup', 'Sewage backs up through basement drains; standard home policies leave it out unless you add sewer-backup cover.', undefined, ['mock_insurance'])],
  job_loss: [sub('recession_layoffs', 'Recession layoffs', 'Layoffs come in waves in a recession, and finding work takes longer. Counted in the job-loss rate.', undefined, ['mock_bls_job_loss'])],
  earthquake: [sub('fire_following_earthquake', 'Fire after an earthquake', 'Broken gas lines and wiring start fires while water mains are broken. Know how to shut off your gas.', undefined, ['mock_cascadia_usgs'])],
};

/** The sub-causes to show on a ranked card, or undefined. */
export function subCausesFor(id: HazardId): SubCause[] | undefined {
  return SUB_CAUSES[id];
}

// ---------------------------------------------------------------------------------------------
// Also checked
// ---------------------------------------------------------------------------------------------

const PLURAL: Partial<Record<HazardId, string>> = {
  avalanche: 'avalanches',
  coastal_flooding: 'coastal floods',
  drought: 'droughts',
  earthquake: 'earthquakes',
  hail: 'hailstorms',
  hurricane: 'hurricanes',
  ice_storm: 'ice storms',
  landslide: 'landslides',
  lightning: 'lightning strikes',
  riverine_flooding: 'floods from rivers or heavy rain',
  strong_wind: 'windstorms',
  tornado: 'tornadoes',
  tsunami: 'tsunamis',
  volcanic_activity: 'volcanic eruptions',
  wildfire: 'wildfires',
  winter_weather: 'winter storms',
  dust_storm: 'dust storms',
  sinkhole: 'sinkholes',
  dam_failure: 'dam or levee failures',
};

function perYearWords(rate: number): string {
  if (rate <= 0) return 'none recorded here';
  if (rate >= 1e-6) return `about ${oneInEngine(rate)} a year`;
  return 'fewer than 1 in 1,000,000 a year';
}

/** The hazards checked for this place and found too rare to list, with their yearly rates. */
function alsoChecked(register: readonly RankedHazard[]): [string, number][] {
  const present = new Set<string>(register.map((h) => h.seed.id));
  const checked: [string, number][] = (Object.keys(PLURAL) as HazardId[])
    .filter((id) => !present.has(id))
    .map((id) => [PLURAL[id]!, 0]);
  checked.push(['an asteroid or comet impact', 3e-9], ['a Yellowstone super-eruption', 1 / 730_000]);
  return checked;
}

/**
 * The binder's "Also checked" line (rr-plan, the "Which checklist?" page): the same hazards as the
 * note, by name only.
 */
export function alsoCheckedLine(register: readonly RankedHazard[]): string {
  const names = alsoChecked(register).map(([name]) => name);
  const joined = names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names.at(-1)!}` : names[0]!;
  return `Also checked, and too unlikely here to need a page: ${joined}.`;
}

/**
 * The "Also checked" note (rr-hazards lib.rs): the natural hazards checked for this place and
 * found under 1 in 100,000 a year (none recorded, in the mock), then the rare sub-rows too small to
 * show, each with its rate.
 */
export function alsoCheckedNote(register: readonly RankedHazard[]): string {
  const list = alsoChecked(register).map(([name, rate]) => `${name} (${perYearWords(rate)})`);
  const joined = list.length > 1 ? `${list.slice(0, -1).join(', ')} and ${list.at(-1)!}` : list[0]!;
  return `Also checked, and under 1 in 100,000 a year here: ${joined}.`;
}

// ---------------------------------------------------------------------------------------------
// The stress test (rr-consequence: the worst event in the region's record)
// ---------------------------------------------------------------------------------------------

interface WorstPower {
  event: string;
  date: string;
  region: string;
  /** Share of all customers still out at 1, 3, 7, 14 and 30 days (peak × share of the peak). */
  out: [number, number, number, number, number];
}

/** The data pack's `core/outage_stress.csv` rows for the fixture counties (peak × share of the peak). */
const WORST_POWER: Record<string, WorstPower> = {
  philadelphia: { event: 'Wind and thunderstorms, March 2018', date: '2018-03-02', region: "your region's records (where it was worst, about 40 miles away)", out: [0.312, 0.147, 0.057, 0, 0] },
  coos_bay: { event: 'Power cut, cause not recorded, February 2021', date: '2021-02-13', region: "your region's records (where it was worst, about 150 miles away)", out: [0.159, 0.137, 0.041, 0, 0] },
  miami: { event: 'Hurricane Ian', date: '2022-09-28', region: "your region's records (where it was worst, about 130 miles away)", out: [0.877, 0.748, 0.438, 0, 0] },
  hays: { event: 'Ice storm, December 2016', date: '2016-12-16', region: "your region's records (where it was worst, about 90 miles away)", out: [0.52, 0.31, 0.08, 0.01, 0] },
  phoenix: { event: 'Winter storm, January 2023', date: '2023-01-01', region: "your region's records (where it was worst, about 100 miles away)", out: [0.0004, 0.0003, 0, 0, 0] },
  sugar_land: { event: 'Hurricane Harvey', date: '2017-08-28', region: "your region's records (where it was worst, about 120 miles away)", out: [0.848, 0.828, 0.245, 0, 0] },
  chicago: { event: 'Ice storm, February 2023', date: '2023-02-22', region: "your region's records (where it was worst, about 150 miles away)", out: [0.546, 0.409, 0.027, 0, 0] },
  southeast: { event: 'Hurricane Helene', date: '2024-09-25', region: "your county's records", out: [0.922, 0.778, 0.694, 0.221, 0] },
};

interface WorstWater {
  event: string;
  date: string;
  place: string;
  median: number;
  p90: number;
}

/**
 * Documented water failures by state, for homes on public water: the rows of rr-consequence's
 * water events table (`effects.toml` in agent/consequence2) for the states the mock knows.
 */
const WORST_WATER: Record<string, { water_out?: WorstWater; water_boil?: WorstWater }> = {
  TX: { water_boil: { event: 'Winter Storm Uri', date: '2021-02-17', place: 'Austin, Texas', median: 6, p90: 6 } },
  NC: {
    water_out: { event: 'Hurricane Helene', date: '2024-09-27', place: 'Asheville, North Carolina', median: 18, p90: 21 },
    water_boil: { event: 'Hurricane Helene', date: '2024-09-27', place: 'Asheville, North Carolina', median: 52, p90: 52 },
  },
  MS: {
    water_out: { event: 'the Jackson water crisis', date: '2022-08-29', place: 'Jackson, Mississippi', median: 7, p90: 10 },
    water_boil: { event: 'the Jackson water crisis', date: '2022-07-29', place: 'Jackson, Mississippi', median: 48, p90: 48 },
  },
  NY: { water_out: { event: 'Superstorm Sandy', date: '2012-10-29', place: 'Long Beach, New York', median: 13, p90: 13 } },
  OH: { water_boil: { event: 'the August 2003 Northeast blackout', date: '2003-08-14', place: 'Cleveland, Ohio', median: 3, p90: 3 } },
};

/** The stress test for a duration bucket, where the mock has a record for the place. */
export function stressTestFor(profileKey: string, state: string, bucket: BucketId, target: number, well: boolean): StressTest | undefined {
  if (target <= 0) return undefined;
  if (bucket === 'power') {
    const w = WORST_POWER[profileKey];
    if (!w) return undefined;
    const days = [1, 3, 7, 14, 30];
    const points: [number, number][] = days.map((d, i) => [d, w.out[i]!]);
    return {
      event: w.event,
      date: w.date,
      region: w.region,
      share_out_at_days: points,
      covered_by_target: shareAfter(points, target) <= 0.1,
      sources: ['mock_eaglei_outages'],
    };
  }
  if ((bucket === 'water_out' || bucket === 'water_boil') && !well) {
    const e = WORST_WATER[state]?.[bucket];
    if (!e) return undefined;
    return {
      event: e.event,
      date: e.date,
      region: e.place,
      share_out_at_days: e.p90 > e.median ? [[e.median, 0.5], [e.p90, 0.1]] : [[e.median, 1]],
      covered_by_target: target >= e.p90,
      sources: ['mock_boil_notices'],
    };
  }
  return undefined;
}

/** rr-consequence's `share_out_after` (lib/targets.ts has the same reading for the stress line). */
function shareAfter(points: [number, number][], days: number): number {
  const [d0, s0] = points[0]!;
  if (days <= d0) return 1 - (1 - s0) * Math.min(1, days / d0);
  for (let i = 1; i < points.length; i++) {
    const [a, sa] = points[i - 1]!;
    const [b, sb] = points[i]!;
    if (days <= b) return sa + ((sb - sa) * (Math.log(days) - Math.log(a))) / (Math.log(b) - Math.log(a));
  }
  return points.at(-1)![1];
}

// ---------------------------------------------------------------------------------------------
// Savings and validation
// ---------------------------------------------------------------------------------------------

/**
 * The first savings step (rr-budget): one month of expenses or $500, whichever is smaller, and the
 * plan month the suggested saving reaches it, once the supplies plan is done. Absent when it is
 * already saved or no saving is suggested.
 */
export function firstMilestone(track: SavingsTrack, expenses: number, doneMonth: number | undefined): SavingsMilestone | undefined {
  const usd = Math.min(expenses, 500);
  const saved = track.current_months * expenses;
  if (saved >= usd || track.monthly_suggestion_usd <= 0 || expenses <= 0) return undefined;
  return {
    months: Math.round((usd / expenses) * 100) / 100,
    usd,
    by_month: (doneMonth ?? 0) + Math.ceil((usd - saved) / track.monthly_suggestion_usd),
  };
}

/** `EngineInfo.validation`: the tally of the frozen backtest (docs/VALIDATION.md), as bundled with the engine. */
export function validationSummary(dataPack: string): ValidationSummary {
  return { events_tested: 22, covered: 6, partial: 9, short: 6, not_modelled: 1, data_pack: dataPack, url_anchor: '#/validation' };
}
