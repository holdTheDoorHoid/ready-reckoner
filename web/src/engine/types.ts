/**
 * The engine contract (docs/ENGINE-API.md), mirrored by hand from `crates/rr-types`.
 *
 * JSON field names are snake_case; enums are snake_case string ids; optional fields are absent
 * rather than null; dates are ISO `YYYY-MM-DD`. Money is US dollars, days are days, probabilities
 * are 0 to 1.
 *
 * Keep this file in step with the Rust crate: `cargo test -p rr-types` compares every interface,
 * field, optional marker and enum list here with the Rust types, and `npm run check` type-checks
 * the fixture households against `PlanInput`. Style rules that test relies on: every enum is an
 * `as const` array plus a `(typeof X)[number]` type; one field per line; no inline object types.
 *
 * Engine-internal types (data-pack records such as `CountyRecord`, and `HouseholdEventRate`) are
 * not mirrored: the web app hands pack bytes to `load_pack` without reading them.
 */

/** Version of the contract; `engine_info().api_version` must equal it. */
export const ENGINE_API_VERSION = 1;

/** A date written `YYYY-MM-DD`. */
export type IsoDate = string;
/** A key into `content/citations.toml`, for example `ready_gov_water`. */
export type CitationId = string;
/** A catalogue item id, for example `water_stored`. */
export type ItemId = string;

// ---------------------------------------------------------------------------------------------
// Ids (docs/DESIGN.md §4.2, §4.3, §4.5). Plain names come from `catalogue()`; never hard-code them.
// ---------------------------------------------------------------------------------------------

/** Where a hazard comes from. */
export const HAZARD_TIERS = ['natural', 'societal', 'personal'] as const;
export type HazardTier = (typeof HAZARD_TIERS)[number];

/** The 35 hazards: 18 natural, 9 societal, 8 personal. */
export const HAZARD_IDS = [
  'avalanche',
  'coastal_flooding',
  'cold_wave',
  'drought',
  'earthquake',
  'hail',
  'heat_wave',
  'hurricane',
  'ice_storm',
  'landslide',
  'lightning',
  'riverine_flooding',
  'strong_wind',
  'tornado',
  'tsunami',
  'volcanic_activity',
  'wildfire',
  'winter_weather',
  'pandemic',
  'grid_failure',
  'cyber_outage',
  'civil_unrest',
  'supply_chain_disruption',
  'hazmat_release',
  'nuclear_plant_incident',
  'nuclear_attack',
  'terrorism',
  'job_loss',
  'house_fire',
  'medical_emergency',
  'vehicle_stranding',
  'local_utility_outage',
  'burglary',
  'earner_death_or_disability',
  'extended_household_illness',
] as const;
export type HazardId = (typeof HAZARD_IDS)[number];

/** How a bucket's target is expressed: the `kind` of a `Target`. */
export const TARGET_KINDS = ['days', 'months', 'evacuate', 'readiness'] as const;
export type TargetKind = (typeof TARGET_KINDS)[number];

/** The three kinds of consequence bucket. */
export const BUCKET_KINDS = ['duration', 'readiness', 'money'] as const;
export type BucketKind = (typeof BUCKET_KINDS)[number];

/**
 * The 14 consequence buckets. Duration: power..comms (targets in days). Readiness: evacuate..security
 * (evacuate has its own target shape; the rest are checklists). Money: income (months), home_loss
 * (a checklist: insurance and documents).
 */
export const BUCKET_IDS = [
  'power',
  'water_boil',
  'water_out',
  'supplies',
  'thermal',
  'medication',
  'comms',
  'evacuate',
  'get_home',
  'medical_emergency',
  'fire',
  'security',
  'income',
  'home_loss',
] as const;
export type BucketId = (typeof BUCKET_IDS)[number];

/** Plan tiers in plan order: free actions, 3 days, 2 weeks, 1, 3 and 6 months, 1 year. */
export const TIER_IDS = ['now', 'h72', 'w2', 'm1', 'm3', 'm6', 'y1'] as const;
export type TierId = (typeof TIER_IDS)[number];

// ---------------------------------------------------------------------------------------------
// Inputs (docs/DESIGN.md §4.1)
// ---------------------------------------------------------------------------------------------

export const SETTINGS = ['urban', 'suburban', 'rural'] as const;
export type Setting = (typeof SETTINGS)[number];

export const HOUSING_KINDS = [
  'apartment_high_rise',
  'apartment_low_rise',
  'rowhouse',
  'detached',
  'mobile_home',
  'rural_property',
] as const;
export type HousingKind = (typeof HOUSING_KINDS)[number];

export const TENURES = ['own', 'rent'] as const;
export type Tenure = (typeof TENURES)[number];

export const WATER_SOURCES = ['municipal', 'well'] as const;
export type WaterSource = (typeof WATER_SOURCES)[number];

/** Where wastewater goes (JSON field `housing.sewer`). */
export const WASTEWATER_KINDS = ['sewer', 'septic'] as const;
export type Wastewater = (typeof WASTEWATER_KINDS)[number];

export const HEATING_KINDS = [
  'gas',
  'electric_resistance',
  'heat_pump',
  'oil',
  'propane',
  'wood',
  'district',
  'none',
] as const;
export type Heating = (typeof HEATING_KINDS)[number];

export const COOLING_KINDS = ['central', 'window', 'none'] as const;
export type Cooling = (typeof COOLING_KINDS)[number];

export const BACKUP_POWER_KINDS = ['none', 'power_station', 'generator', 'solar_battery'] as const;
export type BackupPower = (typeof BACKUP_POWER_KINDS)[number];

/** Under 1, 1–3, 4–12, 13–17, 18–64, 65 and over. */
export const AGE_BANDS = ['infant', 'toddler', 'child', 'teen', 'adult', 'senior'] as const;
export type AgeBand = (typeof AGE_BANDS)[number];

/** Powered medical devices that need no detail. Any other device is `{ other: { watts } }`. */
export const SIMPLE_POWERED_DEVICES = ['none', 'cpap', 'oxygen'] as const;
export type SimplePoweredDevice = (typeof SIMPLE_POWERED_DEVICES)[number];
export type PoweredDevice = SimplePoweredDevice | PoweredDeviceOther;

/** How well a person gets around (JSON field `medical.mobility`). */
export const MOBILITY_LEVELS = ['none', 'limited', 'wheelchair'] as const;
export type Mobility = (typeof MOBILITY_LEVELS)[number];

export const COMMUTE_MODES = ['car', 'transit', 'walk', 'bike'] as const;
export type CommuteMode = (typeof COMMUTE_MODES)[number];

export const FUELS = ['gas', 'diesel', 'hybrid', 'ev'] as const;
export type Fuel = (typeof FUELS)[number];

export const INCOME_STABILITIES = ['very_stable', 'stable', 'variable', 'seasonal', 'gig'] as const;
export type IncomeStability = (typeof INCOME_STABILITIES)[number];

/**
 * How severe an event to be ready for. Labels: "Common disruptions", "Serious", "Very serious"
 * (the default), "Rare catastrophes".
 */
export const RETURN_PERIODS = ['one_in_10', 'one_in_50', 'one_in_100', 'one_in_500'] as const;
export type ReturnPeriod = (typeof RETURN_PERIODS)[number];

export const CLIMATE_HORIZONS = ['today', 'y2050'] as const;
export type ClimateHorizon = (typeof CLIMATE_HORIZONS)[number];

/** Water per person per day: drinking only, the usual one gallon (default), or drinking plus washing. */
export const WATER_LEVELS = ['survival', 'basic', 'comfortable'] as const;
export type WaterLevel = (typeof WATER_LEVELS)[number];

/** Stages of change; tailors the copy, never the numbers. */
export const STAGES = [
  'not_thought_about',
  'thinking',
  'have_some_things',
  'have_a_plan',
  'maintaining',
] as const;
export type Stage = (typeof STAGES)[number];

/** Everything the engine needs about one household; the argument of `assess`. */
export interface PlanInput {
  /** The day the plan starts. The app sets it to today; the engine never reads the clock. */
  planning_date: IsoDate;
  location: LocationInput;
  housing: Housing;
  /** At least one person. */
  people: Person[];
  pets: Pets;
  /** The household's vehicles. */
  mobility: HouseholdMobility;
  finances: Finances;
  /** What the household already has, and free actions already done (qty 1 or more). */
  existing: Owned[];
  dials: Dials;
  /** Where the household says it is with preparing. */
  stage?: Stage;
  /** Self-rated confidence, 1 to 5, asked before and after the plan. */
  confidence_1to5?: number;
}

/**
 * Where the household lives: a ZIP code, a county, or both. With both, `county_fips` decides the
 * county (that is how the app records the pick after an `ambiguous_zip` error).
 */
export interface LocationInput {
  /** ISO country code; only "US" in v1. */
  country: string;
  /** Five digits, for example "19147". */
  zip?: string;
  /** Five-digit county FIPS code, for example "42101". */
  county_fips?: string;
  setting: Setting;
}

export interface Housing {
  kind: HousingKind;
  tenure: Tenure;
  /** Floor the household lives on (1 is street level; negative is below ground). Integer. */
  floor: number;
  basement: boolean;
  water: WaterSource;
  sewer: Wastewater;
  heating: Heating;
  cooling: Cooling;
  backup_power: BackupPower;
  alarms: Alarms;
}

export interface Alarms {
  smoke: boolean;
  co: boolean;
  extinguisher: boolean;
}

export interface Person {
  age_band: AgeBand;
  pregnant_or_nursing: boolean;
  medical: Medical;
  earner: boolean;
  commute?: Commute;
}

export interface Medical {
  daily_rx: boolean;
  refrigerated_rx: boolean;
  powered_device: PoweredDevice;
  mobility: Mobility;
  /** Dietary needs in the person's own words, for example "vegetarian", "formula". */
  dietary: string[];
  epinephrine: boolean;
}

/** A powered medical device not in `SIMPLE_POWERED_DEVICES`: `{ "other": { "watts": 60 } }`. */
export interface PoweredDeviceOther {
  other: OtherPoweredDevice;
}

export interface OtherPoweredDevice {
  /** Power drawn, in watts; more than 0. */
  watts: number;
}

export interface Commute {
  /** One-way distance in kilometres. */
  distance_km: number;
  mode: CommuteMode;
  remote_possible: boolean;
}

/** Counts, 0 to 255. */
export interface Pets {
  dogs: number;
  cats: number;
  small: number;
  large_animals: number;
}

export interface HouseholdMobility {
  vehicles: Vehicle[];
}

export interface Vehicle {
  fuel: Fuel;
}

export interface Finances {
  /** Money for preparing each month; 0 gives a plan of free actions. */
  monthly_budget_usd: number;
  one_off_budget_usd: number;
  /** Months of expenses already saved. */
  emergency_fund_months: number;
  monthly_expenses_usd?: number;
  income: Income;
  insurance: Insurance;
}

export interface Income {
  /** Must equal the number of people with `earner: true`. */
  earners: number;
  stability: IncomeStability;
}

export interface Insurance {
  home_or_renters: boolean;
  flood: boolean;
  earthquake: boolean;
}

/** Something already owned, or a free action done (qty 1 or more). */
export interface Owned {
  item_id: ItemId;
  /** In the item's unit, for example gallons. */
  qty: number;
  /** Total paid for this quantity, so the plan can use real prices. */
  paid_usd?: number;
}

export interface Dials {
  return_period: ReturnPeriod;
  climate: ClimateHorizon;
  /** Years for the natural-frequency sentences ("in the next ten years"): 1 to 50, usually 10. */
  horizon_years: number;
  /** Defaults to "basic" when absent. */
  water_level?: WaterLevel;
  /** On/off choices for named scenarios; empty when absent. */
  scenario_overrides?: ScenarioToggle[];
  /**
   * Allow up to 10% of the monthly budget for rare-catastrophe items (a radiation meter,
   * potassium iodide only on official instruction, Faraday storage). Defaults to false when
   * absent: those items otherwise get $0 (`Item.rare_catastrophic`).
   */
  rare_catastrophic_opt_in?: boolean;
}

export interface ScenarioToggle {
  /** A `ScenarioInfo.id`, for example "cascadia_m9". */
  id: string;
  on: boolean;
}

// ---------------------------------------------------------------------------------------------
// Validation: `bad_input` errors carry `{ problems: Problem[] }` in `details`.
// ---------------------------------------------------------------------------------------------

export const PROBLEM_CODES = [
  'schema',
  'unsupported_country',
  'location_missing',
  'zip_format',
  'county_fips_format',
  'no_people',
  'negative_value',
  'not_finite',
  'out_of_range',
  'earners_mismatch',
  'id_format',
  'duplicate_id',
] as const;
export type ProblemCode = (typeof PROBLEM_CODES)[number];

export interface Problem {
  code: ProblemCode;
  /** JSON path of the field, for example "people[1].commute.distance_km"; empty for the whole input. */
  field: string;
  /** Plain language, fit to show next to the field. */
  message: string;
}

// ---------------------------------------------------------------------------------------------
// Outputs (docs/ENGINE-API.md)
// ---------------------------------------------------------------------------------------------

export interface LocationResolved {
  country: string;
  county_fips: string;
  county_name: string;
  state_abbr: string;
  state_name: string;
  zip?: string;
  /** Share of the ZIP code inside this county, 0 to 1. */
  zip_county_share?: number;
  centroid: LatLon;
  /** Fifth National Climate Assessment region, for example "northeast". */
  nca_region: string;
  coastal: boolean;
  tsunami_zone: boolean;
  facility_flags: FacilityFlags;
  /** For example "your county; tract-level data not yet loaded". */
  data_note?: string;
}

export interface LatLon {
  lat: number;
  lon: number;
}

export interface FacilityFlags {
  nuclear_plant_within_16km: boolean;
  nuclear_plant_within_80km: boolean;
  hazmat_facilities_within_5km: number;
}

/** How much to trust a hazard estimate; "prior" means expert judgement, shown as such. */
export const DATA_CONFIDENCE_LEVELS = ['high', 'medium', 'low', 'prior'] as const;
export type DataConfidence = (typeof DATA_CONFIDENCE_LEVELS)[number];

/** Ranked list, or the separate rare-catastrophe box (never ranked by expected loss). */
export const HAZARD_DISPLAYS = ['ranked', 'rare_catastrophic'] as const;
export type HazardDisplay = (typeof HAZARD_DISPLAYS)[number];

export interface HazardProfile {
  id: HazardId;
  name: string;
  tier: HazardTier;
  display: HazardDisplay;
  /** Household-significant events per year (can exceed 1). */
  rate_per_year: number;
  /** [low, high] */
  rate_range: [number, number];
  /** Chance of at least one event in a year, 0 to 1. */
  annual_probability: number;
  /** [low, high] */
  probability_range: [number, number];
  /** Relative, 0 to 1. */
  severity: number;
  eal_per_household_usd?: number;
  /** 1 for today's climate. */
  climate_multiplier: number;
  confidence: DataConfidence;
  sources: CitationId[];
  frequency_sentence: string;
  buckets: BucketId[];
}

/**
 * The day values targets are rounded to. `Target` values of kind "days" are always on this ladder.
 */
export const TARGET_LADDER_DAYS = [0.5, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365] as const;

/** Duration buckets. `low`/`high` are the 10th and 90th percentiles; in `covered` they equal `value`. */
export interface TargetDays {
  kind: 'days';
  value: number;
  low: number;
  high: number;
}

/** `income`: months of income gap to have saved. */
export interface TargetMonths {
  kind: 'months';
  value: number;
  low: number;
  high: number;
}

/** `evacuate`. */
export interface TargetEvacuate {
  kind: 'evacuate';
  /** Chance of having to leave at least once in ten years, 0 to 1. */
  p_need_10yr: number;
  notice_hours_low: number;
  notice_hours_high: number;
  days_away: number;
}

/** Checklists: get_home, medical_emergency, fire, security, home_loss. */
export interface TargetReadiness {
  kind: 'readiness';
  /** Chance of needing it at least once in ten years, 0 to 1. */
  p_need_10yr: number;
  done: number;
  of: number;
}

export type Target = TargetDays | TargetMonths | TargetEvacuate | TargetReadiness;

export interface Contribution {
  hazard: HazardId;
  /** 0 to 1; a bucket's shares add up to 1. */
  share: number;
}

/** When outside help plausibly arrives and service is mostly restored, for the design event. */
export interface Relief {
  help_arrives_days: number;
  mostly_restored_days: number;
  sources: CitationId[];
}

export interface BucketAssessment {
  id: BucketId;
  name: string;
  /** Its kind matches the bucket's `target_kind` (see `catalogue().buckets`). */
  target: Target;
  /** Same kind as `target`. */
  covered: Target;
  tier_enough: TierId;
  /** Largest share first. */
  contributions: Contribution[];
  frequency_sentences: string[];
  sources: CitationId[];
  relief?: Relief;
}

/** What a requirement line's quantity scales with. */
export const PER_VALUES = ['household', 'person', 'commuter', 'pet'] as const;
export type Per = (typeof PER_VALUES)[number];

export interface RequirementLine {
  id: string;
  bucket: BucketId;
  /** A catalogue item id, or a class of items that several catalogue items can meet. */
  item_class: string;
  /** For the whole household, already multiplied out by `per`. */
  quantity: number;
  unit: string;
  per: Per;
  /** The quantity rule in rr-supply that produced it. */
  rule: string;
  citations: CitationId[];
  plain: string;
}

export const PLAN_ITEM_KINDS = ['free_action', 'purchase', 'reserve'] as const;
export type PlanItemKind = (typeof PLAN_ITEM_KINDS)[number];

/** US dollars for a plan item's whole quantity. */
export interface CostRange {
  low: number;
  high: number;
}

export interface PlanItem {
  item_id: ItemId;
  name: string;
  kind: PlanItemKind;
  quantity: number;
  unit: string;
  /** The recorded price if the household gave one, otherwise the middle of the band. */
  est_cost_usd: number;
  price_band: CostRange;
  buckets: BucketId[];
  hazards: HazardId[];
  why: string;
  /** Larger is better; the allocator's units. */
  risk_reduction: number;
  tier: TierId;
  /** Absent means false. */
  done?: boolean;
  paid_usd?: number;
}

export interface PlanMonth {
  /** Counted from 0: month 0 begins on the planning date and holds the free actions first. */
  index: number;
  /** The one-off amount lands in month 0. */
  budget_usd: number;
  items: PlanItem[];
}

/** Money being saved toward an item that costs more than a month's budget. */
export interface SavingsEnvelope {
  item_id: ItemId;
  saved_usd: number;
  needed_usd: number;
}

/** The emergency-fund goal; never funded from the supplies budget. */
export interface SavingsTrack {
  target_months: number;
  target_usd: number;
  current_months: number;
  monthly_suggestion_usd: number;
  why: string;
}

export interface Plan {
  /** From month 0. */
  months: PlanMonth[];
  /** The month by which every bucket is covered, if the plan gets there. */
  done_month?: number;
  envelopes: SavingsEnvelope[];
  savings_track?: SavingsTrack;
}

/** A named scenario (for example "cascadia_m9") that applies here; toggle via `Dials.scenario_overrides`. */
export interface ScenarioInfo {
  id: string;
  name: string;
  applies_because: string;
  on: boolean;
  effect_summary: string;
  sources: CitationId[];
}

export const WARNING_SEVERITIES = ['note', 'warn'] as const;
export type WarningSeverity = (typeof WARNING_SEVERITIES)[number];

/** A guardrail: the plan looks off, but it never blocks. */
export interface Warning {
  id: string;
  severity: WarningSeverity;
  message: string;
  why: string;
  /** Related hazard, bucket or item ids. */
  related: string[];
}

/** Everything `assess` returns. */
export interface PlanOutput {
  engine_version: string;
  api_version: number;
  data_pack_version: string;
  content_version: string;
  location: LocationResolved;
  /** Ranked hazards first, then the rare-catastrophic ones. */
  register: HazardProfile[];
  /** In `BUCKET_IDS` order. */
  buckets: BucketAssessment[];
  scenarios: ScenarioInfo[];
  tier_reached: TierId;
  tier_recommended: TierId;
  plan: Plan;
  requirements: RequirementLine[];
  warnings: Warning[];
  /** The printable packet as Markdown. */
  packet_markdown: string;
  /** Every citation referenced above. */
  provenance: Citation[];
}

// ---------------------------------------------------------------------------------------------
// Content (docs/DESIGN.md §4.6)
// ---------------------------------------------------------------------------------------------

export interface Citation {
  id: CitationId;
  title: string;
  publisher: string;
  year?: number;
  url: string;
  retrieved: IsoDate;
  quote?: string;
  license: string;
  /** An expert judgement rather than data: numbers citing it are shown as estimates. The engine always sends it. */
  prior?: boolean;
}

export interface Item {
  id: ItemId;
  name: string;
  category: string;
  unit: string;
  buckets: BucketId[];
  tier: TierId;
  free: boolean;
  /** Ordered first within its tier (alarms, water, a dependent's medication). The engine always sends it. */
  life_safety?: boolean;
  /** Capped by the allocator: $0 by default, at most 10% of the budget if the user opts in. The engine always sends it. */
  rare_catastrophic?: boolean;
  spec: string;
  look_for: string[];
  avoid: string[];
  price_band_usd: PriceBand;
  /** When the price band was observed ("prices as of ..."). */
  retrieved?: IsoDate;
  quantity_rule: string;
  maintenance?: Maintenance;
  citations: CitationId[];
  hazard_extras: HazardId[];
  /** Kilocalories in one unit (food), for cost per 2,000 kcal. */
  energy_kcal_per_unit?: number;
  /** Litres in one unit (water and liquids), for cost per litre or gallon. */
  volume_l_per_unit?: number;
}

/** Price of one unit, in US dollars. */
export interface PriceBand {
  low: number;
  high: number;
  per: string;
  note?: string;
}

export interface Maintenance {
  rotate_months?: number;
  check_months?: number;
}

export interface GuidanceMeta {
  id: string;
  title: string;
  /** Bucket, hazard, tier or topic ids. */
  applies_to: string[];
  citations: CitationId[];
}

// ---------------------------------------------------------------------------------------------
// Effects (docs/DESIGN.md §4.3)
// ---------------------------------------------------------------------------------------------

export const EVIDENCE_KINDS = ['empirical', 'prior'] as const;
export type Evidence = (typeof EVIDENCE_KINDS)[number];

/** Log-normal from its median and 90th percentile. */
export interface DurationLogNormal {
  kind: 'log_normal';
  median_days: number;
  p90_days: number;
}

export interface DurationFixed {
  kind: 'fixed';
  days: number;
}

export type DurationDist = DurationLogNormal | DurationFixed;

export interface Effect {
  hazard: HazardId;
  bucket: BucketId;
  p_given_event: number;
  duration: DurationDist;
  evidence: Evidence;
  sources: CitationId[];
}

// ---------------------------------------------------------------------------------------------
// Function arguments, results and errors (docs/ENGINE-API.md)
// ---------------------------------------------------------------------------------------------

export const ERROR_CODES = [
  'bad_input',
  'unknown_zip',
  'unknown_county',
  'ambiguous_zip',
  'pack_missing',
  'pack_corrupt',
  'internal',
] as const;
export type ErrorCode = (typeof ERROR_CODES)[number];

/**
 * `details` is `BadInputDetails` for bad_input and `LocationSuggestions` for unknown_zip,
 * unknown_county and ambiguous_zip; absent otherwise.
 */
export interface EngineError {
  code: ErrorCode;
  /** Plain language, fit to show the user. */
  message: string;
  details?: unknown;
}

export interface BadInputDetails {
  problems: Problem[];
}

export interface LocationSuggestions {
  /** For ambiguous_zip: the counties the ZIP code spans, largest share first. */
  suggestions: LocationResolved[];
}

export interface EnvelopeOk<T> {
  ok: true;
  value: T;
}

export interface EnvelopeErr {
  ok: false;
  error: EngineError;
}

/** What every engine function returns. */
export type Envelope<T> = EnvelopeOk<T> | EnvelopeErr;

/** A credit line or disclaimer the About screen must show (FEMA NRI terms; CC BY sources). */
export interface Attribution {
  source: string;
  /** Show exactly this text. */
  text: string;
  url: string;
  version?: string;
  accessed: IsoDate;
}

export interface EngineInfo {
  engine_version: string;
  api_version: number;
  /** Absent until a pack is loaded. */
  data_pack_version?: string;
  content_version: string;
  packs_loaded: string[];
  attributions: Attribution[];
}

/** What `load_pack` returns. */
export interface PackInfo {
  name: string;
  version: string;
  rows: number;
}

export interface HazardInfo {
  id: HazardId;
  name: string;
  tier: HazardTier;
}

export interface BucketInfo {
  id: BucketId;
  name: string;
  kind: BucketKind;
  target_kind: TargetKind;
}

export interface TierInfo {
  id: TierId;
  name: string;
  days: number;
}

/** What `catalogue()` returns: content plus the plain name of every id. */
export interface Catalogue {
  items: Item[];
  citations: Citation[];
  guidance: GuidanceMeta[];
  hazards: HazardInfo[];
  buckets: BucketInfo[];
  tiers: TierInfo[];
}

export const EXPLAIN_KINDS = ['hazard', 'bucket', 'item', 'requirement', 'warning'] as const;
export type ExplainKind = (typeof EXPLAIN_KINDS)[number];

/** The argument of `explain`. */
export interface ExplainRequest {
  kind: ExplainKind;
  id: string;
  input: PlanInput;
}

export interface Explanation {
  title: string;
  /** One paragraph per entry. */
  plain: string[];
  /** One step of arithmetic per entry, for the expert view. */
  math?: string[];
  sources: Citation[];
}

// ---------------------------------------------------------------------------------------------
// Compile-time checks: `npm run check` fails if a union and its id list drift apart.
// ---------------------------------------------------------------------------------------------

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false;
type Expect<T extends true> = T;

export type ContractChecks = [
  Expect<Equal<Target['kind'], TargetKind>>,
  Expect<Equal<DurationDist['kind'], 'log_normal' | 'fixed'>>,
  Expect<Equal<Envelope<null>['ok'], boolean>>,
];
