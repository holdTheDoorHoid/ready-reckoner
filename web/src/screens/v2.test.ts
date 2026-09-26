/**
 * The contract v2 displays (web-risks, v0.2.0), checked on the screens with the mock engine's v2
 * outputs: the rare-but-severe box by family (REVIEW §2.4, hazard-expansion Deliverable C), the
 * matrix's rare rows and "Also checked", sub-causes on cards, the stress line, badge and driver
 * bars on targets (model review 3.3–3.4), the validation page (3.1), and the Plan and Keep it up
 * additions (bare-minimum mode, decisions, "With:", the first savings goal, the long-horizon
 * section, tests and seasons). The wording rules behind them are pinned in lib/*.test.ts.
 */
import axe from 'axe-core';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { FIXTURES, type FixtureName } from '../engine/fixtures';
import type { PlanInput } from '../engine/types';
import { firstSentence, whatItChanges } from '../lib/rare';
import { formatMonth, addMonths, rangeOnly } from '../lib/format';
import { stressLine } from '../lib/targets';
import type { Purchase } from '../lib/persistence';
import { render, savedFor, until, type Rendered } from '../test/helpers';
import Maintain from './Maintain.svelte';
import PlanScreen from './PlanScreen.svelte';
import Risks from './Risks.svelte';
import Validation from './Validation.svelte';

let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
});

const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

function household(name: FixtureName, change: (i: PlanInput) => void = () => {}): PlanInput {
  const input = JSON.parse(JSON.stringify(FIXTURES[name])) as PlanInput;
  change(input);
  return input;
}

async function show(Screen: typeof Risks, route: string, input: PlanInput, purchases: Purchase[] = []): Promise<Rendered> {
  current = await render(Screen, { plan: savedFor(input, { purchases }), route });
  return current;
}

describe('the rare-but-severe box, by family', () => {
  it('has nine collapsed family rows, most likely here first, with the five columns', async () => {
    const r = await show(Risks, 'risks', household('philadelphia-renters-4'));
    const out = r.app.result.output!;
    const rare = out.register.filter((h) => h.display === 'rare_catastrophic');
    expect(rare).toHaveLength(9);
    // Sorted by how likely here (the middle of the range, never shown), never by how bad.
    const middles = rare.map((h) => h.rate_per_year);
    expect(middles).toEqual([...middles].sort((a, b) => b - a));
    const box = r.target.querySelector('section.rare')!;
    expect([...box.querySelectorAll('thead th')].map(text)).toEqual([
      'What',
      'How likely for you (in the next 10 years)',
      'If it reaches you',
      'Why here',
      'What it changes in your plan',
    ]);
    const rows = [...box.querySelectorAll('tbody.family')];
    expect(rows.map((b) => b.id)).toEqual(rare.map((h) => `rare-${h.id}`));
    rows.forEach((b, i) => {
      const h = rare[i]!;
      const cells = b.querySelectorAll('tr.family__row td');
      expect(text(b.querySelector('.expander__name')), h.id).toBe(h.name);
      expect(text(cells[0]!.querySelector('.likely')), h.id).toBe(rangeOnly(h.rate_range[0], h.rate_range[1], 10));
      if (h.anchor_sentence) expect(text(cells[0]!.querySelector('.anchor')), h.id).toBe(h.anchor_sentence);
      expect(text(cells[1]), h.id).toBe(`If it reaches you ${h.if_it_reaches_you}`);
      const why = h.location_factor ? firstSentence(h.location_factor.label) : 'The same everywhere: this chance does not depend on where you live.';
      expect(text(cells[2]), h.id).toBe(`Why here ${why}`);
      expect(text(cells[3]), h.id).toBe(`What it changes in your plan ${whatItChanges(h)}`);
      // Every row ends with an action or a tick (EPPM: never a threat without what to do).
      expect(cells[3]!.querySelector('.tick') !== null || !/^nothing/i.test(whatItChanges(h)), h.id).toBe(true);
      // Collapsed: the details are hidden until asked for.
      expect(b.querySelector('button.expander')!.getAttribute('aria-expanded'), h.id).toBe('false');
      expect((b.querySelector('tr.more') as HTMLElement).hidden, h.id).toBe(true);
    });
    expect(text(box)).not.toContain('households like yours');
    expect(text(box)).not.toMatch(/about \d+ of 100/);
  });

  it('opens a family to its sub-rows and a "How this number is made" drawer, with sources', async () => {
    const r = await show(Risks, 'risks', household('philadelphia-renters-4'));
    const nuclear = r.app.result.output!.register.find((h) => h.id === 'nuclear_attack')!;
    const family = r.target.querySelector('#rare-nuclear_attack')!;
    (family.querySelector('button.expander') as HTMLButtonElement).click();
    flushSync();
    expect(family.querySelector('button.expander')!.getAttribute('aria-expanded')).toBe('true');
    const more = family.querySelector('tr.more') as HTMLElement;
    expect(more.hidden).toBe(false);
    expect([...more.querySelectorAll('.subrow__name')].map((n) => text(n).split(':')[0])).toEqual(nuclear.sub_causes!.map((s) => s.name));
    const drawer = more.querySelector('details.how')!;
    expect(text(drawer.querySelector('summary'))).toBe('How this number is made');
    const body = text(drawer);
    expect(body).toContain(`For a household like yours: ${rangeOnly(nuclear.rate_range[0], nuclear.rate_range[1], 10)} in the next 10 years`);
    expect(body).toContain(`Where you live: ${nuclear.location_factor!.label}`);
    expect(body).toContain('the chance that your county would be in a blast or dangerous-fallout zone: about 6 in 10 (3 to 9 in 10)');
    expect(body).toContain('Why a range:');
    expect(drawer.querySelector('details.sources')).not.toBeNull();
    // Collapsing again hides it.
    (family.querySelector('button.expander') as HTMLButtonElement).click();
    flushSync();
    expect(more.hidden).toBe(true);
  });

  it('says what the rare-event allowance buys: nothing by default, the radiation meter for the nuclear row when ticked', async () => {
    let r = await show(Risks, 'risks', household('philadelphia-renters-4'));
    expect(text(r.target.querySelector('.allowance'))).toMatch(/^What the plan spends on these Nothing\. The plan spends on these only if you allow it/);
    // "Choose what the plan may spend on" opens the settings.
    const button = [...r.target.querySelectorAll('.allowance button')].find((b) => text(b).startsWith('Choose'))! as HTMLButtonElement;
    expect((r.target.querySelector('#settings-panel') as HTMLElement).hidden).toBe(true);
    button.click();
    await until(() => !(r.target.querySelector('#settings-panel') as HTMLElement).hidden, 'the settings to open');
    r.cleanup();

    r = await show(
      Risks,
      'risks',
      household('philadelphia-renters-4', (i) => {
        i.finances.monthly_budget_usd = 400;
        // Every family, through the v1 switch this branch's mock validates (web-interview's accepts the v2 list too).
        i.dials.rare_catastrophic_opt_in = true;
      }),
    );
    current = r;
    const words = text(r.target.querySelector('.allowance'));
    expect(words).toMatch(/Your rare-event allowance \(up to \$40 a month\) buys radiation meter \(\$\d+, around \w+ \d{4}\) for the nuclear attack or EMP row\./);
    expect(words).toContain('Your basics already cover the other rows you chose.');
  });
});

describe('the matrix, sub-causes and "Also checked"', () => {
  it('links each rare row to its own row in the box, lists named causes, and shows "Also checked"', async () => {
    const r = await show(Risks, 'risks', household('philadelphia-renters-4'));
    const out = r.app.result.output!;
    const matrix = r.target.querySelector('section.matrix')!;
    for (const h of out.register.filter((x) => x.display === 'rare_catastrophic')) {
      expect(matrix.querySelector(`#matrix-${h.id}`)!.getAttribute('href')).toBe(`#rare-${h.id}`);
    }
    // A ranked row with named causes opens to list them (winter storms: blizzards, roof damage).
    const winter = out.register.find((h) => h.id === 'winter_weather')!;
    const row = matrix.querySelector('#matrix-winter_weather')!.closest('tr')!;
    expect(text(row.querySelector('details.includes summary'))).toBe(`What it includes (${winter.sub_causes!.length})`);
    expect([...row.querySelectorAll('details.includes li')].map(text)).toEqual(winter.sub_causes!.map((s) => s.name));
    // A range-only ranked row (an attack closing the area) never shows a single number.
    const attack = out.register.find((h) => h.id === 'attack_disruption')!;
    expect(attack.range_only).toBe(true);
    const attackRow = matrix.querySelector('#matrix-attack_disruption')!.closest('tr')!;
    expect(text(attackRow.querySelectorAll('td')[0])).toBe(
      `${rangeOnly(attack.rate_range[0], attack.rate_range[1], 10)}; ${rangeOnly(attack.rate_range[0], attack.rate_range[1], 1)} a year`,
    );
    // "Also checked", folded: what was checked and found too rare here, each with its rate.
    const also = matrix.querySelector('details.also')!;
    expect(text(also.querySelector('summary'))).toMatch(/^Also checked: \d+ more, too rare here to list$/);
    expect(text(also)).toContain('a Yellowstone super-eruption: about 1 in 730,000 a year');
    // The hazard card lists what it includes too, with each note's sources.
    const card = r.target.querySelector('#hazard-winter_weather')!;
    expect(text(card.querySelector('details.includes summary'))).toBe(`What it includes (${winter.sub_causes!.length})`);
    expect(text(card.querySelector('details.includes'))).toContain(winter.sub_causes![0]!.note);
  });
});

describe('targets: the stress line, the badge and the driver bars', () => {
  it('shows the worst event in the region under the power target, with its source, a badge, and bars in the drawer', async () => {
    const r = await show(Risks, 'risks', household('philadelphia-renters-4'));
    const power = r.app.result.output!.buckets.find((b) => b.id === 'power')!;
    if (power.target.kind !== 'days' || !power.stress_test) throw new Error('power target with a stress test');
    const gauge = r.target.querySelector('#days-title')!.closest('section')!.querySelector('[data-bucket="power"]')!;
    const line = gauge.querySelector('.gauge__stress')!;
    expect(text(line)).toContain(stressLine(power.stress_test, 'power', power.target.value).text);
    expect(text(line)).toContain('Wind and thunderstorms, March 2018');
    expect(line.querySelector('details.sources')).not.toBeNull();
    expect(text(gauge.querySelector('.badge'))).toMatch(/^How sure: (From records|Partly estimates|Estimates)$/);
    const why = gauge.querySelector('details.explain')!;
    const bars = [...why.querySelectorAll('.drivers__list li')];
    expect(bars).toHaveLength(power.contributions.filter((c) => c.share > 0).length);
    expect(text(bars[0])).toMatch(/^.+ \d+ in 100, (from records|an estimate)$/);
    expect(why.querySelector('a[href="#/validation"]')).not.toBeNull();
  });
});

describe('the validation page', () => {
  it('leads with the tally, then every event with its result, in-sample flag and what changed', async () => {
    const r = await show(Validation, 'validation', household('philadelphia-renters-4'));
    expect(text(r.target.querySelector('.tally__sentence'))).toBe(
      'We checked 22 real disasters. This version covered 6, partly covered 9 and fell short on 6. We cannot model one yet. Here is why, event by event.',
    );
    expect([...r.target.querySelectorAll('.tally__boxes li')].map(text)).toEqual(['6 Covered', '9 Partly covered', '6 Short', '1 Not modelled']);
    const rows = [...r.target.querySelectorAll('table.events tbody tr')];
    expect(rows).toHaveLength(22);
    expect(rows.filter((row) => row.querySelector('.in-sample'))).toHaveLength(11);
    expect(text(rows[0])).toContain('Winter Storm Uri');
    expect(text(rows[0])).toContain('Partly covered');
    expect(text(rows[0])).toContain('Before these changes: short');
    expect(text(rows[10])).toContain('Not modelled');
    // The misses stay on the page.
    expect(text(r.target)).toContain('What the misses need');
    const results = await axe.run(r.target, { rules: { 'color-contrast': { enabled: false }, region: { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });
});

describe('the plan: bare minimum, decisions, "With:", the first savings goal and the long horizon', () => {
  it('starts a long plan with the bare minimum, and lists what falls beyond three years', async () => {
    const r = await show(PlanScreen, 'plan', household('philadelphia-renters-4', (i) => (i.finances.monthly_budget_usd = 10)));
    const out = r.app.result.output!;
    expect(out.plan.minimum_kit).toBe(true);
    const banner = r.target.querySelector('section.minimum')!;
    expect(text(banner.querySelector('h2'))).toBe('Bare minimum first');
    expect(text(banner)).toContain('At your budget the full plan would take more than three years, so it starts with the bare minimum.');
    const deferred = out.warnings.find((w) => w.id === 'plan_too_long')!.related;
    expect(text(banner.querySelector('details.deferred summary'))).toBe(`Beyond three years at this budget: ${deferred.length} steps`);
    expect(banner.querySelectorAll('details.deferred li')).toHaveLength(deferred.length);
    // The banner says it; the warning is not repeated below it.
    expect(r.target.querySelectorAll('.warning').length).toBe(out.warnings.filter((w) => w.id !== 'plan_too_long').length);
  });

  it('shows decisions as decisions and accessories with what they need', async () => {
    const r = await show(PlanScreen, 'plan', household('philadelphia-renters-4'));
    const decision = [...r.target.querySelectorAll('article.item')].find((a) => text(a.querySelector('h4')) === 'Decide on renters insurance')!;
    expect(text(decision.querySelector('.item__check label'))).toBe('Decided: Decide on renters insurance');
    expect(text(decision.querySelector('.item__qty'))).toBe('Decision');
    expect(text(decision)).not.toMatch(/\$\d/);
    const batteries = [...r.target.querySelectorAll('article.item')].find((a) => text(a.querySelector('h4')) === 'Spare batteries');
    if (batteries) expect(text(batteries.querySelector('.item__with'))).toBe('With: A light for each person');
  });

  it('uses the engine’s first savings goal', async () => {
    const input = household('philadelphia-renters-4', (i) => (i.finances.emergency_fund_months = 0));
    const r = await show(PlanScreen, 'plan', input);
    const m = r.app.result.output!.plan.first_milestone!;
    expect(m.usd).toBe(500);
    expect(text(r.target.querySelector('[data-bucket="income"] .milestone'))).toBe(`First goal: $500 in savings, by ${formatMonth(addMonths(input.planning_date, m.by_month))}.`);
  });

  it('adds the long-horizon section when a target is a month or more (Coos Bay)', async () => {
    const r = await show(PlanScreen, 'plan', household('coos-bay-well-owner-2'));
    const out = r.app.result.output!;
    expect(out.plan.long_horizon!.length).toBeGreaterThan(0);
    const section = r.target.querySelector('#long-title')!.closest('section')!;
    expect(text(section.querySelector('h2'))).toBe('If it lasts for months');
    expect(text(section)).toContain('One of your targets is a month or more.');
    expect([...section.querySelectorAll('h3')].map(text)).toEqual(['Worth having', 'Worth learning']);
    expect(text(section)).toContain('Hand pump for the well');
    // Philadelphia's targets stay under a month: no section (unless the household asks, Dials.long_horizon).
    r.cleanup();
    const p = await show(PlanScreen, 'plan', household('philadelphia-renters-4'));
    current = p;
    expect(p.app.result.output!.plan.long_horizon).toBeUndefined();
    expect(p.target.querySelector('#long-title')).toBeNull();
  });
});

describe('Keep it up: tests and seasons', () => {
  it('records the day an item was tested, and shows the seasons by month', async () => {
    const r = await show(Maintain, 'maintain', household('philadelphia-renters-4'), [
      { item_id: 'radio_crank', tier: 'h72', qty: 1, date: '2026-01-05' },
      { item_id: 'fans_cooling', tier: 'h72', qty: 2, date: '2026-01-05' },
    ]);
    const due = r.target.querySelector('#due-title')!.closest('section')!;
    const test = [...due.querySelectorAll('li.task')].find((li) => text(li).startsWith('Test: battery or hand-crank weather radio'))!;
    expect(text(test)).toContain('Not tested yet: try it and record the day.');
    const button = test.querySelector('button')!;
    expect(text(button)).toBe('Tested today: Test: battery or hand-crank weather radio');
    button.click();
    flushSync();
    expect(r.app.plan!.purchases.find((p) => p.item_id === 'radio_crank')!.tested_on).toBe('2026-10-01');
    await until(() => text(r.target).includes('Tested today: battery or hand-crank weather radio.'), 'the announcement');
    const seasons = r.target.querySelector('#seasons-title')!.closest('section')!;
    const june = [...seasons.querySelectorAll('li.season')].find((li) => text(li).startsWith('June before summer'))!;
    expect([...june.querySelectorAll('.season__items li')].map(text)).toContain('Battery fans and cooling towels');
  });
});
