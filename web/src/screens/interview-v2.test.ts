/**
 * The contract v2 questions (web-interview): the new interview questions, the Have screen's
 * tested-on date, and the settings' rare families and dials. Every screen here is also checked
 * with axe. The v2 family plan's questions moved to the optional steps 6–8 in v0.3.0; their tests
 * are in interview-v3.test.ts.
 */
import axe from 'axe-core';
import { flushSync, tick, type Component } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { FIXTURES, type FixtureName } from '../engine/fixtures';
import type { PlanInput } from '../engine/types';
import { RARE_HAZARD_IDS } from '../engine/types';
import detroitJson from '../../../fixtures/households/detroit-snap-3.json';
import type { SavedPlan } from '../lib/persistence';
import { render, savedFor, until, type Rendered } from '../test/helpers';
import Have from './Have.svelte';
import Money from './Money.svelte';
import Risks from './Risks.svelte';
import Where from './Where.svelte';
import Who from './Who.svelte';

const detroit = detroitJson as unknown as PlanInput;

let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
  vi.restoreAllMocks();
});

async function open(Screen: Component, route: string, plan: SavedPlan | null): Promise<Rendered> {
  current = await render(Screen, { plan, route });
  await tick();
  flushSync();
  return current;
}

const fixture = (name: FixtureName) => savedFor(FIXTURES[name]);
function button(r: Rendered, text: string): HTMLButtonElement {
  const b = [...r.target.querySelectorAll('button')].find((x) => x.textContent?.includes(text));
  if (!b) throw new Error(`no button "${text}"`);
  return b as HTMLButtonElement;
}

function choice(r: Rendered, text: string, within: ParentNode = r.target): HTMLInputElement {
  const label = [...within.querySelectorAll('label.choice')].find((l) => l.querySelector('.choice__text > span')?.textContent?.trim() === text);
  if (!label) throw new Error(`no choice "${text}"`);
  return label.querySelector('input') as HTMLInputElement;
}

function type(el: HTMLInputElement, text: string, leave = true) {
  el.value = text;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  if (leave) el.dispatchEvent(new Event('blur'));
  flushSync();
}

async function noAxeViolations(r: Rendered, what: string) {
  const results = await axe.run(r.target, {
    rules: { 'color-contrast': { enabled: false }, region: { enabled: false }, 'page-has-heading-one': { enabled: false } },
  });
  const violations = results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).slice(0, 3).join(', ')}`);
  expect(violations, what).toEqual([]);
}

/** Text that should never reach a person: formatting slips and raw ids. */
function slips(text: string): string[] {
  const found = ['undefined', 'NaN', '[object Object]', 'null'].filter((bad) => text.includes(bad));
  const ids = text.match(/\b(?:spare_key|medical_poa|backup_codes|limited_english|home_health|service_animal|snap_wic|ssi_ssdi|federal_pay|surface_nearby|rain_barrel|neighbour_well|occasional_notices|frequent_problems|nuclear_attack|multi_month_blackout)\b/g);
  if (ids) found.push(...ids);
  return found;
}

describe('the new interview questions', () => {
  it('who: help in an emergency, per person, cleared with the medical details', async () => {
    const r = await open(Who, 'who', savedFor(detroit));
    const third = r.target.querySelectorAll('.person')[2]!;
    const panel = [...third.querySelectorAll('details')].find((d) => d.querySelector('summary')?.textContent?.includes('Help in an emergency'))!;
    expect(panel.open).toBe(true);
    expect(choice(r, 'Deaf or hard of hearing', panel).checked).toBe(true);
    choice(r, 'Limited English', panel).click();
    flushSync();
    expect(r.app.plan!.input.people[2]!.access_needs).toEqual(['hearing', 'limited_english']);
    choice(r, 'Deaf or hard of hearing', panel).click();
    flushSync();
    expect(r.app.plan!.input.people[2]!.access_needs).toEqual(['limited_english']);
    await noAxeViolations(r, 'who with access needs');
    const no = [...r.target.querySelectorAll('label.choice')].find((l) => l.textContent?.trim() === 'No')!;
    (no.querySelector('input') as HTMLInputElement).click();
    flushSync();
    expect(r.app.plan!.input.people.every((p) => (p.access_needs ?? []).length === 0)).toBe(true);
  });

  it('where: a bedroom below street level, the water system, raw water and cooking', async () => {
    const r = await open(Where, 'where', fixture('philadelphia-renters-4'));
    expect(r.text()).toContain('Someone sleeps below street level');
    const below = choice(r, 'Someone sleeps below street level');
    below.click();
    flushSync();
    expect(r.app.plan!.input.housing.below_grade_bedroom).toBe(true);
    choice(r, 'The home has a basement').click();
    flushSync();
    expect(r.app.plan!.input.housing.below_grade_bedroom).toBe(false);
    expect(r.text()).not.toContain('Someone sleeps below street level');

    expect(r.app.plan!.input.housing.cooking).toBeUndefined();
    choice(r, 'Gas or propane stove').click();
    choice(r, 'A rain barrel or cistern').click();
    choice(r, 'Occasional problems').click();
    flushSync();
    expect(r.app.plan!.input.housing).toMatchObject({ cooking: 'gas', raw_water_source: 'rain_barrel', water_system_record: 'occasional_notices' });
    await noAxeViolations(r, 'where with the v2 questions');
    choice(r, 'Private well').click();
    flushSync();
    expect(r.app.plan!.input.housing.water_system_record).toBeUndefined();
    expect(r.text()).not.toContain('Has your water system had problems?');
  });

  it('money: bare minimum first, benefits, and the insurance extras', async () => {
    const r = await open(Money, 'money', fixture('philadelphia-renters-4'));
    choice(r, 'Show me the bare minimum first').click();
    choice(r, 'SNAP or WIC').click();
    choice(r, 'Federal pay').click();
    choice(r, 'Sewer or water backup cover').click();
    choice(r, 'Life or disability insurance').click();
    flushSync();
    const input = r.app.plan!.input;
    expect(input.dials.minimum_kit).toBe(true);
    expect(input.finances.benefits).toEqual(['federal_pay', 'snap_wic']);
    expect(input.finances.insurance).toMatchObject({ sewer_backup: true, life_or_disability: true });
    expect(r.text()).toContain('Ticking one only adds one risk to your list');
    await noAxeViolations(r, 'money with the v2 questions');
    // No earner, no life or disability question.
    r.cleanup();
    const retiree = await open(Money, 'money', fixture('miami-condo-retiree-1'));
    expect(retiree.text()).not.toContain('Life or disability insurance');
  });

  it('have: when an item that needs testing was last tried, once the household has some', async () => {
    const plan = fixture('philadelphia-renters-4');
    plan.input.existing = [];
    const r = await open(Have, 'have', plan);
    expect(r.target.querySelector('#tested-flashlights_headlamps')).toBeNull();
    type(r.target.querySelector('#have-flashlights_headlamps') as HTMLInputElement, '4');
    expect(r.target.querySelector('#tested-flashlights_headlamps')).not.toBeNull();
    button(r, 'Tried it today').click();
    flushSync();
    expect(r.app.plan!.input.existing).toEqual([{ item_id: 'flashlights_headlamps', qty: 4, tested_on: '2026-10-01' }]);
    expect(r.text()).toContain('Last tried Oct 1, 2026.');
    const date = r.target.querySelector('#tested-flashlights_headlamps') as HTMLInputElement;
    date.value = '2030-01-01';
    date.dispatchEvent(new Event('change', { bubbles: true }));
    flushSync();
    expect(r.text()).toContain('Enter a day on or before today.');
    expect(r.app.plan!.input.existing[0]!.tested_on).toBe('2026-10-01');
    date.value = '';
    date.dispatchEvent(new Event('change', { bubbles: true }));
    flushSync();
    expect(r.app.plan!.input.existing[0]!.tested_on).toBeUndefined();
    await noAxeViolations(r, 'have with a tested-on date');
  });
});

describe('the settings: rare families, bare minimum and the long horizon', () => {
  async function settings(plan: SavedPlan) {
    const r = await open(Risks, 'risks', plan);
    (r.target.querySelector('button[aria-controls="settings-panel"]') as HTMLButtonElement).click();
    flushSync();
    return r;
  }

  it('turns the rare allowance on family by family, or for all of them', async () => {
    const r = await settings(fixture('philadelphia-renters-4'));
    const panel = r.target.querySelector('.rare-opt-in')!;
    expect(panel.querySelectorAll('input[type="checkbox"]')).toHaveLength(RARE_HAZARD_IDS.length + 1);
    const nuclear = choice(r, 'Nuclear attack or EMP', panel);
    nuclear.click();
    flushSync();
    expect(r.app.plan!.input.dials).toMatchObject({ rare_opt_in: ['nuclear_attack'], rare_catastrophic_opt_in: false });
    // The engine is asked again with the new allowance (what it buys is the engine's to decide).
    await until(() => r.app.engineInput?.dials.rare_opt_in?.join() === 'nuclear_attack' && !r.app.pending && !!r.app.result.output, 'a new plan');
    choice(r, 'All of them', panel).click();
    flushSync();
    expect(r.app.plan!.input.dials.rare_opt_in).toEqual(['all']);
    expect([...panel.querySelectorAll('input[type="checkbox"]')].every((c) => (c as HTMLInputElement).checked)).toBe(true);
    expect(r.text()).toContain('rare-catastrophe allowance: all of them');
    await noAxeViolations(r, 'risks with the settings open');
  });

  it('reads a v1 plan\'s single switch as every family, and retires it on the first change', async () => {
    const plan = fixture('philadelphia-renters-4');
    plan.input.dials.rare_catastrophic_opt_in = true;
    const r = await settings(plan);
    const panel = r.target.querySelector('.rare-opt-in')!;
    expect([...panel.querySelectorAll('input[type="checkbox"]')].every((c) => (c as HTMLInputElement).checked)).toBe(true);
    choice(r, 'Severe pandemic', panel).click();
    flushSync();
    expect(r.app.plan!.input.dials.rare_catastrophic_opt_in).toBe(false);
    expect(r.app.plan!.input.dials.rare_opt_in).toHaveLength(RARE_HAZARD_IDS.length - 1);
    expect((panel.querySelector('.rare-opt-in__all input') as HTMLInputElement).indeterminate).toBe(true);
  });

  it('switches bare-minimum mode and the long-horizon section', async () => {
    const r = await settings(fixture('philadelphia-renters-4'));
    choice(r, 'Show me the bare minimum first').click();
    choice(r, 'Show the long-horizon part of the plan').click();
    flushSync();
    expect(r.app.plan!.input.dials).toMatchObject({ minimum_kit: true, long_horizon: true });
    expect(r.text()).toContain('bare minimum first');
  });
});
