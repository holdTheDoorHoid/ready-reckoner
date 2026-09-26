/**
 * The contract v2 screens and questions (web-interview): the family plan, the new interview
 * questions, the Have screen's tested-on date, the settings' rare families and dials, the packet's
 * wallet cards, and the links between them. Every screen here is also checked with axe.
 */
import axe from 'axe-core';
import { flushSync, tick, type Component } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import SiteHeader from '../components/SiteHeader.svelte';
import { FIXTURES, type FixtureName } from '../engine/fixtures';
import type { PlanInput } from '../engine/types';
import { RARE_HAZARD_IDS } from '../engine/types';
import detroitJson from '../../../fixtures/households/pending/detroit-snap-3.json';
import { STORAGE_KEY, type SavedPlan } from '../lib/persistence';
import { render, savedFor, until, type Rendered } from '../test/helpers';
import FamilyPlan from './FamilyPlan.svelte';
import Have from './Have.svelte';
import Money from './Money.svelte';
import Packet from './Packet.svelte';
import PlanScreen from './PlanScreen.svelte';
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

describe('Your family plan', () => {
  it('renders for every kind of household, and with a plan already written', async () => {
    for (const plan of [fixture('philadelphia-renters-4'), fixture('chicago-student-zero-budget-1'), savedFor(detroit)]) {
      const r = await open(FamilyPlan, 'family', plan);
      expect(r.target.querySelector('h1')?.textContent).toBe('Your family plan');
      expect(slips(r.text())).toEqual([]);
      for (const heading of ['Staying in touch', 'Where to shelter', 'If you have to leave', 'Around the home', 'Your trusted circle', 'A lawyer']) {
        expect(r.text()).toContain(heading);
      }
      r.cleanup();
      current = undefined;
    }
  });

  it('is accessible, empty and filled in', async () => {
    for (const plan of [fixture('philadelphia-renters-4'), savedFor(detroit)]) {
      const r = await open(FamilyPlan, 'family', plan);
      for (const d of r.target.querySelectorAll('details')) d.open = true;
      flushSync();
      await noAxeViolations(r, 'family plan');
      r.cleanup();
      current = undefined;
    }
  });

  it('shows what the household wrote, word for word', async () => {
    const r = await open(FamilyPlan, 'family', savedFor(detroit));
    const value = (id: string) => (r.target.querySelector(`#${id}`) as HTMLInputElement).value;
    expect(value('fp-contact-name')).toBe('Cousin Tanya in Columbus');
    expect(value('fp-meet-near')).toBe('The front steps of the church on our corner');
    expect(value('fp-route-1')).toBe('A ride with Mr. Ellis on I-75 south');
    expect(value('fp-circle-2-name')).toBe('Mr. Ellis');
    expect(value('fp-lawyer-phone')).toBe('555-0143');
    expect(value('fp-number-1')).toBe('555-0141');
    expect(r.target.querySelectorAll('.circle__person')).toHaveLength(3);
    const ellisHolds = [...r.target.querySelectorAll('.circle__person')[2]!.querySelectorAll('input[type="checkbox"]:checked')].map((c) => c.closest('label')?.textContent?.trim());
    expect(ellisHolds.map((t) => t?.split('May')[0]?.split('To sign')[0]?.trim())).toEqual(['Medical power of attorney', 'Backup codes for our accounts']);
  });

  it('saves answers as they are typed, tidies them on leaving, and keeps no empty plan', async () => {
    const r = await open(FamilyPlan, 'family', fixture('philadelphia-renters-4'));
    const near = r.target.querySelector('#fp-meet-near') as HTMLInputElement;
    expect(near.maxLength).toBe(300);
    type(near, '  The corner mailbox ', false);
    expect(r.app.plan!.input.family_plan).toEqual({ meeting_place_near: '  The corner mailbox ' });
    near.dispatchEvent(new Event('blur'));
    flushSync();
    expect(r.app.plan!.input.family_plan).toEqual({ meeting_place_near: 'The corner mailbox' });
    type(r.target.querySelector('#fp-contact-phone') as HTMLInputElement, '555-0100');
    expect(r.app.plan!.input.family_plan?.out_of_area_contact).toEqual({ phone: '555-0100' });
    expect((r.target.querySelector('#fp-contact-phone') as HTMLInputElement).maxLength).toBe(80);
    await until(() => !!r.app.result.output?.packet_markdown.includes('The corner mailbox'), 'the packet to echo the plan');
    type(near, '');
    type(r.target.querySelector('#fp-contact-phone') as HTMLInputElement, '');
    expect(r.app.plan!.input.family_plan).toBeUndefined();
    await new Promise((res) => setTimeout(res, 5));
    expect(JSON.parse(r.storage.getItem(STORAGE_KEY)!).input.family_plan).toBeUndefined();
  });

  it('keeps each note on one line: Enter adds no break, and pasted line breaks become spaces', async () => {
    const r = await open(FamilyPlan, 'family', fixture('philadelphia-renters-4'));
    const box = r.target.querySelector('#fp-shelter-home') as HTMLTextAreaElement;
    expect(box.tagName).toBe('TEXTAREA');
    expect(box.maxLength).toBe(300);
    const enter = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
    box.dispatchEvent(enter);
    expect(enter.defaultPrevented).toBe(true);
    type(box as unknown as HTMLInputElement, 'Inner hallway\ndownstairs', false);
    expect(box.value).toBe('Inner hallway downstairs');
    expect(r.app.plan!.input.family_plan?.shelter_spot_home).toBe('Inner hallway downstairs');
    // Names and numbers stay one-line fields.
    expect((r.target.querySelector('#fp-contact-phone') as HTMLElement).tagName).toBe('INPUT');
  });

  it('keeps up to five numbers and up to four people, with what each holds', async () => {
    const r = await open(FamilyPlan, 'family', fixture('philadelphia-renters-4'));
    for (let i = 0; i < 5; i++) {
      button(r, 'Add a number').click();
      flushSync();
      await tick();
    }
    expect(r.target.querySelectorAll('[id^="fp-number-"]')).toHaveLength(5);
    expect([...r.target.querySelectorAll('button')].some((b) => b.textContent?.includes('Add a number'))).toBe(false);
    type(r.target.querySelector('#fp-number-0') as HTMLInputElement, ' 555-0199 ');
    expect(r.app.plan!.input.family_plan?.numbers_by_heart?.[0]).toBe('555-0199');
    button(r, 'Remove number 5').click();
    flushSync();
    expect(r.target.querySelectorAll('[id^="fp-number-"]')).toHaveLength(4);

    for (let i = 0; i < 4; i++) {
      button(r, 'Add someone').click();
      flushSync();
      await tick();
    }
    expect(r.target.querySelectorAll('.circle__person')).toHaveLength(4);
    expect(r.text()).toContain('That is four people, the most the plan keeps.');
    await until(() => document.activeElement?.id === 'fp-circle-3-name', 'focus on the new name field');
    // The same labels repeat for each person, so screen readers hear whose they are.
    const words = (el: Element | null) => el?.textContent?.replace(/\s+/g, ' ').trim();
    expect(words(r.target.querySelector('label[for="fp-circle-1-name"]'))).toBe('Name of person 2');
    type(r.target.querySelector('#fp-circle-0-name') as HTMLInputElement, 'Rosa');
    expect(words(r.target.querySelectorAll('.circle__person')[0]!.querySelector('legend'))).toBe('What Rosa holds for you');
    choice(r, 'A spare key', r.target.querySelectorAll('.circle__person')[0]!).click();
    flushSync();
    expect(r.app.plan!.input.family_plan?.trusted_circle?.[0]).toEqual({ name: 'Rosa', holds: ['spare_key'] });
    button(r, 'Remove person 2 from your trusted circle').click();
    flushSync();
    expect(r.app.plan!.input.family_plan?.trusted_circle).toHaveLength(3);
  });

  it('asks only what fits the household: children, animals and a car', async () => {
    let r = await open(FamilyPlan, 'family', fixture('chicago-student-zero-budget-1'));
    expect(r.target.querySelector('#fp-school')).toBeNull();
    expect(r.target.querySelector('#fp-animals')).toBeNull();
    expect(r.target.querySelector('#fp-roadside')).toBeNull();
    expect(r.text()).toContain('Work and school');
    r.cleanup();
    r = await open(FamilyPlan, 'family', fixture('philadelphia-renters-4'));
    expect(r.target.querySelector('#fp-school')).not.toBeNull();
    expect(r.target.querySelector('#fp-animals')).not.toBeNull();
    expect(r.target.querySelector('#fp-roadside')).not.toBeNull();
    expect(r.text()).toContain('Children, school and work');
  });

  it('opens the packet at the wallet cards, and marks the household-plan step done', async () => {
    const r = await open(FamilyPlan, 'family', fixture('philadelphia-renters-4'));
    const print = [...r.target.querySelectorAll('a')].find((a) => a.textContent?.includes('Print wallet cards'))!;
    expect(print.getAttribute('href')).toBe('#/packet/wallet-cards');
    button(r, 'Mark "Household plan and contact cards" as done').click();
    flushSync();
    expect(r.app.plan!.purchases.map((p) => p.item_id)).toEqual(['plan_family_contacts']);
    await until(() => r.text().includes('is done on your plan'), 'the step to show as done');
  });

  it('opens at one part when the address names it', async () => {
    const r = await open(FamilyPlan, 'family/circle', fixture('philadelphia-renters-4'));
    await until(() => document.activeElement?.id === 'fp-circle-title', 'focus on the trusted circle');
  });

  it('invites a person with no plan to start one', async () => {
    const r = await open(FamilyPlan, 'family', null);
    expect(r.text()).toContain("You haven't started a plan on this device yet");
  });

  it('is in the site navigation, after "Your answers"', async () => {
    const r = await open(SiteHeader, 'family', fixture('philadelphia-renters-4'));
    const labels = [...r.target.querySelectorAll('nav a')].map((a) => a.textContent?.trim());
    expect(labels.slice(0, 2)).toEqual(['Your answers', 'Family plan']);
    expect(r.target.querySelector('nav a[href="#/family"]')?.getAttribute('aria-current')).toBe('page');
  });
});

describe('the plan links its household-plan steps to the family plan', () => {
  it('shows "Fill in your household plan here" on the step', async () => {
    const r = await open(PlanScreen, 'plan', fixture('philadelphia-renters-4'));
    const link = [...r.target.querySelectorAll('a')].find((a) => a.textContent?.includes('Fill in your household plan here'));
    expect(link?.getAttribute('href')).toBe('#/family');
  });
});

describe('the packet: wallet cards', () => {
  it('opens at the wallet cards, and prints them alone', async () => {
    const print = vi.spyOn(window, 'print').mockImplementation(() => {});
    const r = await open(Packet, 'packet/wallet-cards', savedFor(detroit));
    await until(() => (document.activeElement?.textContent ?? '').includes('Wallet cards'), 'focus on the wallet cards');
    const cards = r.target.querySelector('.packet-section.is-cards')!;
    expect(cards.id).toBe('packet-wallet-cards');
    expect(cards.querySelectorAll('blockquote')).toHaveLength(detroit.people.length);
    button(r, 'Print only the wallet cards').click();
    await until(() => print.mock.calls.length > 0, 'the print window');
    expect(print).toHaveBeenCalledTimes(1);
    expect(r.target.querySelector('.packet-page')?.classList.contains('cards-only')).toBe(true);
    window.dispatchEvent(new Event('afterprint'));
    flushSync();
    expect(r.target.querySelector('.packet-page')?.classList.contains('cards-only')).toBe(false);
    await noAxeViolations(r, 'packet at the wallet cards');
  });
});

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
