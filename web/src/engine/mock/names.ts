/**
 * Plain names for every id, as the mock engine's `catalogue()` returns them. The app reads names
 * from `catalogue()` and never from here, so the real engine can rename things freely.
 */
import type { BucketInfo, HazardId, HazardInfo, HazardTier, TierInfo } from '../types';
import { HAZARD_IDS, RETIRED_HAZARD_IDS } from '../types';

const HAZARD_NAMES: Record<HazardId, string> = {
  avalanche: 'Avalanches',
  coastal_flooding: 'Coastal flooding',
  cold_wave: 'Extreme cold',
  drought: 'Drought',
  earthquake: 'Earthquakes',
  hail: 'Hail',
  heat_wave: 'Extreme heat',
  hurricane: 'Hurricanes',
  ice_storm: 'Ice storms',
  landslide: 'Landslides',
  lightning: 'Lightning',
  riverine_flooding: 'Flooding from rivers and heavy rain',
  strong_wind: 'Windstorms',
  tornado: 'Tornadoes',
  tsunami: 'Tsunamis',
  volcanic_activity: 'Volcanic activity',
  wildfire: 'Wildfires and smoke',
  winter_weather: 'Winter storms',
  pandemic: 'Pandemics',
  grid_failure: 'Regional power grid failure',
  cyber_outage: 'Cyber outages (payments, pharmacies, utilities)',
  civil_unrest: 'Civil unrest',
  supply_chain_disruption: 'Store shortages',
  hazmat_release: 'Chemical spills and releases',
  nuclear_plant_incident: 'Nuclear power plant accident',
  nuclear_attack: 'Nuclear attack or EMP',
  terrorism: 'Terrorism',
  job_loss: 'Job loss',
  house_fire: 'House fire',
  medical_emergency: 'Medical emergency',
  vehicle_stranding: 'Stranded while traveling',
  local_utility_outage: 'Local water, gas or power outage',
  burglary: 'Burglary',
  earner_death_or_disability: "An earner's death or disability",
  extended_household_illness: 'Long illness in the household',
  // Contract v2 (awaiting: web-risks — the mock's own rows for these).
  wildfire_smoke: 'Wildfire smoke',
  dust_storm: 'Dust storm',
  sinkhole: 'Sinkhole or ground collapse',
  geomagnetic_storm: 'Severe solar storm',
  vei7_eruption: 'Very large volcanic eruption',
  dam_failure: 'Dam or levee failure',
  network_outage: 'Phone or internet outage',
  drug_shortage: 'Medicine shortage',
  benefit_interruption: 'Government pay or benefits stop',
  attack_disruption: 'Attack or threat closes your area',
  multi_month_blackout: 'Power out for months (any cause)',
  war_infrastructure: 'War with attacks on US infrastructure',
  cbrn_attack: 'Chemical, biological or radiological attack',
  severe_pandemic: 'Severe pandemic',
  financial_crisis: 'Financial crisis with bank closures',
  mass_violence: 'Mass shooting or bombing',
  water_damage: 'Burst pipe or water leak',
  eviction: 'Eviction',
  arrest_or_detention: 'A household member is arrested or detained',
};

const NATURAL = new Set<HazardId>([
  'avalanche', 'coastal_flooding', 'cold_wave', 'drought', 'earthquake', 'hail', 'heat_wave',
  'hurricane', 'ice_storm', 'landslide', 'lightning', 'riverine_flooding', 'strong_wind', 'tornado',
  'tsunami', 'volcanic_activity', 'wildfire', 'winter_weather', 'wildfire_smoke', 'dust_storm',
  'sinkhole', 'geomagnetic_storm', 'vei7_eruption',
]);

const PERSONAL = new Set<HazardId>([
  'job_loss', 'house_fire', 'medical_emergency', 'vehicle_stranding', 'local_utility_outage',
  'burglary', 'earner_death_or_disability', 'extended_household_illness', 'water_damage',
  'eviction', 'arrest_or_detention',
]);

/** Natural, societal or personal, as `rr-types` defines them. */
export function hazardTier(id: HazardId): HazardTier {
  if (NATURAL.has(id)) return 'natural';
  if (PERSONAL.has(id)) return 'personal';
  return 'societal';
}

/** Every hazard the engine may emit: the retired ids are left out, as in the real catalogue. */
export const HAZARDS: HazardInfo[] = HAZARD_IDS.filter(
  (id) => !(RETIRED_HAZARD_IDS as readonly string[]).includes(id),
).map((id) => ({
  id,
  name: HAZARD_NAMES[id],
  tier: hazardTier(id),
}));

export function hazardName(id: HazardId): string {
  return HAZARD_NAMES[id];
}

export const BUCKETS: BucketInfo[] = [
  { id: 'power', name: 'No grid power at home', kind: 'duration', target_kind: 'days' },
  { id: 'water_boil', name: 'Tap water must be boiled or treated', kind: 'duration', target_kind: 'days' },
  { id: 'water_out', name: 'No tap water at all', kind: 'duration', target_kind: 'days' },
  { id: 'supplies', name: "Can't get to a store", kind: 'duration', target_kind: 'days' },
  { id: 'thermal', name: 'Dangerous heat or cold indoors', kind: 'duration', target_kind: 'days' },
  { id: 'medication', name: 'Medicine and medical supplies', kind: 'duration', target_kind: 'days' },
  { id: 'comms', name: 'No phone, internet or card payments', kind: 'duration', target_kind: 'days' },
  { id: 'evacuate', name: 'Must leave home quickly', kind: 'readiness', target_kind: 'evacuate' },
  { id: 'get_home', name: 'Stranded away from home', kind: 'readiness', target_kind: 'readiness' },
  { id: 'medical_emergency', name: 'Medical emergency when help is slow', kind: 'readiness', target_kind: 'readiness' },
  { id: 'fire', name: 'House fire', kind: 'readiness', target_kind: 'readiness' },
  { id: 'security', name: 'Home and personal security', kind: 'readiness', target_kind: 'readiness' },
  { id: 'clean_air', name: 'Unhealthy air indoors', kind: 'readiness', target_kind: 'readiness' },
  { id: 'income', name: 'Loss of income', kind: 'money', target_kind: 'months' },
  { id: 'home_loss', name: 'Home damaged or unlivable', kind: 'money', target_kind: 'readiness' },
];

export const TIERS: TierInfo[] = [
  { id: 'now', name: 'Free steps', days: 0 },
  { id: 'h72', name: 'Three days', days: 3 },
  { id: 'w2', name: 'Two weeks', days: 14 },
  { id: 'm1', name: 'One month', days: 30 },
  { id: 'm3', name: 'Three months', days: 90 },
  { id: 'm6', name: 'Six months', days: 180 },
  { id: 'y1', name: 'One year', days: 365 },
];
