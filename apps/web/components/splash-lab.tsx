"use client";

import { useEffect, useRef, useState } from "react";
import type { SplashDifference, SplashLayers, SplashRenderer, SplashView } from "@/lib/splash-renderer";

export type SplashLabSlide = { src: string; name: string };

// Pause between loops, so the settled dish can be judged.
const LOOP_GAP_S = 0.8;
const FRAME_WINDOW = 240;
const SHAPES = { hero: "4 / 5", square: "1 / 1", wide: "3 / 2" } as const;
const ALL_LAYERS: SplashLayers = { bubble: true, crown: true, droplets: true, waves: true, lighting: true };

type Stats = { fps: number; p95: number };
type Handoff = { first: SplashDifference; last: SplashDifference };

const describe = ({ mean, max }: SplashDifference) => `mean ${mean.toFixed(2)}, max ${max}`;

/**
 * A workbench for the liquid splash (lib/splash-renderer.ts): plays one
 * transition between two featured dishes on loop, with a scrubber, seed,
 * camera tilt, layer toggles, debug views of the wave grid and reveal mask,
 * and a frame-time readout. `?t=2.1` opens it paused at that moment, and
 * `?seed=` picks the splash. Dev only.
 */
export function SplashLab({
  slides,
  initialTime,
  initialSeed,
}: {
  slides: SplashLabSlide[];
  initialTime: number | null;
  initialSeed: number;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const [renderer, setRenderer] = useState<SplashRenderer | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pair, setPair] = useState<[number, number]>([0, 1]);
  const [seed, setSeed] = useState(initialSeed);
  const [tilt, setTilt] = useState(0.35);
  const [shape, setShape] = useState<keyof typeof SHAPES>("hero");
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [layers, setLayers] = useState<SplashLayers>(ALL_LAYERS);
  const [view, setView] = useState<SplashView>("final");
  const [playing, setPlaying] = useState(initialTime === null);
  const [speed, setSpeed] = useState(1);
  const [newSeedEachLoop, setNewSeedEachLoop] = useState(initialTime === null);
  const [time, setTime] = useState(initialTime ?? 0);
  const [duration, setDuration] = useState(0);
  const [stats, setStats] = useState<Stats | null>(null);
  const [handoff, setHandoff] = useState<Handoff | null>(null);
  // The loop reads these without re-subscribing.
  const clock = useRef({ t: initialTime ?? 0, playing: initialTime === null, speed: 1, duration: 0, newSeed: false });

  useEffect(() => {
    clock.current.playing = playing;
    clock.current.speed = speed;
    clock.current.newSeed = newSeedEachLoop;
  }, [playing, speed, newSeedEachLoop]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let disposed = false;
    let created: SplashRenderer | null = null;
    import("@/lib/splash-renderer")
      .then(({ createSplashRenderer }) => {
        if (disposed) return;
        created = createSplashRenderer(canvas);
        setRenderer(created);
      })
      .catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)));
    return () => {
      disposed = true;
      created?.dispose();
    };
  }, []);

  useEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      if (width && height) setSize({ width: Math.round(width), height: Math.round(height) });
    });
    observer.observe(box);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!renderer || !size || slides.length < 2) return;
    let cancelled = false;
    Promise.all([renderer.loadTexture(slides[pair[0]].src), renderer.loadTexture(slides[pair[1]].src)])
      .then(([current, next]) => {
        if (cancelled) return;
        renderer.prepare(current, next, size, { seed, tilt });
        clock.current.duration = renderer.duration();
        setDuration(renderer.duration());
        renderer.render(clock.current.t);
      })
      .catch((reason: unknown) => setError(`Couldn't load a photo as a texture: ${String(reason)}`));
    return () => {
      cancelled = true;
    };
  }, [renderer, size, pair, seed, tilt, slides]);

  useEffect(() => {
    renderer?.setLayers(layers);
  }, [renderer, layers]);

  useEffect(() => {
    renderer?.setView(view);
  }, [renderer, view]);

  useEffect(() => {
    if (!renderer) return;
    const frames: number[] = [];
    let last = performance.now();
    let lastReport = last;
    let raf = 0;
    const tick = (now: number) => {
      const dt = Math.min((now - last) / 1000, 0.1);
      last = now;
      const c = clock.current;
      if (c.playing && c.duration > 0) {
        frames.push(dt * 1000);
        if (frames.length > FRAME_WINDOW) frames.shift();
        c.t += dt * c.speed;
        if (c.t > c.duration + LOOP_GAP_S) {
          c.t = 0;
          if (c.newSeed) setSeed((s) => s + 1);
        }
      }
      renderer.render(Math.min(c.t, c.duration));
      if (now - lastReport > 250) {
        lastReport = now;
        setTime(c.t);
        if (frames.length > 30) {
          const sorted = [...frames].sort((a, b) => a - b);
          const mean = frames.reduce((sum, f) => sum + f, 0) / frames.length;
          setStats({ fps: 1000 / mean, p95: sorted[Math.floor(sorted.length * 0.95)] });
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [renderer]);

  if (slides.length < 2) {
    return <p>The lab needs at least two featured menu items with photos.</p>;
  }

  const scrub = (t: number) => {
    clock.current.t = t;
    setTime(t);
    setPlaying(false);
  };

  return (
    <div className="grid gap-6 lg:grid-cols-[560px_1fr]">
      <div ref={boxRef} className="relative w-full overflow-hidden rounded-card bg-card" style={{ aspectRatio: SHAPES[shape] }}>
        <canvas ref={canvasRef} className="absolute inset-0 h-full w-full" />
        {error && <p className="absolute inset-0 grid place-items-center bg-card p-6 text-center">{error}</p>}
      </div>

      <div className="grid content-start gap-4 text-sm">
        <div className="flex flex-wrap items-center gap-3">
          <button type="button" className="rounded border px-3 py-1" onClick={() => setPlaying((p) => !p)}>
            {playing ? "Pause" : "Play"}
          </button>
          <span className="tabular-nums">
            {time.toFixed(2)}s / {duration.toFixed(2)}s
          </span>
          <span className="tabular-nums" data-testid="splash-stats">
            {stats ? `${stats.fps.toFixed(0)} fps · p95 ${stats.p95.toFixed(1)} ms` : "measuring…"}
          </span>
        </div>
        <input
          type="range"
          aria-label="Time"
          min={0}
          max={duration || 1}
          step={0.01}
          value={Math.min(time, duration)}
          onChange={(event) => scrub(Number(event.target.value))}
        />

        <label className="flex items-center gap-2">
          From
          <select value={pair[0]} onChange={(event) => setPair([Number(event.target.value), pair[1]])}>
            {slides.map((slide, i) => (
              <option key={slide.src} value={i}>
                {slide.name}
              </option>
            ))}
          </select>
          to
          <select value={pair[1]} onChange={(event) => setPair([pair[0], Number(event.target.value)])}>
            {slides.map((slide, i) => (
              <option key={slide.src} value={i}>
                {slide.name}
              </option>
            ))}
          </select>
          <button type="button" className="rounded border px-2" onClick={() => setPair([pair[1], pair[0]])}>
            Swap
          </button>
        </label>

        <div className="flex flex-wrap items-center gap-4">
          <label className="flex items-center gap-2">
            Seed
            <input type="number" className="w-20 border px-1" value={seed} onChange={(event) => setSeed(Number(event.target.value) || 1)} />
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={newSeedEachLoop} onChange={(event) => setNewSeedEachLoop(event.target.checked)} />
            New seed each loop
          </label>
        </div>

        <div className="flex flex-wrap items-center gap-4">
          <label className="flex items-center gap-2">
            Speed
            <select value={speed} onChange={(event) => setSpeed(Number(event.target.value))}>
              {[0.1, 0.25, 0.5, 1].map((s) => (
                <option key={s} value={s}>
                  {s}×
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-2">
            Shape
            <select value={shape} onChange={(event) => setShape(event.target.value as keyof typeof SHAPES)}>
              {Object.keys(SHAPES).map((key) => (
                <option key={key} value={key}>
                  {key}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-2">
            Tilt {Math.round((tilt * 180) / Math.PI)}°
            <input type="range" min={0} max={0.8} step={0.05} value={tilt} onChange={(event) => setTilt(Number(event.target.value))} />
          </label>
        </div>

        <fieldset className="flex flex-wrap gap-4">
          <legend className="mb-1">Layers</legend>
          {(Object.keys(ALL_LAYERS) as (keyof SplashLayers)[]).map((key) => (
            <label key={key} className="flex items-center gap-1">
              <input type="checkbox" checked={layers[key]} onChange={(event) => setLayers({ ...layers, [key]: event.target.checked })} />
              {key}
            </label>
          ))}
        </fieldset>

        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            className="rounded border px-3 py-1"
            disabled={!renderer || !duration}
            onClick={() => {
              setPlaying(false);
              if (renderer) setHandoff(renderer.measureHandoff());
            }}
          >
            Measure hand-off
          </button>
          {handoff && (
            <span className="tabular-nums" data-testid="splash-handoff">
              first frame vs photo: {describe(handoff.first)} · last frame vs next photo: {describe(handoff.last)} (0–255)
            </span>
          )}
        </div>

        <label className="flex items-center gap-2">
          View
          <select value={view} onChange={(event) => setView(event.target.value as SplashView)}>
            <option value="final">final</option>
            <option value="wave">wave grid</option>
            <option value="reveal">reveal mask</option>
          </select>
        </label>
      </div>
    </div>
  );
}
