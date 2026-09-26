/**
 * Build-time plugins for the installable, offline site:
 * - `serviceWorker()` writes `dist/sw.js` after the build, listing every built file to precache
 *   and naming the cache after a hash of their contents (so any change ships as a new version).
 * - `contentSecurityPolicy()` adds a CSP to the built `index.html`: scripts only from this site
 *   (plus WebAssembly compilation), no inline scripts, no third-party origins (docs/DESIGN.md §10).
 *   It is left out of the dev server, whose hot reload needs inline scripts and a websocket.
 */
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';

import { build } from 'esbuild';
import type { Plugin, ResolvedConfig } from 'vite';

import { precacheList } from '../src/pwa/sw-core';

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

export function serviceWorker(): Plugin {
  let config: ResolvedConfig;
  return {
    name: 'rr-service-worker',
    apply: 'build',
    configResolved(resolved) {
      config = resolved;
    },
    async closeBundle() {
      const outDir = resolve(config.root, config.build.outDir);
      const files = walk(outDir).map((f) => relative(outDir, f).split(sep).join('/'));
      const precache = precacheList(files);
      const hash = createHash('sha256');
      for (const file of precache) {
        hash.update(file);
        hash.update(readFileSync(join(outDir, file)));
      }
      const version = hash.digest('hex').slice(0, 12);
      const result = await build({
        entryPoints: [resolve(config.root, 'src/pwa/sw.js')],
        bundle: true,
        format: 'iife',
        minify: true,
        target: 'es2020',
        write: false,
        define: {
          __RR_PRECACHE__: JSON.stringify(precache),
          __RR_SW_VERSION__: JSON.stringify(version),
        },
      });
      const code = result.outputFiles[0]?.text;
      if (!code) throw new Error('service worker bundle is empty');
      writeFileSync(join(outDir, 'sw.js'), code);
      config.logger.info(`sw.js: ${precache.length} files precached, version ${version}`);
    },
  };
}

/** The policy. `'unsafe-inline'` for styles only: Svelte sets style attributes; scripts stay strict. */
export const CSP = [
  "default-src 'self'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self'",
  "connect-src 'self'",
  "worker-src 'self'",
  "manifest-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'none'",
].join('; ');

export function contentSecurityPolicy(): Plugin {
  return {
    name: 'rr-csp',
    apply: 'build',
    transformIndexHtml: {
      order: 'post',
      handler(html) {
        return html.replace(
          '<head>',
          `<head>\n    <meta http-equiv="Content-Security-Policy" content="${CSP}" />`,
        );
      },
    },
  };
}
