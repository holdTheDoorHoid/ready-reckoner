/**
 * The site's content security policy (docs/DESIGN.md §10), delivered as a `<meta>` tag by
 * `contentSecurityPolicy()` in `pwa.ts`. Kept apart from the build plugins so tests can read it
 * without loading esbuild.
 *
 * Scripts, styles, fonts and data come only from this site. The one exception is the maps'
 * optional, consented services (DESIGN-DELTA-v3 §9.3): exactly the origins in
 * `src/lib/maps/sources.ts` (`MAP_ORIGINS`), in `connect-src` (the composer's requests and the
 * address search) and `img-src` (the pin map's tiles). Nothing requests them before the household
 * presses "Fetch maps" (web/scripts/e2e.mjs checks).
 */
import { MAP_ORIGINS } from '../src/lib/maps/sources';

/**
 * The policy. `'unsafe-inline'` for styles only: Svelte sets style attributes; scripts stay strict.
 * The map services may be fetched (`connect-src`: the composer's requests) and shown as images
 * (`img-src`: the pin map's tiles), and nothing else from outside this site.
 */
export const CSP = [
  "default-src 'self'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "style-src 'self' 'unsafe-inline'",
  ["img-src 'self' data: blob:", ...MAP_ORIGINS].join(' '),
  "font-src 'self'",
  ["connect-src 'self'", ...MAP_ORIGINS].join(' '),
  "worker-src 'self'",
  "manifest-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'none'",
].join('; ');
