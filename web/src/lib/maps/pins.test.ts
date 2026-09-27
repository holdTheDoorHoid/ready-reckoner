import { describe, expect, it } from 'vitest';

import { clearRoute, fromDraft, movePin, place, removePin, toDraft, toolState, undoPoint } from './pins';
import { emptyMapsState, MAX_ROUTE_POINTS } from './state';

const A = { lat: 39.9364, lon: -75.1534 };
const B = { lat: 39.95, lon: -75.16 };
const C = { lat: 40.0379, lon: -76.3055 };

describe('Placing pins and drawing the ways out', () => {
  it('moves a pin to wherever it is placed, and removes it', () => {
    let d = toDraft(undefined);
    d = place(d, 'home', A);
    expect(d.pins.home).toEqual(A);
    d = place(d, 'home', B);
    expect(d.pins.home).toEqual(B);
    d = movePin(d, 'where_go', C);
    expect(toolState(d, 'where_go')).toBe('Placed');
    d = removePin(d, 'home');
    expect(d.pins.home).toBeUndefined();
    expect(toolState(d, 'home')).toBe('Not placed yet');
  });

  it('adds a point to a way out with each click, with undo and clear', () => {
    let d = toDraft(undefined);
    d = place(d, 'route2', A);
    expect(toolState(d, 'route2')).toBe('1 point: click again to draw the line');
    d = place(d, 'route2', B);
    d = place(d, 'route2', C);
    expect(d.routes[1]).toEqual([A, B, C]);
    expect(d.routes[0]).toEqual([]);
    d = undoPoint(d, 'route2');
    expect(d.routes[1]).toEqual([A, B]);
    expect(toolState(d, 'route2')).toBe('2 points');
    d = clearRoute(d, 'route2');
    expect(toolState(d, 'route2')).toBe('Not drawn yet');
  });

  it('never changes the draft it was given', () => {
    const d = toDraft(undefined);
    const e = place(place(d, 'route1', A), 'home', B);
    expect(d).toEqual({ pins: {}, routes: [[], []] });
    expect(e.pins.home).toEqual(B);
  });

  it('ignores a point that is not on Earth, and stops a line at its limit', () => {
    let d = toDraft(undefined);
    d = place(d, 'home', { lat: 200, lon: 0 });
    expect(d.pins.home).toBeUndefined();
    for (let i = 0; i < MAX_ROUTE_POINTS + 5; i++) d = place(d, 'route1', { lat: 40 + i / 1000, lon: -75 });
    expect(d.routes[0]).toHaveLength(MAX_ROUTE_POINTS);
  });

  it('goes back to the saved shape: a one-point line is not a line, and the layers and date are kept', () => {
    const saved = { ...emptyMapsState(), home: A, fetched_on: '2026-10-01', layers: { base: true, places: false, flood: true, surge: false, wildfire: false } };
    let d = toDraft(saved);
    d = place(d, 'route1', A);
    d = place(d, 'route2', B);
    d = place(d, 'route2', C);
    d = removePin(d, 'home');
    d = place(d, 'meeting_near', B);
    const out = fromDraft(d, saved);
    expect(out).toEqual({ routes: [[B, C]], layers: saved.layers, fetched_on: '2026-10-01', meeting_near: B });
  });

  it('round-trips a saved state untouched', () => {
    const saved = { ...emptyMapsState(), home: A, where_go: C, routes: [[A, B], [A, C]] };
    expect(fromDraft(toDraft(saved), saved)).toEqual(saved);
  });
});
