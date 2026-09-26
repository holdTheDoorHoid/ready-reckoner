/** Test helpers: an in-memory Storage, saved plans for fixtures, and mounting a screen with its contexts. */
import { flushSync, mount, tick, unmount, type Component } from 'svelte';

import type { Engine } from '../engine/index';
import type { PackLoader } from '../engine/loader';
import { createMockEngine } from '../engine/mock';
import type { PlanInput } from '../engine/types';
import { APP_CONTEXT, AppState } from '../lib/app.svelte';
import { newPlan, STEP_IDS, STORAGE_KEY, type SavedPlan } from '../lib/persistence';
import { Router, ROUTER_CONTEXT } from '../lib/router.svelte';

export class MemoryStorage implements Storage {
  #data = new Map<string, string>();
  failWrites = false;
  get length(): number {
    return this.#data.size;
  }
  clear(): void {
    this.#data.clear();
  }
  getItem(key: string): string | null {
    return this.#data.get(key) ?? null;
  }
  key(index: number): string | null {
    return [...this.#data.keys()][index] ?? null;
  }
  removeItem(key: string): void {
    this.#data.delete(key);
  }
  setItem(key: string, value: string): void {
    if (this.failWrites) throw new DOMException('quota', 'QuotaExceededError');
    this.#data.set(key, String(value));
  }
}

export function clone<T>(x: T): T {
  return JSON.parse(JSON.stringify(x)) as T;
}

/** A saved plan for a fixture household, with the interview marked complete. */
export function savedFor(input: PlanInput, extra: Partial<SavedPlan> = {}): SavedPlan {
  const plan = newPlan(clone(input));
  plan.progress.completed = [...STEP_IDS];
  return { ...plan, ...extra };
}

export async function until(condition: () => boolean, what = 'condition', ms = 3000): Promise<void> {
  const start = Date.now();
  while (!condition()) {
    if (Date.now() - start > ms) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 5));
    flushSync();
  }
}

export interface Rendered {
  app: AppState;
  router: Router;
  target: HTMLElement;
  storage: MemoryStorage;
  text(): string;
  cleanup(): void;
}

/**
 * Mount a component with a fresh app state (mock engine, in-memory storage, no delays) and a
 * router on `route`, and wait until the first assessment has answered.
 */
export async function render(
  Screen: Component,
  options: { plan?: SavedPlan | null; route?: string; engine?: Engine; loader?: PackLoader | null; today?: string; waitForPlan?: boolean } = {},
): Promise<Rendered> {
  const storage = new MemoryStorage();
  if (options.plan) storage.setItem(STORAGE_KEY, JSON.stringify(options.plan));
  const app = new AppState({
    storage,
    engine: options.engine ?? createMockEngine(),
    loader: options.loader ?? null,
    delay: 0,
    saveDelay: 0,
    today: () => options.today ?? '2026-10-01',
  });
  await app.init();
  flushSync();
  if (app.plan && options.waitForPlan !== false) await until(() => !!app.result.output || !!app.result.error, 'the first assessment');
  window.location.hash = options.route ? `#/${options.route}` : '#/';
  const router = new Router(window);
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(Screen, {
    target,
    context: new Map<string, unknown>([
      [APP_CONTEXT, app],
      [ROUTER_CONTEXT, router],
    ]),
  });
  flushSync();
  await tick();
  flushSync();
  return {
    app,
    router,
    target,
    storage,
    text: () => target.textContent ?? '',
    cleanup() {
      unmount(component);
      target.remove();
      app.destroy();
      router.destroy();
    },
  };
}
