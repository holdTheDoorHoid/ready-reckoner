/**
 * The county hospital list on the Binder screen (verify3 R4-05). The list comes with its own small
 * pack, downloaded the first time the binder is shown. When it could not be had (the binder first
 * opened offline, or a site without the hospital file), the Neighborhood page says so in one
 * sentence, on screen only, and so does the message after "Download PDF"; once the list is in,
 * neither does. The PDF itself is not drawn here: the PDF chunk is replaced by a stand-in.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { PackLoader } from '../engine/loader';
import { createMockEngine } from '../engine/mock';
import { HOSPITAL_LIST_MISSING, pageDomId } from '../lib/binder/model';
import { render, savedFor, until, type Rendered } from '../test/helpers';
import Binder from './Binder.svelte';

vi.mock('../lib/binder/pdf/browser', () => ({
  makeBinderPdf: async () => ({ blob: new Blob(['%PDF-1.7']), pages: 88, missing: [] }),
}));

/** A site whose data has the hospital file or not, and whose network may be down for it. */
function site({ places, offline = false }: { places: boolean; offline?: boolean }) {
  const manifest = {
    pack_version: 'v1',
    packs: {
      core: { files: [{ path: 'core/counties.csv', bytes: 10 }] },
      ...(places ? { places: { files: [{ path: 'places/hospitals.csv', bytes: 10 }] } } : {}),
    },
  };
  return (url: string): Promise<Response> => {
    if (url.includes('manifest.json')) return Promise.resolve(new Response(JSON.stringify(manifest), { headers: { 'content-type': 'application/json' } }));
    if (offline && url.includes('places/')) return Promise.reject(new TypeError('Failed to fetch'));
    return Promise.resolve(new Response('a\nb\n'));
  };
}

let r: Rendered | null = null;
afterEach(() => {
  r?.cleanup();
  r = null;
  vi.restoreAllMocks();
});

/** The Binder screen for Philadelphia, once the hospital pack has loaded or failed. */
async function open(fetch: (url: string) => Promise<Response>): Promise<Rendered> {
  const engine = createMockEngine();
  const loader = new PackLoader(engine, '/s/', { fetch });
  await loader.core();
  const opened = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder', engine, loader });
  await until(() => ['ready', 'failed'].includes(loader.status.places.phase) && opened.app.manifest !== null, 'the hospital pack to settle');
  return opened;
}

/** Press "Download PDF" and return the message the screen shows when it is done. */
async function downloadPdf(opened: Rendered): Promise<string> {
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
  const button = [...opened.target.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Download PDF') as HTMLButtonElement;
  button.click();
  const status = () => opened.target.querySelector('#pdf-status')?.textContent ?? '';
  await until(() => /^Saved|could not be made/.test(status()), 'the PDF message');
  return status();
}

const neighbourhood = (opened: Rendered) => opened.target.querySelector(`#${pageDomId('neighbourhood')}`)?.textContent ?? '';
const count = (text: string, part: string) => text.split(part).length - 1;

describe('the county hospital list on the Binder screen', () => {
  it('first opened offline: the Neighborhood page and the PDF message say it has not been downloaded', async () => {
    r = await open(site({ places: true, offline: true }));
    await until(() => neighbourhood(r!).includes(HOSPITAL_LIST_MISSING), 'the line on the Neighborhood page');
    // Said once, on that page, as a line that does not print (on paper it would be an instruction for the app).
    expect(count(r.text(), HOSPITAL_LIST_MISSING)).toBe(1);
    const line = r.target.querySelector('.binder-hospitals-missing');
    expect(line?.classList.contains('no-print')).toBe(true);
    expect(line?.closest('article')?.getAttribute('data-page')).toBe('neighbourhood');
    const message = await downloadPdf(r);
    expect(message).toContain('Saved ready-reckoner-binder-');
    expect(message).toContain(HOSPITAL_LIST_MISSING);
  });

  it('a site without the hospital file says the same', async () => {
    r = await open(site({ places: false }));
    await until(() => neighbourhood(r!).includes(HOSPITAL_LIST_MISSING), 'the line on the Neighborhood page');
    expect(await downloadPdf(r)).toContain(HOSPITAL_LIST_MISSING);
  });

  it('once the list is in, neither the page nor the PDF message mentions it', async () => {
    r = await open(site({ places: true }));
    expect(r.text()).not.toContain(HOSPITAL_LIST_MISSING);
    expect(r.target.querySelector('.binder-hospitals-missing')).toBeNull();
    const message = await downloadPdf(r);
    expect(message).toContain('Saved ready-reckoner-binder-');
    expect(message).not.toContain(HOSPITAL_LIST_MISSING);
  });

  it('the stand-in engine, with no data to load, never says it', async () => {
    r = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder' });
    expect(r.text()).not.toContain(HOSPITAL_LIST_MISSING);
  });
});
