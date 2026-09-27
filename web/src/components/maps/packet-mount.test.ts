import { IDBFactory } from 'fake-indexeddb';
import { flushSync, tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { FIXTURES } from '../../engine/fixtures';
import Packet from '../../screens/Packet.svelte';
import { render, savedFor, until, type Rendered } from '../../test/helpers';

let current: Rendered | null = null;
const originalIdb = globalThis.indexedDB;

beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
});

afterEach(() => {
  current?.cleanup();
  current = null;
  globalThis.indexedDB = originalIdb;
  vi.restoreAllMocks();
});

const click = (r: Rendered, label: string) => {
  const b = [...r.target.querySelectorAll('button')].find((x) => x.textContent?.trim() === label);
  if (!b) throw new Error(`no button "${label}"`);
  b.click();
  flushSync();
};

describe('The maps mount on the Packet screen (web-maps; web-binder moves it)', () => {
  it('adds an "Add maps" button; the panel opens at the consent screen, and nothing is fetched until "Fetch maps"', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    current = await render(Packet, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'packet' });
    await tick();
    flushSync();
    expect(current.target.querySelector('.maps-panel')).toBeNull();
    click(current, 'Add maps');
    await until(() => current!.text().includes('Before we fetch your maps'), 'the consent screen');
    click(current, 'Not now');
    expect(current.text()).not.toContain('Before we fetch your maps');
    // Each toolbar press opens the consent screen afresh.
    click(current, 'Add maps');
    await until(() => current!.text().includes('Before we fetch your maps'), 'the consent screen again');
    // The only request is the packet's own county outline, from the site itself: nothing leaves it.
    const urls = fetchSpy.mock.calls.map(([u]) => new URL(String(u instanceof Request ? u.url : u), window.location.href));
    expect(urls.map((u) => u.pathname)).toEqual(['/data/geo/counties.json']);
    expect(urls.every((u) => u.origin === window.location.origin)).toBe(true);
    // The maps panel is not part of the printed v2 packet.
    expect(current.target.querySelector('.packet__maps')?.classList.contains('no-print')).toBe(true);
    expect(current.target.querySelector('.packet .maps-panel')).toBeNull();
  });
});
