import { describe, expect, it } from 'vitest';

import type { RawEngine } from './index';
import { FIXTURE_NAMES, FIXTURES } from './fixtures';
import { createMockEngine, mockDefaults } from './mock';
import { adaptRawEngine } from './wasm';

/**
 * A string-level engine shaped exactly like the rr-wasm exports (JSON in, JSON envelope out),
 * answering from a table of prepared replies, so the adapter can be tested before the
 * WebAssembly build exists.
 */
function rawEngine(replies: Map<string, string>): RawEngine {
  const reply = (f: string, arg = '') => replies.get(`${f}:${arg}`) ?? '';
  return {
    engine_info: () => reply('engine_info'),
    load_pack: (name) => reply('load_pack', name),
    county_search: (query) => reply('county_search', query),
    resolve_location: (json) => reply('resolve_location', json),
    assess: (json) => reply('assess', json),
    explain: (json) => reply('explain', json),
    catalogue: () => reply('catalogue'),
    defaults: () => reply('defaults'),
  };
}

describe('the WebAssembly adapter', () => {
  it('passes JSON through unchanged, so the wasm and mock engines are interchangeable', async () => {
    const mock = createMockEngine();
    const replies = new Map<string, string>();
    for (const name of FIXTURE_NAMES) {
      replies.set(`assess:${JSON.stringify(FIXTURES[name])}`, JSON.stringify(await mock.assess(FIXTURES[name])));
    }
    replies.set('catalogue:', JSON.stringify(await mock.catalogue()));
    replies.set('defaults:', JSON.stringify(await mock.defaults()));
    const adapted = adaptRawEngine(rawEngine(replies));
    for (const name of FIXTURE_NAMES) {
      expect(await adapted.assess(FIXTURES[name]), name).toEqual(await mock.assess(FIXTURES[name]));
    }
    expect(await adapted.catalogue()).toEqual(await mock.catalogue());
    const d = await adapted.defaults();
    expect(d.ok && d.value).toEqual(mockDefaults());
  });

  it('turns unreadable output, or a trap, into an "internal" error instead of throwing', async () => {
    const broken: RawEngine = {
      ...rawEngine(new Map([['engine_info:', 'not json']])),
      load_pack: () => {
        throw new Error('unreachable executed');
      },
    };
    const adapted = adaptRawEngine(broken);
    const info = await adapted.engine_info();
    expect(!info.ok && info.error.code).toBe('internal');
    const pack = await adapted.load_pack('core', new Uint8Array([1]));
    expect(!pack.ok && pack.error.code).toBe('internal');
    expect((await adapted.assess(FIXTURES['philadelphia-renters-4'])).ok).toBe(false);
  });
});
