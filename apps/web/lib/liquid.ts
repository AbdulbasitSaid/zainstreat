// Shared machinery for the site's liquid surfaces (LiquidSurface on
// buttons, LiquidCard on cards): a springy outline whose points are coupled
// to their neighbours, so a disturbance sloshes and travels around the
// shape like a wave.

type OutlinePoint = { x: number; y: number; nx: number; ny: number };

// Spring towards the target offset, pull from the two neighbours (the
// wave), and damping. Tuned for one step per animation frame.
const STIFFNESS = 0.11;
const COUPLING = 0.2;
const DAMPING = 0.8;
const REST_EPSILON = 0.04;

export function prefersLiquidMotion(): boolean {
  return (
    window.matchMedia("(pointer: fine)").matches && !window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function reducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

// The element's corner radius in px, capped to what the box can hold
// (`rounded-full` reports a huge value).
export function cornerRadius(el: Element, width: number, height: number): number {
  const radius = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
  return Math.min(radius, width / 2, height / 2);
}

// Evenly spaced points (with outward normals) clockwise around a rounded
// rectangle, offset by `inset` on both axes.
function roundedRectOutline(w: number, h: number, r: number, inset: number, spacing: number): OutlinePoint[] {
  const top = Math.max(w - 2 * r, 0);
  const side = Math.max(h - 2 * r, 0);
  const corner = (Math.PI / 2) * r;
  // [length, point at distance s along the segment]
  const segments: [number, (s: number) => OutlinePoint][] = [
    [top, (s) => ({ x: r + s, y: 0, nx: 0, ny: -1 })],
    [corner, (s) => arcPoint(w - r, r, r, -Math.PI / 2 + s / Math.max(r, 1e-6))],
    [side, (s) => ({ x: w, y: r + s, nx: 1, ny: 0 })],
    [corner, (s) => arcPoint(w - r, h - r, r, s / Math.max(r, 1e-6))],
    [top, (s) => ({ x: w - r - s, y: h, nx: 0, ny: 1 })],
    [corner, (s) => arcPoint(r, h - r, r, Math.PI / 2 + s / Math.max(r, 1e-6))],
    [side, (s) => ({ x: 0, y: h - r - s, nx: -1, ny: 0 })],
    [corner, (s) => arcPoint(r, r, r, Math.PI + s / Math.max(r, 1e-6))],
  ];
  const perimeter = segments.reduce((sum, [length]) => sum + length, 0);
  const count = Math.max(32, Math.round(perimeter / spacing));
  const points: OutlinePoint[] = [];

  for (let i = 0; i < count; i++) {
    let s = (i / count) * perimeter;
    for (let k = 0; k < segments.length; k++) {
      const [length, at] = segments[k];
      if (s < length || k === segments.length - 1) {
        const p = at(Math.min(s, length));
        points.push({ ...p, x: p.x + inset, y: p.y + inset });
        break;
      }
      s -= length;
    }
  }
  return points;
}

function arcPoint(cx: number, cy: number, r: number, angle: number): OutlinePoint {
  const nx = Math.cos(angle);
  const ny = Math.sin(angle);
  return { x: cx + r * nx, y: cy + r * ny, nx, ny };
}

export class LiquidOutline {
  points: OutlinePoint[] = [];
  private offset = new Float32Array(0);
  private velocity = new Float32Array(0);
  private accel = new Float32Array(0);

  // `inset` keeps a stroke of that width inside the box, like a border.
  setShape(width: number, height: number, radius: number, { inset = 0, spacing = 5 } = {}) {
    const w = width - 2 * inset;
    const h = height - 2 * inset;
    this.points = roundedRectOutline(w, h, Math.max(Math.min(radius - inset, w / 2, h / 2), 0), inset, spacing);
    this.offset = new Float32Array(this.points.length);
    this.velocity = new Float32Array(this.points.length);
    this.accel = new Float32Array(this.points.length);
  }

  // Advances one frame towards the per-point target offset (outward +).
  // Returns whether anything is still moving.
  step(target: (point: OutlinePoint, index: number) => number): boolean {
    const n = this.points.length;
    let moving = false;
    for (let i = 0; i < n; i++) {
      const goal = target(this.points[i], i);
      if (goal !== 0) moving = true;
      const neighbours = this.offset[(i - 1 + n) % n] + this.offset[(i + 1) % n] - 2 * this.offset[i];
      this.accel[i] = (goal - this.offset[i]) * STIFFNESS + neighbours * COUPLING;
    }
    for (let i = 0; i < n; i++) {
      this.velocity[i] = (this.velocity[i] + this.accel[i]) * DAMPING;
      this.offset[i] += this.velocity[i];
      if (Math.abs(this.offset[i]) > REST_EPSILON || Math.abs(this.velocity[i]) > REST_EPSILON) moving = true;
    }
    return moving;
  }

  // Knocks the outline near (x, y): positive bulges out, negative dents in.
  kick(x: number, y: number, amount: number, spread: number) {
    this.points.forEach((p, i) => {
      const d2 = (p.x - x) ** 2 + (p.y - y) ** 2;
      this.velocity[i] += amount * Math.exp(-d2 / (2 * spread * spread));
    });
  }

  // A smooth closed path (Catmull-Rom as cubic Béziers), shifted by `shift`.
  path(shift = 0): string {
    const n = this.points.length;
    const xs = this.points.map((p, i) => shift + p.x + p.nx * this.offset[i]);
    const ys = this.points.map((p, i) => shift + p.y + p.ny * this.offset[i]);
    let d = `M${xs[0].toFixed(1)},${ys[0].toFixed(1)}`;
    for (let i = 0; i < n; i++) {
      const p0 = (i - 1 + n) % n;
      const p2 = (i + 1) % n;
      const p3 = (i + 2) % n;
      const c1x = xs[i] + (xs[p2] - xs[p0]) / 6;
      const c1y = ys[i] + (ys[p2] - ys[p0]) / 6;
      const c2x = xs[p2] - (xs[p3] - xs[i]) / 6;
      const c2y = ys[p2] - (ys[p3] - ys[i]) / 6;
      d += `C${c1x.toFixed(1)},${c1y.toFixed(1)} ${c2x.toFixed(1)},${c2y.toFixed(1)} ${xs[p2].toFixed(1)},${ys[p2].toFixed(1)}`;
    }
    return `${d}Z`;
  }
}

// Maps a viewport point into an element's unscaled layout box, so the
// element's own hover scale/tilt doesn't skew the maths.
export function toLocal(el: HTMLElement, clientX: number, clientY: number) {
  const rect = el.getBoundingClientRect();
  return {
    x: ((clientX - rect.left) * el.offsetWidth) / (rect.width || 1),
    y: ((clientY - rect.top) * el.offsetHeight) / (rect.height || 1),
  };
}

// One window listener shared by every liquid element on the page. Called
// with NaN coordinates when the pointer leaves the window.
type PointerListener = (clientX: number, clientY: number) => void;
const pointerListeners = new Set<PointerListener>();

function broadcastMove(event: PointerEvent) {
  pointerListeners.forEach((listener) => listener(event.clientX, event.clientY));
}

function broadcastLeave() {
  pointerListeners.forEach((listener) => listener(NaN, NaN));
}

export function subscribePointer(listener: PointerListener): () => void {
  if (pointerListeners.size === 0) {
    window.addEventListener("pointermove", broadcastMove, { passive: true });
    document.documentElement.addEventListener("pointerleave", broadcastLeave);
  }
  pointerListeners.add(listener);
  return () => {
    pointerListeners.delete(listener);
    if (pointerListeners.size === 0) {
      window.removeEventListener("pointermove", broadcastMove);
      document.documentElement.removeEventListener("pointerleave", broadcastLeave);
    }
  };
}

// A frame loop that runs only while `frame` reports motion.
export function frameLoop(frame: (seconds: number) => boolean) {
  let id = 0;
  let startedAt = 0;
  const tick = (now: number) => {
    id = frame((now - startedAt) / 1000) ? requestAnimationFrame(tick) : 0;
  };
  return {
    wake() {
      if (id) return;
      startedAt ||= performance.now();
      id = requestAnimationFrame(tick);
    },
    stop() {
      cancelAnimationFrame(id);
      id = 0;
    },
  };
}
