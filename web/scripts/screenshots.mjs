// Screenshots of every screen at desktop and phone widths, an axe accessibility check of each
// page in a real browser, and the binder printed to PDF by the browser's own Print (Letter and A4).
//
//   npm run build && npm run shots
//
// Uses the system Chrome through puppeteer-core (no browser download). Output goes to SHOTS_DIR
// (default ~/Desktop/ready-reckoner-briefs/shots/web-shell). Each fixture household is loaded
// into the page's storage before it opens, exactly as if the person had typed it in.
import { createServer } from 'node:http';
import { mkdirSync, readFileSync, existsSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import puppeteer from 'puppeteer-core';

import { chromePath } from './chrome.mjs';

const here = fileURLToPath(new URL('.', import.meta.url));
const dist = join(here, '..', 'dist');
const fixturesDir = join(here, '..', '..', 'fixtures', 'households');
const outDir = process.env.SHOTS_DIR ?? join(homedir(), 'Desktop', 'ready-reckoner-briefs', 'shots', 'web-shell');
const axePath = join(here, '..', 'node_modules', 'axe-core', 'axe.min.js');
const only = process.env.SHOTS_ONLY ? new RegExp(process.env.SHOTS_ONLY) : null;

if (!existsSync(join(dist, 'index.html'))) {
  console.error('Build first: npm run build');
  process.exit(1);
}
mkdirSync(outDir, { recursive: true });

// A tiny static server for dist/ (hash routing means every path is index.html or a file).
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.json': 'application/json', '.webmanifest': 'application/manifest+json', '.map': 'application/json' };
const server = createServer((req, res) => {
  const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  let file = join(dist, path === '/' ? 'index.html' : path);
  if (!file.startsWith(dist) || !existsSync(file)) file = join(dist, 'index.html');
  res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' });
  res.end(readFileSync(file));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}/`;

const fixture = (name) => JSON.parse(readFileSync(join(fixturesDir, `${name}.json`), 'utf8'));
const STEPS = ['where', 'who', 'travel', 'money', 'have'];
function saved(input, extra = {}) {
  return {
    format: 'ready-reckoner-plan',
    version: 1,
    input,
    purchases: [],
    progress: { completed: STEPS },
    confidence: {},
    dismissed_warnings: [],
    done_dates: {},
    ...extra,
  };
}

const philly = fixture('philadelphia-renters-4');
const phillyProgress = saved(philly, {
  purchases: [
    { item_id: 'alarms_test', tier: 'now', qty: 1, date: '2026-10-02' },
    { item_id: 'plan_family_contacts', tier: 'now', qty: 1, date: '2026-10-02' },
    { item_id: 'documents_copies', tier: 'now', qty: 1, date: '2026-10-03' },
    { item_id: 'water_reused_bottles', tier: 'now', qty: 1, date: '2026-10-03' },
    { item_id: 'neighbours_numbers', tier: 'now', qty: 1, date: '2026-10-04' },
    { item_id: 'water_stored', tier: 'h72', qty: 3, paid_usd: 3, date: '2026-10-05' },
    { item_id: 'co_alarm', tier: 'h72', qty: 1, paid_usd: 28, date: '2026-10-05' },
  ],
  confidence: { before: 2, after: 3 },
});
const ambiguous = structuredClone(philly);
ambiguous.location.zip = '19087';
ambiguous.location.setting = 'suburban';

const DESKTOP = { width: 1280, height: 900, deviceScaleFactor: 1 };
const PHONE = { width: 390, height: 844, deviceScaleFactor: 2, isMobile: true, hasTouch: true };

const SCREENS = [
  ['00-start-new', 'start', null],
  ['00-start-returning', 'start', saved(philly)],
  ['01-where', 'where', saved(philly)],
  ['02-who', 'who', saved(philly)],
  ['03-travel', 'travel', saved(philly)],
  ['04-money', 'money', saved(philly)],
  ['05-have', 'have', saved(philly)],
  ['06-risks', 'risks', saved(philly)],
  ['07-prepare', 'prepare', phillyProgress],
  ['08-binder', 'binder', saved(philly)],
  ['09-maintain', 'maintain', phillyProgress],
  ['10-learn', 'learn', saved(philly)],
  ['10-learn-article', 'learn/myths', saved(philly)],
  ['11-about', 'about', saved(philly)],
];

const shots = [];
for (const [name, route, plan] of SCREENS) {
  shots.push({ name: `${name}--philadelphia--desktop`, route, plan, viewport: DESKTOP });
  shots.push({ name: `${name}--philadelphia--phone`, route, plan, viewport: PHONE });
}
const extras = [
  { name: '01-where-zip-spans-counties--desktop', route: 'where', plan: saved(ambiguous), viewport: DESKTOP },
  { name: '06-risks-settings-open--philadelphia--desktop', route: 'risks', plan: saved(philly), viewport: DESKTOP, click: 'button[aria-controls="settings-panel"]' },
  { name: '06-risks--coos-bay-cascadia--desktop', route: 'risks', plan: saved(fixture('coos-bay-well-owner-2')), viewport: DESKTOP },
  { name: '06-risks--coos-bay-cascadia--phone', route: 'risks', plan: saved(fixture('coos-bay-well-owner-2')), viewport: PHONE },
  { name: '06-risks--miami-dark--desktop', route: 'risks', plan: saved(fixture('miami-condo-retiree-1')), viewport: DESKTOP, theme: 'dark' },
  { name: '07-prepare--chicago-zero-budget--desktop', route: 'prepare', plan: saved(fixture('chicago-student-zero-budget-1')), viewport: DESKTOP },
  { name: '07-prepare--phoenix-cpap--phone', route: 'prepare', plan: saved(fixture('phoenix-apartment-cpap-1')), viewport: PHONE },
  { name: '07-prepare--sugar-land-dark--desktop', route: 'prepare', plan: saved(fixture('sugar-land-ev-household-3')), viewport: DESKTOP, theme: 'dark' },
  { name: '06-risks--hays-kansas--desktop', route: 'risks', plan: saved(fixture('hays-kansas-farm-5')), viewport: DESKTOP },
  { name: '06-risks-unknown-zip--desktop', route: 'risks', plan: saved({ ...structuredClone(philly), location: { country: 'US', zip: '00000', setting: 'urban' } }), viewport: DESKTOP },
];
shots.push(...extras);

// v0.2.0 (web-interview): the new questions, the settings' rare families and the binder's wallet
// cards, for a household with its family plan filled in (its questions are steps 7 and 8 since
// v0.3.0; the family-plan screen is gone).
const detroit = fixture('detroit-snap-3');
const detroitTested = saved({ ...structuredClone(detroit), existing: [{ item_id: 'flashlights_headlamps', qty: 3, tested_on: '2026-09-20' }] });
shots.push(
  { name: '12-places-v2-answers--detroit--desktop', route: 'places', plan: saved(detroit), viewport: DESKTOP },
  { name: '12-places-v2-answers--detroit-guides-open--desktop', route: 'places', plan: saved(detroit), viewport: DESKTOP, openDetails: '.plan-guide' },
  { name: '08-binder-wallet-cards--detroit--desktop', route: 'binder/wallet-cards', plan: saved(detroit), viewport: DESKTOP, cardsPrint: true },
  { name: '02-who-help-in-an-emergency--detroit--phone', route: 'who', plan: saved(detroit), viewport: PHONE },
  { name: '01-where-v2--detroit--desktop', route: 'where', plan: saved(detroit), viewport: DESKTOP },
  { name: '04-money-v2--detroit--desktop', route: 'money', plan: saved(detroit), viewport: DESKTOP },
  { name: '05-have-tested--detroit--desktop', route: 'have', plan: detroitTested, viewport: DESKTOP },
  { name: '06-risks-settings-v2--detroit--desktop', route: 'risks', plan: saved(detroit), viewport: DESKTOP, click: 'button[aria-controls="settings-panel"]' },
);

// v0.2.0 (verify2): the validation page, the rare box for a household that ticks the nuclear row
// near a missile field (Minot), the Plan screen's grouped decisions and rare-allowance lines (Minot,
// and Hays with eight decisions), the legal-emergency switch in the settings, and dark and phone
// views of each.
const minot = fixture('minot-missile-field-3');
const minotLegal = structuredClone(minot);
minotLegal.dials.legal_opt_in = true;
const hays = fixture('hays-kansas-farm-5');
shots.push(
  { name: '13-validation--philadelphia--desktop', route: 'validation', plan: saved(philly), viewport: DESKTOP },
  { name: '13-validation--philadelphia--phone', route: 'validation', plan: saved(philly), viewport: PHONE },
  { name: '13-validation--philadelphia-dark--desktop', route: 'validation', plan: saved(philly), viewport: DESKTOP, theme: 'dark' },
  { name: '14-risks--minot--desktop', route: 'risks', plan: saved(minot), viewport: DESKTOP },
  { name: '14-risks--minot--phone', route: 'risks', plan: saved(minot), viewport: PHONE },
  { name: '14-risks--minot-dark--desktop', route: 'risks', plan: saved(minot), viewport: DESKTOP, theme: 'dark' },
  { name: '14-risks-rare-nuclear-open--minot--desktop', route: 'risks', plan: saved(minot), viewport: DESKTOP, click: '#rare-nuclear_attack button.expander', openDetails: '#rare-nuclear_attack details' },
  { name: '14-risks-settings-legal-on--minot--desktop', route: 'risks', plan: saved(minotLegal), viewport: DESKTOP, click: 'button[aria-controls="settings-panel"]' },
  { name: '15-prepare--minot--desktop', route: 'prepare', plan: saved(minot), viewport: DESKTOP, openDetails: 'details.decisions' },
  { name: '15-prepare--minot--phone', route: 'prepare', plan: saved(minot), viewport: PHONE },
  { name: '15-prepare--minot-legal-dark--desktop', route: 'prepare', plan: saved(minotLegal), viewport: DESKTOP, theme: 'dark' },
  { name: '15-prepare--hays-decisions--desktop', route: 'prepare', plan: saved(hays), viewport: DESKTOP },
  { name: '16-places--minot--desktop', route: 'places', plan: saved(minot), viewport: DESKTOP },
  { name: '17-binder--minot--desktop', route: 'binder', plan: saved(minot), viewport: DESKTOP },
  { name: '18-maintain--minot-dark--phone', route: 'maintain', plan: saved(minot), viewport: PHONE, theme: 'dark' },
  { name: '10-learn-before-you-need-them--philadelphia--desktop', route: 'learn/before-you-need-them', plan: saved(philly), viewport: DESKTOP },
  { name: '10-learn-strategic-sites--minot--phone', route: 'learn/strategic-sites', plan: saved(minot), viewport: PHONE },
);

// v0.3.0 (web-interview3): the optional steps 6–8, filled in with obviously sample answers
// (555-01xx numbers, generic medicine names, a made-up street) and empty, at desktop and phone
// widths; the Prepare tab and its printed sheet; the save dialog that protects a file holding them.
const filled = structuredClone(philly);
filled.people[0].profile = {
  name: 'Ana Sample',
  date_of_birth: '3 March 1984',
  phone: '555-0101',
  email: 'ana@example.org',
  place: { kind: 'work', name: 'Sample Logistics', address: '400 Example Ave', phone: '555-0110', plan: 'Stay inside; the building has a shelter floor.', safest_spot: 'Stairwell B, second floor' },
  doctor: { name: 'Dr. R. Example', phone: '555-0120' },
  pharmacy: { name: 'Corner Pharmacy', phone: '555-0121' },
  conditions: 'High blood pressure',
  medications: [
    { name: 'Blood pressure tablet', dose: '10 mg', schedule: 'Every morning', purpose: 'Blood pressure' },
    { name: 'Allergy tablet', dose: '1 tablet', schedule: 'As needed', purpose: 'Hay fever' },
  ],
  allergies: 'Penicillin (rash)',
  blood_type: 'A+',
  insurance: { carrier: 'Sample Health', plan_name: 'Silver', member_id: 'XJ-000000', group_number: 'G-0000', phone: '555-0130' },
  id_notes: 'Passport in the document pouch',
  notes: 'Wears glasses to read',
};
filled.people[2].profile = {
  name: 'Leo',
  place: { kind: 'school', name: 'Sample Elementary', phone: '555-0150', pickup: 'Ana, Sam or Rosa, with photo ID', safest_spot: 'The gym' },
  allergies: 'Peanuts',
};
filled.family_plan = {
  meeting_place_near: 'The mailbox at the corner',
  meeting_place_far: 'The Example Street library',
  out_of_area_contact: { name: 'Cousin Mia', phone: '555-0170' },
  shelter_spot_home: 'The inside hallway downstairs',
  where_we_would_go: 'Cousin Mia, Harrisburg',
  routes: ['I-76 west', 'Route 30 west'],
  shutoff_water: 'Basement, by the meter; the wrench hangs beside it',
  shutoff_electric: 'Basement panel; the top switch turns off everything',
  trusted_circle: [{ name: 'Rosa', phone: '555-0180', holds: ['spare_key', 'documents'] }],
  home: {
    address: '12 Sample Street, Unit 2',
    electric_utility: { name: 'Sample Power', phone: '555-0190' },
    water_utility: { name: 'Sample Water Department', phone: '555-0191' },
    insurer: { name: 'Sample Renters Insurance', phone: '555-0192' },
    policy_number: 'R-000000',
    landlord_or_mortgage: { name: 'Example Property Co.', phone: '555-0193' },
    where_kit: 'Hall closet, bottom shelf',
    where_documents: 'Waterproof pouch in the go-bag',
    where_cash: 'Envelope in the go-bag',
    where_keys: 'With Rosa next door',
  },
  neighbourhood: {
    hospital: { name: 'Sample General Hospital', address: '1 Health Way', phone: '555-0160' },
    pharmacy: { name: 'Corner Pharmacy', address: '20 Example Ave', phone: '555-0121' },
    county_emergency_office: { name: 'County Office of Emergency Management', phone: '555-0161' },
    alerts: 'County text alerts; the local news radio station',
  },
  pets: [{ name: 'Biscuit', kind: 'Dog', description: 'Small, brown, white chest', vet: { name: 'Sample Vets', phone: '555-0140' }, microchip: '985 000 000 000 000', records_where: 'In the go-bag pouch' }],
  vehicles: [{ description: 'Blue 2016 hatchback', plate: 'SMP-0000', insurer: { name: 'Sample Auto', phone: '555-0195' }, policy_number: 'A-000000', kept_in_car: 'Blanket, water, phone charger' }],
  documents: {
    accounts: [
      { institution: 'First Sample Bank', kind: 'Checking', phone: '555-0196', last4: '0042' },
      { institution: 'Sample Credit Union', kind: 'Savings', phone: '555-0197', last4: '0077' },
    ],
    policies: [{ insurer: 'Sample Life', kind: 'Life', policy_number: 'L-000000', phone: '555-0198' }],
    where_originals: 'Fire-resistant box under the bed',
    where_copies: 'With Rosa, and in the go-bag',
    digital_backup: 'A USB stick in the fire-resistant box',
  },
};
const chicago = fixture('chicago-student-zero-budget-1');
shots.push(
  { name: '19-people--philadelphia-filled--desktop', route: 'people', plan: saved(filled), viewport: DESKTOP },
  { name: '19-people--philadelphia-filled-all-open--desktop', route: 'people', plan: saved(filled), viewport: DESKTOP, openDetails: 'details' },
  { name: '19-people--philadelphia-filled--phone', route: 'people', plan: saved(filled), viewport: PHONE, openDetails: '#person-1 details' },
  { name: '19-people--philadelphia-empty--phone', route: 'people', plan: saved(philly), viewport: PHONE },
  { name: '19-people--philadelphia-filled-dark--desktop', route: 'people', plan: saved(filled), viewport: DESKTOP, theme: 'dark' },
  { name: '20-places--philadelphia-filled--desktop', route: 'places', plan: saved(filled), viewport: DESKTOP },
  { name: '20-places--philadelphia-filled--phone', route: 'places', plan: saved(filled), viewport: PHONE },
  { name: '20-places--chicago-empty--desktop', route: 'places', plan: saved(chicago), viewport: DESKTOP },
  { name: '21-contacts--philadelphia-filled--desktop', route: 'contacts', plan: saved(filled), viewport: DESKTOP },
  { name: '21-contacts--philadelphia-filled--phone', route: 'contacts', plan: saved(filled), viewport: PHONE },
  { name: '21-contacts--chicago-empty--phone', route: 'contacts', plan: saved(chicago), viewport: PHONE },
  { name: '22-save-dialog--philadelphia-filled--desktop', route: 'maintain', plan: saved(filled), viewport: DESKTOP, click: '.data__actions > div:first-child button' },
  { name: '22-save-dialog--philadelphia-filled--phone', route: 'maintain', plan: saved(filled), viewport: PHONE, click: '.data__actions > div:first-child button' },
  { name: '23-prepare--philadelphia-filled--desktop', route: 'prepare', plan: saved(filled), viewport: DESKTOP, sheetPrint: true },
  { name: '23-start-returning--philadelphia-filled--desktop', route: 'start', plan: saved(filled), viewport: DESKTOP },
);

const browser = await puppeteer.launch({ executablePath: chromePath(), headless: true, args: ['--no-sandbox', '--font-render-hinting=none'] });
const axeReport = [];
const consoleErrors = [];
try {
  for (const shot of shots) {
    if (only && !only.test(shot.name)) continue;
    const context = await browser.createBrowserContext();
    const page = await context.newPage();
    await page.setBypassCSP(true);
    await page.setViewport(shot.viewport);
    await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: shot.theme === 'dark' ? 'dark' : 'light' }]);
    page.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(`${shot.name}: ${m.text()}`);
    });
    page.on('pageerror', (e) => consoleErrors.push(`${shot.name}: ${e.message}`));
    await page.evaluateOnNewDocument(
      (plan, theme) => {
        if (plan) localStorage.setItem('rr.plan.v1', JSON.stringify(plan));
        if (theme) localStorage.setItem('rr.prefs.v1', JSON.stringify({ theme, expert: false }));
      },
      shot.plan,
      shot.theme ?? null,
    );
    await page.goto(`${base}#/${shot.route}`, { waitUntil: 'networkidle0' });
    await page.waitForSelector('#page-title');
    await new Promise((r) => setTimeout(r, 450));
    if (shot.click) {
      await page.click(shot.click);
      await new Promise((r) => setTimeout(r, 300));
    }
    if (shot.openDetails) {
      await page.evaluate((sel) => document.querySelectorAll(sel).forEach((d) => (d.open = true)), shot.openDetails);
      await new Promise((r) => setTimeout(r, 200));
    }
    const file = join(outDir, `${shot.name}.png`);
    // The whole binder is one very long page (some 80 sheets' worth): Chrome refuses a full-page
    // capture of it on a phone, so binder shots show the screen as it first opens.
    await page.screenshot({ path: file, fullPage: !shot.route.startsWith('binder') });
    await page.addScriptTag({ path: axePath });
    const result = await page.evaluate(async () => {
      // eslint-disable-next-line no-undef
      const r = await axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa', 'best-practice'] } });
      return r.violations.map((v) => ({ id: v.id, impact: v.impact, help: v.help, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
    });
    axeReport.push({ shot: shot.name, violations: result });
    console.log(`${shot.name}: ${result.length ? result.map((v) => `${v.id}(${v.nodes})`).join(', ') : 'axe clean'}`);
    if (shot.cardsPrint) {
      // What "Print this page" on the wallet cards sends to the printer: that page alone.
      await page.evaluate(() => {
        document.querySelector('.binder-screen')?.classList.add('printing-one');
        document.querySelector('#binder-wallet_cards')?.closest('.binder-pagewrap')?.classList.add('print-this');
      });
      await page.emulateMediaType('print');
      await page.screenshot({ path: join(outDir, `${shot.name.replace('--desktop', '')}--print-cards-only.png`), fullPage: true });
      await page.pdf({ path: join(outDir, `${shot.name.replace('--desktop', '')}--cards-only--letter.pdf`), format: 'Letter', printBackground: false });
      await page.emulateMediaType(null);
      console.log('wallet cards printed alone (PNG, Letter PDF)');
    }
    if (shot.name === '17-binder--minot--desktop') {
      await page.emulateMediaType('print');
      await page.pdf({ path: join(outDir, '17-binder--minot--letter.pdf'), format: 'Letter', printBackground: false });
      await page.pdf({ path: join(outDir, '17-binder--minot--a4.pdf'), format: 'A4', printBackground: false });
      console.log('Minot binder PDFs written (Letter, A4)');
    }
    if (shot.name === '08-binder--philadelphia--desktop') {
      await page.emulateMediaType('print');
      await page.screenshot({ path: join(outDir, '08-binder-print-view--philadelphia.png'), fullPage: true });
      await page.pdf({ path: join(outDir, '08-binder--philadelphia--letter.pdf'), format: 'Letter', printBackground: false });
      await page.pdf({ path: join(outDir, '08-binder--philadelphia--a4.pdf'), format: 'A4', printBackground: false });
      console.log('binder PDFs written (Letter, A4)');
    }
    if (shot.sheetPrint) {
      // What "Print your preparation plan" sends to the printer: the Prepare sheet alone.
      await page.evaluate(() => window.dispatchEvent(new Event('beforeprint')));
      await page.emulateMediaType('print');
      await page.screenshot({ path: join(outDir, `${shot.name.replace('--desktop', '')}--print-sheet.png`), fullPage: true });
      await page.emulateMediaType(null);
      await page.evaluate(() => window.dispatchEvent(new Event('afterprint')));
      console.log('Prepare sheet printed alone (PNG)');
    }
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}

const reportPath = process.env.AXE_REPORT ?? join(here, '..', 'node_modules', '.cache', 'axe-report.json');
mkdirSync(join(reportPath, '..'), { recursive: true });
writeFileSync(reportPath, JSON.stringify({ axe: axeReport, consoleErrors }, null, 2));
const total = axeReport.reduce((s, r) => s + r.violations.length, 0);
console.log(`\n${axeReport.length} pages checked; ${total} axe violation types; ${consoleErrors.length} console errors. Report: ${reportPath}`);
if (consoleErrors.length) console.log(consoleErrors.slice(0, 20).join('\n'));
console.log(`Screenshots in ${outDir}`);
