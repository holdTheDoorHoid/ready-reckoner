// Prints the Philadelphia packet from the built site to PDF (US Letter and A4) in headless Chrome
// and reports the page counts, so a change to the print stylesheet can be measured.
//
//   npm run build && node scripts/packet-pages.mjs [out-dir]
//
// The fixture household goes into the page's storage before it opens, as if typed in. Needs
// `pdfinfo` (poppler-utils) for the count; the PDFs are written either way.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import puppeteer from 'puppeteer-core';

import { chromePath } from './chrome.mjs';
import { serve } from './serve.mjs';

const here = fileURLToPath(new URL('.', import.meta.url));
const dist = join(here, '..', 'dist');
const outDir = process.argv[2] ?? join(here, '..', 'node_modules', '.cache', 'packet');
const fixture = process.env.FIXTURE ?? 'philadelphia-renters-4';
if (!existsSync(join(dist, 'index.html'))) throw new Error('Build first: npm run build');
mkdirSync(outDir, { recursive: true });

const input = JSON.parse(readFileSync(join(here, '..', '..', 'fixtures', 'households', `${fixture}.json`), 'utf8'));
const plan = { format: 'ready-reckoner-plan', version: 1, input, purchases: [], progress: { completed: ['where', 'who', 'travel', 'money', 'have'] }, confidence: {}, dismissed_warnings: [], done_dates: {} };

export function pageCount(pdf) {
  try {
    const info = execFileSync('pdfinfo', [pdf]).toString();
    return Number(/^Pages:\s+(\d+)/m.exec(info)?.[1]);
  } catch {
    return NaN;
  }
}

// Serve under the base path the build was made for (BASE_PATH=/ready-reckoner/ for Pages).
const base = /src="(\/[^"]*?)assets\//.exec(readFileSync(join(dist, 'index.html'), 'utf8'))?.[1] ?? '/';
const site = await serve({ root: dist, base });
const browser = await puppeteer.launch({ executablePath: chromePath(), headless: true, args: ['--no-sandbox', '--font-render-hinting=none'] });
try {
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 900 });
  await page.evaluateOnNewDocument((p) => localStorage.setItem('rr.plan.v1', JSON.stringify(p)), plan);
  await page.goto(`${site.url}#/packet`, { waitUntil: 'networkidle0' });
  await page.waitForSelector('[data-screen-ready] .packet', { timeout: 60000 });
  const words = await page.$eval('.packet', (el) => el.innerText.split(/\s+/).filter(Boolean).length);
  const result = { fixture, words };
  for (const format of ['Letter', 'A4']) {
    const file = join(outDir, `${fixture}--${format.toLowerCase()}.pdf`);
    await page.pdf({ path: file, format, printBackground: false });
    result[format] = pageCount(file);
  }
  console.log(JSON.stringify(result));
  console.log(`PDFs in ${outDir}`);
} finally {
  await browser.close();
  await site.close();
}
