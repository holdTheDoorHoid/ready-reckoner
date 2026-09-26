import { execSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, searchForWorkspaceRoot } from 'vite';

import { contentSecurityPolicy, serviceWorker } from './vite-plugins/pwa';

const here = fileURLToPath(new URL('.', import.meta.url));

// Which engine the site talks to. `VITE_ENGINE=wasm|mock` decides; when unset, the WebAssembly
// engine is used if `crates/rr-wasm/build-web.sh` has written `public/pkg/`, otherwise the mock.
const engine = process.env.VITE_ENGINE ?? (existsSync(`${here}public/pkg/rr_wasm.js`) ? 'wasm' : 'mock');
if (engine !== 'wasm' && engine !== 'mock') {
  throw new Error(`VITE_ENGINE must be "wasm" or "mock", not "${engine}"`);
}

const pkg = JSON.parse(readFileSync(`${here}package.json`, 'utf8')) as { version: string };
let commit = 'dev';
try {
  commit = execSync('git rev-parse --short HEAD', { cwd: here, stdio: ['ignore', 'pipe', 'ignore'] })
    .toString()
    .trim();
} catch {
  // Not a git checkout (a tarball build): the About screen shows "dev".
}

// BASE_PATH is set by the Pages workflow to /ready-reckoner/; local dev serves at /.
export default defineConfig({
  base: process.env.BASE_PATH ?? '/',
  plugins: [svelte(), serviceWorker(), contentSecurityPolicy()],
  define: {
    __RR_ENGINE__: JSON.stringify(engine),
    __RR_APP_VERSION__: JSON.stringify(`${pkg.version}+${commit}`),
  },
  // Vitest runs components in jsdom, so Svelte must resolve to its browser build.
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  // The Learn articles are the reviewed topic blocks in content/guidance, imported as text; the dev
  // server and vitest may read that one folder outside web/ (a build reads it anyway).
  server: { fs: { allow: [searchForWorkspaceRoot(process.cwd()), `${here}../content/guidance`] } },
  build: { target: 'es2022', sourcemap: true },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    setupFiles: ['src/test/setup.ts'],
    testTimeout: 30_000,
  },
});
