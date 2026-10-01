/**
 * Drawing the composed maps on a canvas (DESIGN-DELTA-v3 §9.2): the base map made grey and given
 * more contrast so it reads in black and white, overlays as hatching (never colour alone), the
 * county outline, the drawn routes, numbered markers that move aside instead of covering each
 * other, a scale bar in miles and kilometres, a north arrow and the credit.
 *
 * Everything takes a plain 2D context and CSS-pixel coordinates; the composer scales the context
 * by its pixel ratio first, so text and lines stay crisp on paper.
 */

/** A pattern the legend can show beside its words. */
export type PatternId = 'stripes' | 'dots' | 'stripes-back' | 'cross';

/** Inks: dark enough to print as near-black; the hue only helps on screen. */
export const INK = {
  text: '#111111',
  flood: '#15437d',
  fire: '#7a2508',
  route: '#111111',
  county: '#222222',
} as const;

/**
 * Grey the base map and raise its contrast, in place: luminance with a gamma of 1.5, which darkens
 * the middle tones (water, buildings, road casings) and keeps paper-white roads white, so streets,
 * blocks and water stay apart in black and white.
 */
export function toneBase(rgba: Uint8ClampedArray, gamma = 1.5): void {
  const lut = new Uint8ClampedArray(256);
  for (let i = 0; i < 256; i++) lut[i] = Math.round(255 * (i / 255) ** gamma);
  for (let i = 0; i < rgba.length; i += 4) {
    const y = Math.round(0.2126 * rgba[i]! + 0.7152 * rgba[i + 1]! + 0.0722 * rgba[i + 2]!);
    const v = lut[y]!;
    rgba[i] = v;
    rgba[i + 1] = v;
    rgba[i + 2] = v;
  }
}

/** A point on the canvas. */
export interface XY {
  x: number;
  y: number;
}

/** Where a marker ends up: at its point, or nudged aside with a leader line back to it. */
export interface Placed extends XY {
  /** The true position. */
  at: XY;
  nudged: boolean;
}

/**
 * Lay markers out in order. A marker that would overlap one already placed is moved to the first
 * free spot on rings around its point (8 directions, then 16 at twice the distance), inside the
 * canvas; if none is free it stays on its point. `fixed` markers (the household's own) never move.
 */
export function layoutMarkers(points: readonly (XY & { fixed?: boolean })[], radius: number, width: number, height: number): Placed[] {
  const placed: Placed[] = [];
  const gap = 2 * radius + 2;
  const free = (p: XY) => placed.every((q) => Math.hypot(q.x - p.x, q.y - p.y) >= gap) && p.x >= radius && p.y >= radius && p.x <= width - radius && p.y <= height - radius;
  for (const p of points) {
    if (p.fixed || free(p)) {
      placed.push({ x: p.x, y: p.y, at: { x: p.x, y: p.y }, nudged: false });
      continue;
    }
    let spot: XY | undefined;
    for (const [ring, steps] of [
      [gap + 2, 8],
      [2 * gap + 2, 16],
    ] as const) {
      for (let i = 0; i < steps && !spot; i++) {
        const angle = (i / steps) * 2 * Math.PI - Math.PI / 2;
        const c = { x: p.x + ring * Math.cos(angle), y: p.y + ring * Math.sin(angle) };
        if (free(c)) spot = c;
      }
      if (spot) break;
    }
    placed.push(spot ? { ...spot, at: { x: p.x, y: p.y }, nudged: true } : { x: p.x, y: p.y, at: { x: p.x, y: p.y }, nudged: false });
  }
  return placed;
}

/** The 2D context calls the drawing uses (a canvas's own context satisfies it). */
export type Ctx = CanvasRenderingContext2D;

/** A small canvas holding one repeat of a hatch pattern, drawn at `ratio` device pixels per CSS pixel. */
export function drawPattern(ctx: Ctx, id: PatternId, ink: string, ratio: number): void {
  const s = 8 * ratio;
  ctx.clearRect(0, 0, s, s);
  ctx.strokeStyle = ink;
  ctx.fillStyle = ink;
  ctx.lineWidth = 1.6 * ratio;
  ctx.lineCap = 'square';
  const diag = (flip: boolean) => {
    ctx.beginPath();
    for (const o of [-s, 0, s]) {
      if (flip) {
        ctx.moveTo(o, 0);
        ctx.lineTo(o + s, s);
      } else {
        ctx.moveTo(o, s);
        ctx.lineTo(o + s, 0);
      }
    }
    ctx.stroke();
  };
  if (id === 'stripes') diag(false);
  else if (id === 'stripes-back') diag(true);
  else if (id === 'cross') {
    diag(false);
    diag(true);
  } else {
    ctx.beginPath();
    ctx.arc(s / 4, s / 4, 1.3 * ratio, 0, 2 * Math.PI);
    ctx.arc((3 * s) / 4, (3 * s) / 4, 1.3 * ratio, 0, 2 * Math.PI);
    ctx.fill();
  }
}

/** The pixel size of one pattern repeat. */
export const PATTERN_SIZE = 8;

/** A line through canvas points, with a white casing under it so it reads over any map. */
export function drawLine(ctx: Ctx, points: readonly XY[], style: { width: number; dash?: number[]; ink: string; casing?: number }): void {
  if (points.length < 2) return;
  const path = () => {
    ctx.beginPath();
    ctx.moveTo(points[0]!.x, points[0]!.y);
    for (const p of points.slice(1)) ctx.lineTo(p.x, p.y);
  };
  ctx.save();
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.setLineDash([]);
  ctx.strokeStyle = 'rgba(255,255,255,0.9)';
  ctx.lineWidth = style.width + (style.casing ?? 3);
  path();
  ctx.stroke();
  ctx.setLineDash(style.dash ?? []);
  ctx.strokeStyle = style.ink;
  ctx.lineWidth = style.width;
  path();
  ctx.stroke();
  ctx.restore();
}

/** A numbered disc (a place) or a lettered square (the household's own point), with a leader line if nudged. */
export function drawMarker(ctx: Ctx, m: Placed, label: string, own: boolean): void {
  ctx.save();
  if (m.nudged) {
    ctx.strokeStyle = INK.text;
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(m.at.x, m.at.y);
    ctx.lineTo(m.x, m.y);
    ctx.stroke();
    ctx.fillStyle = INK.text;
    ctx.beginPath();
    ctx.arc(m.at.x, m.at.y, 2.2, 0, 2 * Math.PI);
    ctx.fill();
  }
  ctx.font = `700 ${label.length > 1 ? 11 : 12.5}px system-ui, -apple-system, 'Segoe UI', Roboto, Arial, sans-serif`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  if (own) {
    const w = Math.max(22, ctx.measureText(label).width + 9);
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(m.x - w / 2 - 2, m.y - 13, w + 4, 26);
    ctx.fillStyle = INK.text;
    ctx.fillRect(m.x - w / 2, m.y - 11, w, 22);
    ctx.fillStyle = '#ffffff';
    ctx.fillText(label, m.x, m.y + 0.5);
  } else {
    ctx.beginPath();
    ctx.arc(m.x, m.y, 10.5, 0, 2 * Math.PI);
    ctx.fillStyle = '#ffffff';
    ctx.fill();
    ctx.lineWidth = 2;
    ctx.strokeStyle = INK.text;
    ctx.stroke();
    ctx.fillStyle = INK.text;
    ctx.fillText(label, m.x, m.y + 0.5);
  }
  ctx.restore();
}

/** Text on a white box, for the scale bar, the credit and the north arrow. */
function boxedText(ctx: Ctx, text: string, x: number, y: number, align: 'left' | 'right', size = 11): void {
  ctx.font = `600 ${size}px system-ui, -apple-system, 'Segoe UI', Roboto, Arial, sans-serif`;
  ctx.textBaseline = 'middle';
  ctx.textAlign = align;
  const w = ctx.measureText(text).width;
  ctx.fillStyle = 'rgba(255,255,255,0.88)';
  ctx.fillRect(align === 'left' ? x - 3 : x - w - 3, y - size / 2 - 3, w + 6, size + 6);
  ctx.fillStyle = INK.text;
  ctx.fillText(text, x, y);
}

/** Two stacked bars at the bottom left: miles or feet above, kilometres or metres below. */
export function drawScaleBars(ctx: Ctx, bars: readonly { pixels: number; label: string }[], height: number): void {
  ctx.save();
  const left = 12;
  let y = height - 14 - (bars.length - 1) * 24;
  const widest = Math.max(...bars.map((b) => b.pixels));
  ctx.fillStyle = 'rgba(255,255,255,0.88)';
  ctx.fillRect(left - 6, y - 16, widest + 70, bars.length * 24 + 8);
  for (const bar of bars) {
    ctx.strokeStyle = INK.text;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(left, y - 5);
    ctx.lineTo(left, y + 2);
    ctx.lineTo(left + bar.pixels, y + 2);
    ctx.lineTo(left + bar.pixels, y - 5);
    ctx.stroke();
    boxedText(ctx, bar.label, left + bar.pixels + 6, y - 1, 'left');
    y += 24;
  }
  ctx.restore();
}

/** A small north arrow at the top right. */
export function drawNorthArrow(ctx: Ctx, width: number): void {
  ctx.save();
  const x = width - 22;
  ctx.fillStyle = 'rgba(255,255,255,0.88)';
  ctx.fillRect(x - 13, 6, 26, 40);
  ctx.fillStyle = INK.text;
  ctx.beginPath();
  ctx.moveTo(x, 10);
  ctx.lineTo(x + 7, 26);
  ctx.lineTo(x, 22);
  ctx.lineTo(x - 7, 26);
  ctx.closePath();
  ctx.fill();
  ctx.font = "700 12px system-ui, -apple-system, 'Segoe UI', Roboto, Arial, sans-serif";
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('N', x, 36);
  ctx.restore();
}

/** The credit, bottom right on the image (the Attribution Guidelines: visible, in a corner). */
export function drawCredit(ctx: Ctx, text: string, width: number, height: number): void {
  ctx.save();
  boxedText(ctx, text, width - 8, height - 12, 'right', 10.5);
  ctx.restore();
}
