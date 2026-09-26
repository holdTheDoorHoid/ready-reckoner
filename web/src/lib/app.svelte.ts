/**
 * The app's state: the saved plan (household, dials, check-offs), display preferences, the engine
 * and its catalogue, the data packs as they load, and the latest assessment. Any change to the
 * plan re-runs `assess` after a short pause (100 ms), so the risks and plan screens follow every
 * dial and check-off, and saves the plan to this browser. Old answers that arrive after newer ones
 * are dropped.
 *
 * The app is ready as soon as the engine runs (its catalogue and defaults are built in); the
 * county data keeps loading behind the first screens, `data` says how far it has got, and any
 * answer that needs it simply arrives once it is in.
 */
import { getContext, setContext, untrack } from 'svelte';

import { hasPlace } from '../engine/data-files';
import type { Engine, EngineSource } from '../engine/index';
import { getDataLoader, getEngine, getEngineSource } from '../engine/index';
import type { LoaderStatus, PackLoader } from '../engine/loader';
import type { Catalogue, EngineError, EngineInfo, IsoDate, PlanInput, PlanItem, PlanOutput, TierId } from '../engine/types';
import { localToday } from './format';
import { type CountyShapes, fetchCountyShapes, indexShapes } from './geo';
import * as store from './persistence';

export interface AppOptions {
  /** Where the plan is kept; `null` when the browser allows no storage. Defaults to localStorage. */
  storage?: Storage | null;
  /** Defaults to `getEngine()`. */
  engine?: Engine;
  /** The data loader behind `engine` (defaults to `getDataLoader()` when `engine` is not given). */
  loader?: PackLoader | null;
  /** Pause before re-running `assess`, in milliseconds. */
  delay?: number;
  /** Pause before saving, in milliseconds. */
  saveDelay?: number;
  /** Today's date where the viewer is (the engine never reads the clock). */
  today?: () => IsoDate;
}

export interface AssessResult {
  output?: PlanOutput;
  error?: EngineError;
}

/** What `assess` would say about a plan with no place yet (the new-plan placeholder ZIP code). */
export const NO_PLACE_ERROR: EngineError = {
  code: 'bad_input',
  message: 'Tell us where you live first.',
  details: { problems: [{ code: 'location_missing', field: 'location', message: 'Enter your ZIP code, or search for your county.' }] },
};

export class AppState {
  plan = $state<store.SavedPlan | null>(null);
  prefs = $state<store.Prefs>({ ...store.DEFAULT_PREFS });
  catalogue = $state.raw<Catalogue | null>(null);
  info = $state.raw<EngineInfo | null>(null);
  source = $state.raw<EngineSource | null>(null);
  /** The data packs as they load (null for the mock engine, which needs none). */
  data = $state.raw<LoaderStatus | null>(null);
  result = $state.raw<AssessResult>({});
  pending = $state(false);
  /** The engine and catalogue are loaded. */
  ready = $state(false);
  storageAvailable = $state(true);
  /** What was in storage could not be read (it was left untouched). */
  storageDamaged = $state(false);
  saveFailed = $state(false);
  /** A short message for everyone to see and hear (for example after "Forget everything"). */
  status = $state('');

  /** The household as the engine sees it, or null before a plan is started. */
  engineInput = $derived.by((): PlanInput | null =>
    this.plan ? store.engineInput($state.snapshot(this.plan) as store.SavedPlan) : null,
  );

  readonly today: () => IsoDate;
  engine: Engine | null = null;

  #storage: Storage | null;
  #delay: number;
  #saveDelay: number;
  #engineOption: Engine | undefined;
  #loaderOption: PackLoader | null | undefined;
  #loader: PackLoader | null = null;
  #unsubscribe: (() => void) | undefined;
  #shapes: Promise<CountyShapes | null> | undefined;
  #cleanup: (() => void) | undefined;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #saveTimer: ReturnType<typeof setTimeout> | undefined;
  #seq = 0;
  #lastKey = '';
  #waiters: (() => void)[] = [];

  constructor(options: AppOptions = {}) {
    this.#storage = options.storage === undefined ? store.browserStorage() : options.storage;
    this.#delay = options.delay ?? 100;
    this.#saveDelay = options.saveDelay ?? 250;
    this.#engineOption = options.engine;
    this.#loaderOption = options.loader;
    this.today = options.today ?? (() => localToday());
    this.storageAvailable = this.#storage !== null;
    const loaded = store.loadPlan(this.#storage);
    this.plan = loaded.plan;
    this.storageDamaged = loaded.damaged;
    this.prefs = store.loadPrefs(this.#storage);
  }

  /** Load the engine and catalogue, then start following changes. The data keeps loading behind. */
  async init(): Promise<void> {
    this.engine = this.#engineOption ?? (await getEngine());
    this.#loader = this.#engineOption ? (this.#loaderOption ?? null) : await getDataLoader();
    const [cat, info] = await Promise.all([this.engine.catalogue(), this.engine.engine_info()]);
    if (cat.ok) this.catalogue = cat.value;
    if (info.ok) this.info = info.value;
    this.source = this.#engineOption
      ? { kind: info.ok && info.value.engine_version.startsWith('mock') ? 'mock' : 'wasm' }
      : await getEngineSource();
    if (this.#loader) {
      let last = '';
      this.#unsubscribe = this.#loader.subscribe((status) => {
        this.data = status;
        // Versions, loaded packs and credit lines change as packs arrive.
        const key = `${status.core.phase}|${status.zip.phase}|${status.map.phase}`;
        if (key !== last) {
          last = key;
          void this.refreshInfo();
        }
      });
    }
    this.#cleanup = $effect.root(() => {
      $effect(() => {
        const input = this.engineInput;
        untrack(() => this.#schedule(input));
      });
      $effect(() => {
        const snapshot = this.plan ? ($state.snapshot(this.plan) as store.SavedPlan) : null;
        untrack(() => this.#scheduleSave(snapshot));
      });
      $effect(() => {
        const prefs = { theme: this.prefs.theme, expert: this.prefs.expert };
        untrack(() => {
          store.savePrefs(this.#storage, prefs);
          applyTheme(prefs.theme);
        });
      });
    });
    this.ready = true;
  }

  destroy(): void {
    this.#unsubscribe?.();
    this.#cleanup?.();
    clearTimeout(this.#timer);
    clearTimeout(this.#saveTimer);
  }

  /** Ask the engine again for its versions, loaded packs and credit lines. */
  async refreshInfo(): Promise<void> {
    if (!this.engine) return;
    const info = await this.engine.engine_info();
    if (info.ok) this.info = info.value;
  }

  /** Resolves once the core data has loaded, failed, or turned out not to exist (at once for the mock). */
  async dataSettled(): Promise<void> {
    await this.#loader?.core().catch(() => undefined);
  }

  /** Try the data again after a failure; answers that were waiting for it are asked again. */
  retryData(): void {
    if (!this.#loader) return;
    this.#loader.retry();
    void this.#loader.core().catch(() => undefined);
    if (this.result.error?.code === 'pack_missing') {
      this.#lastKey = '';
      this.#schedule(this.engineInput);
    }
  }

  /** Every data file URL fetched so far (the service worker copies them once it takes over). */
  dataUrlsFetched(): string[] {
    return [...(this.#loader?.fetched ?? [])];
  }

  /** Start fetching the ZIP tables before they are needed (when someone starts typing a ZIP code). */
  prefetchZip(): void {
    void this.#loader?.zip().catch(() => undefined);
  }

  /** County outlines for the map, loaded once; null when the site has none. */
  countyShapes(): Promise<CountyShapes | null> {
    if (!this.#shapes) {
      const pending: Promise<CountyShapes | null> = this.#loader
        ? this.#loader.map().then((json) => (json ? indexShapes(json) : null))
        : fetchCountyShapes(import.meta.env.BASE_URL);
      this.#shapes = pending;
      // A failed load is tried again next time a map is shown.
      pending.catch(() => {
        if (this.#shapes === pending) this.#shapes = undefined;
      });
    }
    return this.#shapes;
  }

  /** Resolves once no assessment is waiting or running. */
  settled(): Promise<void> {
    if (!this.pending) return Promise.resolve();
    return new Promise((resolve) => this.#waiters.push(resolve));
  }

  #flush(): void {
    const waiters = this.#waiters;
    this.#waiters = [];
    for (const w of waiters) w();
  }

  #schedule(input: PlanInput | null): void {
    if (!input || !this.engine) {
      clearTimeout(this.#timer);
      this.#lastKey = '';
      this.result = {};
      this.pending = false;
      this.#flush();
      return;
    }
    const key = JSON.stringify(input);
    if (key === this.#lastKey) return;
    this.#lastKey = key;
    clearTimeout(this.#timer);
    if (!hasPlace(input.location)) {
      // A new plan still has the placeholder ZIP code: there is nothing to look up yet.
      this.#seq += 1;
      this.result = { error: NO_PLACE_ERROR };
      this.pending = false;
      this.#flush();
      return;
    }
    this.pending = true;
    const seq = ++this.#seq;
    const engine = this.engine;
    this.#timer = setTimeout(() => {
      engine
        .assess(input)
        .catch((e: unknown) => ({ ok: false as const, error: { code: 'internal' as const, message: String(e) } }))
        .then((r) => {
          if (seq !== this.#seq) return;
          this.result = r.ok ? { output: r.value } : { error: r.error };
          this.pending = false;
          this.#flush();
        });
    }, this.#delay);
  }

  #scheduleSave(snapshot: store.SavedPlan | null): void {
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => {
      if (!this.#storage) return;
      this.saveFailed = !store.savePlan(this.#storage, snapshot);
    }, this.#saveDelay);
  }

  /** Write the plan now (before an export, or when the page is being hidden). */
  saveNow(): void {
    clearTimeout(this.#saveTimer);
    if (this.#storage) this.saveFailed = !store.savePlan(this.#storage, this.plan ? ($state.snapshot(this.plan) as store.SavedPlan) : null);
  }

  // ------------------------------------------------------------------------------------------
  // Changes
  // ------------------------------------------------------------------------------------------

  /** Start a plan from the engine's defaults, dated today. */
  async startNew(): Promise<void> {
    if (!this.engine) await this.init();
    const d = await this.engine!.defaults();
    if (!d.ok) throw new Error(d.error.message);
    const input = d.value;
    input.planning_date = this.today();
    this.plan = store.newPlan(input);
  }

  load(plan: store.SavedPlan): void {
    this.plan = plan;
  }

  completeStep(step: store.StepId): void {
    if (this.plan && !this.plan.progress.completed.includes(step)) this.plan.progress.completed.push(step);
  }

  rememberRoute(hash: string): void {
    if (this.plan && this.plan.progress.last !== hash) this.plan.progress.last = hash;
  }

  purchaseFor(itemId: string, tier: TierId): store.Purchase | undefined {
    return this.plan?.purchases.find((p) => p.item_id === itemId && (p.tier ?? tier) === tier);
  }

  /** Check an item off the plan. */
  record(item: PlanItem): void {
    if (!this.plan || this.purchaseFor(item.item_id, item.tier)) return;
    this.plan.purchases.push({ item_id: item.item_id, tier: item.tier, qty: item.quantity, date: this.today() });
  }

  unrecord(itemId: string, tier: TierId): void {
    if (!this.plan) return;
    this.plan.purchases = this.plan.purchases.filter((p) => !(p.item_id === itemId && (p.tier ?? tier) === tier));
  }

  setPaid(itemId: string, tier: TierId, paid: number | undefined): void {
    const p = this.purchaseFor(itemId, tier);
    if (!p) return;
    if (paid === undefined) delete p.paid_usd;
    else p.paid_usd = paid;
  }

  isDismissed(warningId: string): boolean {
    return this.plan?.dismissed_warnings.includes(warningId) ?? false;
  }

  /** "Keep anyway": the warning folds away, and can be brought back. */
  setDismissed(warningId: string, dismissed: boolean): void {
    if (!this.plan) return;
    const rest = this.plan.dismissed_warnings.filter((w) => w !== warningId);
    this.plan.dismissed_warnings = dismissed ? [...rest, warningId] : rest;
  }

  markDone(key: string, date: IsoDate = this.today()): void {
    if (this.plan) this.plan.done_dates[key] = date;
  }

  /** "Forget everything": clears this app's storage and the plan in memory. */
  forget(): number {
    clearTimeout(this.#saveTimer);
    const removed = store.forgetEverything(this.#storage);
    this.plan = null;
    this.prefs = { ...store.DEFAULT_PREFS };
    this.storageDamaged = false;
    return removed;
  }
}

export function applyTheme(theme: store.Prefs['theme']): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (theme === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', theme);
}

// ---------------------------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------------------------

export const APP_CONTEXT = 'rr-app';

export function provideApp(app: AppState): void {
  setContext(APP_CONTEXT, app);
}

export function useApp(): AppState {
  const app = getContext<AppState | undefined>(APP_CONTEXT);
  if (!app) throw new Error('AppState missing from context');
  return app;
}
