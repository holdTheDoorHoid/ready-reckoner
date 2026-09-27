/**
 * The maps feature's public face (DESIGN-DELTA-v3 §9), for the workstreams that place it:
 *
 * - **web-interview3** (persistence and step 7): `MapsState` and `checkMapsState` for
 *   `SavedPlan.maps`, `mapsHoldLocation` for the passphrase rule, `deleteMapsDatabase` (or the
 *   database name `MAPS_DB`) for "Forget everything", and
 *   `components/maps/PinMapButton.svelte` for the Getting out card.
 * - **web-binder** (the binder screen and PDF): `components/maps/MapsPanel.svelte` (moved from the
 *   Packet screen), `components/maps/MapFigure.svelte` for each `map_slot`, and `getMapImage`,
 *   `mapLegend` and `mapRecordsFor` for the PDF.
 */
export { type ComposedMap, type LayerStatus, type LegendKey, type LegendRow } from './compose';
export { hasChildren, MAP_SLOT_FIXTURES, type MapLocation, type MapSlotBlock, type MapSlotKind, SLOT_KINDS, suggestedLayers } from './slots';
export { MAP_ORIGINS, RECIPIENTS, SURGE_NOTE } from './sources';
export { checkMapsState, emptyMapsState, hasPins, type MapLayers, type MapsState, mapsHoldLocation } from './state';
export { clearMaps, deleteMapsDatabase, getMapImage, mapLegend, type MapRecord, mapRecordsFor, MAPS_DB, type MapsOwner, type MapsStatus } from './store';
