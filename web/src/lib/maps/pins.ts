/**
 * Placing pins and drawing the ways out on the pin map (DESIGN-DELTA-v3 §9.4), as plain functions
 * the component calls: pick what to place, then click the map (or put it at the cross in the
 * middle, which works from the keyboard and needs no dragging, WCAG 2.2 "Dragging movements").
 * A pin moves to where it is placed; a way out gains a point with each click, with undo and clear.
 */
import type { LatLon } from '../../engine/types';
import { checkLatLon, emptyMapsState, MAX_ROUTE_POINTS, type MapsState, PIN_IDS, type PinId } from './state';

/** What the next click places. */
export type Tool = PinId | 'route1' | 'route2';

export const TOOLS: readonly Tool[] = [...PIN_IDS, 'route1', 'route2'];

export const TOOL_LABELS: Record<Tool, string> = {
  home: 'Home',
  meeting_near: 'Meeting place near home',
  meeting_far: 'Meeting place outside your neighbourhood',
  where_go: 'Where you would go',
  route1: 'Way out 1 (a line)',
  route2: 'Way out 2 (a line)',
};

/** The letters on the pins, as the printed maps print them. */
export const PIN_MARKS: Record<PinId, string> = { home: 'H', meeting_near: 'M1', meeting_far: 'M2', where_go: 'D' };

/** The editor's working copy: both ways out kept in their places even while one is empty. */
export interface Draft {
  pins: Partial<Record<PinId, LatLon>>;
  routes: [LatLon[], LatLon[]];
}

export function toDraft(maps: MapsState | undefined): Draft {
  const pins: Partial<Record<PinId, LatLon>> = {};
  for (const id of PIN_IDS) if (maps?.[id]) pins[id] = { ...maps[id]! };
  return { pins, routes: [[...(maps?.routes[0] ?? [])], [...(maps?.routes[1] ?? [])]] };
}

/** Back to `MapsState`: a way out with fewer than two points is not a line and is left out. */
export function fromDraft(draft: Draft, base: MapsState | undefined): MapsState {
  const out: MapsState = { ...(base ?? emptyMapsState()), routes: draft.routes.filter((r) => r.length >= 2).map((r) => r.map((p) => ({ ...p }))) };
  for (const id of PIN_IDS) {
    const p = draft.pins[id] ? checkLatLon(draft.pins[id]) : undefined;
    if (p) out[id] = p;
    else delete out[id];
  }
  return out;
}

const routeIndex = (tool: 'route1' | 'route2') => (tool === 'route1' ? 0 : 1);

/** A click (or "put it at the cross") with a tool: the pin moves there, or the way out gains a point. */
export function place(draft: Draft, tool: Tool, at: LatLon): Draft {
  const p = checkLatLon(at);
  if (!p) return draft;
  if (tool === 'route1' || tool === 'route2') {
    const i = routeIndex(tool);
    if (draft.routes[i].length >= MAX_ROUTE_POINTS) return draft;
    const routes: [LatLon[], LatLon[]] = [[...draft.routes[0]], [...draft.routes[1]]];
    routes[i].push(p);
    return { ...draft, routes };
  }
  return { ...draft, pins: { ...draft.pins, [tool]: p } };
}

/** A pin dragged to a new spot. */
export function movePin(draft: Draft, id: PinId, at: LatLon): Draft {
  return place(draft, id, at);
}

export function removePin(draft: Draft, id: PinId): Draft {
  const pins = { ...draft.pins };
  delete pins[id];
  return { ...draft, pins };
}

export function undoPoint(draft: Draft, tool: 'route1' | 'route2'): Draft {
  const routes: [LatLon[], LatLon[]] = [[...draft.routes[0]], [...draft.routes[1]]];
  routes[routeIndex(tool)].pop();
  return { ...draft, routes };
}

export function clearRoute(draft: Draft, tool: 'route1' | 'route2'): Draft {
  const routes: [LatLon[], LatLon[]] = [[...draft.routes[0]], [...draft.routes[1]]];
  routes[routeIndex(tool)] = [];
  return { ...draft, routes };
}

/** "Placed", "Not placed yet", "3 points" — for the list beside the map. */
export function toolState(draft: Draft, tool: Tool): string {
  if (tool === 'route1' || tool === 'route2') {
    const n = draft.routes[routeIndex(tool)].length;
    return n === 0 ? 'Not drawn yet' : n === 1 ? '1 point: click again to draw the line' : `${n} points`;
  }
  return draft.pins[tool] ? 'Placed' : 'Not placed yet';
}
