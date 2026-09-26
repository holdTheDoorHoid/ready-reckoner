import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import type { Catalogue } from '../engine/types';
import { savedFor } from '../test/helpers';
import { calendarFile, drillItems, maintenanceTasks, nextSeasonStart, seasonalAnchors } from './maintenance';
import { setTestedOn, testedOn } from './persistence';

async function catalogue(): Promise<Catalogue> {
  const r = await createMockEngine().catalogue();
  if (!r.ok) throw new Error('catalogue');
  return r.value;
}

describe('maintenance calendar', () => {
  it('dates rotations and checks from when things were recorded', async () => {
    const cat = await catalogue();
    const plan = savedFor(FIXTURES['philadelphia-renters-4'], {
      purchases: [
        { item_id: 'water_stored', tier: 'h72', qty: 3, date: '2026-10-05' },
        { item_id: 'alarms_test', tier: 'now', qty: 1, date: '2026-10-02' },
      ],
    });
    const tasks = maintenanceTasks(plan, cat);
    const water = tasks.find((t) => t.key === 'rotate:water_stored')!;
    expect(water.due).toBe('2027-04-05');
    expect(water.interval_months).toBe(6);
    expect(tasks.find((t) => t.key === 'check:alarms_test')!.due).toBe('2026-11-02');
    expect(tasks.find((t) => t.key === 'review')!.due).toBe('2027-10-01');
    expect(tasks.map((t) => t.due)).toEqual([...tasks.map((t) => t.due)].sort());
  });

  it('restarts the clock when a task is marked done', async () => {
    const cat = await catalogue();
    const plan = savedFor(FIXTURES['philadelphia-renters-4'], {
      purchases: [{ item_id: 'water_stored', tier: 'h72', qty: 3, date: '2026-10-05' }],
      done_dates: { 'rotate:water_stored': '2027-04-10' },
    });
    expect(maintenanceTasks(plan, cat).find((t) => t.key === 'rotate:water_stored')!.due).toBe('2027-10-10');
  });

  it('marks things the household already had as of unknown age', async () => {
    const cat = await catalogue();
    // The mock catalogue's own water id: the fixtures name real catalogue items (water_jug_7gal)
    // since 0100b0c, which the mock does not know.
    const input = { ...FIXTURES['coos-bay-well-owner-2'], existing: [{ item_id: 'water_stored', qty: 20 }] };
    const plan = savedFor(input);
    const water = maintenanceTasks(plan, cat).find((t) => t.key === 'rotate:water_stored')!;
    expect(water.from_inventory).toBe(true);
  });

  it('lists the drills in the plan', async () => {
    const cat = await catalogue();
    const drills = drillItems(cat, new Set(['escape_plan', 'tsunami_route', 'water_stored']));
    expect(drills.map((d) => d.id).sort()).toEqual(['escape_plan', 'tsunami_route']);
  });
});

describe('calendar file', () => {
  it('is a valid iCalendar file with one repeating event per task', async () => {
    const cat = await catalogue();
    const plan = savedFor(FIXTURES['philadelphia-renters-4'], {
      purchases: [{ item_id: 'water_stored', tier: 'h72', qty: 3, date: '2026-10-05' }],
    });
    const ics = calendarFile(maintenanceTasks(plan, cat), '2026-10-06');
    const lines = ics.split('\r\n');
    expect(lines[0]).toBe('BEGIN:VCALENDAR');
    expect(lines.at(-2)).toBe('END:VCALENDAR');
    expect(ics).not.toMatch(/[^\r]\n/);
    expect(ics).toContain('DTSTART;VALUE=DATE:20270405');
    expect(ics).toContain('RRULE:FREQ=MONTHLY;INTERVAL=6');
    expect(ics).toContain('DTSTAMP:20261006T000000Z');
    for (const line of lines) expect(new TextEncoder().encode(line).length).toBeLessThanOrEqual(75);
    expect((ics.match(/BEGIN:VEVENT/g) ?? []).length).toBe((ics.match(/END:VEVENT/g) ?? []).length);
    // Nothing about where the household lives.
    expect(ics).not.toContain('19147');
    expect(ics).not.toContain('Philadelphia');
  });

  it('escapes commas and semicolons and folds long lines', () => {
    const ics = calendarFile(
      [{ key: 'check:x', kind: 'check', title: 'Check: a, b; c and a very long description that keeps going well past seventy-five characters', interval_months: 12, due: '2027-01-01', from_inventory: false }],
      '2026-10-06',
    );
    expect(ics).toContain(String.raw`SUMMARY:Check: a\, b\; c`);
    expect(ics).toMatch(/\r\n [a-z]/);
  });
});

describe('tests and seasons (contract v2)', () => {
  it('dates a test from the last time the item was tried, and says "Test:"', async () => {
    const cat = await catalogue();
    // The radio needs trying every 6 months (mock catalogue); bought on 5 Oct, tried on 1 Dec.
    const plan = savedFor(FIXTURES['philadelphia-renters-4'], {
      purchases: [{ item_id: 'radio_crank', tier: 'h72', qty: 1, date: '2026-10-05' }],
    });
    const before = maintenanceTasks(plan, cat).find((t) => t.key === 'test:radio_crank')!;
    expect(before).toMatchObject({ kind: 'test', title: 'Test: battery or hand-crank weather radio', interval_months: 6, due: '2027-04-05' });
    expect(before.last).toBeUndefined();
    expect(setTestedOn(plan, 'radio_crank', '2026-12-01')).toBe(true);
    expect(testedOn(plan, 'radio_crank')).toBe('2026-12-01');
    const after = maintenanceTasks(plan, cat).find((t) => t.key === 'test:radio_crank')!;
    expect(after).toMatchObject({ last: '2026-12-01', due: '2027-06-01' });
    // Something never owned cannot be marked tested.
    expect(setTestedOn(plan, 'generator_portable', '2026-12-01')).toBe(false);
  });

  it('keeps one tested-on date (persistence, as the Have screen): on what the household had before the plan when it has the item there', async () => {
    const input = { ...FIXTURES['coos-bay-well-owner-2'], existing: [{ item_id: 'radio_crank', qty: 1, tested_on: '2026-01-10' }] };
    const plan = savedFor(input, { purchases: [{ item_id: 'radio_crank', tier: 'h72', qty: 1, date: '2026-10-05', tested_on: '2026-10-05' }] });
    expect(testedOn(plan, 'radio_crank')).toBe('2026-10-05');
    setTestedOn(plan, 'radio_crank', '2026-11-11');
    expect(plan.input.existing[0]!.tested_on).toBe('2026-11-11');
    expect(plan.purchases[0]!.tested_on).toBeUndefined();
  });

  it('anchors seasonal items to the first day of their season, from the planning date', async () => {
    expect(nextSeasonStart('summer', '2026-10-01')).toBe('2027-06-01');
    expect(nextSeasonStart('winter', '2026-10-01')).toBe('2026-12-01');
    expect(nextSeasonStart('fall', '2026-09-01')).toBe('2026-09-01');
    expect(nextSeasonStart('fall', '2026-09-01', true)).toBe('2027-09-01');
    const cat = await catalogue();
    const plan = savedFor(FIXTURES['philadelphia-renters-4'], {
      purchases: [{ item_id: 'fans_cooling', tier: 'h72', qty: 2, date: '2026-10-05' }],
    });
    const fans = maintenanceTasks(plan, cat).find((t) => t.key === 'season:fans_cooling')!;
    expect(fans).toMatchObject({ kind: 'season', title: 'Before summer: check battery fans and cooling towels', interval_months: 12, due: '2027-06-01' });
    plan.done_dates['season:fans_cooling'] = '2027-06-02';
    expect(maintenanceTasks(plan, cat).find((t) => t.key === 'season:fans_cooling')!.due).toBe('2028-06-01');
  });

  it('groups the seasonal anchors of what the household has or plans, season by season', async () => {
    const cat = await catalogue();
    const anchors = seasonalAnchors(cat, new Set(['fans_cooling', 'blankets_warm', 'alarms_test', 'water_stored']));
    expect(anchors.map((a) => [a.season, a.items.map((i) => i.id)])).toEqual([
      ['summer', ['fans_cooling']],
      ['fall', ['alarms_test']],
      ['winter', ['blankets_warm']],
    ]);
  });

  it('puts tests and seasonal checks in the calendar file', async () => {
    const cat = await catalogue();
    const plan = savedFor(FIXTURES['philadelphia-renters-4'], {
      purchases: [
        { item_id: 'radio_crank', tier: 'h72', qty: 1, date: '2026-10-05' },
        { item_id: 'fans_cooling', tier: 'h72', qty: 2, date: '2026-10-05' },
      ],
    });
    const ics = calendarFile(maintenanceTasks(plan, cat), '2026-10-06');
    expect(ics).toContain('SUMMARY:Test: battery or hand-crank weather radio');
    expect(ics).toContain('UID:test-radio_crank@ready-reckoner.local');
    expect(ics).toContain('SUMMARY:Before summer: check battery fans and cooling towels');
    expect(ics).toContain('DTSTART;VALUE=DATE:20270601');
    expect(ics).toContain('RRULE:FREQ=MONTHLY;INTERVAL=12');
  });
});
