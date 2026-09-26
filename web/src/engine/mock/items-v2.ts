/**
 * The mock catalogue's contract v2 fields (docs/ENGINE-API.md "Content and catalogue"): items
 * that need another first (`requires`), decisions that cost the supplies budget nothing
 * (`decision`), the long-horizon section (`long_horizon`), maintenance anchors by season
 * (`season`) and items to try every few months (`test_interval_months`), plus the few new items
 * those need. Placeholder content like the rest of the mock: `mock_*` citations, no brands.
 *
 * Also the bare-minimum kit (rr-supply `minimum.rs`: three days of water, light, warmth and
 * medicine) and the rare-event items the allowance may buy for a family the household ticks.
 */
import type { BucketId, HazardId, Item, PlanInput, Season, TierId } from '../types';
import type { RankedHazard } from './model';

type V2Fields = Partial<Pick<Item, 'requires' | 'readiness_share' | 'decision' | 'long_horizon' | 'season' | 'test_interval_months'>>;

/**
 * v2 fields on items already in the mock catalogue. (The interview workstream gives the lights, the
 * power station and the generator their test intervals in items.ts itself; they are not repeated.)
 */
const FIELDS: Record<string, V2Fields> = {
  batteries_spare: { requires: ['flashlights_headlamps'], readiness_share: 0.05 },
  radio_crank: { test_interval_months: 6 },
  generator_portable: { season: 'fall' },
  fans_cooling: { season: 'summer' },
  smoke_air_filter: { season: 'summer' },
  n95_masks: { season: 'summer' },
  hurricane_zone_check: { season: 'summer' },
  temperature_plan: { season: 'summer' },
  blankets_warm: { season: 'winter' },
  car_half_tank: { season: 'winter' },
  alarms_test: { season: 'fall' },
  first_aid_kit: { readiness_share: 0.4 },
  fire_extinguisher: { readiness_share: 0.3 },
  get_home_bag: { readiness_share: 0.5 },
  go_bag: { readiness_share: 0.5 },
};

const RETRIEVED = '2026-09-25';

interface Def {
  id: string;
  name: string;
  category: string;
  unit: string;
  buckets: BucketId[];
  tier: TierId;
  spec: string;
  look_for?: string[];
  avoid?: string[];
  /** [low, high, per] a unit; absent for a free step. */
  price?: [number, number, string];
  citations: string[];
  extras?: HazardId[];
  v2: V2Fields;
}

function item(d: Def): Item {
  const free = d.price === undefined;
  const [low, high, per] = d.price ?? [0, 0, 'once'];
  const out: Item = {
    id: d.id,
    name: d.name,
    category: d.category,
    unit: d.unit,
    buckets: d.buckets,
    tier: free ? 'now' : d.tier,
    free,
    life_safety: false,
    rare_catastrophic: false,
    spec: d.spec,
    look_for: d.look_for ?? [],
    avoid: d.avoid ?? [],
    price_band_usd: { low, high, per },
    quantity_rule: free ? 'once' : `mock_${d.id}`,
    citations: d.citations,
    hazard_extras: d.extras ?? [],
    ...d.v2,
  };
  if (!free) out.retrieved = RETRIEVED;
  return out;
}

/** New items: decisions, the long-horizon section, and two things that need testing. */
const NEW_ITEMS: Item[] = [
  // Decisions: no purchase; the Plan screen shows them with a "decided" check-off.
  item({
    id: 'decision_renters_insurance',
    name: 'Decide on renters insurance',
    category: 'money',
    unit: 'decision',
    buckets: ['home_loss'],
    tier: 'now',
    spec: 'Get a quote for renters insurance and decide whether to buy it. It usually pays for belongings and a place to stay after a fire or flood.',
    look_for: ['Cover for somewhere to stay while the home is repaired', 'Replacement cost, not cash value'],
    avoid: ['Assuming the landlord’s policy covers your things'],
    citations: ['mock_insurance'],
    v2: { decision: true },
  }),
  item({
    id: 'decision_flood_insurance',
    name: 'Decide on flood insurance',
    category: 'money',
    unit: 'decision',
    buckets: ['home_loss'],
    tier: 'now',
    spec: 'Standard home insurance leaves out flood damage. Get a quote for a flood policy and decide before storm season; most policies take 30 days to start.',
    look_for: ['Cover for the building and for what is inside it', 'The waiting period before cover starts'],
    avoid: ['Waiting until a storm is forecast'],
    citations: ['mock_insurance'],
    extras: ['riverine_flooding', 'coastal_flooding', 'hurricane'],
    v2: { decision: true, season: 'spring' },
  }),
  item({
    id: 'decision_sewer_backup',
    name: 'Decide on sewer-backup cover',
    category: 'money',
    unit: 'decision',
    buckets: ['home_loss'],
    tier: 'now',
    spec: 'Sewage can back up through basement drains in heavy rain, and standard policies leave it out. Ask your insurer what the add-on costs and decide.',
    citations: ['mock_insurance'],
    extras: ['water_damage'],
    v2: { decision: true },
  }),
  item({
    id: 'id_for_every_person',
    name: 'ID for every person',
    category: 'documents',
    unit: 'decision',
    buckets: ['home_loss', 'evacuate'],
    tier: 'now',
    spec: 'A current photo ID for every adult and papers for every child: a passport or passport card where you can afford one. Replacing ID after a disaster is slow, and you may need it to get aid.',
    look_for: ['Copies kept with your other papers'],
    avoid: ['An ID that expires before you next check'],
    price: [30, 165, 'person'],
    citations: ['mock_insurance'],
    v2: { decision: true },
  }),
  // The long-horizon section: what keeps working for months, and pointers.
  item({
    id: 'rain_catchment',
    name: 'Rain barrel with a first-flush diverter',
    category: 'water',
    unit: 'barrel',
    buckets: ['water_out'],
    tier: 'm6',
    spec: 'A covered barrel on a downspout collects water to filter or treat before drinking. Check your state’s rules on collecting rain first.',
    look_for: ['A tight lid and a screen against mosquitoes', 'A diverter that sends the first dirty water away'],
    avoid: ['Drinking rainwater without filtering or treating it'],
    price: [60, 150, 'barrel'],
    citations: ['mock_water_per_person'],
    v2: { long_horizon: true, season: 'spring' },
  }),
  item({
    id: 'hand_pump_well',
    name: 'Hand pump for the well',
    category: 'water',
    unit: 'pump',
    buckets: ['water_out'],
    tier: 'y1',
    spec: 'A hand pump fitted beside the electric pump lets you draw well water without power. Have a well contractor fit it.',
    price: [300, 900, 'pump'],
    citations: ['mock_water_per_person'],
    v2: { long_horizon: true },
  }),
  item({
    id: 'sanitation_months',
    name: 'Twin-bucket toilet for weeks or months',
    category: 'home',
    unit: 'kit',
    buckets: ['water_out'],
    tier: 'm6',
    spec: 'Two buckets, one for urine and one for solids with cover material, keep a home sanitary when the sewer or water is out for a long time.',
    price: [40, 80, 'kit'],
    citations: ['mock_ready_gov_kit'],
    v2: { long_horizon: true },
  }),
  item({
    id: 'water_carriers',
    name: 'Water carriers for hauling',
    category: 'water',
    unit: 'carrier',
    buckets: ['water_out'],
    tier: 'm6',
    spec: 'Carriers with handles for bringing water from a distribution point or a neighbour’s well.',
    price: [15, 35, 'carrier'],
    citations: ['mock_water_per_person'],
    v2: { long_horizon: true },
  }),
  item({
    id: 'pointer_canning',
    name: 'Learn safe home canning',
    category: 'food',
    unit: 'step',
    buckets: ['supplies'],
    tier: 'now',
    spec: 'If you want to put up food for months, learn tested methods from your county Extension office first. Unsafe canning can cause botulism.',
    citations: ['mock_food_kcal'],
    v2: { long_horizon: true },
  }),
  item({
    id: 'pointer_routines',
    name: 'Keep routines and talk with people you trust',
    category: 'community',
    unit: 'step',
    buckets: ['security'],
    tier: 'now',
    spec: 'In a long disruption, daily routines, breaks from the news and time with others protect everyone’s health.',
    citations: ['mock_social_capital'],
    v2: { long_horizon: true },
  }),
  // Things to try every few months (the "tested?" date on the Have screen, "Test" on Keep it up).
  item({
    id: 'jump_pack',
    name: 'Jump starter pack for the car',
    category: 'transport',
    unit: 'pack',
    buckets: ['get_home'],
    tier: 'w2',
    spec: 'A charged battery pack that can start the car without another vehicle. Charge and try it every few months.',
    price: [50, 120, 'pack'],
    citations: ['mock_get_home'],
    v2: { test_interval_months: 3, season: 'winter' },
  }),
  item({
    id: 'key_safe',
    name: 'Key safe for a spare key',
    category: 'security',
    unit: 'safe',
    buckets: ['security'],
    tier: 'm1',
    spec: 'A mechanical key safe lets someone you trust get in when you cannot. Pick a rated one and try the code once a year.',
    avoid: ['Cheap four-dial boxes that open in seconds'],
    price: [30, 80, 'safe'],
    citations: ['mock_ready_gov_kit'],
    v2: { test_interval_months: 12 },
  }),
];

/**
 * The catalogue with its v2 fields. `requires`, `decision` and `long_horizon` are always sent (the
 * engine sends them on every item); `readiness_share`, `season` and `test_interval_months` only
 * where set.
 */
export function withV2(items: Item[]): Item[] {
  return [...items, ...NEW_ITEMS].map((i) => ({ requires: [], decision: false, long_horizon: false, ...i, ...FIELDS[i.id] }));
}

/** The bare-minimum kit's items in the mock (rr-supply's three days of water, light, warmth and medicine). */
export const MINIMUM_KIT: ReadonlySet<string> = new Set([
  'water_stored',
  'water_bleach',
  'flashlights_headlamps',
  'batteries_spare',
  'blankets_warm',
  'medication_reserve',
  'cooler_ice_packs',
  'device_battery_backup',
  'infant_formula_reserve',
]);

/** The long-horizon section's items, with how many each household needs. */
export const LONG_HORIZON_ITEMS: { id: string; tier: TierId; qty: (people: number) => number; well?: boolean }[] = [
  { id: 'rain_catchment', tier: 'm6', qty: () => 1 },
  { id: 'hand_pump_well', tier: 'y1', qty: () => 1, well: true },
  { id: 'water_carriers', tier: 'm6', qty: (n) => Math.max(2, Math.ceil(n / 2)) },
  { id: 'sanitation_months', tier: 'm6', qty: () => 1 },
  { id: 'pointer_canning', tier: 'now', qty: () => 1 },
  { id: 'pointer_routines', tier: 'now', qty: () => 1 },
];

/** Seasons in calendar order, with the month each starts (meteorological seasons). */
export const SEASON_START: Record<Season, number> = { spring: 3, summer: 6, fall: 9, winter: 12 };

/**
 * What the rare allowance may buy (rr-budget, REVIEW §2.4): an item is a candidate only for a
 * family the household ticked whose local ten-year chance is at least 1 in 1,000. In the mock the
 * nuclear family's radiation meter is the one such item.
 */
export function rareItemsFor(dials: PlanInput['dials'], register: readonly RankedHazard[]): { id: string; tier: TierId; qty: number }[] {
  const list = dials.rare_opt_in ?? [];
  const all = dials.rare_catastrophic_opt_in === true || list.includes('all');
  const nuclear = register.find((h) => h.seed.id === 'nuclear_attack');
  const ticked = all || list.includes('nuclear_attack');
  if (!ticked || !nuclear || nuclear.rate * 10 < 1e-3) return [];
  return [{ id: 'radiation_meter', tier: 'm3', qty: 1 }];
}

/** Why each decision is in the plan (the mock's `PlanItem.why`). */
export const DECISION_WHY: Record<string, string> = {
  decision_renters_insurance: 'Renters insurance usually pays for your things and a place to stay after a fire or flood. Deciding costs nothing; the policy is a monthly bill, not a supply.',
  decision_flood_insurance: 'Standard home insurance leaves out flood damage, and a flood policy takes 30 days to start. Decide before storm season.',
  decision_sewer_backup: 'With a basement, sewage can back up in heavy rain, and standard policies leave it out. The add-on is usually small; ask and decide.',
  id_for_every_person: 'Replacing ID after a disaster is slow, and aid often needs it. The fees are not part of your supplies budget.',
};
