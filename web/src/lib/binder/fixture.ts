/**
 * A hand-written binder for the renderers' tests (DESIGN-DELTA-v3 §4.1): every page kind, every
 * block kind and every inline kind at least once, with a Philadelphia household's sample answers
 * (the fixture household's own, `fixtures/households/philadelphia-renters-4.json`) and some
 * awkward user text: markup, Markdown marks, a table bar, accents and a very long name. It is
 * shaped like the engine's binder (page ids and slots from rr-plan's `binder` module), so the
 * screen and the PDF can be built and measured before the real tree reaches a golden.
 *
 * Test data only: nothing in the app imports it.
 */
import type { Binder, Block, Branch, Card, FieldRow, Inline, Page, PageKind, Part, SourceEntry, Step } from '../../engine/types';

const t = (s: string): Inline => ({ t: s });
const b = (s: string): Inline => ({ b: s });
const cite = (...n: number[]): Inline => ({ cite: n });
const link = (to: string, text: string): Inline => ({ link: { to, text } });
const blank = (n: number): Inline => ({ blank: n });
const h = (level: number, text: string): Block => ({ heading: { level, text } });
const para = (...inl: Inline[]): Block => ({ para: inl });
const bullets = (...items: Inline[][]): Block => ({ bullets: items });
const numbered = (...items: Inline[][]): Block => ({ numbered: items });
const steps = (memory: boolean, ...items: Inline[][]): Block => ({ steps: items.map((text): Step => ({ text, memory })) });
const fields = (...rows: FieldRow[]): Block => ({ fields: rows });
const f = (label: string, value?: string, lines = 1): FieldRow => (value === undefined ? { label, lines } : { label, value, lines });
const table = (header: string[], rows: Inline[][][]): Block => ({ table: { header, rows } });
const callout = (kind: 'stop' | 'warning' | 'note' | 'decision', title: string | undefined, ...blocks: Block[]): Block => ({
  callout: title === undefined ? { kind, blocks } : { kind, title, blocks },
});
const decision = (question: string, ...branches: Branch[]): Block => ({ decision: { question, branches } });
const page = (id: string, title: string, kind: PageKind, fit: 'one' | 'two' | 'flow', ...blocks: Block[]): Page => ({ id, title, kind, fit, blocks });

/** The awkward user text the renderers must print exactly as written. */
export const AWKWARD = {
  /** Would be a tag if it were ever read as markup. */
  markup: 'Aunt <b>Rosa</b> & "Uncle" Lou',
  /** Would be bold and a heading if it were ever read as Markdown. */
  markdown: '**not bold** # not a heading [not](a link)',
  /** Would split a Markdown table cell. */
  bar: 'Keys | spare with Rosa',
  /** Latin Extended letters the PDF font covers. */
  accents: 'Zoë Łukasiewicz-Nguyễn, Ștefan and José',
  /** A long name that must wrap, not overflow. */
  long: 'Grandpa Joseph Alexander Montgomery-Fitzwilliam the Third, known to everyone on Sample Street as Joe',
};

const PEOPLE = [
  { id: 'person_1', name: 'Dana', dob: 'April 12, 1986', phone: '555-0101', blood: 'A+', allergies: 'None known' },
  { id: 'person_2', name: 'Sam', dob: 'September 3, 1988', phone: '555-0106', blood: 'O+', allergies: 'Penicillin' },
  { id: 'person_3', name: 'Riley', dob: 'June 21, 2017', phone: undefined, blood: undefined, allergies: 'Peanuts' },
];

function personPage(p: (typeof PEOPLE)[number], n: number): Page {
  return page(
    p.id,
    p.name,
    'person',
    'one',
    fields(
      f('Name', p.name),
      f('Date of birth', p.dob),
      f('Phone', p.phone),
      f('Doctor', n === 1 ? 'Dr. Morgan, Sample Family Practice, 555-0103' : undefined),
      f('Pharmacy', 'Corner pharmacy on Sample Street, 555-0104'),
      f('Conditions', n === 2 ? 'Asthma, mild' : undefined, 2),
      f('Allergies', p.allergies),
      f('Blood type', p.blood),
      f('Insurance', n === 1 ? 'Sample Health Plan, Employer Silver, member SHP-000-111, group G-4521, 555-0105' : undefined, 2),
      f('ID', n === 1 ? "Driver's license in wallet; passport in the document box" : undefined),
      f('Anything else a helper should know', n === 3 ? AWKWARD.markdown : undefined, 3),
    ),
    h(2, 'Medicines'),
    table(
      ['Medicine', 'Dose', 'When', 'What for'],
      n === 2
        ? [[[t('Rescue inhaler')], [t('As prescribed')], [t('When needed')], [t('Asthma')]], [[], [], [], []], [[], [], [], []]]
        : [[[], [], [], []], [[], [], [], []], [[], [], [], []], [[], [], [], []]],
    ),
    h(2, 'From your answers'),
    bullets([t('Takes a medicine every day: '), b('yes')], [t('Travels to work by car, about 12 miles.')]),
  );
}

function checklist(id: string, title: string, fit: 'one' | 'two', here: string | undefined, extra: Block[] = []): Page {
  const head: Block[] = here ? [para(b('Here: '), t(here), t(' · '), b('How bad: '), t('Severe'))] : [];
  return page(
    id,
    title,
    'checklist',
    fit,
    ...head,
    h(2, 'Use this when'),
    para(t('A smoke alarm sounds, or you see or smell smoke or fire in your home.')),
    h(2, 'Do first'),
    steps(
      true,
      [b('Get out.'), t(' Leave by the nearest safe way; do not stop for anything.'), cite(3)],
      [b('Stay low.'), t(' If there is smoke, crawl under it to your way out.'), cite(4)],
      [b('Feel doors first.'), t(' If a door or its knob is hot, keep it closed and use your second way out.'), cite(4)],
      [b('Meet outside.'), t(' Go to your meeting place and stay there.'), cite(3)],
      [b('Call 911 from outside.'), t(' Call once you are out, not before.'), cite(3)],
    ),
    h(2, 'Then'),
    steps(
      false,
      [b('Count heads.'), t(' Check that everyone is at the meeting place.'), cite(3)],
      [b('Tell firefighters.'), t(' If a person or pet is still inside, tell firefighters right away.'), cite(4)],
      [b('If you are trapped, seal the room.'), t(' Close the door and cover vents and cracks with cloth or tape.'), cite(4)],
      [b('Signal for help.'), t(' Call 911 and wave a light-colored cloth or a flashlight at the window.'), cite(4)],
      [b('If your clothes catch fire, stop, drop and roll.'), cite(4)],
    ),
    h(2, 'Leave or stay?'),
    decision(
      'Leave or stay?',
      { when: [b('Leave if'), t(' there is fire or smoke in the home.')], then: [t('Go to '), t('the corner mailbox at 12th and Sample'), t('.')], go_to: 'neighbourhood' },
      { when: [b('Stay in a room only if'), t(' every way out is blocked.')], then: [t('Seal the door and signal from the window.')] },
      { when: [b('Go to'), t(' another place if the home cannot be lived in.')], then: [t('Go to '), blank(24), t('.')], go_to: 'getting_out' },
    ),
    h(2, 'Where and who'),
    fields(f('Meeting place near home', 'The corner mailbox at 12th and Sample'), f('Second way out of each bedroom', undefined, 2)),
    h(2, 'Do not'),
    bullets(
      [t('Do not go back inside for anything, even a pet.'), cite(3)],
      [t('Do not open a door that is hot or has smoke coming around it.'), cite(4)],
      [t('Do not hide from firefighters.'), cite(4)],
    ),
    h(2, 'When it is over'),
    bullets(
      [t('Go back inside only when the fire department says it is safe.'), cite(4)],
      [t('Call your landlord or insurer, and list what was lost.'), cite(4)],
      [t('Put in new smoke alarms and test them every month.'), cite(5)],
      [t('Use the After pages for the first days. '), link('after', 'Tab 9, After a disaster: the first 30 days')],
    ),
    ...extra,
  );
}

const PARTS: Part[] = [
  {
    id: 'start',
    tab: 1,
    title: 'Start here',
    short_title: 'Start here',
    pages: [
      page('cover', 'Cover', 'cover', 'one', fields(f('Our address', '1234 Sample Street, Philadelphia, PA 19147'), f('Out-of-area contact', AWKWARD.markup))),
      page(
        'how_to_use',
        'How to use this binder',
        'how_to_use',
        'one',
        para(t('This binder is for the day something goes wrong. Keep it where everyone can find it, and take it if you leave.'), cite(1)),
        numbered(
          [t('In an emergency, start with '), link('quick_start', 'Tab 1, Quick start'), t('.')],
          [t('Find what is happening in '), link('index', 'Tab 1, Which checklist?'), t(' and turn to that page.')],
          [t('Numbers for everyone you might call are in '), link('contacts', 'Tab 1, Contacts at a glance'), t('.')],
        ),
        para(t('Blank lines are for you to fill in by hand. Review it by the date on the cover, and whenever something changes.'), cite(2)),
      ),
      page(
        'quick_start',
        'Quick start: the first five minutes of any emergency',
        'quick_start',
        'one',
        steps(
          true,
          [b('Get safe.'), t(' Move away from the danger: fire, smoke, water, a gas smell.'), cite(1)],
          [b('Call 911'), t(' if anyone is hurt or in danger.'), cite(1)],
          [b('Check on everyone'), t(' in your home, and on neighbors who may need help.'), cite(2)],
          [b('Get information'), t(' from local officials: radio, TV, alerts.'), cite(1)],
          [b('Decide: leave or stay.'), t(' Use the checklist for what is happening.'), cite(2)],
        ),
        callout('note', 'If you cannot reach anyone', para(t('Text instead of calling, and call your out-of-area contact: '), blank(20), t('.'))),
      ),
      page(
        'index',
        'Which checklist?',
        'index',
        'one',
        table(
          ['If this happens', 'Turn to'],
          [
            [[t('Gas leak or carbon monoxide alarm')], [link('check_gas_leak_or_co', 'Tab 6, Gas leak or carbon monoxide alarm')]],
            [[t('House fire')], [link('check_house_fire', 'Tab 6, House fire')]],
            [[t('Hurricane')], [link('check_hurricane', 'Tab 7, Hurricane')]],
            [[t('Power outage at home')], [link('check_power_outage', 'Tab 8, Power outage at home')]],
          ],
        ),
        para(t('Not in this binder: rare families you did not add. Tick them under Your settings to add them.')),
      ),
      page(
        'contacts',
        'Contacts at a glance',
        'contacts',
        'one',
        h(2, 'Emergency numbers'),
        fields(f('Emergency', '911'), f('Suicide and crisis line', '988'), f('Poison control', '1-800-222-1222')),
        h(2, 'Your people'),
        fields(f('Dana', '555-0101'), f('Sam', '555-0106'), f('Out-of-area contact', AWKWARD.markup), f('Lawyer'), f('Trusted neighbor', undefined, 2)),
      ),
    ],
  },
  {
    id: 'people',
    tab: 2,
    title: 'People',
    short_title: 'People',
    pages: [
      ...PEOPLE.map((p, i) => personPage(p, i + 1)),
      page('person_4', AWKWARD.long, 'person', 'one', fields(f('Name', AWKWARD.long), f('Date of birth'), f('Phone'), f('Blood type')), bullets([t('Uses a powered medical device: '), b('yes, 60 watts')])),
      page(
        'wallet_cards',
        'Wallet cards',
        'wallet_cards',
        'one',
        para(t('Cut out each card along the marks, fold it once and keep it in a wallet or school bag.')),
        {
          cards: PEOPLE.map(
            (p): Card => ({
              title: p.name,
              lines: [
                [b('Out-of-area contact: '), t(AWKWARD.markup)],
                [b('Meet near home: '), t('The corner mailbox at 12th and Sample')],
                [b('Meet away from home: '), t('Sample Branch Library, 500 Sample Road')],
                [b('Lawyer: '), blank(16)],
                [b('Blood type: '), t(p.blood ?? '—'), t(' · '), b('Allergies: '), t(p.allergies)],
                [t('See binder tab 2.')],
              ],
            }),
          ).concat([{ title: AWKWARD.accents, lines: [[b('Out-of-area contact: '), blank(16)], [t(AWKWARD.bar)], [t('See binder tab 2.')]] }]),
        },
      ),
    ],
  },
  {
    id: 'home_places',
    tab: 3,
    title: 'Home and places',
    short_title: 'Home & places',
    pages: [
      page(
        'home',
        'Home',
        'home',
        'one',
        fields(
          f('Address', '1234 Sample Street, Philadelphia, PA 19147'),
          f('Gas shut-off', 'Basement, left of the stairs; wrench on the hook'),
          f('Water shut-off'),
          f('Electric panel', 'Kitchen closet'),
          f('Electric company', 'Sample Electric, outage line 555-0120'),
          f('Where the kit is', 'Hall closet by the front door'),
          f('Where the spare keys are', AWKWARD.bar),
          f('Neighbors who check on us', undefined, 2),
        ),
        callout('warning', 'Gas', para(t('If you smell gas, leave first and call from outside. Only the gas company turns the gas back on.'), cite(6))),
        para(t('For a fire, see '), link('check_house_fire', 'Tab 6, House fire'), t('.')),
      ),
      page(
        'place_1',
        'Sample Elementary School',
        'place',
        'one',
        fields(
          f('Who goes there', 'Riley'),
          f('Kind', 'School'),
          f('Address', '90 Sample Lane, Philadelphia'),
          f('Phone', '555-0108'),
          f("The school's plan", 'The school shelters in place and sends a text alert; parents collect children from the gym door', 2),
          f('Pick-up', 'Dana, Sam, or Aunt Rosa, who is on the school card'),
          f('Safest spot there'),
        ),
      ),
      page(
        'neighbourhood',
        'Neighborhood',
        'neighbourhood',
        'one',
        fields(f('Meet near home', 'The corner mailbox at 12th and Sample'), f('Meet away from home'), f('Hospital with an emergency room', 'Sample General Hospital, 555-0124')),
        h(2, 'Hospitals with emergency rooms in Philadelphia County'),
        table(
          ['Hospital', 'City', 'Phone'],
          [
            [[t('Sample General Hospital')], [t('Philadelphia')], [t('555-0124')]],
            [[t('Sample Children’s Hospital')], [t('Philadelphia')], [t('555-0132')]],
          ],
        ),
        para(t('From the CMS hospital list of July 2026. Check before you need it.'), cite(7)),
        { map_slot: { id: 'map-neighbourhood', kind: 'neighbourhood', caption: 'Your neighborhood' } },
      ),
      page(
        'getting_out',
        'Getting out',
        'getting_out',
        'two',
        fields(f('Where we would go', 'Aunt Rosa, 45 Sample Court, Harrisburg'), f('First way out', 'I-76 west to the Turnpike'), f('Second way out'), f('Roadside help')),
        para(t('Most evacuations here come from floods and storms. Plan to be away three days to two weeks.'), cite(8)),
        { map_slot: { id: 'map-area', kind: 'area', caption: 'Your city or county' } },
        { map_slot: { id: 'map-region', kind: 'region', caption: 'Getting out: your region' } },
      ),
    ],
  },
  {
    id: 'pets_vehicles_documents',
    tab: 4,
    title: 'Pets, vehicles and documents',
    short_title: 'Pets & docs',
    pages: [
      page(
        'pets',
        'Pets',
        'pets',
        'one',
        h(2, 'Biscuit (dog)'),
        fields(f('Looks like', 'Brown terrier mix, red collar'), f('Vet', 'Sample Animal Hospital, 555-0128'), f('Microchip', '985-000-000-000'), f('Medicines')),
        h(2, 'Who takes the animals if you can’t'),
        fields(f('Name and phone', undefined, 2)),
      ),
      page('vehicles', 'Vehicles', 'vehicles', 'one', h(2, 'Blue 2016 hatchback'), fields(f('Plate', 'SMP-0101'), f('Insurer', 'Sample Auto Insurance, 555-0129'), f('What stays in the car', 'Blanket, water, phone charger, a paper map'))),
      page(
        'documents',
        'Documents and money',
        'documents',
        'one',
        para(t('The Emergency Financial First Aid Kit has four parts.'), cite(9)),
        numbered([t('Identification: '), blank(20)], [t('Household and money: '), blank(20)], [t('Medical: '), blank(20)], [t('Legal: '), blank(20)]),
        h(2, 'Accounts'),
        table(
          ['Where', 'Kind', 'Phone', 'Last four digits'],
          [
            [[t('Sample Credit Union')], [t('Checking')], [t('555-0130')], [t('4821')]],
            [[t('Sample Credit Union')], [t('Savings')], [t('555-0130')], [t('7302')]],
          ],
        ),
        fields(f('Where the originals are', 'Fireproof box on the closet shelf'), f('Where the copies are'), f('Cash on hand', undefined, 1)),
      ),
    ],
  },
  {
    id: 'have',
    tab: 5,
    title: 'What you have',
    short_title: 'What you have',
    pages: [
      page(
        'inventory',
        'What you have',
        'inventory',
        'flow',
        h(2, 'No power'),
        table(
          ['Item', 'How much', 'Status', 'Where kept', 'Next check'],
          Array.from({ length: 18 }, (_, i) => [[t(`Supply line ${i + 1}: flashlights and batteries`)], [t(`${i + 2} each`)], [i % 3 ? t('have') : t('still to get (month 3)')], [], [t('October 2027')]]),
        ),
      ),
      page(
        'risks_glance',
        'Risks at a glance',
        'risks_glance',
        'one',
        table(
          ['Hazard', 'How likely here, 10 years', 'How bad', 'Checklist'],
          [
            [[t('House fire')], [t('about 4 in 100 households')], [t('Severe')], [link('check_house_fire', 'Tab 6, House fire')]],
            [[t('Power outage')], [t('about 60 in 100 households')], [t('Moderate')], [link('check_power_outage', 'Tab 8, Power outage at home')]],
          ],
        ),
        para(t('The chances are for households like yours in this county.'), cite(10)),
      ),
    ],
  },
  {
    id: 'check_now',
    tab: 6,
    title: 'Checklists: happening now',
    short_title: 'Happening now',
    pages: [
      page(
        'check_gas_leak_or_co',
        'Gas leak or carbon monoxide alarm',
        'checklist',
        'one',
        callout('stop', 'Get out first', para(t('Do not use switches, phones or flames inside. Leave, then call.'), cite(6))),
        h(2, 'Do first'),
        steps(true, [b('Leave now.'), t(' Take people and pets outside.'), cite(6)], [b('Call from outside:'), t(' 911 and the gas company.'), cite(6)]),
      ),
      checklist('check_house_fire', 'House fire', 'one', 'about 4 in 100 households like yours over ten years'),
    ],
  },
  {
    id: 'check_coming',
    tab: 7,
    title: 'Checklists: it is coming',
    short_title: 'It is coming',
    pages: [checklist('check_hurricane', 'Hurricane', 'two', 'about 8 in 100 households like yours over ten years', [{ page_break: true }, h(2, 'After the storm passes'), bullets([t('Stay off the roads until officials say they are clear.'), cite(8)])])],
  },
  {
    id: 'check_ongoing',
    tab: 8,
    title: 'Checklists: it goes on',
    short_title: 'It goes on',
    pages: [
      page(
        'check_power_outage',
        'Power outage at home',
        'checklist',
        'one',
        callout('decision', 'Medical devices', para(t('If a device needs power and the battery will not last, go to '), link('neighbourhood', 'Tab 3, Neighborhood'), t('.'))),
        decision('Leave or stay?', { when: [b('Stay if'), t(' the home stays warm or cool enough.')], then: [t('Keep the fridge closed.')] }, { when: [b('Leave if'), t(' it gets too hot or cold.')], then: [t('Go to a cooling or warming center.')], go_to: 'getting_out' }),
      ),
    ],
  },
  {
    id: 'after',
    tab: 9,
    title: 'After',
    short_title: 'After',
    pages: [
      page('after', 'After a disaster: the first 30 days', 'after', 'flow', numbered([t('Check in with family and your out-of-area contact.'), cite(11)], [t('Photograph damage before you clean up.'), cite(11)]), callout('decision', 'Stay or move out?', para(t('If the home is not safe, contact your insurer and the county before you sign anything.'), cite(11)))),
      page('log_damage', 'Damage log', 'log', 'one', { log: { columns: ['What', 'Where', 'Photo taken', 'Reported to'], rows: 14 } }),
      page('log_expenses', 'Expenses log', 'log', 'one', { log: { columns: ['Date', 'What', 'Amount', 'Receipt'], rows: 14 } }),
    ],
  },
  {
    id: 'sources',
    tab: 10,
    title: 'Sources',
    short_title: 'Sources',
    pages: [page('sources', 'Sources', 'sources', 'flow', h(2, 'How this binder was made'), fields(f('App', 'Ready Reckoner 0.3.0'), f('Data pack', '2026.09.27'), f('Content', '2026.09.27')))],
  },
];

const SOURCES: SourceEntry[] = [
  { n: 1, title: 'Make A Plan', publisher: 'FEMA / Ready.gov', year: 2026, url: 'https://www.ready.gov/plan', expert: false },
  { n: 2, title: 'Emergency Preparedness Checklist', publisher: 'American Red Cross', year: 2025, url: 'https://www.redcross.org/get-help/how-to-prepare-for-emergencies.html', expert: false },
  { n: 3, title: 'Home Fire Escape Plans', publisher: 'U.S. Fire Administration', year: 2026, url: 'https://www.usfa.fema.gov/prevention/home-fires/prepare-for-fire/escape-planning/', expert: false },
  { n: 4, title: 'Home Fires', publisher: 'FEMA / Ready.gov', year: 2026, url: 'https://www.ready.gov/home-fires', expert: false },
  { n: 5, title: 'Smoke alarms', publisher: 'U.S. Fire Administration', year: 2026, url: 'https://www.usfa.fema.gov/prevention/home-fires/prepare-for-fire/smoke-alarms/', expert: false },
  { n: 6, title: 'Carbon Monoxide Poisoning', publisher: 'CDC', year: 2024, url: 'https://www.cdc.gov/carbon-monoxide/', expert: false },
  { n: 7, title: 'Hospital General Information', publisher: 'Centers for Medicare & Medicaid Services', year: 2026, url: 'https://data.cms.gov/provider-data/dataset/xubh-q36u', expert: false },
  { n: 8, title: 'Evacuation', publisher: 'FEMA / Ready.gov', year: 2026, url: 'https://www.ready.gov/evacuation', expert: false },
  { n: 9, title: 'Emergency Financial First Aid Kit', publisher: 'FEMA', year: 2021, url: 'https://www.ready.gov/financial-preparedness', expert: false },
  { n: 10, title: 'Household hazard rates (planning estimate)', publisher: 'Ready Reckoner', year: 2026, expert: true },
  { n: 11, title: 'Recovering from Disaster', publisher: 'FEMA / Ready.gov', year: 2026, url: 'https://www.ready.gov/recovering-disaster', expert: false },
];

/** The fixture binder. */
export const FIXTURE_BINDER: Binder = {
  title: 'Emergency binder for Dana, Sam, Riley and Joe',
  generated_on: '2026-10-01',
  household: '2 adults, 1 older adult and 1 child, with 1 dog',
  location: 'Philadelphia County, Pennsylvania (ZIP code 19147)',
  status_line:
    'Ready Reckoner is an independent, open-source planning aid. It is not official emergency guidance, and not medical, legal or financial advice. Follow instructions from your local officials first.',
  review_by: '2027-10-01',
  parts: PARTS,
  sources: SOURCES,
  credits: [
    'FEMA National Risk Index, version 1.20.0 (December 2025), accessed September 26, 2026. This product uses the Federal Emergency Management Agency’s National Risk Index dataset but is not endorsed by FEMA.',
    'Eviction data from the Eviction Lab at Princeton University, evictionlab.org, under the Open Data Commons Attribution License. https://evictionlab.org/#data',
  ],
};
