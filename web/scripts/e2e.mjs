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
//     interview steps with Continue; on "Your risks" every bucket target (the data on each card
//     and the words on screen) equals fixtures/golden/philadelphia-renters-4.json.
//  3. In-browser timing of `assess` (performance.now() around the engine call, recorded by the
//     app as the performance measures rr:assess:engine and rr:assess), over several dial changes.
//  4. The ZIP tables and the map are fetched only when needed, and counted separately.
//  5. Offline: with the network gone, a reload still shows the same plan.
//  6. Screenshots (risks, plan, packet print view, About with the credits, the ambiguous-ZIP
//     picker, the county map, the loading line) into SHOTS_DIR.
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

try {
  // -------------------------------------------------------------------------------------------
  // 1. First visit, on a typical mobile connection (10 Mbps down, 40 ms round trip)
  // -------------------------------------------------------------------------------------------
  const context = await browser.createBrowserContext();
  const { page, problems } = await freshPage(context);
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
  check('the imported ZIP code fetches the ZIP tables, once', zipFiles.length === 3 && zipFiles.every((f) => f.requests === 1), summary.zipTables.files.join(', '));
  check('the first map fetches the county outlines, once', mapFiles.length === 1 && mapFiles[0].requests === 1, `${kb(summary.map.bytes)}`);
  await continueTo(page, 'Who is in your household');
  await continueTo(page, 'How you get around');
  await continueTo(page, 'Money');
  await continueTo(page, 'What you already have');
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

  // Plan and packet.
  await page.goto(`${site.url}#/plan`);
  await page.waitForSelector('[data-screen-ready][aria-busy="false"]', { timeout: 30000 });
  await page.screenshot({ path: join(shots, 'plan--philadelphia--desktop.png'), fullPage: true });
  await page.goto(`${site.url}#/packet`);
  await page.waitForSelector('[data-screen-ready] .packet .county-map__svg', { timeout: 30000 });
  // Media type and colour scheme are set together: puppeteer's separate setters each reset the other.
  const media = await page.createCDPSession();
  const emulate = (type, scheme) => media.send('Emulation.setEmulatedMedia', { media: type, features: [{ name: 'prefers-color-scheme', value: scheme }] });
  await emulate('print', 'light');
  await page.screenshot({ path: join(shots, 'packet-print-view--philadelphia.png'), fullPage: true });
  await page.pdf({ path: join(shots, 'packet--philadelphia--letter.pdf'), format: 'Letter', printBackground: false });
  check('the packet shows the county map in its print view', !!(await page.$('.packet .county-map__svg')));
  // A device in dark mode still prints dark ink on white paper.
  await emulate('print', 'dark');
  const ink = await page.evaluate(() => {
    const p = document.querySelector('.packet p');
    const land = document.querySelector('.packet .county--context');
    return { text: p ? getComputedStyle(p).color : '', land: land ? getComputedStyle(land).fill : '' };
  });
  check('printing from a dark-mode device gives dark text and a white map', ink.text === 'rgb(0, 0, 0)' && ink.land === 'rgb(255, 255, 255)', JSON.stringify(ink));
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
} finally {
  await browser.close();
  await site.close();
}

summary.checks = results;
const passed = results.filter((r) => r.ok).length;
console.log(`\n${passed} of ${results.length} checks passed. Screenshots in ${shots}`);
writeFileSync(join(shots, 'e2e-summary.json'), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ firstVisitBytes: summary.firstVisit?.bytesTotal, zipTablesBytes: summary.zipTables?.bytes, mapBytes: summary.map?.bytes, assessMs: summary.assessMs, importToRisksMs: summary.importToRisksMs }));
