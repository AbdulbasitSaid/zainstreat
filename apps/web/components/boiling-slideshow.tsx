"use client";

import { useEffect, useRef, useState } from "react";
import Image, { type StaticImageData } from "next/image";
import type { SplashRenderer, SplashTexture } from "@/lib/splash-renderer";

export type BoilingSlide = { src: StaticImageData | string; alt: string };

// How long each dish sits still between splashes.
const HOLD_MS = 6000;
// How far the camera leans from straight down at the dish (radians).
const TILT = 0.35;
// Where the bubble may rise (box uv, y up), clear of the hero text: to the
// right of it on wide screens, above it on narrow ones.
const WIDE_BUBBLE_AREA = { left: 0.55, right: 0.85, bottom: 0.25, top: 0.75 };
const NARROW_BUBBLE_AREA = { left: 0.25, right: 0.75, bottom: 0.55, top: 0.85 };
// Fallback without WebGL: a plain crossfade.
const CROSSFADE_MS = 800;

/**
 * The homepage hero as a pot of soup: the featured dishes' photos are
 * stacked, and every few seconds a glossy liquid over the top one comes to
 * a simmer, a bubble swells and bursts into a splash, and the next dish
 * spreads out behind the liquid until it settles perfectly still.
 *
 * The splash is drawn in WebGL (lib/splash-renderer.ts, loaded on first
 * use) on a canvas laid over the photo stack only while it runs. Its first
 * and last frames match the plain photos, so showing and hiding the canvas
 * is invisible. Each splash gets its own seed, so no two look alike. Without
 * WebGL (or if a photo can't be loaded as a texture) the dishes crossfade
 * instead. With reduced motion, or a single slide, it's a still image. It
 * pauses while the tab is hidden.
 */
export function BoilingSlideshow({ slides, className = "" }: { slides: BoilingSlide[]; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [top, setTop] = useState(0);

  useEffect(() => {
    const root = ref.current;
    const canvas = canvasRef.current;
    if (slides.length < 2 || !root || !canvas) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const layers = Array.from(root.querySelectorAll<HTMLElement>("[data-slide]"));
    // Stack order, top first; each layer's z-index follows it.
    let order = layers.map((_, i) => i);
    const restack = () =>
      order.forEach((layer, depth) => {
        layers[layer].style.zIndex = String(order.length - depth);
      });
    restack();

    let disposed = false;
    let renderer: SplashRenderer | null = null;
    let webglFailed = false;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let frame = 0;
    let seed = 1;

    const getRenderer = async () => {
      if (renderer || webglFailed) return renderer;
      try {
        const { createSplashRenderer } = await import("@/lib/splash-renderer");
        if (!disposed) renderer = createSplashRenderer(canvas);
      } catch {
        webglFailed = true;
      }
      return renderer;
    };

    const scheduleNext = () => {
      if (disposed) return;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        // Don't boil in a background tab; pick up again when it's back.
        if (document.visibilityState === "hidden") {
          document.addEventListener("visibilitychange", scheduleNext, { once: true });
          return;
        }
        void boilAway();
      }, HOLD_MS);
    };

    const textureOf = async (gl: SplashRenderer, layer: HTMLElement): Promise<SplashTexture | null> => {
      const img = layer.querySelector("img");
      if (!img?.currentSrc) return null;
      return gl.loadTexture(img.currentSrc).catch(() => null);
    };

    // No WebGL: fade the next dish in over the old one.
    const crossfade = () => {
      const incoming = layers[order[0]];
      incoming.style.transition = "none";
      incoming.style.opacity = "0";
      restack();
      setTop(order[0]);
      requestAnimationFrame(() => {
        incoming.style.transition = `opacity ${CROSSFADE_MS}ms ease-in-out`;
        incoming.style.opacity = "1";
      });
      timer = setTimeout(scheduleNext, CROSSFADE_MS);
    };

    const boilAway = async () => {
      const [currentLayer, nextLayer] = [layers[order[0]], layers[order[1]]];
      const gl = await getRenderer();
      const textures = gl ? await Promise.all([textureOf(gl, currentLayer), textureOf(gl, nextLayer)]) : null;
      if (disposed) return;

      order = [...order.slice(1), order[0]];
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      if (!gl || !textures?.[0] || !textures[1] || !width || !height) {
        crossfade();
        return;
      }

      // The first frame is the old dish untouched, so showing the canvas
      // and moving the stack on underneath it is invisible.
      const area = window.matchMedia("(min-width: 768px)").matches ? WIDE_BUBBLE_AREA : NARROW_BUBBLE_AREA;
      gl.prepare(textures[0], textures[1], { width, height }, { seed: seed++, tilt: TILT, area });
      canvas.style.visibility = "visible";
      restack();
      setTop(order[0]);

      // Plays on the frame clock; a long gap (a hidden tab) counts as one
      // frame, so the splash pauses rather than skipping ahead.
      const duration = gl.duration();
      let elapsed = 0;
      let last = performance.now();
      const tick = (now: number) => {
        if (disposed) return;
        elapsed = Math.min(elapsed + Math.min((now - last) / 1000, 0.1), duration);
        last = now;
        gl.render(elapsed);
        if (elapsed < duration) {
          frame = requestAnimationFrame(tick);
          return;
        }
        canvas.style.visibility = "hidden";
        scheduleNext();
      };
      frame = requestAnimationFrame(tick);
    };

    scheduleNext();
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      cancelAnimationFrame(frame);
      document.removeEventListener("visibilitychange", scheduleNext);
      canvas.style.visibility = "hidden";
      renderer?.dispose();
      renderer = null;
      layers.forEach((layer) => {
        layer.style.transition = "";
        layer.style.opacity = "";
      });
    };
  }, [slides.length]);

  return (
    <div ref={ref} className={`overflow-hidden ${className}`.trim()}>
      {slides.map((slide, i) => (
        <div key={i} data-slide className="absolute inset-0" style={{ zIndex: slides.length - i }}>
          <Image
            src={slide.src}
            alt={i === top ? slide.alt : ""}
            fill
            sizes="100vw"
            priority={i === 0}
            // CORS-mode, so the boil shader can reuse the cached photo as a
            // WebGL texture (the API sends Access-Control-Allow-Origin).
            crossOrigin="anonymous"
            // Dev's http:// API uploads can't go through the optimizer
            // (see MenuItemCard).
            unoptimized={typeof slide.src === "string" && slide.src.startsWith("http://")}
            className="object-cover"
          />
        </div>
      ))}
      <canvas
        ref={canvasRef}
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 z-99 h-full w-full"
        style={{ visibility: "hidden" }}
      />
    </div>
  );
}
