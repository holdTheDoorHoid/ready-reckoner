/**
 * Test doubles for the maps (jsdom has no canvas and no IndexedDB): a canvas whose 2D context
 * records what was drawn, a compose environment with a routed `fetch`, and images that remember
 * what they are. Used only by the maps' tests.
 */
import type { CanvasLike, ComposeEnv, Decoded } from './compose';

/** A drawn image that says what it is ("tile", "flood", …) so tests can tell what reached the canvas. */
export interface FakeImage extends Decoded {
  tag: string;
}

export class FakeContext {
  ops: string[] = [];
  texts: string[] = [];
  fillStyle: unknown = '#000';
  strokeStyle: unknown = '#000';
  lineWidth = 1;
  font = '';
  textAlign = 'start';
  textBaseline = 'alphabetic';
  globalCompositeOperation = 'source-over';
  globalAlpha = 1;
  lineJoin = 'miter';
  lineCap = 'butt';
  imageSmoothingEnabled = true;

  constructor(readonly canvas: FakeCanvas) {}

  drawImage(image: unknown, ...args: number[]): void {
    const tag = (image as FakeImage).tag ?? ((image as FakeCanvas).isFake ? `canvas ${(image as FakeCanvas).width}x${(image as FakeCanvas).height}` : 'image');
    this.ops.push(`drawImage ${tag} ${args.map((a) => Math.round(a)).join(',')}`);
  }
  fillText(text: string): void {
    this.texts.push(text);
    this.ops.push(`fillText ${text}`);
  }
  measureText(text: string): { width: number } {
    return { width: text.length * 6.5 };
  }
  getImageData(_x: number, _y: number, w: number, h: number): { data: Uint8ClampedArray; width: number; height: number } {
    this.ops.push(`getImageData ${w}x${h}`);
    return { data: new Uint8ClampedArray(w * h * 4), width: w, height: h };
  }
  createImageData(w: number, h: number): { data: Uint8ClampedArray; width: number; height: number } {
    return { data: new Uint8ClampedArray(w * h * 4), width: w, height: h };
  }
  putImageData(): void {
    this.ops.push('putImageData');
  }
  createPattern(): object {
    return { pattern: true };
  }
  setLineDash(d: number[]): void {
    this.ops.push(`setLineDash ${d.join(',')}`);
  }
  setTransform(...m: number[]): void {
    this.ops.push(`setTransform ${m.join(',')}`);
  }
  save(): void {}
  restore(): void {}
  beginPath(): void {}
  closePath(): void {}
  moveTo(): void {}
  lineTo(): void {}
  arc(): void {}
  rect(): void {}
  stroke(): void {
    this.ops.push('stroke');
  }
  fill(): void {
    this.ops.push('fill');
  }
  fillRect(): void {
    this.ops.push('fillRect');
  }
  clearRect(): void {}
}

export class FakeCanvas implements CanvasLike {
  readonly isFake = true;
  readonly ctx: FakeContext;
  encoded: { type: string; quality?: number } | undefined;
  constructor(
    public width: number,
    public height: number,
  ) {
    this.ctx = new FakeContext(this);
  }
  getContext(): CanvasRenderingContext2D {
    return this.ctx as unknown as CanvasRenderingContext2D;
  }
  toDataURL(type: string, quality?: number): string {
    this.encoded = { type, quality };
    return `data:${type};base64,${'A'.repeat(4000)}`;
  }
}

export interface RecordedRequest {
  url: string;
  init: RequestInit;
}

export type Route = (url: string, init: RequestInit) => Response | Error | Promise<Response | Error>;

/**
 * A response as the composer reads it (`ok`, `status`, `headers.get`, `blob`, `json`). A plain
 * object: jsdom's Blob and Node's Response do not mix (a jsdom Blob body reads as "[object Blob]",
 * and the Blob Node hands back has no `text()` under jsdom), so the "blob" simply carries its tag.
 */
function fakeResponse(status: number, type: string, body: { tag?: string; json?: unknown }): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: { get: (name: string) => (name.toLowerCase() === 'content-type' ? type : null) },
    blob: async () => ({ tag: body.tag ?? '' }),
    json: async () => body.json,
  } as unknown as Response;
}

/** An image response whose body names what it is. */
export function imageResponse(tag: string, status = 200): Response {
  return fakeResponse(status, 'image/png', { tag });
}

export function jsonResponse(body: unknown, status = 200): Response {
  return fakeResponse(status, 'application/json', { json: body });
}

/** RGBA pixels: `red` of the first pixels red, the next `blue` blue, the rest transparent. */
export function overlayPixels(width: number, height: number, red: number, blue: number): Uint8ClampedArray {
  const data = new Uint8ClampedArray(width * height * 4);
  for (let i = 0; i < red + blue && i < width * height; i++) {
    data[i * 4] = i < red ? 255 : 0;
    data[i * 4 + 2] = i < red ? 0 : 255;
    data[i * 4 + 3] = 255;
  }
  return data;
}

/** A compose environment: `route` answers each request; images decode to their tag. */
export function fakeEnv(route: Route, pixels: (tag: string, w: number, h: number) => ArrayLike<number> = (_t, w, h) => new Uint8ClampedArray(w * h * 4)) {
  const requests: RecordedRequest[] = [];
  const canvases: FakeCanvas[] = [];
  const progress: [number, number][] = [];
  const env: ComposeEnv = {
    async fetch(url, init) {
      requests.push({ url, init });
      const r = await route(url, init);
      if (r instanceof Error) throw r;
      return r;
    },
    canvas(w, h) {
      const c = new FakeCanvas(w, h);
      canvases.push(c);
      return c;
    },
    async decode(blob) {
      return { width: 256, height: 256, tag: (blob as unknown as { tag: string }).tag } as FakeImage;
    },
    readPixels(image, w, h) {
      return pixels((image as FakeImage).tag, w, h);
    },
    pixelRatio: 1,
    onProgress(done, total) {
      progress.push([done, total]);
    },
  };
  return { env, requests, canvases, progress };
}
