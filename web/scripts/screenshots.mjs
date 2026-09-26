// Screenshots of every screen at desktop and phone widths, an axe accessibility check of each
// page in a real browser, and the packet printed to PDF (Letter and A4).
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
  ['07-plan', 'plan', phillyProgress],
  ['08-packet', 'packet', saved(philly)],
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
  { name: '07-plan--chicago-zero-budget--desktop', route: 'plan', plan: saved(fixture('chicago-student-zero-budget-1')), viewport: DESKTOP },
  { name: '07-plan--phoenix-cpap--phone', route: 'plan', plan: saved(fixture('phoenix-apartment-cpap-1')), viewport: PHONE },
  { name: '07-plan--sugar-land-dark--desktop', route: 'plan', plan: saved(fixture('sugar-land-ev-household-3')), viewport: DESKTOP, theme: 'dark' },
  { name: '06-risks--hays-kansas--desktop', route: 'risks', plan: saved(fixture('hays-kansas-farm-5')), viewport: DESKTOP },
  { name: '06-risks-unknown-zip--desktop', route: 'risks', plan: saved({ ...structuredClone(philly), location: { country: 'US', zip: '00000', setting: 'urban' } }), viewport: DESKTOP },
];
shots.push(...extras);

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
    const file = join(outDir, `${shot.name}.png`);
    await page.screenshot({ path: file, fullPage: true });
    await page.addScriptTag({ path: axePath });
    const result = await page.evaluate(async () => {
      // eslint-disable-next-line no-undef
      const r = await axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa', 'best-practice'] } });
      return r.violations.map((v) => ({ id: v.id, impact: v.impact, help: v.help, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
    });
    axeReport.push({ shot: shot.name, violations: result });
    console.log(`${shot.name}: ${result.length ? result.map((v) => `${v.id}(${v.nodes})`).join(', ') : 'axe clean'}`);
    if (shot.name === '08-packet--philadelphia--desktop') {
      await page.emulateMediaType('print');
      await page.screenshot({ path: join(outDir, '08-packet-print-view--philadelphia.png'), fullPage: true });
      await page.pdf({ path: join(outDir, '08-packet--philadelphia--letter.pdf'), format: 'Letter', printBackground: false });
      await page.pdf({ path: join(outDir, '08-packet--philadelphia--a4.pdf'), format: 'A4', printBackground: false });
      console.log('packet PDFs written (Letter, A4)');
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
