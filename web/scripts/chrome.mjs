// Finds the system Chrome or Chromium for puppeteer-core (which never downloads a browser).
import { existsSync } from 'node:fs';

const CANDIDATES = [
  process.env.CHROME_PATH,
  '/usr/bin/google-chrome',
  '/usr/bin/google-chrome-stable',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
];

export function chromePath() {
  const found = CANDIDATES.find((p) => p && existsSync(p));
  if (!found) throw new Error('No Chrome or Chromium found. Set CHROME_PATH to its executable.');
  return found;
}
