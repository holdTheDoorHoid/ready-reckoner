// End-to-end smoke test of the built site with the WebAssembly engine, in the system Chrome
// (puppeteer-core, no browser download). Served the way GitHub Pages serves it (gzip, wasm type,
// cache headers, 404s) under the /ready-reckoner/ base path, with the content security policy.
//
//   bash crates/rr-wasm/build-web.sh                   # once: the engine and the data
//   cd web && BASE_PATH=/ready-reckoner/ npm run build && npm run e2e
//   (a build without BASE_PATH is served at / instead)
//
// What it checks, as a first-time visitor would meet the site:
//  1. First visit, fresh profile, on an emulated 10 Mbps connection: the start screen is ready
//     before the county data is in; every byte the server sends until the county data has loaded
//     and the offline copy is made is counted (gzip at about Pages' level).
//  2. The Philadelphia fixture is opened through "Open a saved plan" and walked through the five
//     required interview steps with Continue, then past the three optional ones (Continue into
//     step 6, "Skip for now" twice, "See your risks"); on "Your risks" every bucket target (the
//     data on each card and the words on screen) equals fixtures/golden/philadelphia-renters-4.json.
//  3. In-browser timing of `assess` (performance.now() around the engine call, recorded by the
//     app as the performance measures rr:assess:engine and rr:assess), over several dial changes.
//  4. The ZIP tables and the map are fetched only when needed, and counted separately.
//  5. Offline: with the network gone, a reload still shows the same plan.
//  6. Screenshots (risks, plan, the binder's print view, About with the credits, the ambiguous-ZIP
//     picker, the county map, the loading line) into SHOTS_DIR.
//  7. Maps behind consent (DESIGN-DELTA-v3 §9): a first visit makes no request off the site;
//     opening the maps panel makes none until "Fetch maps" is pressed, and the consent screen
//     names every recipient in web/src/lib/maps/sources.ts; after consent, requests go only to
//     those origins, with the site's address alone as the Referer, and one press asks for at most
//     250 tiles; the maps are kept in the IndexedDB database rr-maps, which "Forget everything"
//     deletes. Every outside service is answered by a stub here: the check never contacts
//     OpenStreetMap, FEMA or anyone else (the OSMF tile policy forbids automated scans), and the
//     pin-map chunk opens offline from the service worker.
//
// Exits non-zero on the first failed check. Prints a JSON summary last.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { build } from 'esbuild';
import puppeteer from 'puppeteer-core';

import { chromePath } from './chrome.mjs';
import { serve } from './serve.mjs';

const here = fileURLToPath(new URL('.', import.meta.url));
const web = join(here, '..');
const repo = join(web, '..');
const dist = process.env.DIST ? join(web, process.env.DIST) : join(web, 'dist');
const shots = process.env.SHOTS_DIR ?? join(homedir(), 'Desktop', 'ready-reckoner-briefs', 'shots', 'web-engine');
const FIXTURE = 'philadelphia-renters-4';
const fixturePath = join(repo, 'fixtures', 'households', `${FIXTURE}.json`);
const golden = JSON.parse(readFileSync(join(repo, 'fixtures', 'golden', `${FIXTURE}.json`), 'utf8'));

if (!existsSync(join(dist, 'index.html'))) throw new Error(`No build in ${dist}: run npm run build first`);
if (!existsSync(join(dist, 'pkg', 'rr_wasm_bg.wasm'))) throw new Error('The build has no WebAssembly engine: run bash crates/rr-wasm/build-web.sh, then build');
mkdirSync(shots, { recursive: true });
const html = readFileSync(join(dist, 'index.html'), 'utf8');
const base = /src="(\/[^"]*?)assets\//.exec(html)?.[1] ?? '/';

// The app's own number formatting, so "what the words on screen should be" is computed the same
// way from the golden file.
const formatOut = join(web, 'node_modules', '.cache', 'e2e-format.mjs');
await build({ entryPoints: [join(web, 'src', 'lib', 'format.ts')], bundle: true, format: 'esm', platform: 'node', outfile: formatOut, logLevel: 'silent' });
const { targetDays, targetMonths } = await import(pathToFileURL(formatOut).href);
// The maps' origins, from the one module that lists them (the CSP is built from the same list).
const sourcesOut = join(web, 'node_modules', '.cache', 'e2e-map-sources.mjs');
await build({ entryPoints: [join(web, 'src', 'lib', 'maps', 'sources.ts')], bundle: true, format: 'esm', platform: 'node', outfile: sourcesOut, logLevel: 'silent' });
const { MAP_ORIGINS, RECIPIENTS } = await import(pathToFileURL(sourcesOut).href);
const overpassFixture = readFileSync(join(web, 'src', 'lib', 'maps', 'fixtures', 'overpass-philadelphia.json'));
// A 1 × 1 grey PNG: what every stubbed map service answers with (tiles and export images alike).
const STUB_PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR42mO4e/cuAAUyApj98CkjAAAAAElFTkSuQmCC', 'base64');

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `: ${detail}` : ''}`);
  if (!ok) process.exitCode = 1;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const median = (xs) => (xs.length ? [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] : NaN);
const kb = (n) => `${(n / 1000).toFixed(0)} kB`;

const site = await serve({ root: dist, base });
const browser = await puppeteer.launch({ executablePath: chromePath(), headless: true, args: ['--no-sandbox', '--font-render-hinting=none'] });
const summary = { base };

async function freshPage(context, { width = 1280, height = 900, scale = 1, dark = false } = {}) {
  const page = await context.newPage();
  const problems = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || /Content Security Policy/i.test(m.text())) problems.push(m.text());
  });
  page.on('pageerror', (e) => problems.push(e.message));
  await page.setViewport({ width, height, deviceScaleFactor: scale });
  await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: dark ? 'dark' : 'light' }]);
  await page.evaluateOnNewDocument(() => {
    document.addEventListener('securitypolicyviolation', (e) => console.error(`CSP violation: ${e.violatedDirective} ${e.blockedURI}`));
  });
  return { page, problems };
}

async function continueTo(page, heading) {
  await page.click('.interview-nav__next button');
  await page.waitForFunction((h) => document.querySelector('#page-title')?.textContent?.includes(h), { timeout: 20000 }, heading);
}

/** "Skip for now" on an optional step (6–8): on to the next without answering. */
async function skipTo(page, heading) {
  await page.click('.interview-nav__next a');
  await page.waitForFunction((h) => document.querySelector('#page-title')?.textContent?.includes(h), { timeout: 20000 }, heading);
}

try {
  // -------------------------------------------------------------------------------------------
  // 1. First visit, on a typical mobile connection (10 Mbps down, 40 ms round trip)
  // -------------------------------------------------------------------------------------------
  const context = await browser.createBrowserContext();
  const { page, problems } = await freshPage(context);
  const firstVisitRequests = [];
  page.on('request', (r) => firstVisitRequests.push(r.url()));
  const cdp = await page.createCDPSession();
  await cdp.send('Network.enable');
  const NET = { offline: false, latency: 40, downloadThroughput: 1_250_000, uploadThroughput: 500_000 };
  await cdp.send('Network.emulateNetworkConditions', NET);
  site.reset();
  const t0 = Date.now();
  await page.goto(site.url, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('#page-title');
  // The county data loads behind the start screen; the service worker is registered after it.
  await page.waitForFunction(() => performance.getEntriesByName('rr:data:core').length > 0, { timeout: 120000 });
  await page.waitForFunction(() => !!navigator.serviceWorker?.controller, { timeout: 120000 });
  await page.waitForNetworkIdle({ idleTime: 1000, timeout: 120000 });
  const wallMs = Date.now() - t0;
  const marks = await page.evaluate(() => ({
    appReady: performance.getEntriesByName('rr:app:ready')[0]?.startTime ?? NaN,
    dataCore: performance.getEntriesByName('rr:data:core')[0]?.startTime ?? NaN,
    fcp: performance.getEntriesByName('first-contentful-paint')[0]?.startTime ?? NaN,
  }));
  const first = site.sent();
  summary.firstVisit = {
    network: '10 Mbps down, 40 ms round trip (Chrome network emulation)',
    startScreenReadyMs: Math.round(marks.appReady),
    countyDataReadyMs: Math.round(marks.dataCore),
    firstContentfulPaintMs: Math.round(marks.fcp),
    offlineCopyDoneMs: wallMs,
    bytesTotal: first.total,
    files: first.files.map((f) => ({ path: f.path, bytes: f.bytes, requests: f.requests })),
  };
  check(
    'the start screen is ready before the county data is in',
    marks.appReady < marks.dataCore,
    `start screen at ${Math.round(marks.appReady)} ms, county data at ${Math.round(marks.dataCore)} ms; ${kb(first.total)} transferred in all`,
  );
  check('first visit sends no ZIP tables and no map', !first.files.some((f) => /zip_|geo\//.test(f.path)));
  check('no file is sent twice on the first visit (the offline copy reuses the browser cache)', first.files.every((f) => f.requests === 1 || f.path === 'index.html'), first.files.filter((f) => f.requests > 1).map((f) => f.path).join(', '));
  const csp = await page.$eval('meta[http-equiv="Content-Security-Policy"]', (m) => m.getAttribute('content')).catch(() => null);
  check('content security policy in force', !!csp && csp.includes("script-src 'self'"));
  const siteOrigin = new URL(site.url).origin;
  const offsite = firstVisitRequests.filter((u) => !u.startsWith('data:') && !u.startsWith('blob:') && new URL(u).origin !== siteOrigin);
  check('a first visit makes no request off the site', offsite.length === 0, offsite.slice(0, 3).join(', ') || `${firstVisitRequests.length} requests, all to ${siteOrigin}`);
  const mapCsp = MAP_ORIGINS.every((o) => csp?.includes(o));
  check('the content security policy allows exactly the map origins in sources.ts', mapCsp && (csp.match(/https:\/\//g) ?? []).length === 2 * MAP_ORIGINS.length, MAP_ORIGINS.join(' '));
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });

  // -------------------------------------------------------------------------------------------
  // 2. Import the fixture and walk the interview
  // -------------------------------------------------------------------------------------------
  site.reset();
  const input = await page.$('input[type=file]');
  const tImport = Date.now();
  await input.uploadFile(fixturePath);
  await page.waitForFunction(() => document.querySelector('#page-title')?.textContent?.includes('Where you live'), { timeout: 30000 });
  await page.waitForFunction(() => document.body.textContent.includes("That's Philadelphia"), { timeout: 30000 });
  await page.waitForSelector('.found-row .county-map__svg', { timeout: 30000 });
  check('the location screen shows the county map', true);
  await page.waitForNetworkIdle({ idleTime: 500, timeout: 30000 });
  const lazy = site.sent();
  const zipFiles = lazy.files.filter((f) => f.path.includes('zip_'));
  const mapFiles = lazy.files.filter((f) => f.path.startsWith('data/geo/'));
  summary.zipTables = { bytes: zipFiles.reduce((s, f) => s + f.bytes, 0), files: zipFiles.map((f) => f.path) };
  summary.map = { bytes: mapFiles.reduce((s, f) => s + f.bytes, 0), files: mapFiles.map((f) => f.path) };
  // The five ZIP tables: ZIP_FILES in web/src/engine/data-files.ts (zip_county, zip_facilities, zip_surge, zip_centroids, zip_wildfire_places).
  check('the imported ZIP code fetches the five ZIP tables, once each', zipFiles.length === 5 && zipFiles.every((f) => f.requests === 1), summary.zipTables.files.join(', '));
  check('the first map fetches the county outlines, once', mapFiles.length === 1 && mapFiles[0].requests === 1, `${kb(summary.map.bytes)}`);
  await continueTo(page, 'Who is in your household');
  await continueTo(page, 'How you get around');
  await continueTo(page, 'Money');
  await continueTo(page, 'What you already have');
  // Steps 6–8 are optional (v0.3.0): into the first, then skipped, as a first-time visitor may.
  await continueTo(page, 'Your people');
  await skipTo(page, 'Your places');
  await skipTo(page, 'Contacts, pets, vehicles and documents');
  await page.click('.interview-nav__next button');
  await page.waitForFunction(() => document.querySelector('#page-title')?.textContent?.includes('Your risks'), { timeout: 20000 });
  await page.waitForSelector('[data-screen-ready][aria-busy="false"] [data-bucket]', { timeout: 30000 });
  summary.importToRisksMs = Date.now() - tImport;

  // Every bucket target on screen equals the golden.
  const onScreen = await page.$$eval('#main [data-bucket]', (els) =>
    els.map((el) => ({
      id: el.getAttribute('data-bucket'),
      target: JSON.parse(el.getAttribute('data-target')),
      text: el.textContent.replace(/\s+/g, ' ').trim(),
    })),
  );
  const shown = new Map();
  for (const b of onScreen) if (!shown.has(b.id)) shown.set(b.id, b);
  const mismatches = [];
  for (const b of golden.buckets) {
    const s = shown.get(b.id);
    if (!s) {
      mismatches.push(`${b.id}: not on screen`);
      continue;
    }
    if (JSON.stringify(s.target) !== JSON.stringify(b.target)) mismatches.push(`${b.id}: ${JSON.stringify(s.target)} on screen, ${JSON.stringify(b.target)} in the golden`);
    const t = b.target;
    const words = t.kind === 'days' ? targetDays(t.value, t.low, t.high) : t.kind === 'months' ? targetMonths(t.value, t.low, t.high) : null;
    if (words && !s.text.includes(words)) mismatches.push(`${b.id}: "${words}" not in "${s.text.slice(0, 120)}"`);
  }
  check(`every bucket target on "Your risks" equals the golden (${golden.buckets.length} buckets)`, mismatches.length === 0, mismatches.join('; '));
  summary.buckets = golden.buckets.map((b) => ({ id: b.id, golden: b.target, shown: shown.get(b.id)?.target }));
  const county = await page.$eval('.county-map figcaption', (el) => el.textContent.trim()).catch(() => '');
  check('the risks screen shows the county map', county.includes('Philadelphia'), county.slice(0, 60));
  await page.screenshot({ path: join(shots, 'risks--philadelphia--desktop.png'), fullPage: true });
  const map = await page.$('.intro .county-map');
  if (map) await map.screenshot({ path: join(shots, 'county-map--philadelphia.png') });

  // -------------------------------------------------------------------------------------------
  // 3. assess timing in the browser
  // -------------------------------------------------------------------------------------------
  // The very first call warms the engine up (one time); the dial changes below are the steady state.
  const coldMs = await page.evaluate(() => performance.getEntriesByName('rr:assess:engine')[0]?.duration ?? NaN);
  await page.evaluate(() => performance.clearMeasures());
  await page.click('button[aria-controls="settings-panel"]');
  const dialOptions = await page.$$('#settings-panel input[type="radio"]');
  for (let round = 0; round < 3; round++) {
    for (const option of dialOptions.slice(0, 4)) {
      await option.click();
      await sleep(150);
      await page.waitForSelector('[data-screen-ready][aria-busy="false"]', { timeout: 20000 });
    }
  }
  const timing = await page.evaluate(() => ({
    engine: performance.getEntriesByName('rr:assess:engine').map((e) => e.duration),
    total: performance.getEntriesByName('rr:assess').map((e) => e.duration),
  }));
  summary.assessMs = {
    firstCall: +coldMs.toFixed(1),
    calls: timing.engine.length,
    engineMedian: +median(timing.engine).toFixed(1),
    engineMax: +Math.max(...timing.engine).toFixed(1),
    withJsonMedian: +median(timing.total).toFixed(1),
    withJsonMax: +Math.max(...timing.total).toFixed(1),
  };
  check(`assess under 50 ms in the browser (median of ${timing.engine.length} calls)`, median(timing.engine) < 50, `engine ${summary.assessMs.engineMedian} ms median, ${summary.assessMs.engineMax} ms max; with JSON ${summary.assessMs.withJsonMedian} ms; the first call of the visit ${summary.assessMs.firstCall} ms`);
  // The same on a CPU four times slower (Chrome's emulation of a mid-range phone): reported, not checked.
  const cpu = await page.createCDPSession();
  await cpu.send('Emulation.setCPUThrottlingRate', { rate: 4 });
  await page.evaluate(() => performance.clearMeasures());
  for (const option of dialOptions.slice(0, 4)) {
    await option.click();
    await sleep(300);
    await page.waitForSelector('[data-screen-ready][aria-busy="false"]', { timeout: 30000 });
  }
  const slow = await page.evaluate(() => performance.getEntriesByName('rr:assess:engine').map((e) => e.duration));
  await cpu.send('Emulation.setCPUThrottlingRate', { rate: 1 });
  summary.assessMs.phoneCpuMedian = +median(slow).toFixed(1);
  console.log(`INFO  assess with a 4x slower CPU: ${summary.assessMs.phoneCpuMedian} ms median of ${slow.length} calls`);
  // Back to the fixture's own setting.
  await page.click('#settings-panel input[value="one_in_100"]').catch(() => undefined);
  await page.waitForSelector('[data-screen-ready][aria-busy="false"]');

  // Prepare (was the Plan tab) and the binder (was the packet).
  await page.goto(`${site.url}#/prepare`);
  await page.waitForSelector('[data-screen-ready][aria-busy="false"]', { timeout: 30000 });
  await page.screenshot({ path: join(shots, 'plan--philadelphia--desktop.png'), fullPage: true });
  await page.goto(`${site.url}#/binder`);
  await page.waitForSelector('[data-screen-ready] article.binder-page', { timeout: 30000 });
  // Media type and colour scheme are set together: puppeteer's separate setters each reset the other.
  const media = await page.createCDPSession();
  const emulate = (type, scheme) => media.send('Emulation.setEmulatedMedia', { media: type, features: [{ name: 'prefers-color-scheme', value: scheme }] });
  await emulate('print', 'light');
  await page.screenshot({ path: join(shots, 'binder-print-view--philadelphia.png'), fullPage: true });
  await page.pdf({ path: join(shots, 'binder-print--philadelphia--letter.pdf'), format: 'Letter', printBackground: false });
  // The browser's own Print (the fallback to Download PDF) prints every binder page and none of the screen around it.
  const printed = await page.evaluate(() => ({
    pages: document.querySelectorAll('article.binder-page').length,
    hidden: ['.binder-side', '.pdf-options'].every((sel) => { const el = document.querySelector(sel); return !el || getComputedStyle(el).display === 'none'; }),
  }));
  check('the binder prints every page, without the contents or the buttons', printed.pages > 50 && printed.hidden, JSON.stringify(printed));
  // A device in dark mode still prints dark ink on white paper.
  await emulate('print', 'dark');
  const ink = await page.evaluate(() => {
    const p = document.querySelector('article.binder-page p');
    const page = document.querySelector('article.binder-page');
    return { text: p ? getComputedStyle(p).color : '', paper: page ? getComputedStyle(page).backgroundColor : '' };
  });
  check('printing from a dark-mode device gives dark text on white', ink.text === 'rgb(0, 0, 0)' && /^(rgba\(0, 0, 0, 0\)|rgb\(255, 255, 255\)|transparent)$/.test(ink.paper), JSON.stringify(ink));
  await emulate('', 'light');

  // About: versions and every credit line, the NRI statement first.
  await page.goto(`${site.url}#/about`);
  await page.waitForFunction(() => document.body.textContent.includes('The data on this device'), { timeout: 20000 });
  const credits = await page.$$eval('.credits .credit__source', (els) => els.map((e) => e.textContent.trim()));
  check('About lists the credits with the National Risk Index first', credits.length >= 5 && credits[0].startsWith('FEMA National Risk Index'), credits.slice(0, 3).join(' | '));
  const aboutText = await page.$eval('#main', (el) => el.textContent);
  check('About shows the data version', aboutText.includes(golden.data_pack_version), golden.data_pack_version);
  await page.screenshot({ path: join(shots, 'about--attributions--desktop.png'), fullPage: true });
  check('no console errors or policy violations', problems.length === 0, problems.slice(0, 3).join(' | '));

  // -------------------------------------------------------------------------------------------
  // 5. Offline
  // -------------------------------------------------------------------------------------------
  site.setOffline(true);
  await page.setOfflineMode(true);
  await page.goto(`${site.url}#/risks`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-screen-ready][aria-busy="false"] [data-bucket]', { timeout: 30000 });
  const offlinePower = await page.$eval('[data-bucket="power"]', (el) => JSON.parse(el.getAttribute('data-target')));
  check('offline after the first visit: the plan reloads, same targets', JSON.stringify(offlinePower) === JSON.stringify(golden.buckets.find((b) => b.id === 'power').target));
  await page.setOfflineMode(false);
  site.setOffline(false);
  await context.close();

  // -------------------------------------------------------------------------------------------
  // 6. More screenshots: the ambiguous-ZIP picker, a phone, dark mode, the loading line
  // -------------------------------------------------------------------------------------------
  const plan = (location) => {
    const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'));
    fixture.location = location;
    return { format: 'ready-reckoner-plan', version: 1, input: fixture, purchases: [], progress: { completed: ['where', 'who', 'travel', 'money', 'have'] }, confidence: {}, dismissed_warnings: [], done_dates: {} };
  };
  for (const [name, viewport] of [
    ['where--ambiguous-zip-picker--desktop', { width: 1280, height: 900 }],
    ['where--ambiguous-zip-picker--phone', { width: 390, height: 844, scale: 2 }],
  ]) {
    const ctx = await browser.createBrowserContext();
    const { page: p } = await freshPage(ctx, viewport);
    await p.evaluateOnNewDocument((saved) => localStorage.setItem('rr.plan.v1', JSON.stringify(saved)), plan({ country: 'US', zip: '19087', setting: 'suburban' }));
    await p.goto(`${site.url}#/where`);
    await p.waitForSelector('.pick .county-map__svg', { timeout: 60000 });
    const picks = await p.$$eval('.pick label.choice', (els) => els.map((e) => e.textContent.replace(/\s+/g, ' ').trim()));
    if (name.endsWith('desktop')) check('an ambiguous ZIP code offers each county with its share', picks.length === 3 && picks[0].includes('50%'), picks.join(' | '));
    await sleep(300);
    await p.screenshot({ path: join(shots, `${name}.png`), fullPage: true });
    await ctx.close();
  }
  {
    const ctx = await browser.createBrowserContext();
    const { page: p } = await freshPage(ctx, { width: 1280, height: 900, dark: true });
    await p.evaluateOnNewDocument((saved) => localStorage.setItem('rr.plan.v1', JSON.stringify(saved)), plan(JSON.parse(readFileSync(fixturePath, 'utf8')).location));
    await p.goto(`${site.url}#/risks`);
    await p.waitForSelector('[data-screen-ready][aria-busy="false"] .county-map__svg', { timeout: 60000 });
    await p.screenshot({ path: join(shots, 'risks--philadelphia--dark.png') });
    await ctx.close();
  }
  {
    // The loading line, on a slow connection (about 1.6 Mbps, 150 ms latency).
    const ctx = await browser.createBrowserContext();
    const { page: p } = await freshPage(ctx);
    const cdp = await p.createCDPSession();
    await cdp.send('Network.enable');
    await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 150, downloadThroughput: 200_000, uploadThroughput: 100_000 });
    await p.goto(site.url, { waitUntil: 'domcontentloaded' });
    await p.waitForSelector('.data-progress progress', { timeout: 60000 });
    await sleep(2500);
    await p.screenshot({ path: join(shots, 'loading-progress-line--desktop.png') });
    const line = await p.$eval('.data-progress', (el) => el.textContent.replace(/\s+/g, ' ').trim());
    check('a calm progress line shows while the county data loads', /Getting the county data ready/.test(line), line.slice(0, 90));
    await ctx.close();
  }

  // -------------------------------------------------------------------------------------------
  // 7. Maps behind consent, every outside service stubbed
  // -------------------------------------------------------------------------------------------
  {
    const ctx = await browser.createBrowserContext();
    const { page: p, problems: mapProblems } = await freshPage(ctx);
    const siteOrigin = new URL(site.url).origin;
    const external = [];
    await p.setRequestInterception(true);
    p.on('request', (req) => {
      const url = req.url();
      if (url.startsWith('data:') || url.startsWith('blob:') || new URL(url).origin === siteOrigin) {
        req.continue();
        return;
      }
      external.push({ url, referer: req.headers().referer ?? '', method: req.method() });
      const cors = { 'access-control-allow-origin': '*' };
      if (url.includes('overpass')) req.respond({ status: 200, headers: cors, contentType: 'application/json', body: overpassFixture });
      else if (MAP_ORIGINS.includes(new URL(url).origin)) req.respond({ status: 200, headers: cors, contentType: 'image/png', body: STUB_PNG });
      else req.respond({ status: 404, headers: cors, body: '' });
    });
    await p.evaluateOnNewDocument((saved) => localStorage.setItem('rr.plan.v1', JSON.stringify(saved)), {
      format: 'ready-reckoner-plan',
      version: 1,
      input: JSON.parse(readFileSync(fixturePath, 'utf8')),
      purchases: [],
      progress: { completed: ['where', 'who', 'travel', 'money', 'have'] },
      confidence: {},
      dismissed_warnings: [],
      done_dates: {},
    });
    await p.goto(`${site.url}#/binder`);
    await p.waitForSelector('[data-screen-ready][aria-busy="false"] article.binder-page', { timeout: 60000 });
    await p.waitForFunction(() => !!navigator.serviceWorker?.controller, { timeout: 60000 });
    const clickButton = (text) => p.evaluate((t) => {
      const b = [...document.querySelectorAll('button')].find((x) => x.textContent.trim() === t);
      if (!b) throw new Error(`no button "${t}"`);
      b.click();
    }, text);
    await clickButton('Add maps');
    await p.waitForFunction(() => document.body.textContent.includes('Before we fetch your maps'), { timeout: 20000 });
    const consentText = await p.$eval('.consent', (el) => el.textContent.replace(/\s+/g, ' '));
    const unnamed = RECIPIENTS.filter((r) => !consentText.includes(r.who) || !consentText.includes(r.layer));
    check('the consent screen names every recipient in sources.ts and what each receives', unnamed.length === 0 && consentText.includes('Nothing has been sent yet'), unnamed.map((r) => r.id).join(', ') || RECIPIENTS.map((r) => r.layer.split(' (')[0]).join(', '));
    await p.screenshot({ path: join(shots, 'maps-consent--philadelphia--desktop.png'), fullPage: false });
    const consent = await p.$('.consent');
    if (consent) await consent.screenshot({ path: join(shots, 'maps-consent--screen.png') });
    await clickButton('Not now');
    await sleep(300);
    check('opening the maps panel makes no request off the site until "Fetch maps"', external.length === 0, external.slice(0, 3).map((e) => e.url).join(', '));
    await clickButton('Add maps');
    await p.waitForFunction(() => document.body.textContent.includes('Before we fetch your maps'), { timeout: 20000 });
    check('the consent screen appears again on the next press (nothing remembered)', external.length === 0);
    await clickButton('Fetch maps');
    await p.waitForSelector('.pin-map .leaflet-container', { timeout: 30000 });
    await p.waitForFunction(() => document.querySelectorAll('.leaflet-tile-loaded').length > 0, { timeout: 30000 });
    const mapBox = await (await p.$('.pin-map__map')).boundingBox();
    await p.mouse.click(mapBox.x + mapBox.width / 2, mapBox.y + mapBox.height / 2);
    await p.waitForFunction(() => document.querySelector('.pin-map')?.textContent.includes('Home: placed'), { timeout: 10000 });
    const pinMap = await p.$('.pin-map');
    if (pinMap) await pinMap.screenshot({ path: join(shots, 'maps-pin-map--stubbed-tiles.png') });
    await clickButton('Done');
    await p.waitForSelector('.maps-panel[data-maps-phase="idle"] .map-figure__img', { timeout: 60000 });
    const figures = await p.$$eval('.maps-panel .map-figure__img', (els) => els.map((e) => e.getAttribute('src').length));
    check('after consent, the three maps are made and shown', figures.length === 3, `${figures.length} images`);
    const offList = external.filter((e) => !MAP_ORIGINS.includes(new URL(e.url).origin));
    check('after consent, requests go only to the origins in sources.ts', offList.length === 0, offList.slice(0, 3).map((e) => e.url).join(', ') || `${external.length} requests to ${[...new Set(external.map((e) => new URL(e.url).origin))].join(', ')}`);
    const tiles = external.filter((e) => e.url.startsWith('https://tile.openstreetmap.org/'));
    // The OSMF and Overpass rules ask for a Referer; it must be the site's address alone, never the page or its #/ route.
    const badReferer = external.filter((e) => e.referer !== `${siteOrigin}/`);
    check('every map request names the site by its address alone (the Referer the OSMF and Overpass rules ask for)', badReferer.length === 0, badReferer.slice(0, 2).map((e) => `${e.url.slice(0, 50)} -> "${e.referer}"`).join(', ') || `${external.length} requests, Referer "${external[0]?.referer ?? ''}"`);
    const pressTiles = Number(await p.$eval('[data-maps-tiles]', (el) => el.getAttribute('data-maps-tiles')));
    check('one press asks for at most 250 tiles (§9.3)', pressTiles > 0 && pressTiles <= 250, `${pressTiles} tiles for the three maps, ${tiles.length - pressTiles} for the pin map`);
    summary.maps = {
      externalRequests: external.length,
      byOrigin: Object.fromEntries([...new Set(external.map((e) => new URL(e.url).origin))].map((o) => [o, external.filter((e) => new URL(e.url).origin === o).length])),
      composerTiles: pressTiles,
      pinMapTiles: tiles.length - pressTiles,
      imageDataUrlChars: figures,
    };
    const panel = await p.$('.maps-panel');
    if (panel) await panel.screenshot({ path: join(shots, 'maps-panel--stubbed-services.png') });
    // The images live in IndexedDB (rr-maps), one record per map, never in the saved plan.
    const stored = await p.evaluate(
      () =>
        new Promise((resolve) => {
          const req = indexedDB.open('rr-maps');
          req.onsuccess = () => {
            const db = req.result;
            const all = db.transaction('maps', 'readonly').objectStore('maps').getAll();
            all.onsuccess = () => {
              resolve(all.result.map((r) => ({ slot: r.slot, jpeg: r.image.startsWith('data:image/jpeg;base64,') })));
              db.close();
            };
          };
          req.onerror = () => resolve([]);
        }),
    );
    const savedPlan = await p.evaluate(() => localStorage.getItem('rr.plan.v1') ?? '');
    check('the three maps are kept in the IndexedDB database rr-maps, and the saved plan holds pins, never images', stored.length === 3 && stored.every((r) => r.jpeg) && savedPlan.includes('"maps"') && !savedPlan.includes('data:image'), stored.map((r) => r.slot).join(', '));
    check('no console errors or policy violations with the maps', mapProblems.length === 0, mapProblems.slice(0, 3).join(' | '));

    // Offline: the panel and the pin-map chunk still open (from the service worker); it says maps need a connection.
    external.length = 0;
    await p.setOfflineMode(true);
    site.setOffline(true);
    await p.goto(`${site.url}#/binder`, { waitUntil: 'domcontentloaded' });
    await p.waitForSelector('[data-screen-ready][aria-busy="false"] article.binder-page', { timeout: 60000 });
    // The saved plan keeps its maps across a reload, so the toolbar offers "Refresh maps" (either label is accepted).
    const toolbarLabel = await p.evaluate(() => [...document.querySelectorAll('.pdf-options button')].map((b) => b.textContent.trim()).find((t) => t === 'Add maps' || t === 'Refresh maps'));
    await clickButton(toolbarLabel);
    await p.waitForFunction(() => document.body.textContent.includes('Before we fetch your maps'), { timeout: 20000 });
    const offlineNote = await p.evaluate(() => document.querySelector('.consent')?.textContent.includes('seems to be offline') ?? false);
    await clickButton('Not now');
    const canEdit = await p.evaluate(() => [...document.querySelectorAll('.maps-panel button')].some((b) => b.textContent.trim() === 'Edit pins'));
    await clickButton(canEdit ? 'Edit pins' : toolbarLabel);
    await p.waitForFunction(() => document.body.textContent.includes('Before we fetch your maps'), { timeout: 20000 });
    await clickButton('Fetch maps');
    const chunk = await p.waitForSelector('.pin-map .leaflet-container', { timeout: 30000 }).then(() => true).catch(() => false);
    check('offline, the maps panel and the pin-map chunk still open, and the consent screen says maps need a connection', offlineNote && chunk, `offline note ${offlineNote}, pin map ${chunk}`);
    await p.setOfflineMode(false);
    site.setOffline(false);

    // "Forget everything" deletes the maps store with the rest.
    await p.goto(`${site.url}#/maintain`);
    await p.waitForFunction(() => [...document.querySelectorAll('button')].some((b) => b.textContent.trim() === 'Forget everything'), { timeout: 30000 });
    await clickButton('Forget everything');
    await p.waitForSelector('dialog[open]', { timeout: 10000 });
    await p.evaluate(() => [...document.querySelectorAll('dialog[open] button')].find((b) => b.textContent.trim() === 'Forget everything').click());
    await sleep(500);
    const afterForget = await p.evaluate(async () => ({ dbs: (await indexedDB.databases()).map((d) => d.name), plan: localStorage.getItem('rr.plan.v1') }));
    check('"Forget everything" deletes the maps store rr-maps with the plan', !afterForget.dbs.includes('rr-maps') && afterForget.plan === null, `databases left: ${afterForget.dbs.join(', ') || 'none'}`);
    await ctx.close();
  }
} finally {
  await browser.close();
  await site.close();
}

summary.checks = results;
const passed = results.filter((r) => r.ok).length;
console.log(`\n${passed} of ${results.length} checks passed. Screenshots in ${shots}`);
writeFileSync(join(shots, 'e2e-summary.json'), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ firstVisitBytes: summary.firstVisit?.bytesTotal, zipTablesBytes: summary.zipTables?.bytes, mapBytes: summary.map?.bytes, assessMs: summary.assessMs, importToRisksMs: summary.importToRisksMs }));
