/**
 * Hazard profiles for the mock engine. Each profile seeds a hazard register and gives day
 * targets for the seven duration buckets at the four return-period settings, in dial order
 * (1-in-10, 1-in-50, 1-in-100, 1-in-500). Philadelphia and Coos Bay follow the worked examples in
 * docs/research/risk-model.md §8–§9 (rounded to the day ladder); the others are plausible
 * stand-ins. All of it is placeholder data for building the interface.
 */
import type { BucketId, CitationId, DataConfidence, HazardId, TierId } from '../types';

export type DurationBucket = 'power' | 'water_boil' | 'water_out' | 'supplies' | 'thermal' | 'medication' | 'comms';
export const DURATION_BUCKETS: DurationBucket[] = [
  'power',
  'water_boil',
  'water_out',
  'supplies',
  'thermal',
  'medication',
  'comms',
];

/** One value per return-period setting, in dial order. */
export type ByDial = [number, number, number, number];

export interface HazardSeed {
  id: HazardId;
  /** Household-significant events per year, today's climate. */
  rate: number;
  /** rate_range is [rate / spread, rate × spread]. */
  spread: number;
  severity: number;
  /** Multiplier on the rate for "around 2050"; 1 when absent. */
  y2050?: number;
  confidence: DataConfidence;
  sources: CitationId[];
  buckets: BucketId[];
  /** Completes "About N of 100 households like yours will ___ in the next 10 years." */
  what: string;
  eal?: number;
  rare?: boolean;
  /** Share of events that force the household to leave home (for the evacuate bucket). */
  evac?: number;
}

export interface ScenarioSeed {
  id: string;
  name: string;
  applies_because: string;
  default_on: boolean;
  hazard: HazardId;
  sources: CitationId[];
  /** Targets while the scenario is on, for the buckets it changes. */
  with: Partial<Record<DurationBucket, ByDial>>;
  income_with?: ByDial;
  evacuate_with?: EvacuateSeed;
  /** Fixed relief rating while on: [help arrives, mostly restored] in days. */
  relief_with?: Partial<Record<DurationBucket, [number, number]>>;
  relief_sources?: CitationId[];
}

export interface EvacuateSeed {
  notice_hours: [number, number];
  days_away: number;
}

export interface RegionProfile {
  key: string;
  hazards: HazardSeed[];
  targets: Record<DurationBucket, ByDial>;
  /** Months of income gap at each setting, before household adjustments. */
  income: ByDial;
  evacuate: EvacuateSeed;
  /** Help arrives after about this share of the target; service mostly back at about this multiple. */
  relief: { help: number; restore: number; sources: CitationId[] };
  /** Hot climate: more drinking water, fans in the three-day kit. */
  hot: boolean;
  /** Winter matters: warm bedding when heating needs power. */
  cold: boolean;
  /** Heat drives the thermal bucket, so the 2050 setting raises it. */
  heat_driven: boolean;
  scenarios: ScenarioSeed[];
}

const NRI: CitationId[] = ['mock_nri_county'];
const OUTAGES: CitationId[] = ['mock_nri_county', 'mock_eaglei_outages', 'mock_utility_reliability'];
const HEAT: CitationId[] = ['mock_nri_county', 'mock_heat_projections'];
const FLOOD: CitationId[] = ['mock_nri_county', 'mock_climate_multipliers'];

function h(seed: HazardSeed): HazardSeed {
  return seed;
}

const winter = (rate: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'winter_weather', rate, spread: 1.6, severity: 0.35, y2050: 0.9, confidence: 'high', sources: OUTAGES, buckets: ['power', 'supplies', 'thermal', 'comms', 'get_home'], what: 'be snowed in or lose power in a winter storm', ...extra });
const heat = (rate: number, y2050: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'heat_wave', rate, spread: 1.5, severity: 0.45, y2050, confidence: 'high', sources: HEAT, buckets: ['thermal', 'power', 'medication'], what: 'go through a dangerous heat wave', ...extra });
const flood = (rate: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'riverine_flooding', rate, spread: 2, severity: 0.5, y2050: 1.3, confidence: 'medium', sources: FLOOD, buckets: ['home_loss', 'evacuate', 'water_boil', 'power'], what: 'have flooding at or near home', evac: 0.3, ...extra });
const ice = (rate: number): HazardSeed =>
  h({ id: 'ice_storm', rate, spread: 2, severity: 0.4, y2050: 0.9, confidence: 'low', sources: OUTAGES, buckets: ['power', 'thermal', 'supplies'], what: 'lose power in an ice storm' });
const wind = (rate: number): HazardSeed =>
  h({ id: 'strong_wind', rate, spread: 1.5, severity: 0.3, confidence: 'medium', sources: OUTAGES, buckets: ['power', 'comms'], what: 'lose power in a windstorm' });
const hurricane = (rate: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'hurricane', rate, spread: 1.8, severity: 0.7, y2050: 1.1, confidence: 'high', sources: ['mock_hurricane_climatology', 'mock_nri_county'], buckets: ['power', 'water_out', 'supplies', 'evacuate', 'comms', 'home_loss'], what: 'be hit by a hurricane', evac: 0.3, ...extra });
const tornado = (rate: number, severity = 0.7): HazardSeed =>
  h({ id: 'tornado', rate, spread: 2.5, severity, confidence: 'medium', sources: NRI, buckets: ['home_loss', 'power', 'evacuate'], what: 'have a tornado hit close to home', evac: 0.1 });
const quake = (rate: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'earthquake', rate, spread: 3, severity: 0.5, confidence: 'low', sources: NRI, buckets: ['home_loss', 'water_out', 'power'], what: 'feel a damaging earthquake', evac: 0.2, ...extra });
const wildfire = (rate: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'wildfire', rate, spread: 2, severity: 0.35, y2050: 1.4, confidence: 'medium', sources: ['mock_nri_county', 'mock_climate_multipliers'], buckets: ['supplies', 'evacuate', 'thermal'], what: 'have heavy wildfire smoke or a fire nearby', evac: 0.2, ...extra });
const drought = (rate: number, extra: Partial<HazardSeed> = {}): HazardSeed =>
  h({ id: 'drought', rate, spread: 2, severity: 0.25, y2050: 1.25, confidence: 'medium', sources: ['mock_nri_county', 'mock_climate_multipliers'], buckets: ['water_out'], what: 'go through a drought that limits water', ...extra });
const hail = (rate: number): HazardSeed =>
  h({ id: 'hail', rate, spread: 1.6, severity: 0.2, confidence: 'high', sources: NRI, buckets: ['home_loss'], what: 'have hail damage the home or car' });
const cold = (rate: number): HazardSeed =>
  h({ id: 'cold_wave', rate, spread: 1.8, severity: 0.45, y2050: 0.8, confidence: 'medium', sources: NRI, buckets: ['thermal', 'power', 'water_out'], what: 'go through dangerous cold' });
const lightning = (rate: number): HazardSeed =>
  h({ id: 'lightning', rate, spread: 1.6, severity: 0.15, confidence: 'medium', sources: NRI, buckets: ['power', 'fire'], what: 'lose power or appliances to lightning' });
const coastal = (rate: number, severity = 0.35): HazardSeed =>
  h({ id: 'coastal_flooding', rate, spread: 1.8, severity, y2050: 1.5, confidence: 'medium', sources: FLOOD, buckets: ['home_loss', 'evacuate', 'supplies'], what: 'have coastal flooding near home', evac: 0.3 });
const landslide = (rate: number): HazardSeed =>
  h({ id: 'landslide', rate, spread: 2.5, severity: 0.35, confidence: 'low', sources: NRI, buckets: ['supplies', 'home_loss'], what: 'have roads cut by a landslide' });
const tsunami = (rate: number): HazardSeed =>
  h({ id: 'tsunami', rate, spread: 2.5, severity: 0.9, confidence: 'medium', sources: ['mock_tsunami_zone', 'mock_nri_county'], buckets: ['evacuate'], what: 'need to leave for a tsunami warning', evac: 1 });
const volcano = (rate: number): HazardSeed =>
  h({ id: 'volcanic_activity', rate, spread: 3, severity: 0.4, confidence: 'low', sources: NRI, buckets: ['supplies', 'evacuate'], what: 'be affected by ash or volcanic activity', evac: 0.3 });

const DAYS = {
  philadelphia: {
    power: [0.5, 2, 3, 10],
    water_boil: [0.5, 3, 5, 10],
    water_out: [0.5, 2, 3, 14],
    supplies: [3, 7, 10, 30],
    thermal: [0.5, 1, 2, 5],
    medication: [2, 7, 14, 45],
    comms: [0.5, 1, 2, 5],
  },
  coos: {
    power: [1, 3, 3, 7],
    water_boil: [0.5, 2, 3, 7],
    water_out: [2, 7, 14, 30],
    supplies: [3, 7, 10, 21],
    thermal: [0.5, 1, 1, 3],
    medication: [1, 7, 10, 21],
    comms: [0.5, 2, 3, 7],
  },
  miami: {
    power: [1, 3, 5, 10],
    water_boil: [1, 3, 5, 7],
    water_out: [0.5, 2, 3, 7],
    supplies: [3, 5, 7, 14],
    thermal: [1, 2, 3, 7],
    medication: [3, 7, 10, 21],
    comms: [1, 2, 3, 5],
  },
  plains: {
    power: [1, 3, 5, 14],
    water_boil: [0.5, 2, 3, 7],
    water_out: [0.5, 2, 3, 7],
    supplies: [3, 7, 14, 30],
    thermal: [0.5, 2, 3, 7],
    medication: [1, 5, 10, 30],
    comms: [0.5, 2, 3, 7],
  },
  desert: {
    power: [0.5, 1, 2, 5],
    water_boil: [0.5, 2, 3, 7],
    water_out: [0.5, 1, 2, 5],
    supplies: [2, 5, 7, 21],
    thermal: [1, 2, 3, 7],
    medication: [1, 5, 10, 30],
    comms: [0.5, 1, 2, 5],
  },
  gulf: {
    power: [1, 5, 7, 14],
    water_boil: [1, 5, 7, 14],
    water_out: [0.5, 2, 5, 10],
    supplies: [3, 7, 10, 21],
    thermal: [1, 3, 5, 10],
    medication: [2, 7, 14, 30],
    comms: [0.5, 2, 3, 7],
  },
  lakes: {
    power: [0.5, 1, 2, 5],
    water_boil: [0.5, 2, 3, 7],
    water_out: [0.5, 1, 2, 7],
    supplies: [3, 7, 10, 21],
    thermal: [0.5, 1, 2, 5],
    medication: [1, 5, 7, 21],
    comms: [0.5, 1, 2, 5],
  },
  north: {
    power: [1, 3, 7, 21],
    water_boil: [0.5, 2, 5, 10],
    water_out: [0.5, 2, 5, 14],
    supplies: [5, 10, 14, 30],
    thermal: [1, 3, 5, 10],
    medication: [2, 7, 14, 30],
    comms: [1, 3, 5, 14],
  },
  island: {
    power: [1, 3, 7, 21],
    water_boil: [1, 3, 5, 10],
    water_out: [0.5, 2, 5, 14],
    supplies: [3, 10, 14, 30],
    thermal: [0.5, 1, 2, 5],
    medication: [3, 10, 14, 30],
    comms: [1, 3, 5, 10],
  },
} satisfies Record<string, Record<DurationBucket, ByDial>>;

const DEFAULT_RELIEF = { help: 0.34, restore: 1, sources: ['mock_hazus_restoration', 'mock_eaglei_outages'] };

export const PROFILES: Record<string, RegionProfile> = {
  philadelphia: {
    key: 'philadelphia',
    hazards: [
      winter(0.25),
      heat(0.3, 1.8),
      flood(0.03, { eal: 310 }),
      ice(0.05),
      wind(0.15),
      hurricane(0.01, { what: 'be hit by the remains of a hurricane', severity: 0.5 }),
      tornado(0.002, 0.6),
      quake(0.0016),
    ],
    targets: DAYS.philadelphia,
    income: [0.5, 2, 4, 10],
    evacuate: { notice_hours: [0.5, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: true,
    scenarios: [],
  },
  coos_bay: {
    key: 'coos_bay',
    hazards: [
      quake(0.01, {
        severity: 1,
        spread: 2.5,
        confidence: 'medium',
        sources: ['mock_cascadia_usgs', 'mock_oregon_resilience'],
        buckets: ['power', 'water_out', 'supplies', 'medication', 'comms', 'home_loss', 'evacuate'],
        what: 'live through a major Cascadia earthquake',
        eal: 1200,
      }),
      tsunami(0.003),
      wind(0.6),
      winter(0.3, { what: 'lose power or be cut off in a winter storm' }),
      wildfire(0.08),
      flood(0.05, { severity: 0.4 }),
      landslide(0.02),
      coastal(0.02),
      drought(0.1),
    ],
    targets: DAYS.coos,
    income: [0.5, 3, 5, 10],
    evacuate: { notice_hours: [1, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: false,
    scenarios: [
      {
        id: 'cascadia_m9',
        name: 'Plan for a Cascadia earthquake',
        applies_because:
          'Your county is on the Oregon coast, where a magnitude 9 Cascadia earthquake would cut power, water and roads for weeks. Oregon asks households to prepare for at least two weeks.',
        default_on: true,
        hazard: 'earthquake',
        sources: ['mock_cascadia_usgs', 'mock_oregon_resilience'],
        with: {
          power: [1, 5, 14, 180],
          water_boil: [0.5, 2, 5, 14],
          water_out: [2, 14, 45, 180],
          supplies: [3, 10, 21, 60],
          thermal: [0.5, 1, 2, 5],
          medication: [1, 10, 21, 90],
          comms: [0.5, 3, 7, 45],
        },
        income_with: [0.5, 3.5, 6, 15],
        evacuate_with: { notice_hours: [0.25, 12], days_away: 14 },
        relief_with: {
          power: [14, 120],
          water_boil: [14, 90],
          water_out: [14, 365],
          supplies: [14, 60],
          medication: [14, 90],
          comms: [7, 60],
        },
        relief_sources: ['mock_oregon_resilience'],
      },
    ],
  },
  miami: {
    key: 'miami',
    hazards: [
      hurricane(0.3, { eal: 900 }),
      heat(0.35, 2),
      coastal(0.2),
      flood(0.15, { severity: 0.35, what: 'have streets flood in heavy rain', buckets: ['supplies', 'water_boil', 'home_loss'] }),
      wind(0.1),
      lightning(0.05),
      tornado(0.01, 0.5),
    ],
    targets: DAYS.miami,
    income: [0.5, 2, 3, 8],
    evacuate: { notice_hours: [12, 48], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [
      {
        id: 'major_hurricane_direct_hit',
        name: 'Plan for a direct hit by a major hurricane',
        applies_because:
          'Your county is on the Florida coast, where a category 3 or stronger hurricane can cut power and water for a week or more. Florida asks households to be ready for at least a week.',
        default_on: true,
        hazard: 'hurricane',
        sources: ['mock_hurricane_climatology', 'mock_hazus_restoration'],
        with: {
          power: [1, 5, 10, 21],
          water_boil: [1, 5, 7, 14],
          water_out: [0.5, 3, 7, 14],
          supplies: [3, 7, 14, 30],
          thermal: [1, 3, 5, 14],
          medication: [3, 10, 14, 30],
          comms: [1, 3, 5, 10],
        },
        evacuate_with: { notice_hours: [24, 72], days_away: 7 },
      },
    ],
  },
  hays: {
    key: 'hays',
    hazards: [
      tornado(0.02, 0.8),
      hail(0.3),
      winter(0.3, { severity: 0.4 }),
      ice(0.08),
      drought(0.15, { buckets: ['water_out', 'income'], what: 'go through a drought that affects the well or income' }),
      wind(0.3),
      wildfire(0.03, { severity: 0.5, what: 'have a grass fire nearby', buckets: ['evacuate', 'home_loss'] }),
      heat(0.2, 1.5, { severity: 0.35 }),
      cold(0.15),
      lightning(0.05),
    ],
    targets: DAYS.plains,
    income: [1, 3, 5, 10],
    evacuate: { notice_hours: [0.25, 6], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: false,
    scenarios: [],
  },
  phoenix: {
    key: 'phoenix',
    hazards: [
      heat(0.8, 1.6, { severity: 0.6, what: 'go through dangerous heat' }),
      wind(0.3),
      wildfire(0.1, { severity: 0.25, what: 'have days of heavy wildfire smoke', buckets: ['supplies', 'thermal'] }),
      flood(0.05, { severity: 0.35, what: 'be caught by a flash flood on the roads', buckets: ['get_home', 'supplies'] }),
      drought(0.2, { severity: 0.15 }),
    ],
    targets: DAYS.desert,
    income: [1, 3, 5, 10],
    evacuate: { notice_hours: [0.5, 6], days_away: 2 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [],
  },
  sugar_land: {
    key: 'sugar_land',
    hazards: [
      hurricane(0.12),
      flood(0.08, { severity: 0.6, eal: 650, buckets: ['home_loss', 'evacuate', 'supplies', 'power'] }),
      heat(0.35, 1.7),
      winter(0.03, { severity: 0.6, buckets: ['power', 'water_boil', 'thermal', 'water_out'], what: 'lose power and water in a hard freeze' }),
      cold(0.03),
      tornado(0.01, 0.6),
      hail(0.1),
      coastal(0.02, 0.4),
    ],
    targets: DAYS.gulf,
    income: [0.5, 2, 3, 8],
    evacuate: { notice_hours: [12, 72], days_away: 5 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [
      {
        id: 'major_hurricane_direct_hit',
        name: 'Plan for a direct hit by a major hurricane',
        applies_because:
          'Your county is near the Texas coast. A major hurricane passing directly over it would cut power for one to two weeks.',
        default_on: false,
        hazard: 'hurricane',
        sources: ['mock_hurricane_climatology', 'mock_hazus_restoration'],
        with: {
          power: [1, 7, 14, 21],
          water_boil: [1, 7, 10, 14],
          water_out: [0.5, 3, 7, 14],
          supplies: [3, 10, 14, 30],
          thermal: [1, 5, 7, 14],
          medication: [2, 10, 14, 30],
          comms: [0.5, 3, 5, 10],
        },
        evacuate_with: { notice_hours: [24, 72], days_away: 7 },
      },
    ],
  },
  chicago: {
    key: 'chicago',
    hazards: [
      winter(0.3, { severity: 0.4 }),
      cold(0.2),
      heat(0.2, 1.7, { severity: 0.5 }),
      flood(0.05, { severity: 0.35, buckets: ['water_boil', 'supplies', 'home_loss'], what: 'have basement or street flooding' }),
      wind(0.2),
      tornado(0.005, 0.6),
      lightning(0.03),
    ],
    targets: DAYS.lakes,
    income: [0.5, 2, 4, 10],
    evacuate: { notice_hours: [0.25, 6], days_away: 2 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: true,
    scenarios: [],
  },
  // Climate-region stand-ins for places outside the sample counties.
  northeast: {
    key: 'northeast',
    hazards: [winter(0.3), heat(0.2, 1.7), flood(0.04), ice(0.05), wind(0.2), hurricane(0.01, { severity: 0.5, what: 'be hit by the remains of a hurricane' }), quake(0.001)],
    targets: DAYS.philadelphia,
    income: [0.5, 2, 4, 10],
    evacuate: { notice_hours: [0.5, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: true,
    scenarios: [],
  },
  southeast: {
    key: 'southeast',
    hazards: [hurricane(0.08), heat(0.3, 1.7), flood(0.06), tornado(0.01), winter(0.05), wind(0.2), lightning(0.05)],
    targets: DAYS.gulf,
    income: [0.5, 2, 3, 8],
    evacuate: { notice_hours: [6, 48], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [],
  },
  midwest: {
    key: 'midwest',
    hazards: [winter(0.3), tornado(0.01), flood(0.05), heat(0.2, 1.6), wind(0.25), ice(0.05), cold(0.15)],
    targets: DAYS.lakes,
    income: [0.5, 2, 4, 10],
    evacuate: { notice_hours: [0.25, 6], days_away: 2 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: true,
    scenarios: [],
  },
  northern_great_plains: {
    key: 'northern_great_plains',
    hazards: [winter(0.4, { severity: 0.45 }), cold(0.25), tornado(0.01), hail(0.3), drought(0.1), wildfire(0.03), wind(0.3)],
    targets: DAYS.north,
    income: [1, 3, 5, 10],
    evacuate: { notice_hours: [0.5, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: false,
    scenarios: [],
  },
  southern_great_plains: {
    key: 'southern_great_plains',
    hazards: [tornado(0.02, 0.8), hail(0.3), heat(0.3, 1.6), drought(0.15), winter(0.1), ice(0.06), flood(0.05), wind(0.3)],
    targets: DAYS.plains,
    income: [1, 3, 5, 10],
    evacuate: { notice_hours: [0.25, 6], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [],
  },
  southwest: {
    key: 'southwest',
    hazards: [heat(0.5, 1.6, { severity: 0.55 }), drought(0.2), wildfire(0.12), quake(0.003), flood(0.04), wind(0.2)],
    targets: DAYS.desert,
    income: [1, 3, 5, 10],
    evacuate: { notice_hours: [0.5, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [],
  },
  northwest: {
    key: 'northwest',
    hazards: [winter(0.3), wind(0.4), wildfire(0.1), quake(0.003), flood(0.05), landslide(0.02), drought(0.1)],
    targets: DAYS.coos,
    income: [0.5, 3, 5, 10],
    evacuate: { notice_hours: [1, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: false,
    scenarios: [],
  },
  alaska: {
    key: 'alaska',
    hazards: [winter(0.5, { severity: 0.45 }), cold(0.4), quake(0.02, { severity: 0.6 }), wildfire(0.05), wind(0.3), volcano(0.005), tsunami(0.002)],
    targets: DAYS.north,
    income: [1, 3, 5, 10],
    evacuate: { notice_hours: [0.5, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: false,
    cold: true,
    heat_driven: false,
    scenarios: [],
  },
  hawaii_pacific: {
    key: 'hawaii_pacific',
    hazards: [hurricane(0.05), tsunami(0.005), volcano(0.01), flood(0.08), wildfire(0.03), heat(0.1, 1.6)],
    targets: DAYS.island,
    income: [0.5, 3, 5, 10],
    evacuate: { notice_hours: [0.5, 12], days_away: 3 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [],
  },
  caribbean: {
    key: 'caribbean',
    hazards: [hurricane(0.15), flood(0.1), quake(0.01), heat(0.3, 1.8), landslide(0.05), tsunami(0.002)],
    targets: DAYS.island,
    income: [0.5, 3, 5, 10],
    evacuate: { notice_hours: [12, 48], days_away: 5 },
    relief: DEFAULT_RELIEF,
    hot: true,
    cold: false,
    heat_driven: true,
    scenarios: [],
  },
};

/** Tier by duration: the smallest tier whose days cover a target. */
export function tierForDays(days: number): TierId {
  if (days <= 0) return 'now';
  if (days <= 3) return 'h72';
  if (days <= 14) return 'w2';
  if (days <= 30) return 'm1';
  if (days <= 90) return 'm3';
  if (days <= 180) return 'm6';
  return 'y1';
}
