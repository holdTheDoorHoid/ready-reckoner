/**
 * The v0.3.0 addresses (DESIGN-DELTA-v3 §1): the optional steps, Prepare and the binder, and every
 * address from an earlier version redirected to where its content lives now, with the address bar
 * rewritten in place.
 */
import { afterEach, describe, expect, it } from 'vitest';

import { FAMILY_SECTIONS } from './family';
import { FAMILY_REDIRECTS, formatHash, href, isRedirect, isStep, parseHash, ROUTE_IDS, ROUTES, Router, screenFor } from './router.svelte';

describe('the v0.3.0 routes', () => {
  it('has the three optional steps, Prepare and the binder, and no family plan', () => {
    expect(ROUTES.people).toEqual({ path: 'people', title: 'Your people', step: 6, optional: true });
    expect(ROUTES.places).toEqual({ path: 'places', title: 'Your places', step: 7, optional: true });
    expect(ROUTES.contacts).toEqual({ path: 'contacts', title: 'Contacts, pets, vehicles and documents', step: 8, optional: true });
    expect(ROUTES.prepare).toEqual({ path: 'prepare', title: 'Prepare: what to do before' });
    expect(ROUTES.binder).toEqual({ path: 'binder', title: 'Your binder' });
    for (const gone of ['plan', 'packet', 'family']) expect(ROUTE_IDS as readonly string[]).not.toContain(gone);
    expect(['people', 'places', 'contacts'].every((s) => isStep(s as 'people'))).toBe(true);
    expect(isStep('prepare')).toBe(false);
  });

  it('opens the binder at a page, and the optional steps at a card', () => {
    expect(parseHash('#/binder/wallet-cards')).toEqual({ id: 'binder', param: 'wallet-cards' });
    expect(href('binder', 'wallet-cards')).toBe('#/binder/wallet-cards');
    expect(parseHash('#/places/home')).toEqual({ id: 'places', param: 'home' });
    expect(parseHash('#/contacts/circle')).toEqual({ id: 'contacts', param: 'circle' });
    // Other screens still take no second part.
    expect(parseHash('#/people/extra')).toEqual({ id: 'missing' });
    expect(parseHash('#/risks/extra')).toEqual({ id: 'missing' });
  });

  it('sends a problem to the step that asks the question', () => {
    expect(screenFor('people[1].profile.medications[0].dose')).toBe('people');
    expect(screenFor('people[1].commute.distance_km')).toBe('travel');
    expect(screenFor('people[2].access_needs[0]')).toBe('who');
    expect(screenFor('family_plan.routes')).toBe('places');
    expect(screenFor('family_plan.home.address')).toBe('places');
    expect(screenFor('family_plan.neighbourhood.hospital.phone')).toBe('places');
    expect(screenFor('family_plan.trusted_circle[0].holds[0]')).toBe('contacts');
    expect(screenFor('family_plan.lawyer.name')).toBe('contacts');
    expect(screenFor('family_plan.pets[0].vet.name')).toBe('contacts');
    expect(screenFor('family_plan.vehicles[1].plate')).toBe('contacts');
    expect(screenFor('family_plan.documents.accounts[0].last4')).toBe('contacts');
    expect(screenFor('family_plan.who_takes_animals')).toBe('contacts');
    expect(screenFor('housing.cooking')).toBe('where');
    expect(screenFor('finances.benefits[0]')).toBe('money');
    expect(screenFor('dials.rare_opt_in[1]')).toBe('risks');
  });
});

describe('addresses from earlier versions', () => {
  it('open Prepare for the old Plan tab', () => {
    for (const h of ['#/plan', '#/plan/', '#/plan/anything']) {
      expect(parseHash(h), h).toEqual({ id: 'prepare' });
      expect(isRedirect(h), h).toBe(true);
    }
  });

  it('open the binder for the old packet, and its wallet cards for the old wallet-card request', () => {
    expect(parseHash('#/packet')).toEqual({ id: 'binder' });
    expect(parseHash('#/packet/wallet-cards')).toEqual({ id: 'binder', param: 'wallet-cards' });
    for (const section of ['your-risks', 'sources', 'your-family-plan', 'x/y']) expect(parseHash(`#/packet/${section}`), section).toEqual({ id: 'binder' });
    expect(isRedirect('#/packet/wallet-cards')).toBe(true);
  });

  it('open step 7 for the old family plan, and each old part at the card that holds its questions now', () => {
    expect(parseHash('#/family')).toEqual({ id: 'places' });
    expect(parseHash('#/family/nonsense')).toEqual({ id: 'places' });
    expect(Object.keys(FAMILY_REDIRECTS).sort()).toEqual([...FAMILY_SECTIONS].sort());
    const expected: Record<string, string> = {
      contact: '#/places/touch',
      children: '#/places/touch',
      shelter: '#/places/home',
      leave: '#/places/leave',
      home: '#/places/home',
      circle: '#/contacts/circle',
      lawyer: '#/contacts/lawyer',
    };
    for (const [section, hash] of Object.entries(expected)) {
      expect(formatHash(parseHash(`#/family/${section}`)!), section).toBe(hash);
    }
  });

  it('are the only redirects: current addresses and anchors are left alone', () => {
    for (const h of ['#/prepare', '#/binder', '#/binder/wallet-cards', '#/places', '#/people', '#/', '#main', '#/nowhere']) {
      expect(isRedirect(h), h).toBe(false);
    }
  });
});

describe('the router and old addresses', () => {
  let router: Router | undefined;
  afterEach(() => {
    router?.destroy();
    router = undefined;
    window.location.hash = '';
  });

  it('rewrites an old address in the address bar without adding a history entry', async () => {
    window.location.hash = '#/packet/wallet-cards';
    const entries = window.history.length;
    router = new Router(window);
    expect(router.current).toEqual({ id: 'binder', param: 'wallet-cards' });
    expect(window.location.hash).toBe('#/binder/wallet-cards');
    expect(window.history.length).toBe(entries);
    // An old link followed later is rewritten too.
    window.location.hash = '#/family/circle';
    await new Promise((r) => setTimeout(r, 0));
    expect(router.current).toEqual({ id: 'contacts', param: 'circle' });
    expect(window.location.hash).toBe('#/contacts/circle');
    window.location.hash = '#/plan';
    await new Promise((r) => setTimeout(r, 0));
    expect(router.current).toEqual({ id: 'prepare' });
    expect(window.location.hash).toBe('#/prepare');
  });
});
