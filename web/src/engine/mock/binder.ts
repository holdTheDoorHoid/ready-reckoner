/**
 * The mock engine's binder (contract v3, DESIGN-DELTA-v3 §4): the same ten parts, the same pages
 * and the same kinds of block and inline as rr-plan's binder (`crates/rr-plan/src/binder/`, built
 * by the binder workstream), from the mock's own numbers and the household's own answers. The
 * parity test compares the two engines' answers by shape, fixture by fixture, so every block kind
 * appears where the engine puts it, and each run of text holds the same kinds of inline the
 * engine's does in that place:
 *
 * | Where | Inlines |
 * | --- | --- |
 * | paragraphs | text, bold, citations, links |
 * | bullets, steps, table cells | text, bold, blanks, citations, links |
 * | numbered lists | text, citations |
 * | a decision's "if" | text, bold, blanks, citations |
 * | a decision's "then" | text, blanks (where an answer is missing), citations |
 * | wallet card lines | text, bold, blanks |
 * | boxed notes | paragraphs of text (with citations when leaving comes first) |
 *
 * The checklist pages are the content's own (`checklists.ts`, from `content/checklists/`), chosen
 * as the engine chooses them (§5.1) from the mock's ranked risks, with one short sample text
 * each: the mock does not carry the checklists' words, only their shape. The household's answers
 * are echoed exactly as written, as the engine echoes them.
 */
import type {
  Binder,
  Block,
  BucketAssessment,
  Contact,
  FamilyPlan,
  FieldRow,
  HazardProfile,
  Inline,
  Page,
  PageKind,
  Part,
  Person,
  PlanInput,
  SourceEntry,
  Step,
} from '../types';
import { addMonths, formatDate } from '../../lib/format';
import { ATTRIBUTIONS, citation } from './citations';
import { MOCK_CHECKLISTS, type MockChecklist } from './checklists';
import type { ModelResult } from './model';
import { householdPhrase } from './packet';

/** The engine's status line (rr-plan `packet::STATUS_LINE`). */
export const STATUS_LINE =
  'Ready Reckoner is an independent, open-source planning aid. It is not official emergency guidance, ' +
  'and not medical, legal or financial advice. Follow instructions from your local officials first.';

/** The ten parts of DESIGN-DELTA-v3 §4.2: tab, id, title and tab label (rr-plan `binder::PARTS`). */
export const PARTS: readonly (readonly [number, string, string, string])[] = [
  [1, 'start', 'Start here', 'Start here'],
  [2, 'people', 'People', 'People'],
  [3, 'home_places', 'Home and places', 'Home & places'],
  [4, 'pets_vehicles_documents', 'Pets, vehicles and documents', 'Pets & docs'],
  [5, 'have', 'What you have', 'What you have'],
  [6, 'check_now', 'Checklists: happening now', 'Happening now'],
  [7, 'check_coming', 'Checklists: it is coming', 'It is coming'],
  [8, 'check_ongoing', 'Checklists: it goes on', 'It goes on'],
  [9, 'after', 'After', 'After'],
  [10, 'sources', 'Sources', 'Sources'],
];

/** The everyday emergencies (rr_content `ids::EVENTS`) and their names. */
const EVENTS: readonly (readonly [string, string])[] = [
  ['gas_leak_or_co', 'Gas leak or carbon monoxide alarm'],
  ['missing_person', 'Missing person'],
  ['evacuation_order', 'Evacuation order'],
  ['shelter_in_place', 'Shelter-in-place order'],
  ['boil_water_notice', 'Boil-water notice'],
  ['power_outage', 'Power outage at home'],
  ['something_else', 'Something else'],
];

const LOGS: readonly (readonly [string, string, readonly string[]])[] = [
  ['log_damage', 'Damage log', ['What', 'Where', 'Photo taken', 'Reported to']],
  ['log_expenses', 'Expenses log', ['Date', 'What', 'Amount', 'Receipt']],
  ['log_contacts', 'People contacted', ['Date', 'Who', 'Number', 'What they said']],
  ['log_medications', 'Medications given', ['Date', 'Who', 'What', 'Time']],
];
const LOG_ROWS = 24;

// ---------------------------------------------------------------------------------------------
// Small builders
// ---------------------------------------------------------------------------------------------

const t = (s: string): Inline => ({ t: s });
const b = (s: string): Inline => ({ b: s });
const blank = (n: number): Inline => ({ blank: n });
const h = (level: number, text: string): Block => ({ heading: { level, text } });
const para = (...inl: Inline[]): Block => ({ para: inl });
const row = (label: string, value: string | undefined, lines = 1): FieldRow => (value !== undefined && value.trim() !== '' ? { label, value, lines } : { label, lines });
const page = (id: string, title: string, kind: PageKind, fit: Page['fit'], blocks: Block[]): Page => ({ id, title, kind, fit, blocks });

/** The words of a text inline, or undefined for a blank. */
const textOf = (i: Inline): string | undefined => ('t' in i ? i.t : undefined);

/** A contact as written: "name, phone, address"; undefined when all are empty. */
function contactText(c: Contact | undefined): string | undefined {
  const parts = [c?.name, c?.phone, c?.address].filter((x): x is string => !!x && x.trim() !== '');
  return parts.length ? parts.join(', ') : undefined;
}

/** A contact's name and phone as written. */
function contactShort(c: Contact | undefined): string | undefined {
  const parts = [c?.name, c?.phone].filter((x): x is string => !!x && x.trim() !== '');
  return parts.length ? parts.join(', ') : undefined;
}

function joinAnd(items: readonly string[]): string {
  if (items.length <= 1) return items[0] ?? '';
  return `${items.slice(0, -1).join(', ')} and ${items[items.length - 1]}`;
}

const AGE_WORD: Record<string, string> = { infant: 'baby', toddler: 'toddler', child: 'child', teen: 'teenager', adult: 'adult', senior: 'older adult' };

function personTitle(i: number, p: Person): string {
  return p.profile?.name ?? `Person ${i + 1} (${AGE_WORD[p.age_band] ?? 'person'})`;
}

const upperFirst = (s: string) => (s ? s[0]!.toUpperCase() + s.slice(1) : s);

/**
 * Citations by temporary key while the binder is built, renumbered by first use at the end
 * (as rr-plan's `Draft` numbers them), so `cite` n is `sources[n - 1]`.
 */
class Cites {
  readonly ids: string[] = [];

  cite(ids: readonly string[]): Inline[] {
    const keys: number[] = [];
    for (const id of ids) {
      if (!citation(id)) continue;
      let k = this.ids.indexOf(id);
      if (k < 0) k = this.ids.push(id) - 1;
      if (!keys.includes(k + 1)) keys.push(k + 1);
    }
    return keys.length ? [{ cite: keys }] : [];
  }
}

// ---------------------------------------------------------------------------------------------
// The binder
// ---------------------------------------------------------------------------------------------

/** The mock binder for a household and its mock plan. */
export function buildBinder(input: PlanInput, r: ModelResult): Binder {
  const c = new Cites();
  const fp: FamilyPlan = input.family_plan ?? {};
  const o = r.output;
  const loc = o.location;
  // The mock's county names carry their own "County" ("Philadelphia County"), as its packet prints them.
  const place = `${loc.county_name}, ${loc.state_name}`;
  const locationLine = `${place}${loc.zip ? ` (ZIP code ${loc.zip})` : ''}`;
  const reviewBy = addMonths(input.planning_date, 12);
  const years = Math.max(1, input.dials.horizon_years);

  // The checklist pages (§5.1): every ranked hazard's, every everyday emergency's, every opted-in
  // rare family's, by its block; a block that covers several prints once.
  const ranked = o.register.filter((x) => x.display === 'ranked').sort((x, y) => y.rate_per_year - x.rate_per_year);
  const optedFamilies = new Set<string>(input.dials.rare_opt_in ?? []);
  const rare = o.register.filter((x) => x.display === 'rare_catastrophic');
  const chosen: { list: MockChecklist; hazards: HazardProfile[]; events: string[] }[] = [];
  const blockFor = (target: string) => MOCK_CHECKLISTS.find((x) => x.applies_to.includes(target));
  const take = (list: MockChecklist | undefined, hz?: HazardProfile, ev?: string) => {
    if (!list) return;
    let entry = chosen.find((x) => x.list.id === list.id);
    if (!entry) chosen.push((entry = { list, hazards: [], events: [] }));
    if (hz && !entry.hazards.some((x) => x.id === hz.id)) entry.hazards.push(hz);
    if (ev) entry.events.push(ev);
  };
  for (const hz of ranked) take(blockFor(`hazard:${hz.id}`), hz);
  for (const hz of rare) if (hz.family && optedFamilies.has(hz.family)) take(blockFor(`hazard:${hz.id}`), hz);
  for (const [ev] of EVENTS) take(blockFor(`event:${ev}`), undefined, ev);
  const chance = (x: { hazards: HazardProfile[] }) => Math.max(0, ...x.hazards.map((hz) => 1 - Math.exp(-hz.rate_per_year * 10)));
  const group = (x: { events: string[] }) => (x.events.includes('something_else') ? 2 : x.events.length ? 0 : 1);
  const contentOrder = (x: { list: MockChecklist }) => MOCK_CHECKLISTS.indexOf(x.list);
  const onsetRank = { now: 0, coming: 1, ongoing: 2 } as const;
  chosen.sort((x, y) => onsetRank[x.list.onset] - onsetRank[y.list.onset] || group(x) - group(y) || chance(y) - chance(x) || contentOrder(x) - contentOrder(y));
  const tabOf = (list: MockChecklist) => (list.onset === 'now' ? 6 : list.onset === 'coming' ? 7 : 8);

  // The page directory, for links: id → [tab, title].
  const people = input.people.map((p, i) => [`person_${i + 1}`, personTitle(i, p)] as const);
  const places = distinctPlaces(input);
  const hasAnimals = input.pets.dogs + input.pets.cats + input.pets.small + input.pets.large_animals > 0 || (fp.pets?.length ?? 0) > 0;
  const hasVehicles = input.mobility.vehicles.length > 0 || (fp.vehicles?.length ?? 0) > 0;
  const longHorizon = (o.plan.long_horizon?.length ?? 0) > 0;
  const directory = new Map<string, [number, string]>([
    ['cover', [1, 'Cover']],
    ['how_to_use', [1, 'How to use this binder']],
    ['quick_start', [1, 'Quick start: the first five minutes of any emergency']],
    ['index', [1, 'Which checklist?']],
    ['contacts', [1, 'Contacts at a glance']],
    ['people', [2, 'Who is in this binder']],
    ['special_needs', [2, 'Special needs and health']],
    ['wallet_cards', [2, 'Wallet cards']],
    ['home', [3, 'Home']],
    ['neighbourhood', [3, 'Neighborhood']],
    ['getting_out', [3, 'Getting out']],
    ['documents', [4, 'Documents and money']],
    ['inventory', [5, 'What you have']],
    ['what_to_expect', [5, 'What to expect']],
    ['risks_glance', [5, 'Risks at a glance']],
    ['forecast', [7, 'When a storm, freeze or heat wave is forecast']],
    ['after', [9, 'After a disaster: the first 30 days']],
    ['sources', [10, 'Sources']],
  ]);
  for (const [id, title] of people) directory.set(id, [2, title]);
  for (const pl of places) directory.set(pl.id, [3, pl.title]);
  if (hasAnimals) directory.set('pets', [4, 'Pets']);
  if (hasVehicles) directory.set('vehicles', [4, 'Vehicles']);
  for (const x of chosen) directory.set(x.list.id, [tabOf(x.list), x.list.title]);
  if (longHorizon) directory.set('after_months', [9, 'If it lasts for months']);
  for (const [id, title] of LOGS) directory.set(id, [9, title]);
  const link = (id: string): Inline[] => {
    const d = directory.get(id);
    return d ? [{ link: { to: id, text: `Tab ${d[0]}, ${d[1]}` } }] : [];
  };

  // The household's answer for a checklist placeholder (rr-plan `checklists::answer`).
  const hood = fp.neighbourhood ?? {};
  const home = fp.home ?? {};
  const answer = (name: string): string | undefined =>
    ({
      meeting_near: fp.meeting_place_near,
      meeting_far: fp.meeting_place_far,
      shelter_home: fp.shelter_spot_home,
      where_go: fp.where_we_would_go,
      out_of_area_contact: contactShort(fp.out_of_area_contact),
      gas_shutoff: fp.shutoff_gas,
      electric_utility: contactShort(home.electric_utility),
      hospital: contactShort(hood.hospital),
    })[name];
  /** A placeholder as the household's words, or a line to write on. */
  const fill = (name: string, width = 20): Inline => {
    const v = answer(name);
    return v ? t(v) : blank(width);
  };

  const pagesByTab: Page[][] = PARTS.map(() => []);
  const add = (tab: number, p: Page) => pagesByTab[tab - 1]!.push(p);

  // ---- Tab 1, Start here ----------------------------------------------------------------------
  const names = input.people.map((p) => p.profile?.name).filter((n): n is string => !!n);
  const householdLine = names.length === input.people.length && names.length > 0 ? joinAnd(names) : householdPhrase(input);
  add(1, page('cover', 'Cover', 'cover', 'one', [
    para(b('For:'), t(` ${householdPhrase(input)}`)),
    para(b('Where:'), t(` ${locationLine}`)),
    { fields: [row('Home address', home.address)] },
    para(b('Made on:'), t(` ${formatDate(input.planning_date)}`)),
    para(b('Review by:'), t(` ${formatDate(reviewBy)}, and whenever something changes.`), ...c.cite(['mock_ready_gov_kit'])),
    { callout: { kind: 'note', blocks: [para(t(STATUS_LINE))] } },
  ]));
  add(1, page('how_to_use', 'How to use this binder', 'how_to_use', 'one', [
    para(t('This binder is for the day something goes wrong. Keep it where everyone can find it, and take it if you leave.'), ...c.cite(['mock_make_a_plan'])),
    para(b('In an emergency, '), t('start with the Quick start page, then find what is happening on "Which checklist?" and turn to its page. '), ...link('index')),
    para(t('Blank lines are for you to fill in by hand. Review it by the date on the cover, and whenever something changes.'), ...c.cite(['mock_ready_gov_kit'])),
  ]));
  add(1, page('quick_start', 'Quick start: the first five minutes of any emergency', 'quick_start', 'one', [
    para(t('Do these first, from memory, whatever is happening.'), ...c.cite(['mock_make_a_plan'])),
    {
      steps: [
        { text: [b('Get safe.'), t(' Move away from fire, smoke, water or a gas smell.'), ...c.cite(['mock_make_a_plan'])], memory: true },
        { text: [b('Call 911'), t(' if anyone is hurt or in danger.'), ...c.cite(['mock_emergency_numbers'])], memory: true },
        { text: [b('Check on everyone'), t(' at home, and on neighbors who may need help.'), ...c.cite(['mock_social_capital'])], memory: true },
        { text: [b('Get information'), t(' from local officials: radio, TV or alerts.'), ...c.cite(['mock_forecasts'])], memory: true },
        { text: [b('Decide: leave or stay.'), t(' Use the checklist for what is happening.'), ...c.cite(['mock_evacuation'])], memory: true },
      ],
    },
    para(t('Then find the page for what is happening: '), ...link('index'), t('.')),
  ]));
  const turnTo = (id: string | undefined): Inline[] => {
    if (id) {
      const l = link(id);
      if (l.length) return l;
    }
    const fallback = chosen.find((x) => x.events.includes('something_else'))?.list.id ?? 'quick_start';
    return [t('No page of its own yet: use '), ...link(fallback)];
  };
  const indexRows: Inline[][][] = [];
  for (const [ev, name] of EVENTS.filter(([e]) => e !== 'something_else')) indexRows.push([[t(name)], turnTo(chosen.find((x) => x.events.includes(ev))?.list.id)]);
  for (const hz of ranked) indexRows.push([[t(hz.name)], turnTo(chosen.find((x) => x.hazards.some((y) => y.id === hz.id))?.list.id)]);
  indexRows.push([[t('Anything else')], turnTo(chosen.find((x) => x.events.includes('something_else'))?.list.id)]);
  const notHere = rare.filter((hz) => !(hz.family && optedFamilies.has(hz.family))).map((hz) => hz.name.toLowerCase());
  add(1, page('index', 'Which checklist?', 'index', 'two', [
    para(t('Find what is happening, then turn to its page. Everyday emergencies come first, then the risks where you live, most likely first.')),
    { table: { header: ['If this happens', 'Turn to'], rows: indexRows } },
    para(t(notHere.length ? `Not in this binder: ${joinAnd(notHere)}. To add one, tick the family under Your settings.` : 'Every rare family you opted into has its page here.')),
    para(t('Also checked, and too unlikely here to need a page: the hazards listed under "Also checked" on the Risks screen.')),
  ]));
  add(1, contacts(input, fp, c, people));

  // ---- Tab 2, People --------------------------------------------------------------------------
  add(2, page('people', 'Who is in this binder', 'index', 'one', [
    { table: { header: ['Who', 'Age group', 'Their page'], rows: input.people.map((p, i) => [[t(people[i]![1])], [t(upperFirst(AGE_WORD[p.age_band] ?? 'person'))], link(people[i]![0])]) } },
    para(t('Cut out a wallet card for each person: '), ...link('wallet_cards'), t('. Medicine, devices and other health needs: '), ...link('special_needs'), t('.')),
  ]));
  input.people.forEach((p, i) => add(2, personPage(p, i, people[i]!, places, link)));
  add(2, page('special_needs', 'Special needs and health', 'person', 'one', [
    h(2, 'Medicine'),
    { bullets: [[t(`Keep at least ${r.facts.dailyRx > 0 ? 'two weeks' : 'a week'} of each daily medicine on hand.`), ...c.cite(['mock_medication_reserve'])]] },
    h(2, 'Stress and mental health'),
    para(t('Feeling stressed after a disaster is normal. Call or text 988 any time to talk with someone.'), ...c.cite(['mock_emergency_numbers'])),
  ]));
  const circle = (fp.trusted_circle ?? []).map((x) => [x.name, x.phone].filter(Boolean).join(' ')).filter((s) => s !== '');
  const cardLine = (label: string, value: string | undefined): Inline[] => (value ? [b(`${label}:`), t(` ${value}`)] : [b(`${label}:`), t(' '), blank(20)]);
  add(2, page('wallet_cards', 'Wallet cards', 'wallet_cards', 'one', [
    para(t("Cut out a card for each person's wallet or phone case. Fill in any blank by hand.")),
    {
      cards: input.people.map((p, i) => {
        const health = [p.profile?.blood_type ? `blood type ${p.profile.blood_type}` : undefined, p.profile?.allergies ? `allergies: ${p.profile.allergies}` : undefined].filter((x): x is string => !!x);
        return {
          title: `Wallet card: ${personTitle(i, p)}`,
          lines: [
            cardLine('Out-of-area contact', contactShort(fp.out_of_area_contact)),
            cardLine('Meet near home', fp.meeting_place_near),
            cardLine('Meet outside the neighborhood', fp.meeting_place_far),
            cardLine('Lawyer', contactShort(fp.lawyer)),
            cardLine('Trusted circle', circle.length ? circle.join('; ') : undefined),
            cardLine('Know by heart', fp.numbers_by_heart?.length ? fp.numbers_by_heart.join('; ') : undefined),
            cardLine('Health', health.length ? health.join('; ') : undefined),
            [t('Medicines and doctors: see binder tab 2.')],
          ],
        };
      }),
    },
  ]));

  // ---- Tab 3, Home and places -----------------------------------------------------------------
  add(3, page('home', 'Home', 'home', 'one', [
    {
      fields: [
        row('Address', home.address),
        row('Water shut-off', fp.shutoff_water),
        row('Electrical panel', fp.shutoff_electric),
        row('Electric company (outage number)', contactShort(home.electric_utility)),
        row('Home insurer', contactShort(home.insurer)),
        row(input.housing.tenure === 'rent' ? 'Landlord' : 'Mortgage company', contactText(home.landlord_or_mortgage)),
      ],
    },
    h(2, 'Where things are'),
    { fields: [row('Emergency kit', home.where_kit), row('Documents', home.where_documents), row('Cash', home.where_cash), row('Spare keys', home.where_keys)] },
    h(2, 'Safest spots'),
    { fields: [row('Safest spot at home', fp.shelter_spot_home), row('Neighbors who check on us', fp.neighbours_who_check, 2)] },
    para(t('For a tornado or a chemical release, go to the safest spot and stay until officials say it is over.'), ...c.cite(['mock_ready_gov_kit'])),
    ...checklistLinks(['check_house_fire', 'check_gas_leak_or_co', 'check_power_outage', 'check_shelter_in_place', 'check_tornado'], 'Checklists that turn here:', link),
  ]));
  places.forEach((pl, n) =>
    add(3, page(pl.id, pl.title, 'place', 'one', [
      {
        fields: [
          row('Who goes there', joinAnd(pl.people.map((i) => people[i]![1]))),
          row('What it is', upperFirst(pl.kind === 'childcare' ? 'child care' : pl.kind)),
          row('Address', pl.place.address),
          row('Phone', pl.place.phone),
          row('Its emergency plan', pl.place.plan, 3),
          row('Pick-up', pl.place.pickup, 2),
          row('Safest spot there', pl.place.safest_spot),
        ],
      },
      ...(n === 0 ? [h(2, 'Work and school'), { fields: [row('What each of us does at work or school', fp.work_plans, 2), row('Shelter spot at work or school', fp.shelter_spot_work)] }] : []),
      para(t('If you are apart when it happens, meet as your plan says: '), ...link('neighbourhood'), t('. Who to call: '), ...link('contacts'), t('.')),
    ])),
  );
  add(3, page('neighbourhood', 'Neighborhood', 'neighbourhood', 'two', [
    {
      fields: [
        row('Meeting place near home', fp.meeting_place_near),
        row('Meeting place outside the neighborhood', fp.meeting_place_far),
        row('Hospital with an emergency room', contactText(hood.hospital)),
        row('Pharmacy', contactText(hood.pharmacy)),
        row('Community shelter', contactText(hood.shelter)),
        row('How we get local alerts', hood.alerts),
        row('Our evacuation zone', undefined),
      ],
    },
    ...(places.length === 0 ? [h(2, 'Work and school'), { fields: [row('What each of us does at work or school', fp.work_plans, 2)] }] : []),
    h(2, `Where to start in ${loc.state_name}`),
    { table: { header: ['For', 'Where to start'], rows: [[[t('Alerts')], [t("Sign up for the county's alerts."), ...c.cite(['mock_forecasts'])]], [[t('Registry')], [t('Ask the county emergency office about its registry for people who may need help.')]]] } },
    para(t('From the federal list of hospitals (CMS). Check before you need it: a hospital can close or stop taking patients.'), ...c.cite(['mock_hospital_list'])),
    { table: { header: ['Hospital', 'City', 'Phone'], rows: [[[t('Sample County General Hospital')], [t(loc.county_name)], [t('555-0199')]]] } },
    { map_slot: { id: 'map-neighbourhood', kind: 'neighbourhood', caption: 'Map: your neighborhood' } },
  ]));
  const evac = o.buckets.find((x) => x.id === 'evacuate');
  const leaveFirst = leavesFirst(evac, o.register, loc.state_abbr);
  add(3, page('getting_out', 'Getting out', 'getting_out', 'two', [
    { fields: [row('Where we would go', fp.where_we_would_go, 2), row('First way out', fp.routes?.[0]), row('Second way out', fp.routes?.[1]), ...(hasVehicles || fp.roadside_assistance ? [row('Roadside assistance', fp.roadside_assistance)] : [])] },
    ...(leaveFirst
      ? [{ callout: { kind: 'warning' as const, title: 'Leaving comes first here', blocks: [para(t('Know your evacuation zone and where you would go; leave when told.'), ...c.cite(['mock_evacuation']))] } }]
      : []),
    para(t(evac?.frequency_sentences.slice(leaveFirst ? 1 : 0).join(' ') || 'Plan to be away from home for a few days if you have to leave.'), ...c.cite(evac?.sources ?? ['mock_evacuation'])),
    ...checklistLinks(['check_evacuation_order', 'check_wildfire', 'check_hurricane', 'check_flooding'], 'When you are told to leave:', link),
    { map_slot: { id: 'map-area', kind: 'area', caption: 'Map: your city or county' } },
    { map_slot: { id: 'map-region', kind: 'region', caption: 'Map: your region and the ways out' } },
  ]));

  // ---- Tab 4, Pets, vehicles and documents ----------------------------------------------------
  if (hasAnimals) {
    const pets = fp.pets ?? [];
    add(4, page('pets', 'Pets', 'pets', 'one', [
      ...(pets.length
        ? pets.flatMap((p, i) => [h(2, p.name ?? `Animal ${i + 1}`), { fields: [row('Name', p.name), row('Kind', p.kind), row('What it looks like', p.description), row('Vet', contactText(p.vet)), row('Microchip or tag number', p.microchip)] } as Block])
        : [h(2, 'Animal 1'), { fields: [row('Name', undefined), row('Kind', undefined), row('Vet', undefined)] } as Block]),
      h(2, 'If we cannot care for them'),
      { fields: [row('Who takes the animals', fp.who_takes_animals, 2)] },
      h(2, 'What to have for them'),
      { bullets: [[b('Food:'), t(' two weeks for each animal.'), ...c.cite(['mock_ready_gov_kit'])]] },
    ]));
  }
  if (hasVehicles) {
    const vs = fp.vehicles ?? [];
    add(4, page('vehicles', 'Vehicles', 'vehicles', 'one', [
      ...(vs.length ? vs.flatMap((v, i) => [h(2, v.description ?? `Vehicle ${i + 1}`), { fields: [row('License plate', v.plate), row('Insurer', contactText(v.insurer)), row('What stays in the car', v.kept_in_car, 2)] } as Block]) : [h(2, 'Vehicle 1'), { fields: [row('License plate', undefined), row('What stays in the car', undefined, 2)] } as Block]),
      { fields: [row('Roadside assistance', fp.roadside_assistance)] },
      para(t('If you are stuck on the road: '), ...link(chosen.some((x) => x.list.id === 'check_vehicle_stranding') ? 'check_vehicle_stranding' : 'quick_start'), t('.')),
    ]));
  }
  const docs = fp.documents ?? {};
  const cell = (v: string | undefined, n = 16): Inline[] => (v ? [t(v)] : [blank(n)]);
  const accounts = (docs.accounts ?? []).map((a) => [cell(a.institution), cell(a.kind), cell(a.phone), cell(a.last4, 4)]);
  while (accounts.length < 2) accounts.push([[blank(16)], [blank(16)], [blank(16)], [blank(4)]]);
  add(4, page('documents', 'Documents and money', 'documents', 'one', [
    para(t('The Emergency Financial First Aid Kit has four parts. Tick each when it is in the box.'), ...c.cite(['mock_insurance'])),
    { table: { header: ['Part', 'What goes in it', 'In the box?'], rows: [[[b('Identification')], [t('IDs, passports, birth certificates')], [blank(6)]], [[b('Money')], [t('Account details, cash, bills')], [blank(6)]]] } },
    { fields: [row('Where the originals are', docs.where_originals), row('Where the copies are', docs.where_copies), row('Digital backup', docs.digital_backup)] },
    h(2, 'Accounts'),
    { table: { header: ['Where', 'Kind', 'Phone', 'Last four digits'], rows: accounts } },
    h(2, 'Cash'),
    { fields: [row('Cash on hand, and where', undefined)] },
  ]));

  // ---- Tab 5, What you have -------------------------------------------------------------------
  const items = o.plan.months.flatMap((m) => m.items.map((it) => ({ it, month: m.index })));
  const byBucket = new Map<string, typeof items>();
  for (const x of items) {
    const key = x.it.buckets[0] ?? 'supplies';
    byBucket.set(key, [...(byBucket.get(key) ?? []), x]);
  }
  const inventory: Block[] = [para(t('Tick each thing when you have it, and write where it is kept. "Have" means you listed it, or the plan counts it as an everyday basic most homes have.'))];
  for (const [bucket, list] of byBucket) {
    inventory.push(h(2, o.buckets.find((x) => x.id === bucket)?.name ?? bucket));
    inventory.push({
      table: {
        header: ['What', 'How much', 'Have it?', 'Where kept', 'Next check'],
        rows: list.slice(0, 8).map(({ it, month }) => [[t(it.name)], [t(`${it.quantity} ${it.unit}`)], it.done ? [t('have')] : [t(`still to get (month ${month})`)], [blank(12)], [t('—')]]),
      },
    });
  }
  inventory.push(para(t('The medicine list for each person is on their page: '), ...link('people'), t('.')));
  add(5, page('inventory', 'What you have', 'inventory', 'two', inventory));
  add(5, page('what_to_expect', 'What to expect', 'risks_glance', 'one', [
    para(t('How long to be ready for each need at your setting, and when help usually arrives.')),
    { table: { header: ['Need', 'Be ready for', 'Help likely in'], rows: o.buckets.filter((x) => x.target.kind === 'days').slice(0, 6).map((x) => [[t(x.name)], [t(x.target.kind === 'days' ? `${x.target.value} days` : '')], [t('a few days'), ...c.cite(x.sources)]]) } },
    para(t('Targets are for one need at a time; more than one can run out at once.')),
    para(t('These numbers come from the plan.'), ...c.cite(['mock_duration_prior'])),
  ]));
  const likely = (hz: HazardProfile) => {
    const n = Math.round(100 * (1 - Math.exp(-hz.rate_per_year * years)));
    return n < 1 ? 'fewer than 1 in 100' : `about ${n} in 100`;
  };
  const severityWord = (s: number) => (['', 'Minor', 'Moderate', 'Serious', 'Severe', 'Catastrophic'][Math.min(5, Math.max(1, Math.round(s)))] ?? 'Serious');
  const pageFor = (hz: HazardProfile): Inline[] => {
    const x = chosen.find((y) => y.hazards.some((z) => z.id === hz.id));
    return x ? link(x.list.id) : [blank(12)];
  };
  add(5, page('risks_glance', 'Risks at a glance', 'risks_glance', 'two', [
    para(t(`How likely each risk is for households like yours here over ${years} years, how bad it gets, and its checklist.`)),
    { table: { header: ['Risk', `How likely, ${years} years`, 'How bad', 'Checklist'], rows: ranked.map((hz) => [[t(hz.name)], [t(likely(hz)), ...c.cite(hz.sources)], [b(severityWord(hz.severity))], pageFor(hz)]) } },
    h(2, 'Notes on these numbers'),
    { bullets: [[t('The chances are for households like yours in this county, not for your home alone.'), ...c.cite(['mock_nri_county'])]] },
  ]));

  // ---- Tabs 6 to 8, the checklists ------------------------------------------------------------
  const forecast = page('forecast', 'When a storm, freeze or heat wave is forecast', 'checklist', 'one', [
    h(1, 'Use this when'),
    para(t('A watch or warning is issued for a storm, a hard freeze or a heat wave.'), ...c.cite(['mock_forecasts'])),
    h(1, 'In the next two days'),
    { steps: [{ text: [b('Charge'), t(' phones and batteries.'), ...c.cite(['mock_forecasts'])], memory: false }, { text: [b('Fill'), t(' the car and refill medicines.'), ...c.cite(['mock_medication_reserve'])], memory: false }] },
  ]);
  for (const x of chosen) add(tabOf(x.list), checklistPage(x, c, fill, link, likely, severityWord, years));
  pagesByTab[6]!.unshift(forecast);

  // ---- Tab 9, After ---------------------------------------------------------------------------
  add(9, page('after', 'After a disaster: the first 30 days', 'after', 'one', [
    para(b('First, stay safe.'), t(' Go home only when officials say it is safe.'), ...c.cite(['mock_evacuation'])),
    para(t('Photograph the damage before you clean up, and call your insurer.'), ...c.cite(['mock_insurance'])),
    para(t('Keep every receipt; write down who you spoke to and when: '), ...link('log_contacts'), t('.')),
  ]));
  if (longHorizon) add(9, page('after_months', 'If it lasts for months', 'after', 'one', [para(t('Plan for a long outage: what to keep going, and what to drop.'), ...c.cite(['mock_oregon_resilience']))]));
  for (const [id, title, columns] of LOGS) {
    add(9, page(id, title, 'log', 'one', [para(t('Write it down as it happens: insurers and FEMA ask for dates, names and receipts.')), { log: { columns: [...columns], rows: LOG_ROWS } }]));
  }

  // ---- Tab 10, Sources: numbered by first use -------------------------------------------------
  const sourcesPage = page('sources', 'Sources', 'sources', 'flow', []);
  add(10, sourcesPage);
  const parts: Part[] = PARTS.map(([tab, id, title, short]) => ({ id, tab, title, short_title: short, pages: pagesByTab[tab - 1]! }));
  for (const p of parts) if (p.pages.length === 0) p.pages.push(page(`${p.id}_pending`, 'No checklist here yet', 'checklist', 'one', [para(t('No checklist in this binder goes behind this tab yet. Until one does, use '), ...link('quick_start'), t('.'))]));
  const binder: Binder = {
    title: `Emergency binder for ${householdLine}`,
    generated_on: input.planning_date,
    household: householdPhrase(input),
    location: locationLine,
    status_line: STATUS_LINE,
    review_by: reviewBy,
    parts,
    sources: [],
    credits: ATTRIBUTIONS.map((at) => `${at.source}${at.version ? `, version ${at.version}` : ''}, accessed ${formatDate(at.accessed)}. ${at.text}${at.text.includes(at.url) ? '' : ` ${at.url}`}`),
  };
  // Renumber the citations by first use, then add the Sources page that lists them.
  const order: string[] = [];
  walkCites(binder, (keys) => {
    for (const k of keys) {
      const id = c.ids[k - 1]!;
      if (!order.includes(id)) order.push(id);
    }
  });
  walkCites(binder, (keys) => {
    const n = [...new Set(keys.map((k) => order.indexOf(c.ids[k - 1]!) + 1))].sort((x, y) => x - y);
    keys.splice(0, keys.length, ...n);
  });
  binder.sources = order.map((id, i): SourceEntry => {
    const cit = citation(id)!;
    const entry: SourceEntry = { n: i + 1, title: cit.title, publisher: cit.publisher, expert: cit.prior ?? false };
    if (cit.year !== undefined) entry.year = cit.year;
    if (cit.url.trim() !== '') entry.url = cit.url;
    return entry;
  });
  sourcesPage.blocks.push(
    para(t('The numbers in brackets point to this list; "expert estimate" marks a judgement, not measured data.')),
    { numbered: binder.sources.map((s) => [t(`${s.title}. ${s.publisher}${s.year ? `, ${s.year}` : ''}.${s.url ? ` ${s.url}` : ''}`)]) },
    h(1, 'Data credits'),
    { bullets: binder.credits.map((cr) => [t(cr)]) },
    h(1, 'How this binder was made'),
    { bullets: [[t(`Made by the Ready Reckoner mock engine on ${formatDate(input.planning_date)}, from the plan's answers.`)]] },
  );
  return binder;
}

// ---------------------------------------------------------------------------------------------
// Pages with more to them
// ---------------------------------------------------------------------------------------------

/** "Contacts at a glance" (rr-plan `start::contacts`): every number on one page. */
function contacts(input: PlanInput, fp: FamilyPlan, c: Cites, people: readonly (readonly [string, string])[]): Page {
  const home = fp.home ?? {};
  const hood = fp.neighbourhood ?? {};
  const call = (number: string, id: string): Inline[] => [b(number), ...c.cite([id])];
  const blocks: Block[] = [
    h(2, 'Emergency numbers'),
    {
      table: {
        header: ['For', 'Call'],
        rows: [
          [[t('Police, fire or an ambulance (text 911 where you cannot call)')], call('911', 'mock_emergency_numbers')],
          [[t('Suicide and Crisis Lifeline: call, text or chat')], call('988', 'mock_emergency_numbers')],
          [[t('Poison Control')], call('1-800-222-1222', 'mock_emergency_numbers')],
        ],
      },
    },
    h(2, 'Our people'),
    { fields: input.people.map((p, i) => row(people[i]![1], p.profile?.phone)) },
    h(2, 'Out-of-area contact and lawyer'),
    { fields: [row('Out-of-area contact', contactShort(fp.out_of_area_contact)), row('Lawyer', contactShort(fp.lawyer)), row('Numbers we know by heart', fp.numbers_by_heart?.join('; '))] },
    h(2, 'Trusted circle'),
  ];
  const circle: Inline[][][] = (fp.trusted_circle ?? []).map((x) => [x.name ? [t(x.name)] : [blank(20)], x.phone ? [t(x.phone)] : [blank(12)], [blank(20)]]);
  while (circle.length < 2) circle.push([[blank(20)], [blank(12)], [blank(20)]]);
  blocks.push({ table: { header: ['Name', 'Phone', 'Holds for us'], rows: circle } });
  blocks.push(h(2, 'Home and utilities (outage numbers)'));
  blocks.push({ fields: [row('Electric company', contactShort(home.electric_utility)), row('Gas company', contactShort(home.gas_utility)), row('Water company', contactShort(home.water_utility))] });
  blocks.push(h(2, 'Help nearby'));
  blocks.push({ fields: [row('Hospital', contactShort(hood.hospital)), row('Urgent care', contactShort(hood.urgent_care)), row('County emergency office', contactShort(hood.county_emergency_office))] });
  blocks.push(h(2, 'How to reach each other'));
  blocks.push(para(t('Plan four ways to reach each other, in order.'), ...c.cite(['mock_make_a_plan'])));
  blocks.push({ numbered: [[t('Primary: call.')], [t('Alternate: text.'), ...c.cite(['mock_make_a_plan'])], [t('Contingency: your out-of-area contact.')], [t('Emergency: go to the place you agreed to meet.')]] });
  return page('contacts', 'Contacts at a glance', 'contacts', 'one', blocks);
}

/** One person's page (rr-plan `people::person_page`). */
function personPage(p: Person, i: number, [id, title]: readonly [string, string], places: Place3[], link: (id: string) => Inline[]): Page {
  const pr = p.profile ?? {};
  const ins = pr.insurance ?? {};
  const rows: Inline[][][] = (pr.medications ?? []).map((m) => [m.name, m.dose, m.schedule, m.purpose].map((v) => (v ? [t(v)] : [blank(12)])));
  while (rows.length < 4) rows.push([[blank(12)], [blank(12)], [blank(12)], [blank(12)]]);
  const said: string[] = [];
  if (p.medical.daily_rx) said.push('Takes prescription medicine every day.');
  if (p.medical.refrigerated_rx) said.push('Has a medicine that must stay cold.');
  if (p.medical.epinephrine) said.push('Carries an epinephrine auto-injector.');
  if (p.pregnant_or_nursing) said.push('Pregnant or nursing.');
  if (p.commute) said.push(`Goes to work or school ${p.commute.mode === 'car' ? 'by car' : p.commute.mode === 'transit' ? 'by bus or train' : p.commute.mode === 'walk' ? 'on foot' : 'by bike'}.`);
  const myPlace = places.find((pl) => pl.people.includes(i));
  const blocks: Block[] = [
    {
      fields: [
        row('Name', pr.name),
        row('Date of birth', pr.date_of_birth),
        row('Phone', pr.phone),
        row('Email', pr.email),
        row('Blood type', pr.blood_type),
        row('Allergies', pr.allergies),
        row('Medical conditions', pr.conditions, 2),
        row('Doctor', contactText(pr.doctor)),
        row('Pharmacy', contactText(pr.pharmacy)),
      ],
    },
    h(2, 'Health insurance'),
    { fields: [row('Insurance company', ins.carrier), row('Plan', ins.plan_name), row('Member ID', ins.member_id), row('Group number', ins.group_number), row('Insurance phone', ins.phone)] },
    h(2, 'Medicines'),
    { table: { header: ['Medicine', 'Dose', 'When', 'What for'], rows } },
    h(2, 'Where they spend the day'),
    { fields: [row('Place', pr.place ? `${pr.place.name ?? upperFirst(pr.place.kind)}${pr.place.name ? ` (${pr.place.kind === 'childcare' ? 'child care' : pr.place.kind})` : ''}` : undefined), row('ID documents', pr.id_notes), row('Anything else a helper should know', pr.notes, 2)] },
    ...(myPlace ? [para(t('Its address, plan and pick-up rules: '), ...link(myPlace.id), t('.'))] : []),
    h(2, 'From your answers'),
    said.length ? { bullets: said.map((s) => [t(s)]) } : para(t('No daily medicine, medical device or access need was listed for this person.')),
  ];
  return page(id, title, 'person', 'one', blocks);
}

/** A place in the household's life, merged by name (rr-plan `places::distinct`). */
interface Place3 {
  id: string;
  title: string;
  people: number[];
  kind: string;
  place: { address?: string; phone?: string; plan?: string; pickup?: string; safest_spot?: string };
}

function distinctPlaces(input: PlanInput): Place3[] {
  const out: Place3[] = [];
  input.people.forEach((person, i) => {
    const pl = person.profile?.place;
    if (!pl) return;
    const key = pl.name?.trim().toLowerCase();
    const same = key ? out.find((e) => e.title.trim().toLowerCase() === key) : undefined;
    if (same) {
      same.people.push(i);
      for (const k of ['address', 'phone', 'plan', 'pickup', 'safest_spot'] as const) same.place[k] ??= pl[k];
      return;
    }
    out.push({ id: `place_${out.length + 1}`, title: pl.name ?? `Where ${personTitle(i, person)} spends the day`, people: [i], kind: pl.kind, place: { address: pl.address, phone: pl.phone, plan: pl.plan, pickup: pl.pickup, safest_spot: pl.safest_spot } });
  });
  return out;
}

/** "Checklists that turn here: (Tab 6, House fire) · …", when the binder has any of them. */
function checklistLinks(ids: readonly string[], lead: string, link: (id: string) => Inline[]): Block[] {
  const links = ids.flatMap((id) => link(id));
  if (!links.length) return [];
  return [para(b(lead), ...links.flatMap((l, i) => [t(i ? ' · ' : ' '), l]))];
}

/**
 * A checklist page (§5.3) in the engine's order: the household's line for each hazard it covers,
 * "Use this when", "Do first" (from memory), "Then", "Leave or stay?", "Where and who", "Do not"
 * and "When it is over". The mock's sample text uses the placeholders the content uses, filled
 * with the household's answers or left as lines to write on.
 */
function checklistPage(
  x: { list: MockChecklist; hazards: HazardProfile[]; events: string[] },
  c: Cites,
  fill: (name: string, width?: number) => Inline,
  link: (id: string) => Inline[],
  likely: (hz: HazardProfile) => string,
  severity: (s: number) => string,
  years: number,
): Page {
  const src = x.hazards[0]?.sources.length ? x.hazards[0].sources : ['mock_ready_gov_kit'];
  const cite = () => c.cite(src);
  const several = x.hazards.length > 1;
  const blocks: Block[] = x.hazards.map((hz) => para(b(several ? `${hz.name} here: ` : 'Here: '), t(`${likely(hz)} households like yours in the next ${years} years · How bad: ${severity(hz.severity)}`), ...c.cite(hz.sources)));
  const step = (lead: string, rest: string, memory: boolean, extra: Inline[] = []): Step => ({ text: [b(lead), t(rest), ...extra, ...cite()], memory });
  blocks.push(
    h(1, 'Use this when'),
    para(t(`Something tells you "${x.list.title.toLowerCase()}" is happening, or officials say so.`)),
    h(1, 'Do first'),
    {
      steps: [
        step('Get safe.', ' Move away from the danger.', true),
        step('Tell your contact.', ' Text ', true, [fill('out_of_area_contact'), t(' where you are going.')]),
        step('Meet up.', ' Go to ', true, [fill('meeting_near'), t(' if you are apart.')]),
      ],
    },
    h(1, 'Then'),
    {
      steps: [
        step('Call the utility', ' if lines are down: ', false, [fill('electric_utility')]),
        step('Check the home.', ' Use the home page: ', false, link('home')),
      ],
    },
    {
      decision: {
        question: 'Leave or stay?',
        branches: [
          { when: [b('Leave if'), t(' officials tell you to.'), ...cite()], then: [t('Go to '), fill('where_go'), t('.'), ...cite()], go_to: 'getting_out' },
          { when: [b('Stay if'), t(' the way out is not safe; call '), fill('hospital'), t(' for medical help, and shut the gas at '), fill('gas_shutoff')], then: [t('Shelter where you are.'), ...cite()] },
        ],
      },
    },
    h(1, 'Where and who'),
    { fields: [row('Meeting place near home', textOf(fill('meeting_near'))), row('Second way out', undefined)] },
    h(1, 'Do not'),
    { bullets: [[t('Do not go back inside until it is safe.'), ...cite()]] },
    h(1, 'When it is over'),
    {
      bullets: [
        [b('Check'), t(' on neighbors, then call '), fill('electric_utility'), t(' about power, and meet at '), fill('meeting_far'), t('.'), ...cite()],
        [t('Use the After pages for the first days. '), ...link('after')],
      ],
    },
  );
  return page(x.list.id, x.list.title, 'checklist', x.list.pages === 2 ? 'two' : 'one', blocks);
}

/** States with a storm-surge coast: the mock's stand-in for the engine's surge zones. */
const SURGE_STATES = new Set(['TX', 'LA', 'MS', 'AL', 'FL', 'PR']);
/** The wildland West: the mock's stand-in for the engine's wildfire exposure. */
const WILDLAND_STATES = new Set(['MT', 'ID', 'OR', 'WA', 'WY', 'CO', 'NM', 'UT', 'NV']);

/**
 * Whether leaving comes first here (rr-plan `summary::leave_first`): the chance of having to
 * leave home in ten years is high (the mock's numbers run high, so 3 in 10 here, the engine's
 * 1 in 4), or the home is in a storm-surge area, or wildfire is a ranked risk in the wildland
 * West. The mock has no surge or wildland data, so a state stands in for them; on the fixture
 * households it agrees with the engine.
 */
function leavesFirst(evac: BucketAssessment | undefined, register: readonly HazardProfile[], state: string): boolean {
  const p = evac?.target.kind === 'evacuate' ? evac.target.p_need_10yr : 0;
  const wildfire = register.some((hz) => hz.display === 'ranked' && hz.id === 'wildfire');
  return p >= 0.3 || SURGE_STATES.has(state) || (wildfire && WILDLAND_STATES.has(state));
}

/** Visits every citation's numbers in reading order (and lets them be rewritten in place). */
function walkCites(b: Binder, f: (keys: number[]) => void): void {
  const inl = (v: readonly Inline[]) => v.forEach((i) => 'cite' in i && f(i.cite));
  const block = (bl: Block) => {
    if ('para' in bl) inl(bl.para);
    else if ('bullets' in bl) bl.bullets.forEach(inl);
    else if ('numbered' in bl) bl.numbered.forEach(inl);
    else if ('steps' in bl) bl.steps.forEach((s) => inl(s.text));
    else if ('table' in bl) bl.table.rows.forEach((r) => r.forEach(inl));
    else if ('callout' in bl) bl.callout.blocks.forEach(block);
    else if ('decision' in bl) bl.decision.branches.forEach((br) => (inl(br.when), inl(br.then)));
    else if ('cards' in bl) bl.cards.forEach((card) => card.lines.forEach(inl));
  };
  for (const p of b.parts) for (const pg of p.pages) pg.blocks.forEach(block);
}
