/**
 * What the pin map needs from an interactive map library, as a small interface: the Leaflet
 * implementation (`leaflet-view.ts`) is loaded as a separate chunk the first time a pin map opens
 * (and precached by the service worker, so it loads offline), and tests pass a stub instead.
 */
import type { LatLon } from '../../engine/types';
import type { PinId } from './state';

export interface ViewPin {
  id: PinId;
  at: LatLon;
  /** "H", "M1", "M2", "D". */
  mark: string;
  /** "Home pin", for the tooltip and screen readers. */
  label: string;
}

export interface MapView {
  setPins(pins: readonly ViewPin[]): void;
  /** Both ways out; a line with one point shows as a dot. */
  setRoutes(routes: readonly (readonly LatLon[])[]): void;
  getCenter(): LatLon;
  setView(center: LatLon, zoom?: number): void;
  destroy(): void;
}

export interface ViewOptions {
  center: LatLon;
  zoom: number;
  /** A click (or tap) on the map. */
  onClick(at: LatLon): void;
  /** A pin dragged to a new spot. */
  onPinMoved(id: PinId, at: LatLon): void;
  /** The OpenStreetMap tiles did not load, so the Census map is shown instead. */
  onFallback(): void;
}

export type CreateView = (element: HTMLElement, options: ViewOptions) => Promise<MapView>;

/** The real map: Leaflet, loaded on first use. */
export const createLeafletView: CreateView = async (element, options) => (await import('./leaflet-view')).leafletView(element, options);

/** The pin map's closest zoom: tiles at 17 are about 230 m across, enough to place a pin to a metre or two. */
export const PIN_MAP_MAX_ZOOM = 17;
