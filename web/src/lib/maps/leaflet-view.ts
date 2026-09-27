/**
 * The pin map's interactive map: Leaflet 1.9 (BSD-2-Clause) with the OpenStreetMap standard tiles
 * and the Census map as the fallback, in its own lazily loaded chunk.
 *
 * Tile requests follow the OSMF policy (see `sources.ts`): only the tiles in view (Leaflet asks
 * for no others), the site's address as the Referer (`referrerPolicy`, overriding the page-wide
 * `no-referrer`, as the policy requires), no cookies (`crossOrigin: anonymous`, which also lets
 * the composer reuse these tiles from the browser's cache), zoom no closer than 17, and the credit
 * in the corner. After three tiles fail with none loaded, the layer switches to the Census map.
 */
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';

import type { LatLon } from '../../engine/types';
import { censusTileUrl } from './overlays';
import { CENSUS_BASE, OSM_TILES } from './sources';
import { type MapView, PIN_MAP_MAX_ZOOM, type ViewOptions, type ViewPin } from './view';

const OSM_ATTRIBUTION = '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors';

const CensusLayer = L.TileLayer.extend({
  getTileUrl(coords: L.Coords) {
    return censusTileUrl({ x: coords.x, y: coords.y, z: coords.z });
  },
}) as unknown as new (url: string, options?: L.TileLayerOptions) => L.TileLayer;

const toLatLng = (p: LatLon): L.LatLngExpression => [p.lat, p.lon];

export function leafletView(element: HTMLElement, options: ViewOptions): MapView {
  const map = L.map(element, {
    center: toLatLng(options.center),
    zoom: Math.min(options.zoom, PIN_MAP_MAX_ZOOM),
    minZoom: 4,
    maxZoom: PIN_MAP_MAX_ZOOM,
    keyboard: true,
    zoomControl: true,
    attributionControl: true,
  });
  map.attributionControl.setPrefix(false);

  const osm = L.tileLayer(OSM_TILES.url, {
    maxZoom: PIN_MAP_MAX_ZOOM,
    crossOrigin: 'anonymous',
    referrerPolicy: OSM_TILES.referrerPolicy,
    attribution: OSM_ATTRIBUTION,
  });
  const census = new CensusLayer('', {
    maxZoom: PIN_MAP_MAX_ZOOM,
    crossOrigin: 'anonymous',
    referrerPolicy: CENSUS_BASE.referrerPolicy,
    attribution: 'Base map: U.S. Census Bureau',
  });
  let loaded = 0;
  let failed = 0;
  let fellBack = false;
  osm.on('tileload', () => (loaded += 1));
  osm.on('tileerror', () => {
    failed += 1;
    if (!fellBack && loaded === 0 && failed >= 3) {
      fellBack = true;
      map.removeLayer(osm);
      census.addTo(map);
      options.onFallback();
    }
  });
  osm.addTo(map);

  map.on('click', (e: L.LeafletMouseEvent) => options.onClick({ lat: e.latlng.lat, lon: e.latlng.lng }));

  const pins = L.layerGroup().addTo(map);
  const routes = L.layerGroup().addTo(map);

  // The container may get its size after Leaflet starts (a panel opening): measure again.
  requestAnimationFrame(() => map.invalidateSize());

  return {
    setPins(list: readonly ViewPin[]) {
      pins.clearLayers();
      for (const pin of list) {
        const icon = L.divIcon({
          className: 'rr-pin',
          html: `<span class="rr-pin__mark" aria-hidden="true">${pin.mark}</span>`,
          iconSize: [32, 26],
          iconAnchor: [16, 13],
        });
        const marker = L.marker(toLatLng(pin.at), { icon, draggable: true, keyboard: true, title: pin.label, autoPan: true });
        marker.on('dragend', () => {
          const p = marker.getLatLng();
          options.onPinMoved(pin.id, { lat: p.lat, lon: p.lng });
        });
        marker.addTo(pins);
      }
    },
    setRoutes(list) {
      routes.clearLayers();
      list.forEach((route, i) => {
        if (route.length === 1) {
          L.circleMarker(toLatLng(route[0]!), { radius: 5, color: '#111', weight: 2, fillColor: '#fff', fillOpacity: 1, interactive: false }).addTo(routes);
        } else if (route.length > 1) {
          const points = route.map(toLatLng);
          L.polyline(points, { color: '#fff', weight: 8, opacity: 0.9, interactive: false }).addTo(routes);
          L.polyline(points, { color: '#111', weight: 4, dashArray: i === 0 ? undefined : '10 7', interactive: false }).addTo(routes);
        }
      });
    },
    getCenter() {
      const c = map.getCenter();
      return { lat: c.lat, lon: c.lng };
    },
    setView(center, zoom) {
      map.setView(toLatLng(center), Math.min(zoom ?? map.getZoom(), PIN_MAP_MAX_ZOOM));
    },
    destroy() {
      map.remove();
    },
  };
}
