import axe from 'axe-core';
import { flushSync, tick, type Component } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { FIXTURE_NAMES, FIXTURES, type FixtureName } from '../engine/fixtures';
import { STORAGE_KEY } from '../lib/persistence';
import { render, savedFor, until, type Rendered } from '../test/helpers';
import About from './About.svelte';
import Have from './Have.svelte';
import Learn from './Learn.svelte';
import Maintain from './Maintain.svelte';
import Money from './Money.svelte';
import NotFound from './NotFound.svelte';
import Packet from './Packet.svelte';
import PlanScreen from './PlanScreen.svelte';
import Risks from './Risks.svelte';
import Start from './Start.svelte';
import Travel from './Travel.svelte';
import Where from './Where.svelte';
import Who from './Who.svelte';

const SCREENS: [string, Component, string, string][] = [
  ['start', Start, '', 'Get ready for what is likely where you live'],
  ['where', Where, 'where', 'Where you live'],
  ['who', Who, 'who', 'Who is in your household'],
  ['travel', Travel, 'travel', 'How you get around'],
  ['money', Money, 'money', 'Money'],
  ['have', Have, 'have', 'What you already have'],
  ['risks', Risks, 'risks', 'Your risks'],
  ['plan', PlanScreen, 'plan', 'Your plan'],
  ['packet', Packet, 'packet', 'Your packet'],
  ['maintain', Maintain, 'maintain', 'Keep it up'],
  ['learn', Learn, 'learn', 'Learn'],
  ['learn article', Learn, 'learn/myths', 'Disaster myths'],
  ['about', About, 'about', 'About and method'],
  ['not found', NotFound, 'missing', 'Page not found'],
];

let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
});

async function screen(Screen: Component, route: string, fixture: FixtureName | null) {
  current = await render(Screen, { plan: fixture ? savedFor(FIXTURES[fixture]) : null, route });
  await tick();
  flushSync();
  return current;
}

/** Text that should never reach a person: formatting slips and raw ids. */
function slips(text: string): string[] {
  const found: string[] = [];
  for (const bad of ['undefined', 'NaN', '[object Object]', 'null', 'Infinity']) if (text.includes(bad)) found.push(bad);
  const ids = text.match(/\b(?:one_in_\d+|water_out|water_boil|medical_emergency|home_loss|get_home|free_action)\b/g);
  if (ids) found.push(...ids);
  return found;
}

describe('every screen renders with every fixture', () => {
  for (const [name, Screen, route, heading] of SCREENS) {
    it(name, async () => {
      for (const fixture of FIXTURE_NAMES) {
        const r = await screen(Screen, route, fixture);
        const h1 = r.target.querySelector('h1');
        expect(h1?.textContent, `${name} with ${fixture}`).toContain(heading);
        expect(h1?.id).toBe('page-title');
        expect(slips(r.text()), `${name} with ${fixture}`).toEqual([]);
        r.cleanup();
        current = undefined;
      }
    });
  }
});

describe('screens with no plan yet', () => {
  it('invite the person to start instead of failing', async () => {
    for (const [name, Screen, route] of SCREENS.filter(([n]) => ['risks', 'plan', 'packet'].includes(n))) {
      const r = await screen(Screen, route, null);
      expect(r.text(), name).toContain("You haven't started a plan on this device yet");
      r.cleanup();
      current = undefined;
    }
  });
});

describe('accessibility (axe in jsdom; colour contrast is checked in a real browser by npm run shots)', () => {
  for (const [name, Screen, route] of SCREENS) {
    it(name, async () => {
      for (const fixture of ['philadelphia-renters-4', 'coos-bay-well-owner-2'] as const) {
        const r = await screen(Screen, route, fixture);
        const results = await axe.run(r.target, {
          rules: { 'color-contrast': { enabled: false }, region: { enabled: false }, 'page-has-heading-one': { enabled: false } },
        });
        const violations = results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).slice(0, 3).join(', ')}`);
        expect(violations, `${name} with ${fixture}`).toEqual([]);
        r.cleanup();
        current = undefined;
      }
    });
  }
});

describe('what the screens show', () => {
  it('risks: ranked hazards, gauges with ranges, readiness cards, rare box and savings track', async () => {
    const r = await screen(Risks, 'risks', 'philadelphia-renters-4');
    const text = r.text();
    expect(text).toMatch(/About \d+.* of 100 households like yours/);
    expect(text).toContain('about 3 days (2–5)');
    expect(text).toContain('Help likely arrives in about');
    expect(text).toContain('Rare but severe');
    expect(text).toContain('Separate from your supplies budget');
    expect(r.target.querySelectorAll('[role="meter"]').length).toBeGreaterThanOrEqual(7);
    // Philadelphia has no named scenarios, so none are offered.
    expect(text).not.toContain('Named scenarios');
  });

  it('risks: changing the dial re-runs the assessment and the targets follow', async () => {
    const r = await screen(Risks, 'risks', 'philadelphia-renters-4');
    (r.target.querySelector('button[aria-controls="settings-panel"]') as HTMLButtonElement).click();
    flushSync();
    const rare = r.target.querySelector('input[value="one_in_500"]') as HTMLInputElement;
    rare.click();
    flushSync();
    await until(() => r.app.result.output?.buckets.find((b) => b.id === 'power')?.target.kind === 'days' && (r.app.result.output!.buckets.find((b) => b.id === 'power')!.target as { value: number }).value === 10, 'the 1-in-500 targets');
    flushSync();
    expect(r.text()).toContain('about 10 days');
    expect(JSON.parse(r.storage.getItem(STORAGE_KEY) ?? '{}').input?.dials.return_period).toBe('one_in_500');
  });

  it('risks: Coos Bay offers the Cascadia scenario with a with/without comparison and a cliff warning', async () => {
    const r = await screen(Risks, 'risks', 'coos-bay-well-owner-2');
    await until(() => r.text().includes('Without it'), 'the with/without comparison');
    const text = r.text();
    expect(text).toContain('Plan for a Cascadia earthquake');
    expect(text).toContain('Your plan depends mostly on one event');
    expect(text).toContain('Keep my plan as it is');
    const toggle = r.target.querySelector('input[role="switch"]') as HTMLInputElement;
    expect(toggle.checked).toBe(true);
    toggle.click();
    flushSync();
    await until(() => r.app.result.output?.scenarios[0]?.on === false, 'the scenario to switch off');
    expect(r.app.plan!.input.dials.scenario_overrides).toEqual([{ id: 'cascadia_m9', on: false }]);
  });

  it('warnings fold away with "keep my plan as it is" and come back', async () => {
    const r = await screen(Risks, 'risks', 'philadelphia-renters-4');
    const keep = [...r.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Keep my plan as it is'))!;
    keep.click();
    flushSync();
    expect(r.text()).toContain('You chose to keep your plan as it is');
    const again = [...r.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Show again'))!;
    again.click();
    flushSync();
    expect(r.app.plan!.dismissed_warnings).toEqual([]);
  });

  it('plan: free steps first, check-offs record the date, and "done" counts go up', async () => {
    const r = await screen(PlanScreen, 'plan', 'philadelphia-renters-4');
    expect(r.text()).toContain('Free steps');
    const firstBox = r.target.querySelector('.item input[type="checkbox"]') as HTMLInputElement;
    const before = r.text().match(/(\d+) of (\d+) steps/)!;
    firstBox.click();
    flushSync();
    expect(r.app.plan!.purchases).toHaveLength(1);
    expect(r.app.plan!.purchases[0]!.date).toBe('2026-10-01');
    await until(() => {
      const m = r.text().match(/(\d+) of (\d+) steps/);
      return !!m && Number(m[1]) === Number(before[1]) + 1;
    }, 'the done count to rise');
  });

  it('plan: a zero budget gives free steps only and says so', async () => {
    const r = await screen(PlanScreen, 'plan', 'chicago-student-zero-budget-1');
    const text = r.text();
    expect(text).toContain('Free steps only');
    expect(text).toContain('Your plan is free steps only');
    expect(r.target.querySelectorAll('.item').length).toBeGreaterThan(5);
  });

  it('plan: the medical-device household sees the device battery early', async () => {
    const r = await screen(PlanScreen, 'plan', 'phoenix-apartment-cpap-1');
    expect(r.text()).toContain('Battery backup for a medical device');
  });

  it('packet: renders the engine packet safely with a print button', async () => {
    const r = await screen(Packet, 'packet', 'philadelphia-renters-4');
    expect(r.target.querySelector('.packet h2')?.textContent).toBe('Your preparedness packet');
    expect(r.target.querySelector('.packet script, .packet img, .packet iframe')).toBeNull();
    expect([...r.target.querySelectorAll('button')].some((b) => b.textContent?.includes('Print or save as PDF'))).toBe(true);
  });

  it('where: a ZIP code that spans counties asks which one, and keeps the ZIP code', async () => {
    const plan = savedFor(FIXTURES['philadelphia-renters-4']);
    plan.input.location.zip = '19087';
    current = await render(Where, { plan, route: 'where' });
    await until(() => current!.text().includes('covers more than one county'), 'the county choice');
    const chester = [...current.target.querySelectorAll('label.choice')].find((l) => l.textContent?.includes('Chester County'))!;
    (chester.querySelector('input') as HTMLInputElement).click();
    flushSync();
    expect(current.app.plan!.input.location).toMatchObject({ zip: '19087', county_fips: '42029' });
    await until(() => current!.text().includes("That's Chester County"), 'the confirmation');
  });

  it('where: an unknown ZIP code is flagged, and Continue lists it but never blocks', async () => {
    const plan = savedFor(FIXTURES['philadelphia-renters-4']);
    plan.input.location.zip = '00000';
    current = await render(Where, { plan, route: 'where' });
    const zip = current.target.querySelector('#zip') as HTMLInputElement;
    zip.value = '0000';
    zip.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    const cont = [...current.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Continue'))!;
    cont.click();
    flushSync();
    await tick();
    expect(current.text()).toContain('Check this answer');
    const anyway = [...current.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Continue anyway'))!;
    anyway.click();
    flushSync();
    expect(current.router.current.id).toBe('who');
  });

  it('who: adding a person adds a card; the medical question reveals details', async () => {
    const plan = savedFor(FIXTURES['chicago-student-zero-budget-1']);
    plan.input.people[0]!.medical.dietary = [];
    current = await render(Who, { plan, route: 'who' });
    expect(current.target.querySelectorAll('.person').length).toBe(1);
    ([...current.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Add a person')) as HTMLButtonElement).click();
    flushSync();
    expect(current.target.querySelectorAll('.person').length).toBe(2);
    expect(current.target.querySelectorAll('.medical').length).toBe(0);
    const yes = [...current.target.querySelectorAll('label.choice')].find((l) => l.textContent?.trim() === 'Yes')!;
    (yes.querySelector('input') as HTMLInputElement).click();
    flushSync();
    expect(current.target.querySelectorAll('.medical').length).toBe(2);
    expect(current.text()).toContain('Takes a daily prescription medicine');
  });

  it('money: the budget field validates and never saves a bad number', async () => {
    current = await render(Money, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'money' });
    const input = current.target.querySelector('#monthly-budget') as HTMLInputElement;
    input.value = '-5';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('blur'));
    flushSync();
    expect(current.text()).toContain('Enter 0 or more.');
    expect(current.app.plan!.input.finances.monthly_budget_usd).toBe(60);
    input.value = '85';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(current.app.plan!.input.finances.monthly_budget_usd).toBe(85);
  });

  it('maintain: "forget everything" asks first, then clears this browser', async () => {
    current = await render(Maintain, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'maintain' });
    ([...current.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Forget everything')) as HTMLButtonElement).click();
    flushSync();
    const dialog = document.querySelector('dialog[open]');
    expect(dialog?.textContent).toContain('It cannot be undone');
    ([...dialog!.querySelectorAll('button')].find((b) => b.textContent === 'Forget everything') as HTMLButtonElement).click();
    flushSync();
    await new Promise((r) => setTimeout(r, 10));
    expect(current.app.plan).toBeNull();
    expect(current.storage.getItem(STORAGE_KEY)).toBeNull();
    expect(current.router.current.id).toBe('start');
  });

  it('about: shows the engine versions and the attributions exactly', async () => {
    const r = await screen(About, 'about', 'philadelphia-renters-4');
    for (const a of r.app.info!.attributions) expect(r.text()).toContain(a.text);
    expect(r.text()).toContain('stand-in engine');
  });

  it('start: starting a plan takes the person to the first step with a dated plan', async () => {
    current = await render(Start, { plan: null, route: '', today: '2026-12-01' });
    ([...current.target.querySelectorAll('button')].find((b) => b.textContent?.includes('Start a plan')) as HTMLButtonElement).click();
    await until(() => current!.router.current.id === 'where', 'the first step');
    expect(current.app.plan?.input.planning_date).toBe('2026-12-01');
  });
});
