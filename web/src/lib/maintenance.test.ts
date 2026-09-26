import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import type { Catalogue } from '../engine/types';
import { savedFor } from '../test/helpers';
import { calendarFile, drillItems, maintenanceTasks } from './maintenance';

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
    const plan = savedFor(FIXTURES['coos-bay-well-owner-2']);
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
