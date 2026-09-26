/**
 * Getting the data to the WebAssembly engine, in three parts that load at different times
 * (docs/ENGINE-API.md, "Loading"):
 *
 * - `core()`: `data/manifest.json`, then every core file except the ZIP tables, fetched at once and
 *   handed over one by one with the county list last. Started as soon as the engine is up.
 * - `zip()`: the ZIP tables, when a ZIP code is typed or a saved plan has one.
 * - `map()`: the county outlines, when a map is shown.
 *
 * Each part is fetched from the site's own origin with `?v=<pack_version>`, handed to the engine
 * (which checks every file against the manifest's sha256), and loaded once; a second call returns
 * the same promise. A failure is remembered until `retry()`. Progress is counted in the manifest's
 * byte sizes as the bytes arrive, so a progress line can move smoothly.
 *
 * No manifest (HTTP 404, or the dev server's HTML page in its place) means the site was built
 * without data: every part resolves at once and the engine plans with its built-in sample counties.
 */
import type { Engine } from './index';
import type { Manifest } from './data-files';
import { dataUrl, MANIFEST_PATH, MAP_FILE, startupFiles, zipFiles } from './data-files';

export type PartName = 'core' | 'zip' | 'map';
/** `none`: the site has no data files, so there is nothing to load. */
export type Phase = 'idle' | 'loading' | 'ready' | 'failed' | 'none';

export interface PartStatus {
  phase: Phase;
  /** Bytes received so far and in all, by the manifest's sizes (uncompressed). */
  bytesLoaded: number;
  bytesTotal: number;
  /** Why it failed, in plain words. */
  error?: string;
}

export interface LoaderStatus {
  core: PartStatus;
  zip: PartStatus;
  map: PartStatus;
  /** The loaded manifest's `pack_version`. */
  packVersion?: string;
}

export interface LoaderOptions {
  /** Defaults to the browser's `fetch`. */
  fetch?: (url: string) => Promise<Response>;
}

const idle = (): PartStatus => ({ phase: 'idle', bytesLoaded: 0, bytesTotal: 0 });

/** Read a response body, reporting bytes as they arrive (when the browser can stream it). */
async function readBody(response: Response, onBytes: (n: number) => void): Promise<Uint8Array> {
  const reader = response.body?.getReader?.();
  if (!reader) {
    const bytes = new Uint8Array(await response.arrayBuffer());
    onBytes(bytes.length);
    return bytes;
  }
  const chunks: Uint8Array[] = [];
  let length = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    length += value.length;
    onBytes(value.length);
  }
  const out = new Uint8Array(length);
  let at = 0;
  for (const c of chunks) {
    out.set(c, at);
    at += c.length;
  }
  return out;
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export class PackLoader {
  readonly #engine: Engine;
  readonly #base: string;
  readonly #fetch: (url: string) => Promise<Response>;
  readonly #listeners = new Set<(status: LoaderStatus) => void>();
  #manifest: Promise<Manifest | null> | undefined;
  #manifestFailed = false;
  #parts: Partial<Record<PartName, Promise<unknown>>> = {};
  /** Data URLs fetched so far (for warming the service worker's cache after it takes over). */
  readonly fetched: string[] = [];
  status: LoaderStatus = { core: idle(), zip: idle(), map: idle() };

  constructor(engine: Engine, base: string, options: LoaderOptions = {}) {
    this.#engine = engine;
    this.#base = base;
    this.#fetch = options.fetch ?? ((url: string) => fetch(url));
  }

  /** Called with a fresh status object after every change; returns the unsubscribe function. */
  subscribe(listener: (status: LoaderStatus) => void): () => void {
    this.#listeners.add(listener);
    listener(this.status);
    return () => this.#listeners.delete(listener);
  }

  #set(part: PartName, patch: Partial<PartStatus>, packVersion?: string): void {
    const next: PartStatus = { ...this.status[part], ...patch };
    if (patch.phase && patch.phase !== 'failed') delete next.error;
    this.status = { ...this.status, [part]: next, ...(packVersion ? { packVersion } : {}) };
    for (const l of this.#listeners) l(this.status);
  }

  /** The manifest, loaded into the engine; null when the site has no data. */
  manifest(): Promise<Manifest | null> {
    if (!this.#manifest) {
      const pending = this.#loadManifest();
      this.#manifest = pending;
      pending.catch(() => {
        if (this.#manifest === pending) this.#manifestFailed = true;
      });
    }
    return this.#manifest;
  }

  async #loadManifest(): Promise<Manifest | null> {
    const url = dataUrl(this.#base, MANIFEST_PATH);
    let response: Response;
    try {
      response = await this.#fetch(url);
    } catch {
      throw new Error('The county data could not be downloaded. Check your internet connection.');
    }
    const type = response.headers.get('content-type') ?? '';
    if (response.status === 404 || (response.ok && type.includes('text/html'))) return null;
    if (!response.ok) throw new Error(`The data manifest did not download (HTTP ${response.status}).`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    let manifest: unknown;
    try {
      manifest = JSON.parse(new TextDecoder().decode(bytes));
    } catch {
      throw new Error('The data manifest is damaged: it is not JSON.');
    }
    const packs = (manifest as { packs?: unknown } | null)?.packs;
    if (typeof packs !== 'object' || packs === null) throw new Error('The data manifest is damaged: it lists no packs.');
    const loaded = await this.#engine.load_pack(MANIFEST_PATH, bytes);
    if (!loaded.ok) throw new Error(loaded.error.message);
    this.fetched.push(url);
    return manifest as Manifest;
  }

  /** Everything the engine needs to plan for a county: the manifest and the core pack without the ZIP tables. */
  core(): Promise<void> {
    return this.#part('core', async (manifest) => {
      await this.#files('core', manifest, startupFiles(manifest));
    }) as Promise<void>;
  }

  /** The ZIP tables (after the core files; downloads start at once). */
  zip(): Promise<void> {
    return this.#part('zip', async (manifest) => {
      await this.#files('zip', manifest, zipFiles(manifest), () => this.core());
    }) as Promise<void>;
  }

  /** The county outlines: handed to the engine (which checks them) and returned parsed. Null without data. */
  map(): Promise<unknown> {
    return this.#part('map', async (manifest) => {
      const files = manifest.packs.geo?.files.some((f) => f.path === MAP_FILE) ? [MAP_FILE] : [];
      if (files.length === 0) return null;
      const [bytes] = await this.#files('map', manifest, files, () => this.core());
      return JSON.parse(new TextDecoder().decode(bytes));
    });
  }

  /** Forget failed parts (and a failed manifest), so the next call tries again. */
  retry(): void {
    if (this.#manifestFailed) {
      this.#manifest = undefined;
      this.#manifestFailed = false;
    }
    for (const part of ['core', 'zip', 'map'] as const) {
      if (this.status[part].phase === 'failed') {
        delete this.#parts[part];
        this.#set(part, { phase: 'idle', bytesLoaded: 0 });
      }
    }
  }

  /** True when a call that needs `part` can go straight to the engine. */
  isReady(part: PartName): boolean {
    const phase = this.status[part].phase;
    return phase === 'ready' || phase === 'none';
  }

  #part(name: PartName, run: (manifest: Manifest) => Promise<unknown>): Promise<unknown> {
    const existing = this.#parts[name];
    if (existing) return existing;
    const started = (async () => {
      this.#set(name, { phase: 'loading' });
      try {
        const manifest = await this.manifest();
        if (!manifest) {
          this.#set(name, { phase: 'none' });
          return null;
        }
        const result = await run(manifest);
        this.#set(name, { phase: 'ready' }, manifest.pack_version);
        return result;
      } catch (e) {
        this.#set(name, { phase: 'failed', error: message(e) });
        throw e;
      }
    })();
    // A failure is reported through the status and to whoever awaits it; never as unhandled.
    started.catch(() => undefined);
    this.#parts[name] = started;
    return started;
  }

  /**
   * Fetch files all at once and hand them to the engine in the order given, after `before`
   * (another part that must be in first). Throws on the first file that fails.
   */
  async #files(part: PartName, manifest: Manifest, paths: readonly string[], before?: () => Promise<unknown>): Promise<Uint8Array[]> {
    const sizes = new Map(Object.values(manifest.packs).flatMap((p) => p.files.map((f) => [f.path, f.bytes ?? 0] as const)));
    const bytesTotal = paths.reduce((s, p) => s + (sizes.get(p) ?? 0), 0);
    let bytesLoaded = 0;
    this.#set(part, { bytesTotal, bytesLoaded: 0 });
    const downloads = paths.map(async (path) => {
      const url = dataUrl(this.#base, path, manifest.pack_version);
      let response: Response;
      try {
        response = await this.#fetch(url);
      } catch {
        throw new Error(`The data file ${path} could not be downloaded. Check your internet connection.`);
      }
      if (!response.ok) throw new Error(`The data file ${path} did not download (HTTP ${response.status}).`);
      const expected = sizes.get(path) ?? 0;
      let seen = 0;
      const bytes = await readBody(response, (n) => {
        // Never count past a file's size, so a wrong size in the manifest cannot overshoot.
        const add = Math.max(0, Math.min(n, expected - seen));
        seen += n;
        bytesLoaded += add;
        this.#set(part, { bytesLoaded: Math.min(bytesLoaded, bytesTotal) });
      });
      this.fetched.push(url);
      return bytes;
    });
    // Every download is awaited below in order; this only keeps a later failure from being
    // reported as unhandled while an earlier file is still loading.
    for (const d of downloads) d.catch(() => undefined);
    if (before) await before();
    const out: Uint8Array[] = [];
    for (const [i, path] of paths.entries()) {
      const bytes = await downloads[i]!;
      const result = await this.#engine.load_pack(path, bytes);
      if (!result.ok) throw new Error(result.error.message);
      out.push(bytes);
    }
    this.#set(part, { bytesLoaded: bytesTotal });
    return out;
  }
}
