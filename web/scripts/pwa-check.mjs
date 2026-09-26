// Checks the built site the way GitHub Pages serves it, in the system Chrome:
//   1. under the /ready-reckoner/ base path, with the Content Security Policy in force: the app
//      loads, renders a plan, and nothing is blocked or logged as an error;
//   2. the service worker takes control, and the app reloads and works with the network off;
//   3. a new version of the service worker is offered with "Reload to update", and taking it
//      reloads onto the new version with the plan intact.
//
//   BASE_PATH=/ready-reckoner/ npx vite build --outDir <dir> && DIST=<dir> node scripts/pwa-check.mjs
import { createServer } from 'node:http';
import { appendFileSync, cpSync, existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import puppeteer from 'puppeteer-core';

import { chromePath } from './chrome.mjs';

const here = fileURLToPath(new URL('.', import.meta.url));
const source = process.env.DIST ?? join(here, '..', 'dist');
const BASE = '/ready-reckoner/';
if (!existsSync(join(source, 'index.html'))) throw new Error(`No build in ${source}`);
if (!readFileSync(join(source, 'index.html'), 'utf8').includes(`${BASE}assets/`)) {
  throw new Error(`The build in ${source} was not made with BASE_PATH=${BASE}`);
}

// Serve a copy, so step 3 can publish a changed service worker without touching the build.
const root = mkdtempSync(join(process.env.COPY_PARENT ?? tmpdir(), 'rr-pwa-'));
cpSync(source, root, { recursive: true });

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.json': 'application/json', '.webmanifest': 'application/manifest+json' };
const server = createServer((req, res) => {
  const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  if (!path.startsWith(BASE)) {
    res.writeHead(404).end();
    return;
  }
  let file = join(root, path.slice(BASE.length) || 'index.html');
  if (!existsSync(file) || !extname(file)) file = join(root, 'index.html');
  res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream', 'cache-control': 'no-cache' });
  res.end(readFileSync(file));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const site = `${origin}${BASE}`;

const philly = JSON.parse(readFileSync(join(here, '..', '..', 'fixtures', 'households', 'philadelphia-renters-4.json'), 'utf8'));
const plan = { format: 'ready-reckoner-plan', version: 1, input: philly, purchases: [], progress: { completed: ['where', 'who', 'travel', 'money', 'have'] }, confidence: {}, dismissed_warnings: [], done_dates: {} };

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `: ${detail}` : ''}`);
}

const browser = await puppeteer.launch({ executablePath: chromePath(), headless: true, args: ['--no-sandbox'] });
try {
  const page = await browser.newPage();
  const problems = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || /Content Security Policy/i.test(m.text())) problems.push(m.text());
  });
  page.on('pageerror', (e) => problems.push(e.message));
  await page.evaluateOnNewDocument((saved) => {
    document.addEventListener('securitypolicyviolation', (e) => console.error(`CSP violation: ${e.violatedDirective} ${e.blockedURI}`));
    if (!localStorage.getItem('rr.plan.v1')) localStorage.setItem('rr.plan.v1', JSON.stringify(saved));
  }, plan);

  // 1. Loads under the base path, with the policy in force.
  await page.goto(`${site}#/risks`, { waitUntil: 'networkidle0' });
  await page.waitForSelector('[data-screen-ready]', { timeout: 10000 });
  const csp = await page.$eval('meta[http-equiv="Content-Security-Policy"]', (m) => m.getAttribute('content'));
  check('Content Security Policy is present', !!csp && csp.includes("script-src 'self'"), csp ?? 'missing');
  check('risks screen renders under the base path', (await page.$eval('h1', (h) => h.textContent)) === 'Your risks');
  check('no console errors or policy violations', problems.length === 0, problems.join(' | '));

  // 2. Service worker in control; works offline.
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await page.reload({ waitUntil: 'networkidle0' });
  const controlled = await page.evaluate(() => !!navigator.serviceWorker.controller);
  check('service worker controls the page', controlled);
  const scope = await page.evaluate(async () => (await navigator.serviceWorker.getRegistration())?.scope);
  check('service worker scope is the base path', scope === site, scope);
  const caches = await page.evaluate(async () => (await self.caches.keys()).join(','));
  check('app shell is precached', /rr-precache-/.test(caches), caches);

  const cdp = await page.createCDPSession();
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-screen-ready]', { timeout: 10000 });
  check('reloads and plans with the network off', (await page.$eval('h1', (h) => h.textContent)) === 'Your risks');
  await page.goto(`${site}#/packet`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.packet h2', { timeout: 10000 });
  check('packet renders offline', true);
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });

  // 3. A new version is offered, and taken only when the person chooses.
  appendFileSync(join(root, 'sw.js'), '\n// next version\n');
  await page.evaluate(async () => {
    const reg = await navigator.serviceWorker.getRegistration();
    await reg?.update();
  });
  await page.waitForFunction(() => document.body.textContent?.includes('Reload to update'), { timeout: 15000 });
  check('"Reload to update" appears when a new version is ready', true);
  const reloaded = page.waitForNavigation({ waitUntil: 'domcontentloaded', timeout: 15000 });
  await page.evaluate(() => {
    const button = [...document.querySelectorAll('button')].find((b) => b.textContent?.includes('Reload to update'));
    button?.click();
  });
  await reloaded;
  await page.waitForSelector('#page-title', { timeout: 10000 });
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem('rr.plan.v1') ?? '{}').input?.location?.zip);
  check('reloads onto the new version with the plan kept', saved === '19147', saved);
  check('no errors during the whole run', problems.length === 0, problems.join(' | '));
} finally {
  await browser.close();
  server.close();
  rmSync(root, { recursive: true, force: true });
}

const failed = results.filter((r) => !r.ok);
console.log(`\n${results.length - failed.length} of ${results.length} checks passed.`);
process.exit(failed.length ? 1 : 0);
