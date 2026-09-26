/**
 * The mock engine: a deterministic implementation of every ENGINE-API function, so the whole
 * interface can be built, tested and screenshotted before the WebAssembly engine exists.
 *
 * It is plausible, not real: targets for Philadelphia and Coos Bay follow the worked examples in
 * docs/research/risk-model.md, other places use climate-region stand-ins, and every source is a
 * `mock_*` placeholder. Results pass through JSON exactly as they would across the WebAssembly
 * boundary, so the app never depends on anything the real engine could not send.
 */
import type { Engine } from './index';
import type {
  Catalogue,
  EngineError,
  EngineInfo,
  Envelope,
  ErrorCode,
  Explanation,
  ExplainRequest,
  GuidanceMeta,
  LocationInput,
  LocationResolved,
  PackInfo,
  PlanInput,
  PlanOutput,
} from './types';
import { ENGINE_API_VERSION, EXPLAIN_KINDS } from './types';
import { ATTRIBUTIONS, CITATIONS } from './mock/citations';
import { explainFrom } from './mock/explain';
import { ITEMS } from './mock/items';
import { assessModel, MOCK_CONTENT_VERSION, MOCK_DATA_VERSION, MOCK_ENGINE_VERSION } from './mock/model';
import { BUCKETS, HAZARDS, TIERS } from './mock/names';
import type { CountyRow, StateRow } from './mock/places';
import { COUNTIES, countyByFips, stateByFips, stateForZip, STATES, ZIP_MAJORITY, ZIPS } from './mock/places';
import { validatePlanInput } from './mock/validate';

/** The JSON round trip the WebAssembly boundary imposes: no undefined, no shared references. */
function wire<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function ok<T>(value: T): Envelope<T> {
  return { ok: true, value: wire(value) };
}

function err<T>(code: ErrorCode, message: string, details?: unknown): Envelope<T> {
  const error: EngineError = { code, message };
  if (details !== undefined) error.details = wire(details);
  return { ok: false, error };
}

const MESSAGES = {
  unknown_zip: "We couldn't find that ZIP code. Check the digits, or search for your county by name.",
  unknown_county: "We couldn't find that county. Try searching by name.",
  ambiguous_zip: 'That ZIP code covers more than one county. Choose the one you live in.',
  bad_input: 'Some answers need another look.',
  internal: 'Something went wrong inside the planner. Your answers are safe. Try again, or export your plan and report the problem.',
} as const;

// ---------------------------------------------------------------------------------------------
// Places
// ---------------------------------------------------------------------------------------------

const REGION_CENTROID: Record<string, [number, number]> = {
  northeast: [41.5, -75],
  southeast: [33, -84],
  midwest: [41.5, -89],
  northern_great_plains: [45, -103],
  southern_great_plains: [33, -98],
  southwest: [36, -112],
  northwest: [45, -120],
  alaska: [61, -150],
  hawaii_pacific: [20.8, -157],
  caribbean: [18.2, -66.5],
};

interface Resolved {
  location: LocationResolved;
  profile: string | undefined;
}

function fromCounty(row: CountyRow, zip?: string, share?: number): Resolved {
  const state = stateByFips(row.fips.slice(0, 2))!;
  const location: LocationResolved = {
    country: 'US',
    county_fips: row.fips,
    county_name: row.name,
    state_abbr: state.abbr,
    state_name: state.name,
    centroid: { lat: row.lat, lon: row.lon },
    nca_region: state.nca,
    coastal: row.coastal,
    tsunami_zone: row.tsunami_zone,
    facility_flags: row.facilities,
    data_note:
      share !== undefined && share < 1
        ? `Sample data. About ${Math.round(share * 100)} in 100 addresses in this ZIP code are in ${row.name}; the plan uses ${row.name}.`
        : 'Sample data for your county as a whole; street-level detail is not loaded.',
  };
  if (zip !== undefined) location.zip = zip;
  if (share !== undefined) location.zip_county_share = share;
  return { location, profile: row.profile };
}

function sampleCounty(state: StateRow, zip?: string): Resolved {
  const [lat, lon] = REGION_CENTROID[state.nca] ?? [39, -98];
  const location: LocationResolved = {
    country: 'US',
    county_fips: `${state.fips}999`,
    county_name: 'Sample county',
    state_abbr: state.abbr,
    state_name: state.name,
    centroid: { lat, lon },
    nca_region: state.nca,
    coastal: false,
    tsunami_zone: false,
    facility_flags: { nuclear_plant_within_16km: false, nuclear_plant_within_80km: false, hazmat_facilities_within_5km: 0 },
    data_note: `Sample data. The mock engine does not know this ${zip ? 'ZIP code' : 'county'}, so it uses typical hazards for ${state.name}.`,
  };
  if (zip !== undefined) location.zip = zip;
  return { location, profile: undefined };
}

function resolve(input: LocationInput): Envelope<Resolved> {
  const problems = [];
  if (input.country !== 'US') problems.push({ code: 'unsupported_country', field: 'location.country', message: 'Only places in the United States are supported so far.' });
  if (input.zip === undefined && input.county_fips === undefined) problems.push({ code: 'location_missing', field: 'location', message: 'Enter a ZIP code or choose a county.' });
  if (input.zip !== undefined && !/^\d{5}$/.test(input.zip)) problems.push({ code: 'zip_format', field: 'location.zip', message: 'A ZIP code is five digits, like 19147.' });
  if (input.county_fips !== undefined && !/^\d{5}$/.test(input.county_fips)) problems.push({ code: 'county_fips_format', field: 'location.county_fips', message: 'A county code is five digits, like 42101.' });
  if (problems.length) return err('bad_input', MESSAGES.bad_input, { problems });

  if (input.county_fips !== undefined) {
    const row = countyByFips(input.county_fips);
    const share = input.zip !== undefined ? ZIPS[input.zip]?.find((z) => z.county === input.county_fips)?.share : undefined;
    if (row) return { ok: true, value: fromCounty(row, input.zip, share) };
    const state = input.county_fips.endsWith('999') ? stateByFips(input.county_fips.slice(0, 2)) : undefined;
    if (state) return { ok: true, value: sampleCounty(state, input.zip) };
    const inState = COUNTIES.filter((c) => c.fips.startsWith(input.county_fips!.slice(0, 2))).slice(0, 5);
    return err('unknown_county', MESSAGES.unknown_county, { suggestions: inState.map((c) => fromCounty(c).location) });
  }

  const zip = input.zip!;
  const known = ZIPS[zip];
  if (known) {
    const top = known[0]!;
    if (top.share >= ZIP_MAJORITY) return { ok: true, value: fromCounty(countyByFips(top.county)!, zip, top.share) };
    return err('ambiguous_zip', MESSAGES.ambiguous_zip, {
      suggestions: known.map((z) => fromCounty(countyByFips(z.county)!, zip, z.share).location),
    });
  }
  const state = zip === '00000' ? undefined : stateForZip(zip);
  if (!state) return err('unknown_zip', MESSAGES.unknown_zip, { suggestions: [] });
  return { ok: true, value: sampleCounty(state, zip) };
}

function search(query: string): LocationResolved[] {
  const q = query.trim().toLowerCase().replace(/\s+/g, ' ');
  if (!q) return [];
  if (/^\d{5}$/.test(q)) {
    const row = countyByFips(q);
    if (row) return [fromCounty(row).location];
    const zip = ZIPS[q];
    if (zip) return zip.map((z) => fromCounty(countyByFips(z.county)!, q, z.share).location);
    return [];
  }
  if (/^\d{1,4}$/.test(q)) return COUNTIES.filter((c) => c.fips.startsWith(q)).slice(0, 10).map((c) => fromCounty(c).location);
  const [namePart, statePart] = q.split(',').map((s) => s.trim());
  const stateFilter = statePart
    ? STATES.filter((s) => s.abbr.toLowerCase() === statePart || s.name.toLowerCase().startsWith(statePart)).map((s) => s.fips)
    : undefined;
  const name = (namePart ?? '').replace(/ county$/, '');
  const scored = COUNTIES.flatMap((c) => {
    const state = stateByFips(c.fips.slice(0, 2))!;
    if (stateFilter && !stateFilter.includes(state.fips)) return [];
    const county = c.name.toLowerCase().replace(/ county$/, '');
    let score = -1;
    if (county.startsWith(name)) score = 0;
    else if (county.includes(name)) score = 1;
    else if (!statePart && (state.name.toLowerCase().startsWith(name) || state.abbr.toLowerCase() === name)) score = 2;
    return score < 0 ? [] : [{ c, score }];
  });
  scored.sort((a, b) => a.score - b.score || a.c.name.localeCompare(b.c.name));
  return scored.slice(0, 10).map((s) => fromCounty(s.c).location);
}

// ---------------------------------------------------------------------------------------------
// Content and defaults
// ---------------------------------------------------------------------------------------------

const GUIDANCE: GuidanceMeta[] = [
  { id: 'bucket_power', title: 'Going without power', applies_to: ['bucket:power'], citations: ['mock_eaglei_outages'] },
  { id: 'bucket_water', title: 'Going without safe tap water', applies_to: ['bucket:water_out', 'bucket:water_boil'], citations: ['mock_water_per_person'] },
  { id: 'bucket_supplies', title: "When you can't get to a store", applies_to: ['bucket:supplies'], citations: ['mock_food_kcal'] },
  { id: 'bucket_medication', title: 'Keeping medicine going', applies_to: ['bucket:medication'], citations: ['mock_medication_reserve'] },
  { id: 'hazard_nuclear', title: 'Nuclear emergencies: get inside, stay inside, stay tuned', applies_to: ['hazard:nuclear_attack', 'hazard:nuclear_plant_incident'], citations: ['mock_nuclear_guidance'] },
  { id: 'topic_myths', title: 'Disaster myths', applies_to: ['topic:myths'], citations: ['mock_social_capital'] },
];

export function mockDefaults(): PlanInput {
  return {
    planning_date: '2026-10-01',
    location: { country: 'US', zip: '00000', setting: 'suburban' },
    housing: {
      kind: 'detached',
      tenure: 'own',
      floor: 1,
      basement: false,
      water: 'municipal',
      sewer: 'sewer',
      heating: 'gas',
      cooling: 'central',
      backup_power: 'none',
      alarms: { smoke: false, co: false, extinguisher: false },
    },
    people: [
      {
        age_band: 'adult',
        pregnant_or_nursing: false,
        medical: { daily_rx: false, refrigerated_rx: false, powered_device: 'none', mobility: 'none', dietary: [], epinephrine: false },
        earner: true,
      },
    ],
    pets: { dogs: 0, cats: 0, small: 0, large_animals: 0 },
    mobility: { vehicles: [] },
    finances: {
      monthly_budget_usd: 0,
      one_off_budget_usd: 0,
      emergency_fund_months: 0,
      income: { earners: 1, stability: 'stable' },
      insurance: { home_or_renters: false, flood: false, earthquake: false },
    },
    existing: [],
    dials: { return_period: 'one_in_100', climate: 'today', horizon_years: 10, water_level: 'basic', scenario_overrides: [] },
  };
}

// ---------------------------------------------------------------------------------------------
// The engine
// ---------------------------------------------------------------------------------------------

function run(input: PlanInput): Envelope<ReturnType<typeof assessModel>> {
  const problems = validatePlanInput(input);
  if (problems.length) return err('bad_input', MESSAGES.bad_input, { problems });
  const place = resolve(input.location);
  if (!place.ok) return place;
  return { ok: true, value: assessModel(input, place.value.location, place.value.profile) };
}

function guard<T>(f: () => Envelope<T>): Promise<Envelope<T>> {
  try {
    return Promise.resolve(f());
  } catch (e) {
    return Promise.resolve(err<T>('internal', MESSAGES.internal, { reason: String(e) }));
  }
}

export function createMockEngine(): Engine {
  const packs = new Map<string, PackInfo>();
  return {
    engine_info: () =>
      guard(() => {
        const info: EngineInfo = {
          engine_version: MOCK_ENGINE_VERSION,
          api_version: ENGINE_API_VERSION,
          content_version: MOCK_CONTENT_VERSION,
          packs_loaded: [...packs.keys()],
          attributions: ATTRIBUTIONS,
        };
        if (packs.size > 0) info.data_pack_version = MOCK_DATA_VERSION;
        return ok(info);
      }),
    load_pack: (name, bytes) =>
      guard(() => {
        if (!name) return err('bad_input', MESSAGES.bad_input, { problems: [{ code: 'schema', field: 'name', message: 'A data pack needs a name.' }] });
        if (bytes.length === 0) return err('pack_corrupt', 'That data pack is empty or damaged.');
        let rows = 0;
        let sum = 0;
        for (const byte of bytes) {
          if (byte === 10) rows += 1;
          sum = (sum * 31 + byte) >>> 0;
        }
        const info: PackInfo = { name, version: `mock-${sum.toString(16).padStart(8, '0')}`, rows };
        packs.set(name, info);
        return ok(info);
      }),
    county_search: (query) => guard(() => ok(search(query))),
    resolve_location: (input) =>
      guard(() => {
        const r = resolve(input);
        return r.ok ? ok(r.value.location) : r;
      }),
    assess: (input) =>
      guard<PlanOutput>(() => {
        const r = run(input);
        return r.ok ? ok(r.value.output) : r;
      }),
    explain: (req: ExplainRequest) =>
      guard<Explanation>(() => {
        if (!(EXPLAIN_KINDS as readonly string[]).includes(req.kind)) {
          return err('bad_input', MESSAGES.bad_input, { problems: [{ code: 'schema', field: 'kind', message: 'Choose hazard, bucket, item, requirement or warning.' }] });
        }
        const r = run(req.input);
        if (!r.ok) return r;
        const explanation = explainFrom(req, r.value);
        if (!explanation) {
          return err('bad_input', MESSAGES.bad_input, { problems: [{ code: 'schema', field: 'id', message: 'There is nothing with that id in this plan.' }] });
        }
        return ok(explanation);
      }),
    catalogue: () =>
      guard(() => ok<Catalogue>({ items: ITEMS, citations: CITATIONS, guidance: GUIDANCE, hazards: HAZARDS, buckets: BUCKETS, tiers: TIERS })),
    defaults: () => guard(() => ok(mockDefaults())),
  };
}
