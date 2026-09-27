/**
 * Every place the maps feature may ask for anything (DESIGN-DELTA-v3 §9.3), as settings in one
 * module: the endpoints, the request limits each owner's policy sets, who runs each service and
 * what it receives. The content security policy (`vite-plugins/pwa.ts`) is built from
 * `MAP_ORIGINS`, so a request to anywhere not listed here is blocked by the browser, and the
 * consent screen names the recipients from `RECIPIENTS`.
 *
 * Nothing here is contacted until the household presses "Fetch maps" on the consent screen (or
 * "Search" after the address warning). A first visit, and a household that never presses it,
 * makes no request to any of these (web/scripts/e2e.mjs checks).
 *
 * ## What was checked, and when
 *
 * Every endpoint and policy below was fetched on **2026-09-27** from the development machine
 * (requests identified as "ReadyReckoner-dev-verification", a handful each). Findings:
 *
 * - **OpenStreetMap tiles** (`https://tile.openstreetmap.org/{z}/{x}/{y}.png`): answered 200,
 *   a 256×256 PNG, `access-control-allow-origin: *`, `cache-control: max-age=21164,
 *   stale-while-revalidate=604800`. The OSMF Tile Usage Policy
 *   (https://operations.osmfoundation.org/policies/tiles/, read 2026-09-27) requires: exactly that
 *   URL over HTTPS; visible attribution; a valid User-Agent (a browser's own is fine); **from web
 *   pages, a valid Referer, and no Referrer-Policy that prevents it** (this site's page-wide
 *   `no-referrer` would, so the map requests carry `strict-origin-when-cross-origin`, which
 *   sends only the site's address, never the page or its `#/…` route); caching by the HTTP headers
 *   (no `no-cache` request headers); **no bulk downloading, no prefetching, no "download for
 *   offline" or "save area for later" features**, and it says offline use is not permitted on
 *   tile.openstreetmap.org. What this module lets the app do: tiles only on the household's press,
 *   only the ones inside the three map frames and the pin map's view, no look-ahead and no zoom
 *   stacks (at most 20 per map, so at most 60 a press, well under the 250 cap of §9.3), through
 *   the browser's HTTP cache; the service worker never stores them. The composed map images, not
 *   tiles, are kept for the printed binder, with the credit printed beside each map. **Open
 *   question for the owner:** the policy does not say whether keeping a composed print image
 *   counts as "offline use"; see the report. The Attribution Guidelines
 *   (https://osmfoundation.org/wiki/Licence/Attribution_Guidelines, adopted 2021-06-25) ask printed
 *   maps and PDFs to carry the credit beside the map with the URL `openstreetmap.org/copyright`
 *   printed out; one credit per document is enough for several static images.
 * - **Census TIGERweb** (the base-map fallback, D5): `tigerWMS_PhysicalFeatures/MapServer/export`
 *   answered 200 with a PNG at zooms 9, 12 and 16 over Philadelphia; CORS reflects the site's
 *   origin; "Source: U.S. Census Bureau", January 1, 2026 vintage, public domain; no published
 *   usage limit found. Usable at zooms 9–16: primary roads from 1:1,400,000 (zoom 9), secondary
 *   roads from 1:650,000 (zoom 10), local roads with street names from 1:90,000 (zoom 13), rail
 *   from 1:42,000 (zoom 14), water at every zoom. (`tigerWMS_Current` holds only boundaries.)
 * - **Overpass API** (`https://overpass-api.de/api/interpreter`): the first try answered 504
 *   ("the server is probably too busy") after 14 s, a retry 200 in 1.9 s with JSON and
 *   `Access-Control-Allow-Origin: *`; the app's own query for a Philadelphia household later took
 *   9.2 s (83 kB). Its entry in the OSM wiki's list of public instances
 *   (https://wiki.openstreetmap.org/wiki/Overpass_API, read 2026-09-27) says: "Nowadays this server
 *   is overloaded … do not expect high reliability"; under 10,000 queries and 1 GB a day is fine
 *   for one-off use, and regular use should divide that by 100; **an app's usage counts as the sum
 *   of all its users' requests**; requests should carry a User-Agent or Referer identifying the
 *   app; after a 429 or 406, pause 30 seconds before asking again; commercial use should pay or
 *   self-host. The FOSSGIS manual
 *   (https://dev.overpass-api.de/overpass-doc/en/preface/commons.html) also names "an app for more
 *   than just OSM mappers relying on the public instances as backend" as problematic. The
 *   fallback the delta names, `overpass.kumi.systems`, answered 200 after 54 s (data as of
 *   2026-07-15) in the morning and nothing at all in 90 s in the afternoon; the wiki lists it as
 *   **renamed to `overpass.private.coffee`** ("feel free to use our service in any project, there
 *   is no rate limit"), which was also not answering (even `/api/status`) when checked. What the
 *   app does: one query a press covering both map boxes, the second instance only if the first
 *   fails (so at most two a press), `[timeout:25]`, 45 s in the browser for each, and a server
 *   that answered 429, 406 or 504 is not asked again for 30 seconds.
 * - **FEMA National Flood Hazard Layer**: `https://hazards.fema.gov/arcgis/rest/services/public/NFHL/MapServer`
 *   is the address FEMA documents (hazards.fema.gov/femaportal/wps/portal/NFHLWMS, "NFHL
 *   (effective data only)"); the older `/gis/nfhl/` path answers 404. Layer **28, "Flood Hazard
 *   Zones"**, draws at 1:36,112 and closer, so only on the neighbourhood map (zoom 15–16). A
 *   transparent PNG export of Eastwick, Philadelphia answered in 0.37 s; CORS reflects the site's
 *   origin; the service supports dynamic layers, which is how the export is drawn in two exact
 *   colours (see `floodExportUrl`). Public; federal.
 * - **USDA Forest Service Wildfire Hazard Potential** (2023 edition, 270 m, classes 1 very low
 *   to 5 very high, 6 non-burnable, 7 water): the old `apps.fs.usda.gov` services answer 403
 *   "migrated to IIPP"; the service is now
 *   `https://imagery.geoplatform.gov/iipp/rest/services/Fire_Aviation/USFS_EDW_RMRS_WildfireHazardPotentialClassified/ImageServer`.
 *   `exportImage` with a Colormap rendering rule returns only the high and very high classes
 *   (tested over Missoula, Montana); CORS reflects the site's origin. Public; federal.
 * - **Storm surge**: NOAA's National Hurricane Center publishes its National Storm Surge Risk
 *   Maps (version 4, 2025) as GeoTIFF downloads and an ArcGIS Online viewer. The viewer's layers
 *   are "TilesOnly" caches hosted by Esri (`tiles.arcgis.com/.../Storm_Surge_HazardMaps_Category3_v3`,
 *   titled "v4"; no `_v4` service exists), and the "Tile Layer" links on
 *   https://www.nhc.noaa.gov/nationalsurge/ are commented out of the page. That is not a stable
 *   public image service, and it would add a private company as a recipient, so **surge is
 *   left out** (`SURGE = null`) and each map says so, pointing to the state's evacuation-zone page.
 * - **Nominatim** (`https://nominatim.openstreetmap.org/search`, `format=jsonv2`): answered 200 with
 *   `access-control-allow-origin: *` for a public landmark. The Nominatim Usage Policy
 *   (https://operations.osmfoundation.org/policies/nominatim/, read 2026-09-27): an **absolute
 *   maximum of one request a second, counted for the whole application across all its users**; a
 *   valid Referer or User-Agent identifying the application; attribution; **autocomplete is
 *   forbidden**; results cached, never the same query sent over and over; **no personal data**;
 *   the service must be switchable at their request; code written by a language model must follow
 *   the whole policy. What the app does: a request only on an explicit "Search" press after the
 *   D6 warning (which asks for the street address only, no names), at most one a second (a quicker
 *   press waits), the same text answered from memory, at most three results, US only, the
 *   site's address as the Referer, and the credit beside the results.
 *
 * Every request is made with `credentials: 'omit'` (no cookies) and the default cache mode (the
 * HTTP cache is honoured, never bypassed), and carries the site's address (the `Origin` header of
 * a CORS request, and an origin-only Referer), never the page's path.
 */

/** The date the endpoints and policies above were last checked. */
export const CHECKED_ON = '2026-09-27';

/**
 * How a request identifies the site. Every request here is a CORS request, which carries the site's
 * address in its `Origin` header whatever the referrer policy, so `no-referrer` would hide nothing
 * from these services. They all get `strict-origin-when-cross-origin`: the site's address as the
 * Referer (which the OSMF and Overpass rules ask for), never the page's path or its `#/…` route.
 */
export type ReferrerPolicyValue = 'no-referrer' | 'strict-origin-when-cross-origin';

export interface TileSource {
  /** `{z}`, `{x}`, `{y}` are replaced; OSMF asks for exactly this address. */
  url: string;
  tileSize: number;
  maxZoom: number;
  referrerPolicy: ReferrerPolicyValue;
  /** Drawn on the image. */
  credit: string;
  /** Printed beside the map: the Attribution Guidelines ask printed maps for the URL. */
  creditLong: string;
  creditUrl: string;
}

export interface ExportSource {
  /** The ArcGIS REST `export` (MapServer) or `exportImage` (ImageServer) address. */
  url: string;
  referrerPolicy: ReferrerPolicyValue;
  credit: string;
}

export interface OverpassSource {
  /** Tried in order; the second only when the first fails. */
  urls: readonly string[];
  referrerPolicy: ReferrerPolicyValue;
  /** Seconds, sent as `[timeout:…]`. */
  timeoutS: number;
  /** The browser gives up after this many milliseconds. */
  clientTimeoutMs: number;
  /** At most this many queries a press, counting a retry on the second instance. */
  maxQueries: number;
  /** A server that answered 429, 406 or 504 is not asked again for this long (the main instance's rule for 429 and 406). */
  busyPauseMs: number;
  credit: string;
}

export interface SearchSource {
  url: string;
  referrerPolicy: ReferrerPolicyValue;
  /** At most one request in this many milliseconds (the policy's absolute maximum is one a second). */
  minIntervalMs: number;
  maxResults: number;
  /** ISO 3166 codes the search is limited to. */
  countryCodes: string;
  credit: string;
}

/** The OpenStreetMap standard tiles (D5): the base map for the pin map and the three maps. */
export const OSM_TILES: TileSource = {
  url: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
  tileSize: 256,
  maxZoom: 19,
  referrerPolicy: 'strict-origin-when-cross-origin',
  credit: '© OpenStreetMap contributors',
  creditLong: 'Map data © OpenStreetMap contributors, openstreetmap.org/copyright',
  creditUrl: 'https://www.openstreetmap.org/copyright',
};

/** The public-domain fallback base map (D5): one export image per map instead of tiles. */
export const CENSUS_BASE: ExportSource & { layers: string; layersNearby: string } = {
  url: 'https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/tigerWMS_PhysicalFeatures/MapServer/export',
  referrerPolicy: 'strict-origin-when-cross-origin',
  credit: 'Base map: U.S. Census Bureau TIGERweb',
  // Roads and their labels, rail, and water (lines, areas, labels). Each layer draws only at the
  // scales the service allows, so one list serves every zoom from 9 to 16.
  layers: 'show:1,2,3,4,5,6,7,10,11,12,13',
  layersNearby: 'show:1,2,3,4,5,6,7,8,10,11,12,13',
};

/** OpenStreetMap places (pharmacies, fire stations…) for the legend. */
export const OVERPASS: OverpassSource = {
  urls: ['https://overpass-api.de/api/interpreter', 'https://overpass.private.coffee/api/interpreter'],
  referrerPolicy: 'strict-origin-when-cross-origin',
  timeoutS: 25,
  clientTimeoutMs: 45_000,
  maxQueries: 2,
  busyPauseMs: 30_000,
  credit: 'Places © OpenStreetMap contributors, openstreetmap.org/copyright',
};

/** FEMA flood zones, layer 28 of the National Flood Hazard Layer. */
export const FLOOD: ExportSource & { layer: number; maxScale: number } = {
  url: 'https://hazards.fema.gov/arcgis/rest/services/public/NFHL/MapServer/export',
  referrerPolicy: 'strict-origin-when-cross-origin',
  credit: 'Flood zones: FEMA National Flood Hazard Layer',
  layer: 28,
  /** The layer draws only at this scale or closer (1:36,112). */
  maxScale: 36111.909643,
};

/** USDA Forest Service Wildfire Hazard Potential, 2023, classified. */
export const WILDFIRE: ExportSource & { high: number; veryHigh: number } = {
  url: 'https://imagery.geoplatform.gov/iipp/rest/services/Fire_Aviation/USFS_EDW_RMRS_WildfireHazardPotentialClassified/ImageServer/exportImage',
  referrerPolicy: 'strict-origin-when-cross-origin',
  credit: 'Wildfire hazard: USDA Forest Service Wildfire Hazard Potential (2023)',
  high: 4,
  veryHigh: 5,
};

/**
 * Storm-surge zones: none. NOAA offers no stable public image service for its storm-surge risk
 * maps (see the note at the top), so the maps say so instead of drawing them. Set this to an
 * export source if NOAA publishes one, and add its origin to `MAP_ORIGINS` through it.
 */
export const SURGE = null as ExportSource | null;

/** Why surge is not drawn, in the words the maps and the consent screen use. */
export const SURGE_NOTE =
  'Storm-surge zones are not on these maps: NOAA does not offer them as a map service this app can use.';

/** OpenStreetMap's search (D6), only after the warning and only on a "Search" press. */
export const NOMINATIM: SearchSource = {
  url: 'https://nominatim.openstreetmap.org/search',
  referrerPolicy: 'strict-origin-when-cross-origin',
  minIntervalMs: 1100,
  maxResults: 3,
  countryCodes: 'us',
  credit: 'Search by OpenStreetMap Nominatim, © OpenStreetMap contributors',
};

/** Tiles fetched in one press, across the three maps (§9.3; the frames need at most 60). */
export const MAX_TILES_PER_PRESS = 250;

/** OpenStreetMap's "report a map issue" page (recommended by the tile policy). */
export const FIX_THE_MAP_URL = 'https://www.openstreetmap.org/fixthemap';

function originOf(url: string): string {
  return new URL(url).origin;
}

/**
 * Every origin the maps feature may contact, for the content security policy's `connect-src` and
 * `img-src`. Built from the settings above, so it can never drift from what the code requests.
 */
export const MAP_ORIGINS: readonly string[] = [
  ...new Set(
    [
      OSM_TILES.url.replace(/\{[xyz]\}/g, '0'),
      CENSUS_BASE.url,
      ...OVERPASS.urls,
      FLOOD.url,
      WILDFIRE.url,
      ...(SURGE ? [SURGE.url] : []),
      NOMINATIM.url,
    ].map(originOf),
  ),
];

// ---------------------------------------------------------------------------------------------
// Who sees what (the consent screen, docs/PRIVACY.md)
// ---------------------------------------------------------------------------------------------

/** A layer the consent screen offers, with who receives what. */
export type RecipientId = 'base' | 'places' | 'flood' | 'wildfire';

export interface Recipient {
  id: RecipientId;
  /** The checkbox label. */
  layer: string;
  /** Who runs the service, in plain words. */
  who: string;
  /** One plain sentence: what that recipient receives. */
  receives: string;
  /** The web addresses contacted. */
  origins: readonly string[];
}

/** The recipients in the order the consent screen lists them. */
export const RECIPIENTS: readonly Recipient[] = [
  {
    id: 'base',
    layer: 'Street map',
    who: 'The OpenStreetMap Foundation (UK), which runs the OpenStreetMap map servers. If they do not answer, the U.S. Census Bureau instead.',
    receives:
      'The squares of map to draw: those around your home, your meeting places and where you would go, and any you look at on the pin map. Together they show roughly where those places are.',
    origins: [originOf(OSM_TILES.url.replace(/\{[xyz]\}/g, '0')), originOf(CENSUS_BASE.url)],
  },
  {
    id: 'places',
    layer: 'Nearby places (pharmacies, grocery stores, fire stations, hospitals…)',
    who: 'The Overpass service (FOSSGIS e.V., Germany), which searches OpenStreetMap. If it is busy, a second Overpass server (Private.coffee, formerly kumi.systems) instead.',
    receives: 'Boxes on the map around your home, from your neighbourhood out to your city or county. Never your home’s exact spot.',
    origins: OVERPASS.urls.map(originOf),
  },
  {
    id: 'flood',
    layer: 'Flood zones',
    who: 'FEMA, the Federal Emergency Management Agency.',
    receives: 'The box of your neighbourhood map, about 1.5 km (1 mile) across. Never your home’s exact spot.',
    origins: [originOf(FLOOD.url)],
  },
  {
    id: 'wildfire',
    layer: 'Wildfire hazard',
    who: 'The U.S. Forest Service, through the federal imagery service at geoplatform.gov.',
    receives: 'The box of your city or county map. Never your home’s exact spot.',
    origins: [originOf(WILDFIRE.url)],
  },
];

/** What every recipient also learns, as any website would. */
export const EVERY_RECIPIENT_SEES =
  'Each of these also sees your internet (IP) address, what browser you use and this site’s address, as any website does. None of them is sent your name, your household or your answers.';

/** The address search's recipient (D6). */
export const SEARCH_RECIPIENT = {
  who: 'The OpenStreetMap Foundation (UK), which runs the OpenStreetMap search service (Nominatim).',
  receives: 'The address you type, your internet (IP) address and this site’s address.',
  origins: [originOf(NOMINATIM.url)],
} as const;
