/**
 * The mock engine's sources. Every id starts with `mock_`, every title with "Mock:", and every URL
 * points at example.org, so nobody mistakes a placeholder for a real source. Each one names the
 * real source it stands in for, so the shape of the sources list is realistic.
 */
import type { Attribution, Citation } from '../types';

const RETRIEVED = '2026-09-25';
const PUBLISHER = 'Ready Reckoner mock engine (placeholder, not a real source)';
const LICENSE = 'Placeholder for testing the interface';

function mock(id: string, stands_for: string, prior = false): Citation {
  return {
    id,
    title: `Mock: ${stands_for}`,
    publisher: PUBLISHER,
    year: 2026,
    url: `https://example.org/ready-reckoner-mock/${id}`,
    retrieved: RETRIEVED,
    license: LICENSE,
    prior,
  };
}

export const CITATIONS: Citation[] = [
  mock('mock_nri_county', 'county hazard frequencies (stands in for the FEMA National Risk Index)'),
  mock('mock_eaglei_outages', 'county power outage records (stands in for EAGLE-I)'),
  mock('mock_utility_reliability', 'utility reliability report (stands in for EIA-861)'),
  mock('mock_boil_notices', 'boil-water notice durations (stands in for state records)'),
  mock('mock_hazus_restoration', 'service restoration times (stands in for Hazus tables)'),
  mock('mock_oregon_resilience', 'regional recovery timelines (stands in for the Oregon Resilience Plan)'),
  mock('mock_cascadia_usgs', 'Cascadia earthquake chances (stands in for USGS and DOGAMI)'),
  mock('mock_tsunami_zone', 'tsunami evacuation zones and warning times'),
  mock('mock_hurricane_climatology', 'hurricane frequencies (stands in for NOAA)'),
  mock('mock_heat_projections', 'heat projections for around 2050 (stands in for NCA5 and CMRA)'),
  mock('mock_climate_multipliers', 'climate multipliers for around 2050 (stands in for NCA5 and CMRA)'),
  mock('mock_bls_job_loss', 'job loss rates (stands in for the Bureau of Labor Statistics)'),
  mock('mock_ui_benefits', 'unemployment insurance durations'),
  mock('mock_usfa_fires', 'home fire rates (stands in for USFA and NFPA)'),
  mock('mock_ed_visits', 'emergency department visits (stands in for CDC survey data)'),
  mock('mock_crash_injury', 'crash and stranding rates'),
  mock('mock_burglary', 'burglary rates (stands in for the Bureau of Justice Statistics)'),
  mock('mock_pandemic_stayhome', '2020 stay-home durations'),
  mock('mock_societal_prior', 'expert estimate for societal hazards', true),
  mock('mock_rare_prior', 'expert estimate for rare catastrophic hazards', true),
  mock('mock_duration_prior', 'expert estimate of how long disruptions last', true),
  mock('mock_water_per_person', 'water per person per day (stands in for Ready.gov and Sphere)'),
  mock('mock_food_kcal', 'calories per person per day (stands in for the Dietary Guidelines)'),
  mock('mock_medication_reserve', 'medication reserve guidance (stands in for the Red Cross and CDC)'),
  mock('mock_price_survey', 'retail price observations'),
  mock('mock_ready_gov_kit', 'emergency kit guidance (stands in for Ready.gov)'),
  mock('mock_fire_safety', 'smoke and CO alarm guidance (stands in for USFA and NFPA)'),
  mock('mock_social_capital', 'neighbours and disaster recovery (stands in for published research)'),
  mock('mock_evacuation', 'evacuation notice times'),
  mock('mock_get_home', 'walking pace for getting home'),
  mock('mock_savings_track', 'emergency savings guidance (stands in for the CFPB)'),
  mock('mock_insurance', 'insurance guidance (stands in for NFIP and state insurance offices)'),
  mock('mock_typical_budget', 'a typical monthly preparedness budget', true),
  mock('mock_nuclear_guidance', 'shelter guidance (stands in for Ready.gov)'),
  mock('mock_firearm_storage', 'safe storage guidance (stands in for public-health guidance)'),
  mock('mock_drills', 'drills and practice (stands in for published drill evaluations)'),
];

const BY_ID = new Map(CITATIONS.map((c) => [c.id, c]));

export function citation(id: string): Citation | undefined {
  return BY_ID.get(id);
}

/**
 * Sample credit lines. The real engine carries the FEMA National Risk Index terms (dataset
 * version, access date, "not endorsed by FEMA") and the CC BY credits; these say so plainly.
 */
export const ATTRIBUTIONS: Attribution[] = [
  {
    source: 'FEMA National Risk Index',
    text:
      'Sample credit line from the mock engine. The real engine shows: This product uses the FEMA ' +
      'National Risk Index (version 1.20.0, December 2025), accessed 2026-09-25, but is not endorsed ' +
      'by FEMA. For planning purposes only.',
    url: 'https://www.fema.gov/about/reports-and-data/openfema/nri/v120/',
    version: '1.20.0',
    accessed: RETRIEVED,
  },
  {
    source: 'EAGLE-I recorded outages',
    text:
      'Sample credit line from the mock engine. The real engine shows: Power outage statistics ' +
      'derived from EAGLE-I recorded outages 2014–2025, Oak Ridge National Laboratory, licensed CC BY 4.0.',
    url: 'https://doi.org/10.6084/m9.figshare.24237376.v4',
    version: 'v4',
    accessed: RETRIEVED,
  },
  {
    source: 'Fifth National Climate Assessment Atlas',
    text:
      'Sample credit line from the mock engine. The real engine shows: Climate projections derived ' +
      'from the Fifth National Climate Assessment Atlas, licensed CC BY 4.0.',
    url: 'https://www.globalchange.gov/',
    accessed: RETRIEVED,
  },
];
