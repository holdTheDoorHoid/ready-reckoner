/// <reference types="svelte" />
/// <reference types="vite/client" />

/** "wasm" or "mock", fixed at build time by vite.config.ts (VITE_ENGINE, or whether public/pkg exists). */
declare const __RR_ENGINE__: 'wasm' | 'mock';
/** package.json version plus the git commit, for the About screen. */
declare const __RR_APP_VERSION__: string;
