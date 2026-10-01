/**
 * The content's incident checklists (`content/checklists/*.md` front matter: id, title, onset,
 * what they apply to, pages), for the mock binder's page list. `checklists.test.ts` holds this
 * table to the files, so it cannot drift from the content.
 */
export interface MockChecklist {
  id: string;
  title: string;
  onset: 'now' | 'coming' | 'ongoing';
  /** `hazard:<id>` and `event:<id>` targets. */
  applies_to: string[];
  pages: 1 | 2;
}

export const MOCK_CHECKLISTS: readonly MockChecklist[] = [
  { id: 'check_arrest_or_detention', title: 'A household member is arrested or detained', onset: 'ongoing', applies_to: ['hazard:arrest_or_detention'], pages: 1 },
  { id: 'check_attack_disruption', title: 'An attack or threat closes your area', onset: 'now', applies_to: ['hazard:attack_disruption'], pages: 1 },
  { id: 'check_avalanche', title: 'Avalanche', onset: 'now', applies_to: ['hazard:avalanche'], pages: 1 },
  { id: 'check_benefit_interruption', title: 'Government pay or benefits stop', onset: 'ongoing', applies_to: ['hazard:benefit_interruption'], pages: 1 },
  { id: 'check_boil_water', title: 'Boil-water notice', onset: 'coming', applies_to: ['event:boil_water_notice'], pages: 1 },
  { id: 'check_break_in', title: 'Break-in', onset: 'now', applies_to: ['hazard:burglary'], pages: 1 },
  { id: 'check_cbrn_attack', title: 'Chemical, biological or radiological attack', onset: 'now', applies_to: ['hazard:cbrn_attack'], pages: 1 },
  { id: 'check_chemical_release', title: 'Chemical spill or release', onset: 'now', applies_to: ['hazard:hazmat_release'], pages: 1 },
  { id: 'check_civil_unrest', title: 'Civil unrest', onset: 'ongoing', applies_to: ['hazard:civil_unrest'], pages: 1 },
  { id: 'check_coastal_flooding', title: 'Coastal flooding and storm surge', onset: 'coming', applies_to: ['hazard:coastal_flooding'], pages: 1 },
  { id: 'check_cold_wave', title: 'Extreme cold', onset: 'coming', applies_to: ['hazard:cold_wave'], pages: 1 },
  { id: 'check_cyber_outage', title: 'Cyberattack on services', onset: 'ongoing', applies_to: ['hazard:cyber_outage'], pages: 1 },
  { id: 'check_dam_failure', title: 'Dam or levee failure', onset: 'now', applies_to: ['hazard:dam_failure'], pages: 1 },
  { id: 'check_drought', title: 'Drought', onset: 'coming', applies_to: ['hazard:drought'], pages: 1 },
  { id: 'check_drug_shortage', title: 'Medicine shortage', onset: 'ongoing', applies_to: ['hazard:drug_shortage'], pages: 1 },
  { id: 'check_dust_storm', title: 'Dust storm', onset: 'coming', applies_to: ['hazard:dust_storm'], pages: 1 },
  { id: 'check_earner_death_or_disability', title: 'An earner dies or can no longer work', onset: 'ongoing', applies_to: ['hazard:earner_death_or_disability'], pages: 1 },
  { id: 'check_earthquake', title: 'Earthquake', onset: 'now', applies_to: ['hazard:earthquake'], pages: 1 },
  { id: 'check_evacuation_order', title: 'Evacuation order', onset: 'coming', applies_to: ['event:evacuation_order'], pages: 1 },
  { id: 'check_eviction', title: 'Eviction', onset: 'ongoing', applies_to: ['hazard:eviction'], pages: 1 },
  { id: 'check_extended_household_illness', title: 'Long illness in the household', onset: 'ongoing', applies_to: ['hazard:extended_household_illness'], pages: 1 },
  { id: 'check_financial_crisis', title: 'Bank closures or a bank failure', onset: 'ongoing', applies_to: ['hazard:financial_crisis'], pages: 1 },
  { id: 'check_flooding', title: 'Flooding and flash floods', onset: 'now', applies_to: ['hazard:riverine_flooding'], pages: 1 },
  { id: 'check_gas_leak_or_co', title: 'Gas leak or carbon monoxide alarm', onset: 'now', applies_to: ['event:gas_leak_or_co'], pages: 1 },
  { id: 'check_grid_failure', title: 'Regional blackout', onset: 'ongoing', applies_to: ['hazard:grid_failure'], pages: 1 },
  { id: 'check_heat_wave', title: 'Heat wave', onset: 'coming', applies_to: ['hazard:heat_wave'], pages: 1 },
  { id: 'check_house_fire', title: 'House fire', onset: 'now', applies_to: ['hazard:house_fire'], pages: 1 },
  { id: 'check_hurricane', title: 'Hurricane or tropical storm', onset: 'coming', applies_to: ['hazard:hurricane'], pages: 2 },
  { id: 'check_ice_storm', title: 'Ice storm', onset: 'coming', applies_to: ['hazard:ice_storm'], pages: 1 },
  { id: 'check_job_loss', title: 'Job loss', onset: 'ongoing', applies_to: ['hazard:job_loss'], pages: 1 },
  { id: 'check_landslide', title: 'Landslide or debris flow', onset: 'now', applies_to: ['hazard:landslide'], pages: 1 },
  { id: 'check_local_utility_outage', title: 'No tap water, or a local water or gas outage', onset: 'ongoing', applies_to: ['hazard:local_utility_outage'], pages: 1 },
  { id: 'check_mass_violence', title: 'An attack or explosion in a public place', onset: 'now', applies_to: ['hazard:mass_violence'], pages: 1 },
  { id: 'check_medical_emergency', title: 'Medical emergency', onset: 'now', applies_to: ['hazard:medical_emergency'], pages: 1 },
  { id: 'check_missing_person', title: 'Missing person', onset: 'now', applies_to: ['event:missing_person'], pages: 1 },
  { id: 'check_multi_month_blackout', title: 'Power out for months', onset: 'ongoing', applies_to: ['hazard:multi_month_blackout'], pages: 1 },
  { id: 'check_network_outage', title: 'Phone or internet outage', onset: 'ongoing', applies_to: ['hazard:network_outage'], pages: 1 },
  { id: 'check_nuclear_attack', title: 'Nuclear attack or EMP', onset: 'now', applies_to: ['hazard:nuclear_attack'], pages: 2 },
  { id: 'check_nuclear_plant', title: 'Nuclear power plant accident', onset: 'now', applies_to: ['hazard:nuclear_plant_incident'], pages: 1 },
  { id: 'check_pandemic', title: 'Pandemic', onset: 'coming', applies_to: ['hazard:pandemic'], pages: 1 },
  { id: 'check_power_outage', title: 'Power outage at home', onset: 'ongoing', applies_to: ['event:power_outage'], pages: 1 },
  { id: 'check_severe_pandemic', title: 'Severe pandemic', onset: 'coming', applies_to: ['hazard:severe_pandemic'], pages: 1 },
  { id: 'check_severe_thunderstorm', title: 'Severe thunderstorm, strong wind, hail or lightning', onset: 'now', applies_to: ['hazard:strong_wind', 'hazard:hail', 'hazard:lightning'], pages: 1 },
  { id: 'check_shelter_in_place', title: 'Shelter-in-place order', onset: 'coming', applies_to: ['event:shelter_in_place'], pages: 1 },
  { id: 'check_sinkhole', title: 'Sinkhole', onset: 'now', applies_to: ['hazard:sinkhole'], pages: 1 },
  { id: 'check_solar_storm', title: 'Severe solar storm', onset: 'coming', applies_to: ['hazard:geomagnetic_storm'], pages: 1 },
  { id: 'check_something_else', title: 'Something else', onset: 'ongoing', applies_to: ['event:something_else'], pages: 1 },
  { id: 'check_supply_chain_disruption', title: 'Supply chain disruption', onset: 'ongoing', applies_to: ['hazard:supply_chain_disruption'], pages: 1 },
  { id: 'check_tornado', title: 'Tornado', onset: 'now', applies_to: ['hazard:tornado'], pages: 1 },
  { id: 'check_tsunami', title: 'Tsunami', onset: 'now', applies_to: ['hazard:tsunami'], pages: 1 },
  { id: 'check_vehicle_stranding', title: 'Stranded in a vehicle', onset: 'now', applies_to: ['hazard:vehicle_stranding'], pages: 1 },
  { id: 'check_volcanic_eruption', title: 'Volcanic eruption and ashfall', onset: 'now', applies_to: ['hazard:volcanic_activity', 'hazard:vei7_eruption'], pages: 1 },
  { id: 'check_war_infrastructure', title: 'War with attacks on US infrastructure', onset: 'ongoing', applies_to: ['hazard:war_infrastructure'], pages: 1 },
  { id: 'check_water_leak', title: 'Burst pipe or water leak', onset: 'now', applies_to: ['hazard:water_damage'], pages: 1 },
  { id: 'check_wildfire', title: 'Wildfire', onset: 'now', applies_to: ['hazard:wildfire'], pages: 1 },
  { id: 'check_wildfire_smoke', title: 'Wildfire smoke', onset: 'coming', applies_to: ['hazard:wildfire_smoke'], pages: 1 },
  { id: 'check_winter_storm', title: 'Winter storm or blizzard', onset: 'coming', applies_to: ['hazard:winter_weather'], pages: 1 },
];
