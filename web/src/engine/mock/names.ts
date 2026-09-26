/**
 * Plain names for every id, as the mock engine's `catalogue()` returns them. The app reads names
 * from `catalogue()` and never from here, so the real engine can rename things freely.
 */
import type { BucketInfo, HazardId, HazardInfo, HazardTier, TierInfo } from '../types';
import { HAZARD_IDS } from '../types';

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
};

function hazardTier(index: number): HazardTier {
  if (index < 18) return 'natural';
  if (index < 27) return 'societal';
  return 'personal';
}

export const HAZARDS: HazardInfo[] = HAZARD_IDS.map((id, i) => ({
  id,
  name: HAZARD_NAMES[id],
  tier: hazardTier(i),
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
