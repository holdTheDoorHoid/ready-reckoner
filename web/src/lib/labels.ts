/**
 * Plain words for the choices in the interview and on the dials. Engine ids never reach the
 * screen; the plain names of hazards, buckets, items and tiers come from `catalogue()`, and these
 * cover the household's own answers.
 */
import type {
  AccessNeed,
  AgeBand,
  BackupPower,
  Benefit,
  ClimateHorizon,
  CommuteMode,
  CookingFuel,
  Cooling,
  Fuel,
  Heating,
  Holds,
  HousingKind,
  IncomeStability,
  Mobility,
  RawWaterSource,
  ReturnPeriod,
  Setting,
  SimplePoweredDevice,
  Stage,
  Tenure,
  Wastewater,
  WaterLevel,
  WaterSource,
  WaterSystemRecord,
} from '../engine/types';

export interface Choice {
  label: string;
  help?: string;
}

export const SETTING: Record<Setting, Choice> = {
  urban: { label: 'City', help: 'Apartments, rowhouses or small lots, with shops nearby.' },
  suburban: { label: 'Suburb or town', help: 'Houses with yards, and shops a short drive away.' },
  rural: { label: 'Countryside', help: 'Farms or open land, and a long drive to stores.' },
};

export const HOUSING_KIND: Record<HousingKind, Choice> = {
  apartment_high_rise: { label: 'Apartment in a tall building', help: 'A building with elevators.' },
  apartment_low_rise: { label: 'Apartment in a low building', help: 'Stairs, no elevators needed.' },
  rowhouse: { label: 'Rowhouse or townhouse', help: 'Shares a wall with neighbours.' },
  detached: { label: 'House', help: 'Stands on its own.' },
  mobile_home: { label: 'Mobile or manufactured home' },
  rural_property: { label: 'Farm or rural property' },
};

export const TENURE: Record<Tenure, Choice> = {
  own: { label: 'We own it' },
  rent: { label: 'We rent' },
};

export const WATER: Record<WaterSource, Choice> = {
  municipal: { label: 'City or town water' },
  well: { label: 'Private well', help: 'Most wells need electricity to pump.' },
};

export const SEWER: Record<Wastewater, Choice> = {
  sewer: { label: 'City sewer' },
  septic: { label: 'Septic tank' },
};

export const HEATING: Record<Heating, Choice> = {
  gas: { label: 'Gas furnace or boiler' },
  electric_resistance: { label: 'Electric heaters or baseboards' },
  heat_pump: { label: 'Heat pump' },
  oil: { label: 'Oil furnace or boiler' },
  propane: { label: 'Propane' },
  wood: { label: 'Wood stove' },
  district: { label: 'Heat supplied by the building' },
  none: { label: 'No heating' },
};

export const COOLING: Record<Cooling, Choice> = {
  central: { label: 'Central air conditioning' },
  window: { label: 'Window or portable units' },
  none: { label: 'No air conditioning' },
};

export const BACKUP: Record<BackupPower, Choice> = {
  none: { label: 'None' },
  power_station: { label: 'Portable power station', help: 'A rechargeable battery with outlets.' },
  generator: { label: 'Generator' },
  solar_battery: { label: 'Solar panels with a battery' },
};

export const AGE: Record<AgeBand, Choice> = {
  infant: { label: 'Baby (under 1)' },
  toddler: { label: 'Toddler (1 to 3)' },
  child: { label: 'Child (4 to 12)' },
  teen: { label: 'Teenager (13 to 17)' },
  adult: { label: 'Adult (18 to 64)' },
  senior: { label: 'Older adult (65 or over)' },
};

export const MOBILITY: Record<Mobility, Choice> = {
  none: { label: 'Gets around without help' },
  limited: { label: 'Needs some help', help: 'Stairs or long walks are hard.' },
  wheelchair: { label: 'Uses a wheelchair' },
};

export const DEVICE: Record<SimplePoweredDevice | 'other', Choice> = {
  none: { label: 'None' },
  cpap: { label: 'CPAP or BiPAP machine' },
  oxygen: { label: 'Oxygen concentrator' },
  other: { label: 'Another powered device' },
};

export const COMMUTE_MODE: Record<CommuteMode, Choice> = {
  car: { label: 'Car' },
  transit: { label: 'Bus or train' },
  walk: { label: 'Walking' },
  bike: { label: 'Bike' },
};

export const FUEL: Record<Fuel, Choice> = {
  gas: { label: 'Gasoline' },
  diesel: { label: 'Diesel' },
  hybrid: { label: 'Hybrid' },
  ev: { label: 'Electric' },
};

export const STABILITY: Record<IncomeStability, Choice> = {
  very_stable: { label: 'Very steady', help: 'Tenured, public sector, or a pension.' },
  stable: { label: 'Steady paycheck' },
  variable: { label: 'Changes from month to month' },
  seasonal: { label: 'Seasonal work' },
  gig: { label: 'Gig or freelance work' },
};

// Contract v2 answers (web-interview) -------------------------------------------------------

/**
 * Needs that change how a person gets warnings, help or care in an emergency, grouped as
 * planners group them (CMIST: communication, maintaining health, independence, support and
 * safety, transportation). `short` is the phrase used inside a sentence ("person 3 is deaf or
 * hard of hearing").
 */
export const ACCESS_NEED: Record<AccessNeed, Choice & { short: string }> = {
  hearing: { label: 'Deaf or hard of hearing', help: 'May not hear a smoke alarm, a siren or a phone alert.', short: 'is deaf or hard of hearing' },
  vision: { label: 'Blind or low vision', help: 'May not see a text alert or read a printed notice.', short: 'is blind or has low vision' },
  limited_english: { label: 'Limited English', help: 'Alerts and officials may not use their language.', short: 'has limited English' },
  cognitive: { label: 'Memory, thinking or understanding', help: 'For example dementia, or a developmental or mental health condition.', short: 'has trouble with memory, thinking or understanding' },
  supervision: { label: 'Needs someone with them', help: 'Can’t be left alone, or needs help to leave.', short: 'needs someone with them' },
  service_animal: { label: 'Has a service animal', help: 'A trained helper animal, not a pet.', short: 'has a service animal' },
  dialysis: { label: 'Needs dialysis', help: 'Treatment can’t wait long if the usual center closes.', short: 'needs dialysis' },
  home_health: { label: 'Gets home health care', help: 'A nurse or aide who visits, or equipment a service looks after.', short: 'gets home health care' },
};

/** The CMIST headings the access needs are shown under, in order. */
export const ACCESS_NEED_GROUPS: { title: string; needs: AccessNeed[] }[] = [
  { title: 'Getting warnings', needs: ['hearing', 'vision', 'limited_english'] },
  { title: 'Support and safety', needs: ['cognitive', 'supervision', 'service_animal'] },
  { title: 'Care that can’t wait', needs: ['dialysis', 'home_health'] },
];

export const COOKING: Record<CookingFuel, Choice> = {
  gas: { label: 'Gas or propane stove', help: 'Can often still boil water in a power cut, while the gas flows.' },
  electric: { label: 'Electric stove' },
  induction: { label: 'Induction cooktop' },
  none: { label: 'No stove', help: 'A microwave or hot plate only, or nothing.' },
};

export const RAW_WATER: Record<RawWaterSource, Choice> = {
  none: { label: 'None nearby' },
  well: { label: 'A well', help: 'One you could draw from without power, with a hand pump or bucket.' },
  surface_nearby: { label: 'A river, lake, pond or creek', help: 'Within a short walk or drive.' },
  rain_barrel: { label: 'A rain barrel or cistern' },
  neighbour_well: { label: 'A neighbour’s well', help: 'Someone who has agreed to share it.' },
};

export const WATER_RECORD: Record<WaterSystemRecord, Choice> = {
  fine: { label: 'No problems we know of' },
  occasional_notices: { label: 'Now and then', help: 'A boil-water notice or a main break every year or two.' },
  frequent_problems: { label: 'Often', help: 'Notices, outages or bad-tasting water several times a year.' },
  unknown: { label: 'Not sure' },
};

export const BENEFIT: Record<Benefit, Choice> = {
  federal_pay: { label: 'Federal pay', help: 'A federal job, or a contract paid by the federal government.' },
  snap_wic: { label: 'SNAP or WIC', help: 'Food benefits.' },
  ssi_ssdi: { label: 'SSI or SSDI', help: 'Supplemental Security Income or Social Security Disability.' },
  va: { label: 'VA benefits', help: 'Veterans’ pay, pension or disability.' },
  unemployment: { label: 'Unemployment benefits' },
};

/** What someone in the trusted circle holds for the household. */
export const HOLD: Record<Holds, Choice & { short: string }> = {
  spare_key: { label: 'A spare key', short: 'a spare key' },
  documents: { label: 'Copies of our papers', short: 'copies of our papers' },
  medical_poa: { label: 'Medical power of attorney', help: 'May talk with doctors for someone who can’t.', short: 'medical power of attorney' },
  backup_codes: { label: 'Backup codes for our accounts', help: 'To sign in without the phone.', short: 'backup codes' },
};

export const STAGE: Record<Stage, Choice> = {
  not_thought_about: { label: "I haven't really thought about it" },
  thinking: { label: "I've been meaning to" },
  have_some_things: { label: 'I have some things put by' },
  have_a_plan: { label: 'I have a plan' },
  maintaining: { label: 'I keep a plan up to date' },
};

const N: Record<ReturnPeriod, number> = { one_in_10: 10, one_in_50: 50, one_in_100: 100, one_in_500: 500 };

export const RETURN_PERIOD: Record<ReturnPeriod, Choice & { jargon: string }> = {
  one_in_10: { label: 'Common disruptions', jargon: '1-in-10' },
  one_in_50: { label: 'Serious', jargon: '1-in-50' },
  one_in_100: { label: 'Very serious', jargon: '1-in-100' },
  one_in_500: { label: 'Rare catastrophes', jargon: '1-in-500' },
};

/**
 * How many ten-year stretches bring something worse than one need's target at this setting:
 * "about 1 of every 10" at 1-in-100. Each target is set for its own need; the needs together run
 * past at least one target more often (see `dialJointSentence`).
 */
export function perNeedWords(rp: ReturnPeriod): string {
  const p = 1 - Math.exp(-10 / N[rp]);
  if (p >= 0.095) return `about ${Math.max(1, Math.round(p * 10))} of every 10`;
  return `about ${Math.max(1, Math.round(p * 100))} of every 100`;
}

/** One dial setting's line: what its targets promise for any one need. */
export function returnPeriodHelp(rp: ReturnPeriod): string {
  return `For any one need, something worse than its target comes in ${perNeedWords(rp)} ten-year stretches.`;
}

/**
 * What the targets mean for all the needs together. The "roughly 1 in 3" was worked out for the
 * usual 1-in-100 setting (round-2 model review, M-04: 32 to 47 of 100 across seven needs), so the
 * other settings say "higher" without a number rather than a figure nobody computed.
 */
export function dialJointSentence(rp: ReturnPeriod): string {
  const joint =
    rp === 'one_in_100'
      ? 'Across all your needs together, the chance that at least one runs out is higher, roughly 1 in 3.'
      : 'Across all your needs together, the chance that at least one runs out is higher.';
  return `${joint} That is why the plan also gives you ways to cope when a target runs out.`;
}

/** The dial sentence (COMMON-P1's canonical wording at the usual 1-in-100 setting). */
export function dialSentence(rp: ReturnPeriod): string {
  return `${returnPeriodHelp(rp)} ${dialJointSentence(rp)}`;
}

/** Start screen and About (the packet prints it from the engine). Canonical wording; do not edit. */
export const STATUS_LINE =
  'Ready Reckoner is an independent, open-source planning aid. It is not official emergency guidance, and not medical, legal or financial advice. Follow instructions from your local officials first.';

/** Under every table that shows the nuclear row, until the location-aware version lands. */
export const NUCLEAR_NOTE =
  'The nuclear figure is the chance of a nuclear catastrophe anywhere in the world, not for your county; a location-aware version is coming.';

export const CLIMATE: Record<ClimateHorizon, Choice> = {
  today: { label: "Today's climate" },
  y2050: { label: 'Around 2050', help: 'Uses climate projections for your region.' },
};

export const WATER_LEVEL: Record<WaterLevel, Choice> = {
  survival: { label: 'Survival', help: 'About 3 litres (¾ gallon) per person a day: drinking only.' },
  basic: { label: 'Basic', help: 'About 1 gallon per person a day: drinking and a little washing. The usual advice.' },
  comfortable: { label: 'Comfortable', help: 'About 15 litres (4 gallons) per person a day: drinking, washing and cleaning.' },
};

export const HORIZONS = [1, 10, 30] as const;

export const KM_PER_MILE = 1.609344;

export function kmToMiles(km: number): number {
  return Math.round((km / KM_PER_MILE) * 10) / 10;
}

export function milesToKm(miles: number): number {
  return Math.round(miles * KM_PER_MILE * 1000) / 1000;
}

/** One sentence that meets the household where it says it is (stage of change); never changes a number. */
export function stageLine(stage: Stage | undefined): string {
  switch (stage) {
    case 'not_thought_about':
      return 'Starting from nothing is normal. The first steps cost nothing.';
    case 'thinking':
      return 'Here is where to start, one small step at a time.';
    case 'have_some_things':
      return 'What you already have counts, and the plan builds on it.';
    case 'have_a_plan':
      return 'This checks your plan against your risks and fills the gaps.';
    case 'maintaining':
      return 'You keep a plan up to date; this shows what is due and what changed.';
    default:
      return '';
  }
}

export const CONFIDENCE_QUESTION =
  'How sure are you that your household could manage on its own for three days if the power, water or stores were out?';

export const CONFIDENCE_SCALE: Record<number, string> = {
  1: 'Not at all sure',
  2: 'A little sure',
  3: 'Fairly sure',
  4: 'Quite sure',
  5: 'Very sure',
};
