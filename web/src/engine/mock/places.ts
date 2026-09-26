/**
 * The mock engine's geography: every state, a ZIP-prefix-to-state table, a small sample of real
 * counties (the seven fixture counties and some neighbours) and a handful of ZIP codes. Any other
 * well-formed ZIP code resolves to a "sample county" in its state, so the interface can be tried
 * with any address; the location's `data_note` says so. County shares for the multi-county ZIP
 * are illustrative, not measured.
 */
import type { FacilityFlags } from '../types';

export interface StateRow {
  fips: string;
  abbr: string;
  name: string;
  /** Fifth National Climate Assessment region. */
  nca: string;
}

export const STATES: StateRow[] = [
  { fips: '01', abbr: 'AL', name: 'Alabama', nca: 'southeast' },
  { fips: '02', abbr: 'AK', name: 'Alaska', nca: 'alaska' },
  { fips: '04', abbr: 'AZ', name: 'Arizona', nca: 'southwest' },
  { fips: '05', abbr: 'AR', name: 'Arkansas', nca: 'southeast' },
  { fips: '06', abbr: 'CA', name: 'California', nca: 'southwest' },
  { fips: '08', abbr: 'CO', name: 'Colorado', nca: 'southwest' },
  { fips: '09', abbr: 'CT', name: 'Connecticut', nca: 'northeast' },
  { fips: '10', abbr: 'DE', name: 'Delaware', nca: 'northeast' },
  { fips: '11', abbr: 'DC', name: 'District of Columbia', nca: 'northeast' },
  { fips: '12', abbr: 'FL', name: 'Florida', nca: 'southeast' },
  { fips: '13', abbr: 'GA', name: 'Georgia', nca: 'southeast' },
  { fips: '15', abbr: 'HI', name: 'Hawaii', nca: 'hawaii_pacific' },
  { fips: '16', abbr: 'ID', name: 'Idaho', nca: 'northwest' },
  { fips: '17', abbr: 'IL', name: 'Illinois', nca: 'midwest' },
  { fips: '18', abbr: 'IN', name: 'Indiana', nca: 'midwest' },
  { fips: '19', abbr: 'IA', name: 'Iowa', nca: 'midwest' },
  { fips: '20', abbr: 'KS', name: 'Kansas', nca: 'southern_great_plains' },
  { fips: '21', abbr: 'KY', name: 'Kentucky', nca: 'southeast' },
  { fips: '22', abbr: 'LA', name: 'Louisiana', nca: 'southeast' },
  { fips: '23', abbr: 'ME', name: 'Maine', nca: 'northeast' },
  { fips: '24', abbr: 'MD', name: 'Maryland', nca: 'northeast' },
  { fips: '25', abbr: 'MA', name: 'Massachusetts', nca: 'northeast' },
  { fips: '26', abbr: 'MI', name: 'Michigan', nca: 'midwest' },
  { fips: '27', abbr: 'MN', name: 'Minnesota', nca: 'midwest' },
  { fips: '28', abbr: 'MS', name: 'Mississippi', nca: 'southeast' },
  { fips: '29', abbr: 'MO', name: 'Missouri', nca: 'midwest' },
  { fips: '30', abbr: 'MT', name: 'Montana', nca: 'northern_great_plains' },
  { fips: '31', abbr: 'NE', name: 'Nebraska', nca: 'northern_great_plains' },
  { fips: '32', abbr: 'NV', name: 'Nevada', nca: 'southwest' },
  { fips: '33', abbr: 'NH', name: 'New Hampshire', nca: 'northeast' },
  { fips: '34', abbr: 'NJ', name: 'New Jersey', nca: 'northeast' },
  { fips: '35', abbr: 'NM', name: 'New Mexico', nca: 'southwest' },
  { fips: '36', abbr: 'NY', name: 'New York', nca: 'northeast' },
  { fips: '37', abbr: 'NC', name: 'North Carolina', nca: 'southeast' },
  { fips: '38', abbr: 'ND', name: 'North Dakota', nca: 'northern_great_plains' },
  { fips: '39', abbr: 'OH', name: 'Ohio', nca: 'midwest' },
  { fips: '40', abbr: 'OK', name: 'Oklahoma', nca: 'southern_great_plains' },
  { fips: '41', abbr: 'OR', name: 'Oregon', nca: 'northwest' },
  { fips: '42', abbr: 'PA', name: 'Pennsylvania', nca: 'northeast' },
  { fips: '44', abbr: 'RI', name: 'Rhode Island', nca: 'northeast' },
  { fips: '45', abbr: 'SC', name: 'South Carolina', nca: 'southeast' },
  { fips: '46', abbr: 'SD', name: 'South Dakota', nca: 'northern_great_plains' },
  { fips: '47', abbr: 'TN', name: 'Tennessee', nca: 'southeast' },
  { fips: '48', abbr: 'TX', name: 'Texas', nca: 'southern_great_plains' },
  { fips: '49', abbr: 'UT', name: 'Utah', nca: 'southwest' },
  { fips: '50', abbr: 'VT', name: 'Vermont', nca: 'northeast' },
  { fips: '51', abbr: 'VA', name: 'Virginia', nca: 'southeast' },
  { fips: '53', abbr: 'WA', name: 'Washington', nca: 'northwest' },
  { fips: '54', abbr: 'WV', name: 'West Virginia', nca: 'northeast' },
  { fips: '55', abbr: 'WI', name: 'Wisconsin', nca: 'midwest' },
  { fips: '56', abbr: 'WY', name: 'Wyoming', nca: 'northern_great_plains' },
  { fips: '72', abbr: 'PR', name: 'Puerto Rico', nca: 'caribbean' },
];

const STATE_BY_ABBR = new Map(STATES.map((s) => [s.abbr, s]));
const STATE_BY_FIPS = new Map(STATES.map((s) => [s.fips, s]));

export function stateByFips(fips: string): StateRow | undefined {
  return STATE_BY_FIPS.get(fips);
}

/** USPS three-digit ZIP prefixes by state: [first prefix, last prefix, state]. Unlisted prefixes are unassigned or military. */
const ZIP3: [number, number, string][] = [
  [5, 5, 'NY'], [6, 7, 'PR'], [9, 9, 'PR'], [10, 27, 'MA'], [28, 29, 'RI'], [30, 38, 'NH'],
  [39, 49, 'ME'], [50, 54, 'VT'], [55, 55, 'MA'], [56, 59, 'VT'], [60, 69, 'CT'], [70, 89, 'NJ'],
  [100, 149, 'NY'], [150, 196, 'PA'], [197, 199, 'DE'], [200, 200, 'DC'], [201, 201, 'VA'],
  [202, 205, 'DC'], [206, 219, 'MD'], [220, 246, 'VA'], [247, 268, 'WV'], [270, 289, 'NC'],
  [290, 299, 'SC'], [300, 319, 'GA'], [320, 339, 'FL'], [341, 342, 'FL'], [344, 344, 'FL'],
  [346, 347, 'FL'], [349, 349, 'FL'], [350, 352, 'AL'], [354, 369, 'AL'], [370, 385, 'TN'],
  [386, 397, 'MS'], [398, 399, 'GA'], [400, 418, 'KY'], [420, 427, 'KY'], [430, 459, 'OH'],
  [460, 479, 'IN'], [480, 499, 'MI'], [500, 516, 'IA'], [520, 528, 'IA'], [530, 532, 'WI'],
  [534, 535, 'WI'], [537, 549, 'WI'], [550, 551, 'MN'], [553, 567, 'MN'], [569, 569, 'DC'],
  [570, 577, 'SD'], [580, 588, 'ND'], [590, 599, 'MT'], [600, 620, 'IL'], [622, 629, 'IL'],
  [630, 631, 'MO'], [633, 641, 'MO'], [644, 658, 'MO'], [660, 662, 'KS'], [664, 679, 'KS'],
  [680, 681, 'NE'], [683, 693, 'NE'], [700, 708, 'LA'], [710, 714, 'LA'], [716, 729, 'AR'],
  [730, 731, 'OK'], [733, 733, 'TX'], [734, 741, 'OK'], [743, 749, 'OK'], [750, 770, 'TX'],
  [772, 799, 'TX'], [800, 816, 'CO'], [820, 831, 'WY'], [832, 838, 'ID'], [840, 847, 'UT'],
  [850, 853, 'AZ'], [855, 857, 'AZ'], [859, 860, 'AZ'], [863, 865, 'AZ'], [870, 875, 'NM'],
  [877, 884, 'NM'], [885, 885, 'TX'], [889, 891, 'NV'], [893, 895, 'NV'], [897, 898, 'NV'],
  [900, 908, 'CA'], [910, 928, 'CA'], [930, 961, 'CA'], [967, 968, 'HI'], [970, 979, 'OR'],
  [980, 986, 'WA'], [988, 994, 'WA'], [995, 999, 'AK'],
];

export function stateForZip(zip: string): StateRow | undefined {
  const prefix = Number(zip.slice(0, 3));
  const row = ZIP3.find(([lo, hi]) => prefix >= lo && prefix <= hi);
  return row ? STATE_BY_ABBR.get(row[2]) : undefined;
}

export interface CountyRow {
  fips: string;
  name: string;
  lat: number;
  lon: number;
  coastal: boolean;
  tsunami_zone: boolean;
  facilities: FacilityFlags;
  /** Which hazard profile in regions.ts; defaults to the state's climate region. */
  profile?: string;
}

const NONE: FacilityFlags = {
  nuclear_plant_within_16km: false,
  nuclear_plant_within_80km: false,
  hazmat_facilities_within_5km: 0,
};

function near80(hazmat: number): FacilityFlags {
  return { nuclear_plant_within_16km: false, nuclear_plant_within_80km: true, hazmat_facilities_within_5km: hazmat };
}

export const COUNTIES: CountyRow[] = [
  { fips: '42101', name: 'Philadelphia County', lat: 40.0, lon: -75.13, coastal: false, tsunami_zone: false, facilities: near80(12), profile: 'philadelphia' },
  { fips: '42045', name: 'Delaware County', lat: 39.92, lon: -75.4, coastal: false, tsunami_zone: false, facilities: near80(6), profile: 'philadelphia' },
  { fips: '42029', name: 'Chester County', lat: 39.97, lon: -75.75, coastal: false, tsunami_zone: false, facilities: near80(3), profile: 'philadelphia' },
  { fips: '42091', name: 'Montgomery County', lat: 40.21, lon: -75.37, coastal: false, tsunami_zone: false, facilities: near80(5), profile: 'philadelphia' },
  { fips: '42017', name: 'Bucks County', lat: 40.34, lon: -75.11, coastal: false, tsunami_zone: false, facilities: near80(3), profile: 'philadelphia' },
  { fips: '41011', name: 'Coos County', lat: 43.18, lon: -124.09, coastal: true, tsunami_zone: true, facilities: { ...NONE, hazmat_facilities_within_5km: 1 }, profile: 'coos_bay' },
  { fips: '41015', name: 'Curry County', lat: 42.46, lon: -124.16, coastal: true, tsunami_zone: true, facilities: NONE, profile: 'coos_bay' },
  { fips: '41019', name: 'Douglas County', lat: 43.28, lon: -123.18, coastal: true, tsunami_zone: true, facilities: NONE, profile: 'coos_bay' },
  { fips: '12086', name: 'Miami-Dade County', lat: 25.61, lon: -80.5, coastal: true, tsunami_zone: false, facilities: near80(8), profile: 'miami' },
  { fips: '12011', name: 'Broward County', lat: 26.15, lon: -80.45, coastal: true, tsunami_zone: false, facilities: near80(5), profile: 'miami' },
  { fips: '20051', name: 'Ellis County', lat: 38.91, lon: -99.32, coastal: false, tsunami_zone: false, facilities: NONE, profile: 'hays' },
  { fips: '04013', name: 'Maricopa County', lat: 33.35, lon: -112.49, coastal: false, tsunami_zone: false, facilities: near80(9), profile: 'phoenix' },
  { fips: '48157', name: 'Fort Bend County', lat: 29.53, lon: -95.77, coastal: false, tsunami_zone: false, facilities: { ...NONE, hazmat_facilities_within_5km: 6 }, profile: 'sugar_land' },
  { fips: '48201', name: 'Harris County', lat: 29.86, lon: -95.39, coastal: true, tsunami_zone: false, facilities: { ...NONE, hazmat_facilities_within_5km: 14 }, profile: 'sugar_land' },
  { fips: '17031', name: 'Cook County', lat: 41.84, lon: -87.82, coastal: false, tsunami_zone: false, facilities: near80(10), profile: 'chicago' },
  { fips: '17043', name: 'DuPage County', lat: 41.85, lon: -88.09, coastal: false, tsunami_zone: false, facilities: near80(4), profile: 'chicago' },
  { fips: '06037', name: 'Los Angeles County', lat: 34.32, lon: -118.22, coastal: true, tsunami_zone: true, facilities: { ...NONE, hazmat_facilities_within_5km: 11 } },
  { fips: '53033', name: 'King County', lat: 47.49, lon: -121.83, coastal: true, tsunami_zone: false, facilities: { ...NONE, hazmat_facilities_within_5km: 5 } },
  { fips: '36047', name: 'Kings County', lat: 40.64, lon: -73.95, coastal: true, tsunami_zone: false, facilities: near80(9) },
  { fips: '25025', name: 'Suffolk County', lat: 42.33, lon: -71.07, coastal: true, tsunami_zone: false, facilities: near80(4) },
  { fips: '13121', name: 'Fulton County', lat: 33.79, lon: -84.47, coastal: false, tsunami_zone: false, facilities: { ...NONE, hazmat_facilities_within_5km: 4 } },
  { fips: '08031', name: 'Denver County', lat: 39.76, lon: -104.88, coastal: false, tsunami_zone: false, facilities: { ...NONE, hazmat_facilities_within_5km: 3 } },
];

const COUNTY_BY_FIPS = new Map(COUNTIES.map((c) => [c.fips, c]));

export function countyByFips(fips: string): CountyRow | undefined {
  return COUNTY_BY_FIPS.get(fips);
}

/** ZIP code to counties, largest share first. Shares for 19087 are illustrative. */
export const ZIPS: Record<string, { county: string; share: number }[]> = {
  '19147': [{ county: '42101', share: 1 }],
  '19103': [{ county: '42101', share: 1 }],
  '19087': [
    { county: '42045', share: 0.46 },
    { county: '42029', share: 0.34 },
    { county: '42091', share: 0.2 },
  ],
  '19010': [
    { county: '42091', share: 0.86 },
    { county: '42045', share: 0.14 },
  ],
  '97420': [{ county: '41011', share: 1 }],
  '33139': [{ county: '12086', share: 1 }],
  '85008': [{ county: '04013', share: 1 }],
  '77479': [{ county: '48157', share: 1 }],
  '60637': [{ county: '17031', share: 1 }],
  '67601': [{ county: '20051', share: 1 }],
};

/** The share at or above which a ZIP code resolves to its largest county without asking. */
export const ZIP_MAJORITY = 0.8;
