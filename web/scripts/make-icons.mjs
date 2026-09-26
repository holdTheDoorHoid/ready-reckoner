// Renders the app icon (the house-and-check mark) to the PNG sizes the manifest and browsers use.
// Run with `npm run icons` after changing the mark; the PNGs are committed (a few KB each).
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import puppeteer from 'puppeteer-core';

import { chromePath } from './chrome.mjs';

const out = fileURLToPath(new URL('../public/icons/', import.meta.url));
const TEAL = '#0e5c69';
const INK = '#ffffff';

// The mark on a 32-unit grid; `pad` shrinks it inside the tile (maskable icons need a safe zone).
function svg({ rounded, pad }) {
  const s = 32 - pad * 2;
  const k = s / 32;
  const t = (x) => (pad + x * k).toFixed(3);
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <rect width="32" height="32" rx="${rounded ? 7 : 0}" fill="${TEAL}"/>
  <path d="M${t(8)} ${t(16.5)} L${t(16)} ${t(9.5)} L${t(24)} ${t(16.5)} M${t(10.5)} ${t(14.5)} V${t(23.5)} H${t(21.5)} V${t(14.5)}" fill="none" stroke="${INK}" stroke-width="${(2.2 * k).toFixed(3)}" stroke-linejoin="round" stroke-linecap="round"/>
  <path d="M${t(13)} ${t(18.5)} l${(2.2 * k).toFixed(3)} ${(2.2 * k).toFixed(3)} L${t(19.5)} ${t(16)}" fill="none" stroke="${INK}" stroke-width="${(2.2 * k).toFixed(3)}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>`;
}

writeFileSync(`${out}favicon.svg`, `${svg({ rounded: true, pad: 0 })}\n`);

const targets = [
  { file: 'icon-32.png', size: 32, rounded: true, pad: 0 },
  { file: 'apple-touch-icon.png', size: 180, rounded: false, pad: 2 },
  { file: 'icon-192.png', size: 192, rounded: true, pad: 0 },
  { file: 'icon-512.png', size: 512, rounded: true, pad: 0 },
  { file: 'icon-maskable-512.png', size: 512, rounded: false, pad: 4 },
];

const browser = await puppeteer.launch({ executablePath: chromePath(), headless: true, args: ['--no-sandbox'] });
try {
  const page = await browser.newPage();
  for (const t of targets) {
    await page.setViewport({ width: t.size, height: t.size, deviceScaleFactor: 1 });
    await page.setContent(`<html><body style="margin:0;background:transparent">${svg(t).replace('<svg ', `<svg width="${t.size}" height="${t.size}" `)}</body></html>`);
    await page.screenshot({ path: `${out}${t.file}`, omitBackground: true, clip: { x: 0, y: 0, width: t.size, height: t.size } });
    console.log(`wrote public/icons/${t.file}`);
  }
} finally {
  await browser.close();
}
