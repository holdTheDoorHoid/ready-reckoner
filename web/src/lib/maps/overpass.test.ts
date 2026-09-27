import { describe, expect, it } from 'vitest';

import fixture from './fixtures/overpass-philadelphia.json';
import { AREA_QUOTAS, kindOf, NEIGHBOURHOOD_QUOTAS, overpassQuery, parseOverpass, pickPlaces } from './overpass';
import { emptyMapsState } from './state';
import { type MapLocation, slotFrame } from './slots';
import { frameBox, type LonLatBox } from './tiles';

const HOME = { lat: 39.9364, lon: -75.1534 };
const PHILLY: MapLocation = { county_fips: '42101', county_name: 'Philadelphia', state_abbr: 'PA', state_name: 'Pennsylvania', centroid: { lat: 40.0094, lon: -75.1333 } };
const BOX: LonLatBox = [-75.276, 39.866, -74.963, 40.138];
const maps = { ...emptyMapsState(), home: HOME };
const nbFrame = slotFrame('neighbourhood', maps, PHILLY, BOX);
const areaFrame = slotFrame('area', maps, PHILLY, BOX);

describe('The Overpass query', () => {
  const boxes = { neighbourhood: frameBox(nbFrame), nearby: frameBox({ ...areaFrame, zoom: areaFrame.zoom + 1 }), area: frameBox(areaFrame) };

  it('asks once for every kind, each box as south,west,north,east, with a 25-second timeout', () => {
    const q = overpassQuery(boxes, { children: true, timeoutS: 25 });
    expect(q.startsWith('[out:json][timeout:25];')).toBe(true);
    expect(q.trimEnd().endsWith('out center tags qt;')).toBe(true);
    const [w, s, e, n] = boxes.neighbourhood;
    expect(q).toContain(`nwr["amenity"="pharmacy"](${s.toFixed(5)},${w.toFixed(5)},${n.toFixed(5)},${e.toFixed(5)});`);
    expect(q).toContain('nwr["shop"~"^(supermarket|grocery)$"]');
    expect(q).toContain('nwr["amenity"~"^(school|childcare|kindergarten)$"]');
    expect(q).toContain('nwr["amenity"="hospital"]["emergency"!="no"]');
    // Exactly one union: one query for the whole press.
    expect(q.match(/\(\n/g)).toHaveLength(1);
  });

  it('matches the query built for a Philadelphia household on 2026-09-27, apart from the smaller fire and police box', () => {
    const q = overpassQuery(boxes, { children: true, timeoutS: 25 });
    expect(q).toContain('nwr["amenity"="pharmacy"](39.93146,-75.16193,39.94067,-75.14477);');
    expect(q).toContain('nwr["amenity"="hospital"]["emergency"!="no"](39.79588,-75.46509,40.09068,-74.91577);');
    // Fire stations and police: the middle quarter of the area box.
    const fire = /nwr\["amenity"~"\^\(fire_station\|police\)\$"\]\(([^)]*)\)/.exec(q)?.[1]?.split(',').map(Number) ?? [];
    expect(fire).toHaveLength(4);
    const [fs, fw, fn, fe] = fire as [number, number, number, number];
    expect(fn - fs).toBeCloseTo((40.09068 - 39.79588) / 2, 2);
    expect(fe - fw).toBeCloseTo((-74.91577 - -75.46509) / 2, 2);
  });

  it('leaves schools and child care out for a household without children', () => {
    expect(overpassQuery(boxes, { children: false, timeoutS: 25 })).not.toContain('school');
  });

  it('sends only boxes: the home pin never appears in the query', () => {
    const q = overpassQuery(boxes, { children: true, timeoutS: 25 });
    expect(q).not.toContain(String(HOME.lat));
    expect(q).not.toContain(String(HOME.lon));
    expect(q).not.toMatch(/around/);
  });
});

describe('Reading places', () => {
  it('knows each kind the maps show from its tags', () => {
    expect(kindOf({ amenity: 'hospital', emergency: 'yes' })).toBe('hospital_er');
    expect(kindOf({ amenity: 'hospital' })).toBe('hospital');
    expect(kindOf({ amenity: 'hospital', emergency: 'no' })).toBeUndefined();
    expect(kindOf({ shop: 'chemist', healthcare: 'pharmacy' })).toBe('pharmacy');
    expect(kindOf({ healthcare: 'urgent_care' })).toBe('clinic');
    expect(kindOf({ amenity: 'kindergarten' })).toBe('childcare');
    expect(kindOf({ shop: 'grocery' })).toBe('grocery');
    expect(kindOf({ amenity: 'bench' })).toBeUndefined();
  });

  it('reads the fixture: points and outlines (by their centre), skipping what has no kind or no position', () => {
    const places = parseOverpass(fixture);
    // 28 elements: a bench, an outline with no position and a hospital with no emergency room are skipped.
    expect(places).toHaveLength(25);
    const acme = places.find((p) => p.name === 'Acme');
    expect(acme).toEqual({
      id: 'way/74973040',
      kind: 'grocery',
      name: 'Acme',
      at: { lat: 39.9322757, lon: -75.1623635 },
      address: '1400 East Passyunk Avenue',
      phone: '+1 215-467-2221',
    });
    expect(places.find((p) => p.id === 'relation/20163080')?.kind).toBe('hospital_er');
  });

  it('keeps the first of several phone numbers, and a place with no name stays nameless', () => {
    const unnamed = parseOverpass(fixture).find((p) => p.id === 'node/-2');
    expect(unnamed?.name).toBeUndefined();
    expect(unnamed?.phone).toBe('+1 215 555 0100');
  });

  it('echoes names as tagged, only trimmed and capped', () => {
    const [p] = parseOverpass({ elements: [{ type: 'node', id: 1, lat: 1, lon: 1, tags: { amenity: 'pharmacy', name: `  Dr. Ö'Brien   &   Sons <b>  ${'x'.repeat(100)}` } }] });
    expect(p?.name?.startsWith("Dr. Ö'Brien & Sons <b> x")).toBe(true);
    expect(p?.name).toHaveLength(80);
  });

  it('survives an answer that is not what it should be', () => {
    expect(parseOverpass(null)).toEqual([]);
    expect(parseOverpass({ elements: 'nope' })).toEqual([]);
    expect(parseOverpass({ remark: 'runtime error: Query timed out' })).toEqual([]);
  });
});

describe('Picking places for each map (nearest first, from the fixture)', () => {
  const places = parseOverpass(fixture);

  it('lists the neighbourhood places in legend order, nearest first, a duplicate dropped, named before unnamed', () => {
    const picked = pickPlaces(places, NEIGHBOURHOOD_QUOTAS, nbFrame, HOME, { children: true });
    const rows = picked.map((p) => `${p.kind}: ${p.name ?? '(no name)'}`);
    // The frame is about 1.46 km east to west and 1.02 km north to south: Passyunk Market (550 m
    // south), Company 11 (645 m north) and the South Street police sub-station (740 m north) fall
    // outside it and are left for the area map.
    expect(rows).toEqual([
      'pharmacy: Rite Aid', // 704 East Passyunk Avenue, 449 m
      'pharmacy: Rite Aid', // 501 South 9th Street, 464 m (its duplicate point is dropped)
      'pharmacy: (no name)',
      'grocery: First Oriental Market',
      'grocery: El Paisano Supermarket',
      'fuel: Sunoco',
      'school: Nebinger Elementary School',
      'school: Vare-Washington Elementary School',
      'fire_station: Philadelphia Company 3 Fire Station',
    ]);
    expect(picked[0]?.address).toBe('704 East Passyunk Avenue');
    expect(picked[1]?.address).toBe('501 South 9th Street');
  });

  it('leaves schools out for a household without children', () => {
    expect(pickPlaces(places, NEIGHBOURHOOD_QUOTAS, nbFrame, HOME, { children: false }).some((p) => p.kind === 'school')).toBe(false);
  });

  it('lists hospitals with an emergency room on the area map, and no others when three or more have one', () => {
    const picked = pickPlaces(places, AREA_QUOTAS, areaFrame, HOME, { children: false });
    expect(picked.filter((p) => p.kind === 'hospital_er').map((p) => p.name)).toEqual([
      'Pennsylvania Hospital',
      'Wills Eye Hospital',
      'Thomas Jefferson University Hospital',
      'Methodist Hospital',
      'Hahnemann University Hospital',
    ]);
    expect(picked.some((p) => p.kind === 'hospital')).toBe(false);
    expect(picked.filter((p) => p.kind === 'fire_station')).toHaveLength(2);
    expect(picked.filter((p) => p.kind === 'police')).toHaveLength(2);
  });

  it('fills in a hospital without a listed emergency room only when fewer than three have one', () => {
    const few = [
      { id: 'node/1', kind: 'hospital_er' as const, name: 'General', at: { lat: 39.95, lon: -75.16 } },
      { id: 'node/2', kind: 'hospital' as const, name: 'Community', at: { lat: 39.94, lon: -75.16 } },
    ];
    expect(pickPlaces(few, AREA_QUOTAS, areaFrame, HOME, { children: false }).map((p) => p.name)).toEqual(['General', 'Community']);
  });

  it('never lists a place outside the map', () => {
    const far = [{ id: 'node/9', kind: 'pharmacy' as const, name: 'Far away', at: { lat: 40.3, lon: -75.9 } }];
    expect(pickPlaces(far, NEIGHBOURHOOD_QUOTAS, nbFrame, HOME, { children: false })).toEqual([]);
  });
});
