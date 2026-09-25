import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// BASE_PATH is set by the Pages workflow to /ready-reckoner/; local dev serves at /.
export default defineConfig({
  base: process.env.BASE_PATH ?? '/',
  plugins: [svelte()],
  build: { target: 'es2022', sourcemap: true },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
});
