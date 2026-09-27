/**
 * v0.3.0 (web-interview3, DESIGN-DELTA-v3 §1, §2, §7): the optional interview steps 6–8 (render,
 * the questions in order and in their v2 words, rows added and removed, "Skip for now", saving),
 * the renamed tabs and old addresses, the Prepare tab's printed sheet, and saving and opening a
 * plan file protected with a passphrase. Every new screen is checked with axe.
 */
import axe from 'axe-core';
import { flushSync, tick, type Component } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import SiteHeader from '../components/SiteHeader.svelte';
import { FIXTURES, type FixtureName } from '../engine/fixtures';
import type { PlanInput } from '../engine/types';
import detroitJson from '../../../fixtures/households/detroit-snap-3.json';
import { STORAGE_KEY, type SavedPlan } from '../lib/persistence';
import { AGE_WORD } from '../lib/profile';
import { ENCRYPTED_FORMAT, protectedExportText } from '../lib/protect';
import { render, savedFor, until, withoutOptional, type Rendered } from '../test/helpers';
import Contacts from './Contacts.svelte';
import Have from './Have.svelte';
import Maintain from './Maintain.svelte';
import Packet from './Packet.svelte';
import People from './People.svelte';
import Places from './Places.svelte';
import PlanScreen from './PlanScreen.svelte';
import Start from './Start.svelte';

const detroit = detroitJson as unknown as PlanInput;

let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
  vi.restoreAllMocks();
});

async function open(Screen: Component, route: string, plan: SavedPlan | null): Promise<Rendered> {
  current?.cleanup();
  current = await render(Screen, { plan, route });
  await tick();
  flushSync();
  return current;
}

/** A fixture household as it is (contract v3 fixtures carry sample answers to the optional steps). */
const fixture = (name: FixtureName) => savedFor(FIXTURES[name]);
/** A fixture household that has not answered any optional step yet. */
const blank = (name: FixtureName) => savedFor(withoutOptional(FIXTURES[name]));

/** A blank household with the five required steps answered and the optional ones not. */
function requiredOnly(name: FixtureName): SavedPlan {
  const plan = blank(name);
  plan.progress.completed = ['where', 'who', 'travel', 'money', 'have'];
  return plan;
}

function button(r: Rendered, text: string): HTMLButtonElement {
  const b = [...r.target.querySelectorAll('button')].find((x) => x.textContent?.replace(/\s+/g, ' ').includes(text));
  if (!b) throw new Error(`no button "${text}"`);
  return b as HTMLButtonElement;
}

function link(r: Rendered, text: string): HTMLAnchorElement {
  const a = [...r.target.querySelectorAll('a')].find((x) => x.textContent?.replace(/\s+/g, ' ').includes(text));
  if (!a) throw new Error(`no link "${text}"`);
  return a as HTMLAnchorElement;
}

function field(r: Rendered, id: string): HTMLInputElement {
  const el = r.target.querySelector(`#${id}`);
  if (!el) throw new Error(`no field #${id}`);
  return el as HTMLInputElement;
}

function type(el: HTMLInputElement, text: string, leave = true) {
  el.value = text;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  if (leave) el.dispatchEvent(new Event('blur'));
  flushSync();
}

/** What a label says on screen: without the words only screen readers hear, or "(optional)". */
function shown(el: Element): string {
  const copy = el.cloneNode(true) as Element;
  copy.querySelectorAll('.visually-hidden, .optional').forEach((n) => n.remove());
  return copy.textContent?.replace(/\s+/g, ' ').trim() ?? '';
}

function openAll(r: Rendered) {
  for (const d of r.target.querySelectorAll('details')) d.open = true;
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
  const ids = text.match(/\b(?:childcare|spare_key|medical_poa|backup_codes|where_kit|last4|electric_utility|county_emergency_office)\b/g);
  if (ids) found.push(...ids);
  return found;
}

const saved = (r: Rendered) => JSON.parse(r.storage.getItem(STORAGE_KEY)!) as SavedPlan;

/** The text of the screen (or of one element) with line breaks and runs of spaces collapsed. */
function said(from: Rendered | Element): string {
  const text = 'target' in from ? from.text() : (from.textContent ?? '');
  return text.replace(/\s+/g, ' ');
}

// ---------------------------------------------------------------------------------------------
// Step 6
// ---------------------------------------------------------------------------------------------

describe('Step 6, Your people', () => {
  it('has a card per person from step 2, in order, headed by age group until a name is given', async () => {
    for (const plan of [blank('philadelphia-renters-4'), blank('chicago-student-zero-budget-1'), savedFor(detroit), blank('hays-kansas-farm-5')]) {
      const r = await open(People, 'people', plan);
      expect(r.target.querySelector('h1')?.textContent).toBe('Your people');
      expect(r.text()).toContain('Step 6 of 8 · optional');
      const headings = [...r.target.querySelectorAll('.person h2')].map((h) => h.textContent);
      expect(headings).toEqual(plan.input.people.map((p, i) => `Person ${i + 1} (${AGE_WORD[p.age_band]})`));
      expect(slips(r.text())).toEqual([]);
    }
  });

  it('asks §2.1’s questions in its order, every one optional', async () => {
    const r = await open(People, 'people', blank('philadelphia-renters-4'));
    openAll(r);
    const card = r.target.querySelector('#person-1')!;
    const labels = [...card.querySelectorAll('label:not(.choice), legend')].map(shown);
    expect(labels).toEqual([
      'Name or nickname',
      'Date of birth',
      'Phone',
      'Email',
      'What kind of place',
      'Name of the place',
      'Address',
      'Phone',
      'Its own emergency plan',
      'Pick-up rules',
      'The safest spot there',
      'Doctor',
      'Name',
      'Phone',
      'Pharmacy',
      'Name',
      'Phone',
      'Medical conditions',
      'Medicines',
      'Allergies',
      'Blood type',
      'Health insurance',
      'Insurance company',
      'Plan name',
      'Member ID',
      'Group number',
      'Phone on the card',
      'ID notes',
      'Anything else a helper should know',
    ]);
    expect([...card.querySelectorAll('label.choice')].map(shown)).toEqual(['Work', 'School', 'Child care', 'Somewhere else']);
    // Nothing is required, and each limit is the engine's.
    expect(card.querySelectorAll('[required], [aria-required="true"]')).toHaveLength(0);
    expect(field(r, 'pp-0-name').maxLength).toBe(60);
    expect(field(r, 'pp-0-conditions').maxLength).toBe(400);
    expect(field(r, 'pp-0-blood_type').maxLength).toBe(8);
    // Repeated labels say whose they are to screen readers.
    expect(r.target.querySelector('label[for="pp-0-doctor-phone"]')?.textContent?.replace(/\s+/g, ' ').trim()).toBe("Phone of person 1's doctor");
  });

  it('saves answers as typed, tidies them when left, takes the name as the heading, and keeps no empty profile', async () => {
    const r = await open(People, 'people', blank('philadelphia-renters-4'));
    const name = field(r, 'pp-1-name');
    type(name, '  Ana ', false);
    expect(r.app.plan!.input.people[1]!.profile).toEqual({ name: '  Ana ' });
    expect(r.target.querySelector('#pp-1-title')?.textContent).toBe('Ana');
    name.dispatchEvent(new Event('blur'));
    flushSync();
    expect(r.app.plan!.input.people[1]!.profile).toEqual({ name: 'Ana' });
    await until(() => saved(r).input.people[1]!.profile?.name === 'Ana', 'the answer to be saved in this browser');
    type(field(r, 'pp-1-doctor-phone'), '555-0101');
    expect(r.app.plan!.input.people[1]!.profile).toEqual({ name: 'Ana', doctor: { phone: '555-0101' } });
    type(name, '');
    type(field(r, 'pp-1-doctor-phone'), '');
    expect('profile' in r.app.plan!.input.people[1]!).toBe(false);
    expect(r.target.querySelector('#pp-1-title')?.textContent).toBe('Person 2 (adult)');
  });

  it('gives where a person spends the day a kind for their age, and asks the six things about it', async () => {
    const plan = blank('philadelphia-renters-4');
    const child = plan.input.people.findIndex((p) => p.age_band === 'child');
    const r = await open(People, 'people', plan);
    openAll(r);
    type(field(r, `pp-${child}-place-name`), 'Sample Elementary');
    expect(r.app.plan!.input.people[child]!.profile?.place).toEqual({ kind: 'school', name: 'Sample Elementary' });
    const card = r.target.querySelector(`#person-${child + 1}`)!;
    const school = [...card.querySelectorAll('label.choice')].find((l) => shown(l) === 'School')!.querySelector('input')!;
    expect(school.checked).toBe(true);
    const childcare = [...card.querySelectorAll('label.choice')].find((l) => shown(l) === 'Child care')!.querySelector('input')!;
    childcare.click();
    flushSync();
    for (const [id, text] of [
      ['address', '1 School Lane'],
      ['phone', '555-0150'],
      ['plan', 'Keeps children inside until a parent comes'],
      ['pickup', 'Parents and Rosa, with photo ID'],
      ['safest_spot', 'The gym'],
    ] as const) {
      type(field(r, `pp-${child}-place-${id}`), text);
    }
    expect(r.app.plan!.input.people[child]!.profile?.place).toEqual({
      kind: 'childcare',
      name: 'Sample Elementary',
      address: '1 School Lane',
      phone: '555-0150',
      plan: 'Keeps children inside until a parent comes',
      pickup: 'Parents and Rosa, with photo ID',
      safest_spot: 'The gym',
    });
  });

  it('adds and removes medicines, a legend per row, up to 12, and moves focus with them', async () => {
    const r = await open(People, 'people', blank('chicago-student-zero-budget-1'));
    openAll(r);
    expect(r.target.querySelectorAll('#pp-0-med-0')).toHaveLength(0);
    for (let i = 0; i < 12; i++) {
      button(r, 'Add a medicine').click();
      await tick();
      flushSync();
    }
    const rows = r.target.querySelectorAll('fieldset[id^="pp-0-med-"]');
    expect(rows).toHaveLength(12);
    expect(shown(rows[0]!.querySelector('legend')!)).toBe('Medicine 1 for person 1');
    expect([...r.target.querySelectorAll('button')].some((b) => b.textContent?.includes('Add a medicine'))).toBe(false);
    expect(said(r)).toContain('That is 12 medicines, the most the binder keeps for one person.');
    await until(() => document.activeElement?.id === 'pp-0-medications-11-name', 'focus in the new row');
    type(field(r, 'pp-0-medications-0-name'), 'Inhaler');
    type(field(r, 'pp-0-medications-0-dose'), '2 puffs');
    expect(shown(rows[0]!.querySelector('legend')!)).toBe('Inhaler (medicine 1 for person 1)');
    button(r, 'Remove Medicine 12 for person 1').click();
    await tick();
    flushSync();
    expect(r.target.querySelectorAll('fieldset[id^="pp-0-med-"]')).toHaveLength(11);
    await until(() => document.activeElement?.id === 'pp-0-med-add', 'focus back on "Add a medicine"');
    for (let i = 10; i >= 1; i--) {
      button(r, `Remove Medicine ${i + 1} for person 1`).click();
      await tick();
    }
    flushSync();
    expect(r.app.plan!.input.people[0]!.profile).toEqual({ medications: [{ name: 'Inhaler', dose: '2 puffs' }] });
  });

  it('never blocks: "Skip for now" moves on without marking the step answered; Continue marks it', async () => {
    const r = await open(People, 'people', requiredOnly('philadelphia-renters-4'));
    const skip = link(r, 'Skip for now');
    expect(skip.getAttribute('href')).toBe('#/places');
    expect(r.text()).toContain('Next: Your places (optional)');
    skip.click();
    await until(() => r.router.current.id === 'places', 'the next step');
    expect(r.app.plan!.progress.completed).not.toContain('people');
    r.router.go('people');
    flushSync();
    button(r, 'Continue').click();
    flushSync();
    expect(r.router.current.id).toBe('places');
    expect(r.app.plan!.progress.completed).toContain('people');
  });

  it('keeps the plan working with profiles in it, and the plan echoes them', async () => {
    const r = await open(People, 'people', blank('philadelphia-renters-4'));
    openAll(r);
    type(field(r, 'pp-0-name'), 'Ana Sample');
    type(field(r, 'pp-0-allergies'), 'Penicillin');
    button(r, 'Add a medicine').click();
    await tick();
    flushSync();
    type(field(r, 'pp-0-medications-0-name'), 'Blood pressure tablet');
    await until(() => !!r.app.result.output && r.app.result.output.prepare_markdown.includes('Blood pressure tablet') && !r.app.pending, 'the plan to echo the profile');
    const md = r.app.result.output!.prepare_markdown;
    expect(md).toContain('Ana Sample (person 1, adult)');
    expect(md).toContain('Allergies: Penicillin');
    expect(r.app.result.error).toBeUndefined();
  });

  it('is accessible, empty and filled in', async () => {
    for (const plan of [blank('philadelphia-renters-4'), fixture('philadelphia-renters-4'), savedFor(detroit)]) {
      const r = await open(People, 'people', plan);
      openAll(r);
      button(r, 'Add a medicine').click();
      await tick();
      flushSync();
      await noAxeViolations(r, 'your people');
    }
  }, 60_000);

  it('invites a person with no plan to start one', async () => {
    const r = await open(People, 'people', null);
    expect(r.text()).toContain("You haven't started a plan on this device yet");
  });
});

describe('step 5 leads into the optional steps', () => {
  it('says Continue, names the optional step next, and marks step 5 answered', async () => {
    const plan = requiredOnly('philadelphia-renters-4');
    plan.progress.completed = ['where', 'who', 'travel', 'money'];
    const r = await open(Have, 'have', plan);
    expect(r.text()).toContain('Step 5 of 8');
    expect(r.text()).not.toContain('Step 5 of 8 · optional');
    expect([...r.target.querySelectorAll('.interview-nav a')].map((a) => a.textContent?.trim())).not.toContain('Skip for now');
    expect(r.text()).toContain('Next: Your people (optional)');
    button(r, 'Continue').click();
    flushSync();
    expect(r.router.current.id).toBe('people');
    expect(r.app.plan!.progress.completed).toContain('have');
  });
});

// ---------------------------------------------------------------------------------------------
// Step 7
// ---------------------------------------------------------------------------------------------

describe('Step 7, Your places', () => {
  it('has the four cards, and every v2 question moved here in its own words', async () => {
    for (const plan of [fixture('philadelphia-renters-4'), fixture('chicago-student-zero-budget-1'), savedFor(detroit)]) {
      const r = await open(Places, 'places', plan);
      expect(r.target.querySelector('h1')?.textContent).toBe('Your places');
      expect(r.text()).toContain('Step 7 of 8 · optional');
      expect([...r.target.querySelectorAll('section h2')].map((h) => h.textContent)).toEqual([
        'Your home',
        'Meeting places and staying in touch',
        'Your neighbourhood',
        'Getting out',
      ]);
      const labels = [...r.target.querySelectorAll('label:not(.choice), legend')].map(shown);
      for (const v2 of [
        'Someone out of the area everyone checks in with',
        'Where to meet near home',
        'Where to meet outside the neighbourhood',
        'Numbers to know by heart',
        'What each person does at work or school',
        'The safest spot at home',
        'The safest spot at work or school',
        'Where you would go',
        'Two ways out',
        'First way out',
        'Second way out',
        'Gas shut-off',
        'Main water shut-off',
        'Electrical panel or main breaker',
        'Neighbours who check on you, and whom you check on',
      ]) {
        expect(labels, v2).toContain(v2);
      }
      for (const v3 of ['Street address', 'Electricity', 'Gas', 'Water', 'Outage number', 'Policy number', 'Landlord or mortgage company', 'Where the emergency kit is', 'Where the documents are', 'Where the cash is', 'Where the spare keys are', 'Nearest hospital with an emergency room', 'Urgent care', 'The pharmacy you use', 'Where the community opens a shelter', 'County emergency management office', 'How you get local alerts']) {
        expect(labels, v3).toContain(v3);
      }
      expect(slips(r.text())).toEqual([]);
    }
  });

  it('asks only what fits the household: children and a car', async () => {
    let r = await open(Places, 'places', blank('chicago-student-zero-budget-1'));
    expect(r.target.querySelector('#fp-school')).toBeNull();
    expect(r.target.querySelector('#fp-roadside')).toBeNull();
    r = await open(Places, 'places', blank('philadelphia-renters-4'));
    expect(r.target.querySelector('#fp-school')).not.toBeNull();
    expect(r.target.querySelector('#fp-roadside')).not.toBeNull();
  });

  it('shows what the household wrote in v2, word for word', async () => {
    const r = await open(Places, 'places', savedFor(detroit));
    expect(field(r, 'fp-contact-name').value).toBe('Cousin Tanya in Columbus');
    expect(field(r, 'fp-meet-near').value).toBe('The front steps of the church on our corner');
    expect(field(r, 'fp-route-1').value).toBe('A ride with Mr. Ellis on I-75 south');
    expect(field(r, 'fp-number-1').value).toBe('555-0141');
  });

  it('saves v2 and v3 answers as typed, tidies them when left, and the plan echoes them', async () => {
    const r = await open(Places, 'places', blank('philadelphia-renters-4'));
    const near = field(r, 'fp-meet-near');
    expect(near.maxLength).toBe(300);
    type(near, '  The corner mailbox ', false);
    expect(r.app.plan!.input.family_plan).toEqual({ meeting_place_near: '  The corner mailbox ' });
    near.dispatchEvent(new Event('blur'));
    flushSync();
    expect(r.app.plan!.input.family_plan).toEqual({ meeting_place_near: 'The corner mailbox' });
    type(field(r, 'pl-home-address'), ' 12 Sample St, Unit 2 ');
    type(field(r, 'pl-home-electric_utility-phone'), '555-0120');
    type(field(r, 'pl-neighbourhood-hospital-name'), 'Sample General Hospital');
    type(field(r, 'pl-neighbourhood-hospital-address'), '1 Health Way');
    expect(r.app.plan!.input.family_plan).toEqual({
      meeting_place_near: 'The corner mailbox',
      home: { address: '12 Sample St, Unit 2', electric_utility: { phone: '555-0120' } },
      neighbourhood: { hospital: { name: 'Sample General Hospital', address: '1 Health Way' } },
    });
    await until(() => !!r.app.result.output && r.app.result.output.prepare_markdown.includes('Sample General Hospital'), 'the plan to echo the answers');
    const md = r.app.result.output!.prepare_markdown;
    expect(md).toContain('The corner mailbox');
    expect(md).toContain('Address: 12 Sample St, Unit 2');
    // Clearing everything leaves no family plan behind.
    for (const id of ['fp-meet-near', 'pl-home-address', 'pl-home-electric_utility-phone', 'pl-neighbourhood-hospital-name', 'pl-neighbourhood-hospital-address']) type(field(r, id), '');
    expect(r.app.plan!.input.family_plan).toBeUndefined();
    await until(() => saved(r).input.family_plan === undefined, 'the empty plan to be saved');
  });

  it('keeps up to five numbers by heart', async () => {
    const r = await open(Places, 'places', blank('philadelphia-renters-4'));
    for (let i = 0; i < 5; i++) {
      button(r, 'Add a number').click();
      flushSync();
      await tick();
    }
    expect(r.target.querySelectorAll('[id^="fp-number-"]')).toHaveLength(5);
    expect(said(r)).toContain('That is the most the wallet card holds.');
    type(field(r, 'fp-number-0'), ' 555-0199 ');
    expect(r.app.plan!.input.family_plan?.numbers_by_heart?.[0]).toBe('555-0199');
    button(r, 'Remove number 5').click();
    flushSync();
    expect(r.target.querySelectorAll('[id^="fp-number-"]')).toHaveLength(4);
  });

  it('opens at one card, and the old family-plan addresses land on the card that holds their questions', async () => {
    let r = await open(Places, 'places/home', fixture('philadelphia-renters-4'));
    await until(() => document.activeElement?.id === 'pl-home-title', 'focus on the home card');
    r = await open(Places, 'family/leave', fixture('philadelphia-renters-4'));
    expect(r.router.current).toEqual({ id: 'places', param: 'leave' });
    expect(window.location.hash).toBe('#/places/leave');
    await until(() => document.activeElement?.id === 'pl-leave-title', 'focus on the getting-out card');
  });

  it('is accessible', async () => {
    for (const plan of [blank('philadelphia-renters-4'), fixture('philadelphia-renters-4'), savedFor(detroit)]) {
      const r = await open(Places, 'places', plan);
      openAll(r);
      await noAxeViolations(r, 'your places');
    }
  }, 60_000);
});

// ---------------------------------------------------------------------------------------------
// Step 8
// ---------------------------------------------------------------------------------------------

describe('Step 8, Contacts, pets, vehicles and documents', () => {
  it('has the trusted circle and lawyer as in v2, then pets, vehicles and documents', async () => {
    const r = await open(Contacts, 'contacts', fixture('philadelphia-renters-4'));
    expect(r.target.querySelector('h1')?.textContent).toBe('Contacts, pets, vehicles and documents');
    expect(r.text()).toContain('Step 8 of 8 · optional');
    expect([...r.target.querySelectorAll('section h2')].map((h) => h.textContent)).toEqual([
      'Your trusted circle',
      'A lawyer',
      'Pets and animals',
      'Vehicles',
      'Documents and money',
      'Put it on paper',
    ]);
    expect(said(r)).toContain('when you save your plan to a file you can protect it with a passphrase');
    expect(slips(r.text())).toEqual([]);
  });

  it('keeps up to four people in the trusted circle, with what each holds', async () => {
    const r = await open(Contacts, 'contacts', blank('philadelphia-renters-4'));
    for (let i = 0; i < 4; i++) {
      button(r, 'Add someone').click();
      flushSync();
      await tick();
    }
    expect(r.target.querySelectorAll('.circle__person')).toHaveLength(4);
    expect(said(r)).toContain('That is four people, the most the plan keeps.');
    type(field(r, 'fp-circle-0-name'), 'Rosa');
    expect(shown(r.target.querySelectorAll('.circle__person')[0]!.querySelector('legend')!)).toBe('What Rosa holds for you');
    const key = [...r.target.querySelectorAll('.circle__person')[0]!.querySelectorAll('label.choice')].find((l) => l.textContent?.includes('A spare key'))!.querySelector('input')!;
    key.click();
    flushSync();
    expect(r.app.plan!.input.family_plan?.trusted_circle?.[0]).toEqual({ name: 'Rosa', holds: ['spare_key'] });
    button(r, 'Remove person 2 from your trusted circle').click();
    flushSync();
    expect(r.app.plan!.input.family_plan?.trusted_circle).toHaveLength(3);
  });

  it('adds animals (up to 8) and vehicles (up to 4), each with its own fields, and removes them', async () => {
    const r = await open(Contacts, 'contacts', blank('philadelphia-renters-4'));
    for (let i = 0; i < 8; i++) {
      button(r, 'Add an animal').click();
      await tick();
    }
    flushSync();
    expect(r.target.querySelectorAll('fieldset[id^="ct-pet-"]')).toHaveLength(8);
    expect(said(r)).toContain('That is 8 animals, the most the binder keeps.');
    type(field(r, 'ct-pets-0-name'), 'Biscuit');
    type(field(r, 'ct-pets-0-kind'), 'Dog');
    type(field(r, 'ct-pets-0-vet-phone'), '555-0160');
    type(field(r, 'ct-pets-0-microchip'), '985 000 000');
    expect(shown(r.target.querySelector('#ct-pet-0 legend')!)).toBe('Biscuit (animal 1)');
    for (let i = 7; i >= 1; i--) {
      button(r, `Remove Animal ${i + 1}`).click();
      await tick();
    }
    flushSync();
    expect(r.app.plan!.input.family_plan?.pets).toEqual([{ name: 'Biscuit', kind: 'Dog', vet: { phone: '555-0160' }, microchip: '985 000 000' }]);
    for (let i = 0; i < 4; i++) {
      button(r, 'Add a vehicle').click();
      await tick();
    }
    flushSync();
    expect(r.target.querySelectorAll('fieldset[id^="ct-vehicle-"]')).toHaveLength(4);
    expect(said(r)).toContain('That is 4 vehicles, the most the binder keeps.');
    type(field(r, 'ct-vehicles-1-description'), 'Blue 2016 hatchback');
    type(field(r, 'ct-vehicles-1-plate'), 'ABC-1234');
    expect(field(r, 'ct-vehicles-1-plate').maxLength).toBe(20);
    type(field(r, 'ct-vehicles-1-policy_number'), 'P-555');
    expect(r.app.plan!.input.family_plan?.vehicles?.[1]).toEqual({ description: 'Blue 2016 hatchback', plate: 'ABC-1234', policy_number: 'P-555' });
  });

  it('keeps only the last four digits of an account, even when a whole number is pasted', async () => {
    const r = await open(Contacts, 'contacts', blank('philadelphia-renters-4'));
    button(r, 'Add an account').click();
    await tick();
    flushSync();
    type(field(r, 'ct-documents-accounts-0-institution'), 'First Sample Bank');
    const last4 = field(r, 'ct-documents-accounts-0-last4');
    type(last4, '4000 1234 5678 9010', false);
    expect(last4.value).toBe('9010');
    expect(r.app.plan!.input.family_plan?.documents?.accounts?.[0]).toEqual({ institution: 'First Sample Bank', last4: '9010' });
    await until(() => saved(r).input.family_plan?.documents?.accounts?.[0]?.last4 === '9010', 'the account to be saved');
    expect(r.storage.getItem(STORAGE_KEY)).not.toContain('4000');
    type(last4, 'none', false);
    expect(last4.value).toBe('');
    expect(r.app.plan!.input.family_plan?.documents?.accounts?.[0]).toEqual({ institution: 'First Sample Bank' });
    button(r, 'Add a policy').click();
    await tick();
    flushSync();
    type(field(r, 'ct-documents-policies-0-kind'), 'Life');
    type(field(r, 'ct-documents-where_copies'), 'With Rosa');
    expect(r.app.plan!.input.family_plan?.documents).toEqual({
      accounts: [{ institution: 'First Sample Bank' }],
      policies: [{ kind: 'Life' }],
      where_copies: 'With Rosa',
    });
  });

  it('points a household with no animals or vehicles to the steps that ask about them', async () => {
    const r = await open(Contacts, 'contacts', blank('chicago-student-zero-budget-1'));
    expect(r.target.querySelector('#ct-pet-add')).toBeNull();
    expect(r.target.querySelector('#ct-vehicle-add')).toBeNull();
    expect(r.target.querySelector('#fp-animals')).toBeNull();
    expect(said(r)).toContain('No pets or animals on Who is in your household');
    expect(said(r)).toContain('No vehicles on How you get around');
  });

  it('puts it on paper: the wallet cards and the binder, and the household-plan step marked done', async () => {
    const r = await open(Contacts, 'contacts', fixture('philadelphia-renters-4'));
    expect(link(r, 'Print wallet cards').getAttribute('href')).toBe('#/binder/wallet-cards');
    expect(link(r, 'See your binder').getAttribute('href')).toBe('#/binder');
    button(r, 'Mark "Household plan and contact cards" as done').click();
    flushSync();
    expect(r.app.plan!.purchases.map((p) => p.item_id)).toEqual(['plan_family_contacts']);
    await until(() => r.text().includes('is done on your plan'), 'the step to show as done');
  });

  it('ends the interview: Continue says "See your risks", and skipping goes there too', async () => {
    const r = await open(Contacts, 'contacts', requiredOnly('philadelphia-renters-4'));
    expect(link(r, 'Skip for now').getAttribute('href')).toBe('#/risks');
    expect(r.text()).toContain('Next: Your risks');
    button(r, 'See your risks').click();
    flushSync();
    expect(r.router.current.id).toBe('risks');
    expect(r.app.plan!.progress.completed).toContain('contacts');
  });

  it('opens at one card (an old family-plan address too)', async () => {
    const r = await open(Contacts, 'family/circle', fixture('philadelphia-renters-4'));
    expect(window.location.hash).toBe('#/contacts/circle');
    await until(() => document.activeElement?.id === 'fp-circle-title', 'focus on the trusted circle');
  });

  it('is accessible, empty and with a row of each kind', async () => {
    const r = await open(Contacts, 'contacts', blank('philadelphia-renters-4'));
    await noAxeViolations(r, 'contacts, empty');
    for (const add of ['Add someone', 'Add an animal', 'Add a vehicle', 'Add an account', 'Add a policy']) {
      button(r, add).click();
      await tick();
    }
    flushSync();
    await noAxeViolations(r, 'contacts, with rows');
  }, 60_000);
});

// ---------------------------------------------------------------------------------------------
// Tabs, links and old addresses
// ---------------------------------------------------------------------------------------------

describe('the tabs after v0.3.0', () => {
  it('are Your answers, Risks, Prepare, Binder, Keep it up, Learn and About', async () => {
    const r = await open(SiteHeader, 'binder', fixture('philadelphia-renters-4'));
    expect([...r.target.querySelectorAll('nav a')].map((a) => a.textContent?.trim())).toEqual(['Your answers', 'Risks', 'Prepare', 'Binder', 'Keep it up', 'Learn', 'About']);
    expect(r.target.querySelector('nav a[href="#/binder"]')?.getAttribute('aria-current')).toBe('page');
    expect(r.target.querySelector('nav a[href="#/prepare"]')).not.toBeNull();
  });

  it('"Your answers" opens the first step still to answer: a required one, then an optional one', async () => {
    const plan = fixture('philadelphia-renters-4');
    plan.progress.completed = ['where', 'who', 'money', 'have', 'people'];
    let r = await open(SiteHeader, 'risks', plan);
    expect(link(r, 'Your answers').getAttribute('href')).toBe('#/travel');
    r = await open(SiteHeader, 'people', requiredOnly('philadelphia-renters-4'));
    expect(link(r, 'Your answers').getAttribute('href')).toBe('#/people');
    expect(link(r, 'Your answers').getAttribute('aria-current')).toBe('page');
    r = await open(SiteHeader, 'risks', fixture('philadelphia-renters-4'));
    expect(link(r, 'Your answers').getAttribute('href')).toBe('#/where');
  });

  it('Prepare says what it is for, links plan steps to the optional steps, and prints its sheet', async () => {
    const print = vi.spyOn(window, 'print').mockImplementation(() => {});
    const r = await open(PlanScreen, 'prepare', fixture('philadelphia-renters-4'));
    expect(r.target.querySelector('h1')?.textContent).toBe('Prepare: what to do before');
    expect(said(r)).toContain('The things to do before anything happens: free steps, what to buy and when, decisions to make, and money to set aside.');
    expect(link(r, 'Add your people and places for the binder').getAttribute('href')).toBe('#/people');
    const sheet = r.target.querySelector('.prepare-sheet')!;
    expect(sheet.classList.contains('print-only')).toBe(true);
    expect(sheet.innerHTML).toBe('');
    button(r, 'Print your preparation plan').click();
    await until(() => print.mock.calls.length > 0, 'the print window');
    expect(r.target.querySelector('.prepare-page')?.classList.contains('printing')).toBe(true);
    // The engine's Prepare sheet, through the site's Markdown renderer.
    expect(sheet.querySelector('h2')?.textContent).toBe(r.app.result.output!.prepare_markdown.match(/^# (.+)$/m)![1]);
    expect(sheet.querySelector('script, img, iframe')).toBeNull();
    window.dispatchEvent(new Event('afterprint'));
    flushSync();
    expect(sheet.innerHTML).toBe('');
    // The browser's own Print fills it too.
    window.dispatchEvent(new Event('beforeprint'));
    flushSync();
    expect(sheet.querySelector('h2')).not.toBeNull();
    window.dispatchEvent(new Event('afterprint'));
  });

  it('the old Plan address opens Prepare', async () => {
    const r = await open(PlanScreen, 'plan', fixture('philadelphia-renters-4'));
    expect(r.router.current).toEqual({ id: 'prepare' });
    expect(window.location.hash).toBe('#/prepare');
  });

  it('the binder opens at the wallet cards, from the new address and from the old one', async () => {
    for (const route of ['binder/wallet-cards', 'packet/wallet-cards']) {
      const r = await open(Packet, route, savedFor(detroit));
      expect(r.target.querySelector('h1')?.textContent).toBe('Your binder');
      expect(window.location.hash).toBe('#/binder/wallet-cards');
      await until(() => (document.activeElement?.textContent ?? '').includes('Wallet cards'), 'focus on the wallet cards');
      expect(r.target.querySelector('.packet-section.is-cards')?.querySelectorAll('blockquote')).toHaveLength(detroit.people.length);
    }
  });

  it('Start: "Continue your plan" still goes to Prepare once the five required steps are done, and points at the optional steps', async () => {
    const plan = requiredOnly('philadelphia-renters-4');
    let r = await open(Start, '', plan);
    expect(link(r, 'Continue your plan').getAttribute('href')).toBe('#/prepare');
    expect(link(r, 'Add your people and places for the binder').getAttribute('href')).toBe('#/people');
    expect(said(r)).toContain('Three optional steps after them add your people, places and contacts for the binder.');
    // A household that left off on the old Plan tab continues on Prepare.
    const old = requiredOnly('philadelphia-renters-4');
    old.progress.last = '#/plan';
    r = await open(Start, '', old);
    expect(link(r, 'Continue your plan').getAttribute('href')).toBe('#/prepare');
    const midway = fixture('philadelphia-renters-4');
    midway.progress.completed = ['where', 'who'];
    r = await open(Start, '', midway);
    expect(link(r, 'Continue your plan').getAttribute('href')).toBe('#/travel');
  });
});

// ---------------------------------------------------------------------------------------------
// Saving and opening a protected plan file (§7)
// ---------------------------------------------------------------------------------------------

/** Watch downloads: each saved file's text, in order. */
function watchDownloads(): Promise<string>[] {
  const files: Promise<string>[] = [];
  vi.spyOn(URL, 'createObjectURL').mockImplementation((blob) => {
    files.push((blob as Blob).text());
    return 'blob:test';
  });
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
  return files;
}

/** Choose `text` as the file in the screen's hidden file picker. */
function chooseFile(r: Rendered, text: string) {
  const input = r.target.querySelector('input[type="file"]') as HTMLInputElement;
  Object.defineProperty(input, 'files', { value: [new File([text], 'ready-reckoner-plan.json', { type: 'application/json' })], configurable: true });
  // A real file picker's change event bubbles; Svelte listens for it at the root.
  input.dispatchEvent(new Event('change', { bubbles: true }));
}

function sensitive(): SavedPlan {
  const plan = blank('philadelphia-renters-4');
  plan.input.people[0]!.profile = { name: 'Ana Sample', allergies: 'Penicillin' };
  plan.input.family_plan = { home: { address: '12 Sample St' } };
  return plan;
}

describe('saving a copy', () => {
  it('saves a plan with no sensitive answers as a plain file at once, as before', async () => {
    const files = watchDownloads();
    const r = await open(Maintain, 'maintain', blank('philadelphia-renters-4'));
    button(r, 'Save a copy of your plan').click();
    flushSync();
    expect(r.target.querySelector('dialog[open]')).toBeNull();
    expect(files).toHaveLength(1);
    const file = JSON.parse(await files[0]!) as SavedPlan;
    expect(file.format).toBe('ready-reckoner-plan');
    expect(file.version).toBe(2);
    expect(file.input).toEqual(withoutOptional(FIXTURES['philadelphia-renters-4']));
    expect(said(r)).toContain('Keep it somewhere safe: it can hold names, phone numbers, medical details, insurance IDs and your address.');
  });

  it('offers a passphrase, on by default, once the plan holds sensitive answers, and checks it', async () => {
    const files = watchDownloads();
    const r = await open(Maintain, 'maintain', sensitive());
    button(r, 'Save a copy of your plan').click();
    flushSync();
    const dialog = r.target.ownerDocument.querySelector('dialog[open]') as HTMLDialogElement;
    expect(dialog).not.toBeNull();
    expect(said(dialog)).toContain('The file will hold names, phone numbers, medical details, insurance IDs and your address.');
    const protect = [...dialog.querySelectorAll('label.choice')].find((l) => l.textContent?.includes('Protect this file with a passphrase'))!.querySelector('input')!;
    expect(protect.checked).toBe(true);
    expect(said(dialog)).toContain('Nobody can recover a forgotten passphrase, not even Ready Reckoner. Keep your printed binder as the backup.');
    const [pass, again] = [...dialog.querySelectorAll('input[type="password"]')] as HTMLInputElement[];
    type(pass!, 'short', false);
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    flushSync();
    await tick();
    expect(said(dialog)).toContain('Use at least 8 characters.');
    expect(files).toHaveLength(0);
    type(pass!, 'blue river lamp', false);
    type(again!, 'blue river lam', false);
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    flushSync();
    await tick();
    expect(said(dialog)).toContain('The two passphrases are not the same.');
    type(again!, 'blue river lamp', false);
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    await until(() => files.length === 1, 'the protected file', 10_000);
    const text = await files[0]!;
    const file = JSON.parse(text) as { format: string; kdf: { iterations: number } };
    expect(file.format).toBe(ENCRYPTED_FORMAT);
    expect(file.kdf.iterations).toBe(600_000);
    expect(text).not.toContain('Penicillin');
    expect(text).not.toContain('12 Sample St');
    await until(() => said(r).includes('Saved a protected copy as ready-reckoner-plan.json'), 'the saved message');
    expect(pass!.value).toBe('');
  }, 30_000);

  it('saves it plain after a one-sentence warning when the box is unticked', async () => {
    const files = watchDownloads();
    const r = await open(Maintain, 'maintain', sensitive());
    button(r, 'Save a copy of your plan').click();
    flushSync();
    const dialog = r.target.ownerDocument.querySelector('dialog[open]') as HTMLDialogElement;
    const protect = dialog.querySelector('input[type="checkbox"]') as HTMLInputElement;
    protect.click();
    flushSync();
    expect(dialog.querySelectorAll('input[type="password"]')).toHaveLength(0);
    expect(said(dialog)).toContain('Without a passphrase, anyone who gets the file can read everything in it.');
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    await until(() => files.length === 1, 'the plain file');
    const file = JSON.parse(await files[0]!) as SavedPlan;
    expect(file.input.family_plan?.home?.address).toBe('12 Sample St');
    await until(() => said(r).includes('It is not protected, so keep it somewhere safe.'), 'the saved message');
  });
});

describe('opening a protected file', () => {
  it('asks for the passphrase, says plainly when it is wrong, and opens with the right one (Keep it up)', async () => {
    const plan = sensitive();
    const text = await protectedExportText(plan, 'blue river lamp');
    const r = await open(Maintain, 'maintain', blank('chicago-student-zero-budget-1'));
    chooseFile(r, text);
    await until(() => !!r.target.ownerDocument.querySelector('dialog[open]'), 'the passphrase dialog');
    const dialog = r.target.ownerDocument.querySelector('dialog[open]') as HTMLDialogElement;
    expect(said(dialog)).toContain('Nobody can recover a forgotten passphrase, not even Ready Reckoner; if it is lost, your printed binder is your copy.');
    const pass = dialog.querySelector('input[type="password"]') as HTMLInputElement;
    type(pass, 'blue river lamb', false);
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    await until(() => said(dialog).includes('That passphrase does not open this file.'), 'the wrong-passphrase message', 10_000);
    expect(r.app.plan!.input.location).toEqual(FIXTURES['chicago-student-zero-budget-1'].location);
    type(pass, 'blue river lamp', false);
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    // A plan is already open here, so the household confirms the replacement first.
    await until(() => [...r.target.ownerDocument.querySelectorAll('dialog[open] button')].some((b) => b.textContent === 'Open the file' && !b.closest('form')), 'the replace question', 10_000);
    const confirm = [...r.target.ownerDocument.querySelectorAll('dialog[open] button')].find((b) => b.textContent === 'Open the file' && !b.closest('form')) as HTMLButtonElement;
    confirm.click();
    flushSync();
    expect(r.app.plan!.input.people[0]!.profile).toEqual({ name: 'Ana Sample', allergies: 'Penicillin' });
    expect(r.app.plan!.input.family_plan?.home?.address).toBe('12 Sample St');
  }, 30_000);

  it('opens a protected file from the Start page too', async () => {
    const text = await protectedExportText(sensitive(), 'blue river lamp');
    const r = await open(Start, '', null);
    chooseFile(r, text);
    await until(() => !!r.target.ownerDocument.querySelector('dialog[open]'), 'the passphrase dialog');
    const dialog = r.target.ownerDocument.querySelector('dialog[open]') as HTMLDialogElement;
    type(dialog.querySelector('input[type="password"]') as HTMLInputElement, 'blue river lamp', false);
    (dialog.querySelector('button[type="submit"]') as HTMLButtonElement).click();
    await until(() => !!r.app.plan, 'the plan to open', 10_000);
    expect(r.app.plan!.input.family_plan?.home?.address).toBe('12 Sample St');
    expect(r.router.current.id).toBe('prepare');
  }, 30_000);

  it('still opens a plain file, and a version-1 file from v0.2.0, without asking anything', async () => {
    const old = { ...fixture('chicago-student-zero-budget-1'), version: 1 };
    const r = await open(Start, '', null);
    chooseFile(r, JSON.stringify(old));
    await until(() => !!r.app.plan, 'the plan to open');
    expect(r.target.ownerDocument.querySelector('dialog[open]')).toBeNull();
    expect(r.app.plan!.version).toBe(2);
    expect(r.app.plan!.input).toEqual(FIXTURES['chicago-student-zero-budget-1']);
  });
});
