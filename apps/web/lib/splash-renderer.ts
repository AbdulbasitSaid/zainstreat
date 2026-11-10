// The liquid splash between two dishes, drawn in WebGL (three.js) and
// timed with a GSAP timeline. A thin glossy liquid lies over the photo: it
// simmers, one bubble swells and bursts into a crown and droplets, and the
// next dish spreads out behind the liquid. Loaded with a dynamic import, so
// three.js stays out of the initial bundle.
//
// What's simulated and what's drawn: the ripples are a real 2D wave
// equation on a small grid (bubbles popping and droplets landing push into
// it); the bubble, its rupture and the crown are analytic shapes; droplets
// are projectiles (gravity, drag) moved on the CPU; reflections are a
// photo studio drawn in the shader, with Fresnel; refraction shifts the
// photo by the surface slope. A flat, still surface shows the photo
// exactly, so the first and last frames match the plain images.
import { gsap } from "gsap";
import {
  DataTexture,
  DynamicDrawUsage,
  HalfFloatType,
  InstancedBufferAttribute,
  InstancedBufferGeometry,
  LinearFilter,
  LinearMipmapLinearFilter,
  Mesh,
  OrthographicCamera,
  PlaneGeometry,
  RGBAFormat,
  RepeatWrapping,
  Scene,
  ShaderMaterial,
  type Texture,
  TextureLoader,
  UnsignedByteType,
  Vector2,
  Vector3,
  Vector4,
  WebGLRenderTarget,
  WebGLRenderer,
} from "three";

export type SplashTexture = {
  texture: Texture;
  width: number;
  height: number;
  // The liquid's colour: the photo's average, leaning on its saturated
  // (food) pixels rather than the plate or table.
  tint: [number, number, number];
  // A small RGBA copy, for choosing where the bubble rises.
  thumb: Uint8ClampedArray;
};

// Part of the box (uv, y up) the bubble may rise in.
export type SplashArea = { left: number; right: number; bottom: number; top: number };

export type SplashLayers = { bubble: boolean; crown: boolean; droplets: boolean; waves: boolean; lighting: boolean };
export type SplashView = "final" | "wave" | "reveal";
// Mean and largest per-channel difference (0–255) between a frame and the
// plain photo it should match.
export type SplashDifference = { mean: number; max: number };

export type SplashRenderer = {
  loadTexture: (src: string) => Promise<SplashTexture>;
  // Sizes the canvas and sets up one transition; `tilt` (radians) is how
  // far the camera leans from looking straight down at the dish.
  prepare: (
    current: SplashTexture,
    next: SplashTexture,
    size: { width: number; height: number },
    options: { seed: number; tilt: number; area?: SplashArea },
  ) => void;
  duration: () => number;
  setLayers: (layers: SplashLayers) => void;
  setView: (view: SplashView) => void;
  // Draws the transition at `t` seconds. Deterministic: seeking back
  // replays the simulation from the start.
  render: (t: number) => void;
  // Compares the first and last frames with the plain photos, which the
  // static <img> takes over from.
  measureHandoff: () => { first: SplashDifference; last: SplashDifference };
  dispose: () => void;
};

const STEP_S = 1 / 60;
// Wave grid steps per simulation step: sets how fast ripples travel.
const WAVE_SUBSTEPS = 3;
const WAVE_GRID = 256;
const THUMB = 48;
// Canvas pixels the composite may shade per frame: a full-screen banner
// renders below the device pixel ratio rather than overload the GPU.
const PIXEL_BUDGET = 2.4e6;
const COARSE_PIXEL_BUDGET = 1.2e6;
const MIDDLE: SplashArea = { left: 0.22, right: 0.78, bottom: 0.22, top: 0.78 };
const MAX_IMPULSES = 24;
const MAX_STAMPS = 24;
const MAX_BUBBLES = 12;
const MAX_DROPS = 192;
// Surface units (screen heights) per s², slowed down like high-speed footage.
const GRAVITY = 2.6;
const IDLE_BUBBLES_PER_S = 9;
const LIGHT = new Vector3(-0.45, 0.55, 0.7).normalize();

const QUAD_VERTEX = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;

// Smooth value noise from a random texture (one fetch per octave).
const NOISE = /* glsl */ `
  uniform sampler2D uNoise;
  float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return textureLod(uNoise, (i + f + 0.5) / 256.0, 0.0).r;
  }
  float fbm(vec2 p) {
    return 0.5 * noise(p) + 0.3 * noise(p * 2.07 + 17.3) + 0.2 * noise(p * 4.13 + 41.1);
  }
`;

// The crown's height around its ring: a few unequal ripples, never
// symmetrical. Mirrored on the CPU, which throws droplets off its peaks.
const CROWN = /* glsl */ `
  uniform vec3 uCrownK;
  uniform vec3 uCrownA;
  uniform vec3 uCrownPhi;
  float crownShape(float th) {
    vec3 s = sin(uCrownK * th + uCrownPhi);
    return 1.0 + dot(uCrownA, s);
  }
`;

const PHOTO = /* glsl */ `
  uniform sampler2D uCurrent;
  uniform sampler2D uNext;
  uniform sampler2D uReveal;
  uniform vec4 uCoverCurrent; // xy: scale, zw: offset (object-cover framing)
  uniform vec4 uCoverNext;
  uniform vec3 uTintCurrent;
  uniform vec3 uTintNext;
  uniform float uAspect;
  uniform float uPx; // one canvas pixel, in screen heights
  uniform float uTime;

  vec2 toUv(vec2 p) { return vec2(p.x / uAspect, p.y); }
  vec3 photoLod(vec2 uv, float m) {
    uv = clamp(uv, 0.0, 1.0);
    vec3 a = textureLod(uCurrent, uCoverCurrent.zw + uv * uCoverCurrent.xy, 0.0).rgb;
    vec3 b = textureLod(uNext, uCoverNext.zw + uv * uCoverNext.xy, 0.0).rgb;
    return mix(a, b, m);
  }
`;

// A controlled food-photography studio, reflected by the liquid: a big
// softbox up left, a strip light right, a small hard light for sparkle, a
// soft warm ceiling, and the dish's own colour from below.
const LIGHTING = /* glsl */ `
  uniform vec3 uLight;
  const float F0 = 0.03; // water/oil: glossy, never metallic

  float fresnel(float cosine) {
    return F0 + (1.0 - F0) * pow(1.0 - clamp(cosine, 0.0, 1.0), 5.0);
  }
  float ggx(float nh, float a) {
    float a2 = a * a;
    float d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / (3.14159 * d * d);
  }
  float softbox(vec3 dir, vec3 axis, vec2 size) {
    float c = dot(dir, axis);
    vec3 t = normalize(cross(vec3(0.0, 0.0, 1.0), axis));
    vec3 b = cross(axis, t);
    vec2 q = abs(vec2(dot(dir, t), dot(dir, b)) / max(c, 1e-3)) - size;
    float dist = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0);
    return step(0.0, c) * (1.0 - smoothstep(-0.03, 0.08, dist));
  }
  vec3 studio(vec3 dir, vec3 dish) {
    vec3 color = mix(dish * 0.5, vec3(0.86, 0.83, 0.78), smoothstep(-0.2, 0.7, dir.z));
    color += vec3(16.0, 15.5, 14.5) * softbox(dir, normalize(vec3(-0.55, 0.55, 0.63)), vec2(0.42, 0.3));
    color += vec3(8.0, 8.0, 8.5) * softbox(dir, normalize(vec3(0.75, -0.15, 0.64)), vec2(0.07, 0.45));
    color += vec3(40.0) * pow(max(dot(dir, normalize(vec3(-0.25, 0.35, 0.9))), 0.0), 1400.0);
    return color;
  }
  // Reflection plus specular for a surface normal; absolute.
  vec3 highlights(vec3 n, vec3 dish) {
    vec3 v = vec3(0.0, 0.0, 1.0);
    vec3 h = normalize(uLight + v);
    return fresnel(n.z) * studio(reflect(-v, n), dish)
      + vec3(1.0, 0.97, 0.92) * ggx(max(dot(n, h), 0.0), 0.07) * 0.012;
  }
`;

const WAVE_FRAGMENT = /* glsl */ `
  precision highp float;
  uniform sampler2D uState; // r: height, g: velocity
  uniform vec2 uTexel;
  uniform float uAspect;
  uniform float uDamping;
  uniform vec4 uImpulses[${MAX_IMPULSES}]; // xy: centre, z: radius, w: strength
  uniform int uImpulseCount;
  varying vec2 vUv;

  void main() {
    vec2 state = texture(uState, vUv).rg;
    float around = texture(uState, vUv - vec2(uTexel.x, 0.0)).r
      + texture(uState, vUv + vec2(uTexel.x, 0.0)).r
      + texture(uState, vUv - vec2(0.0, uTexel.y)).r
      + texture(uState, vUv + vec2(0.0, uTexel.y)).r;
    float velocity = (state.g + (around * 0.25 - state.r) * 1.6) * uDamping;
    float height = (state.r + velocity) * 0.999;
    vec2 p = vec2(vUv.x * uAspect, vUv.y);
    for (int i = 0; i < ${MAX_IMPULSES}; i++) {
      if (i >= uImpulseCount) break;
      vec2 d = p - uImpulses[i].xy;
      height += uImpulses[i].w * exp(-dot(d, d) / (uImpulses[i].z * uImpulses[i].z));
    }
    gl_FragColor = vec4(height, velocity, 0.0, 1.0);
  }
`;

// Where the next dish shows: the liquid front (the crown's leading edge,
// rough and lobed where the crown peaks), plus a splat wherever a droplet
// lands. It only ever grows.
const REVEAL_FRAGMENT = /* glsl */ `
  precision highp float;
  uniform sampler2D uPrevious;
  uniform sampler2D uWave;
  uniform float uAspect;
  uniform float uFore;
  uniform vec2 uCentre;
  uniform float uFront;
  uniform float uFloor;
  uniform vec4 uStamps[${MAX_STAMPS}]; // xy: centre, z: radius, w: direction of travel
  uniform int uStampCount;
  varying vec2 vUv;
  ${NOISE}
  ${CROWN}

  void main() {
    vec2 p = vec2(vUv.x * uAspect, vUv.y);
    vec2 q = (p - uCentre) / vec2(1.0, uFore);
    float d = length(q);
    float th = atan(q.y, q.x + 1e-6);
    float rough = fbm(p * 6.0 + 3.7) - 0.5;
    float front = uFront * (1.0 + 0.3 * (crownShape(th) - 1.0))
      + 0.08 * rough * min(uFront * 5.0, 1.0)
      + 0.01 * texture(uWave, vUv).r;
    float m = uFront > 0.0 ? smoothstep(-0.02, 0.02, front - d) : 0.0;
    for (int i = 0; i < ${MAX_STAMPS}; i++) {
      if (i >= uStampCount) break;
      // A splat runs on in the direction the drop was going, with
      // uneven lobes round its edge.
      vec4 s = uStamps[i];
      vec2 dir = vec2(cos(s.w), sin(s.w));
      vec2 dd = p - s.xy;
      vec2 local = vec2(dot(dd, dir), dot(dd, vec2(-dir.y, dir.x)));
      local.x /= 1.0 + 0.7 * step(0.0, local.x);
      float a = atan(local.y, local.x + 1e-6);
      float lobes = 1.0 + 0.07 * sin(3.0 * a + s.w * 7.0) + 0.4 * (noise(local / s.z * 1.1 + s.xy * 40.0) - 0.5);
      m = max(m, 1.0 - smoothstep(s.z * 0.3, s.z, length(local) / lobes));
    }
    m = max(max(m, texture(uPrevious, vUv).r), uFloor);
    gl_FragColor = vec4(m, m, m, 1.0);
  }
`;

const COMPOSITE_FRAGMENT = /* glsl */ `
  precision highp float;
  uniform sampler2D uWave;
  uniform float uFore;
  uniform float uWaveScale;
  uniform float uWaveAmp;
  uniform float uSimmer;
  uniform float uRefract;
  uniform vec2 uCentre;
  uniform float uBulge;
  uniform float uForm;
  uniform float uRadius;
  uniform float uInstability;
  uniform float uHole;
  uniform vec2 uHoleDir;
  uniform float uCrater;
  uniform float uCrownAmp;
  uniform float uCrownRadius;
  uniform float uCrownWidth;
  uniform float uRidge;
  uniform vec4 uBubbles[${MAX_BUBBLES}]; // xy: centre, z: radius
  uniform int uBubbleCount;
  uniform vec4 uLayers; // bubble, crown, waves, lighting
  uniform int uView;
  varying vec2 vUv;
  ${NOISE}
  ${CROWN}
  ${PHOTO}
  ${LIGHTING}

  // From the screen to the liquid's surface around c, undoing the tilt.
  vec2 surf(vec2 p, vec2 c) { return (p - c) / vec2(1.0, uFore); }

  // The liquid's height, in screen heights.
  float heightAt(vec2 p) {
    vec2 uv = toUv(p);
    float h = uLayers.z * uWaveAmp * uWaveScale * texture(uWave, uv).r;
    h += uSimmer * 0.004 * (fbm(p * 5.0 + vec2(uTime * 0.21, -uTime * 0.17)) - 0.5);

    vec2 q = surf(p, uCentre);
    float d2 = dot(q, q);
    float r = max(uRadius, 1e-4);
    // The swell that becomes the bubble, the meniscus round its base, and
    // the crater it leaves.
    h += uLayers.x * uBulge * (1.0 - 0.75 * uForm) * r * 0.4 * exp(-d2 / (r * r * 1.3));
    float rho = sqrt(d2) / r;
    h += uLayers.x * uForm * r * 0.06 * exp(-max(rho - 1.0, 0.0) * 7.0) * step(1.0, rho);
    h -= uCrater * r * 0.25 * exp(-d2 / (r * r * 0.9));

    float d = sqrt(d2);
    float th = atan(q.y, q.x + 1e-6);
    float ring = (d - uCrownRadius) / max(uCrownWidth, 1e-4);
    float lumps = 0.75 + 0.5 * noise(vec2(cos(th), sin(th)) * 3.0 + d * 30.0);
    h += uLayers.y * uCrownAmp * 0.014 * max(crownShape(th), 0.15) * lumps * exp(-ring * ring);

    // The wet ridge along the advancing liquid front.
    float m = texture(uReveal, uv).r;
    float lip = (0.5 + noise(p * 14.0 + uTime * vec2(0.6, -0.4))) * (0.5 + noise(p * 4.0 - uTime * 0.3));
    h += uRidge * 0.0045 * m * (1.0 - m) * 4.0 * lip;

    for (int i = 0; i < ${MAX_BUBBLES}; i++) {
      if (i >= uBubbleCount) break;
      vec2 bq = surf(p, uBubbles[i].xy);
      float rr = dot(bq, bq) / (uBubbles[i].z * uBubbles[i].z);
      h += uWaveAmp * uBubbles[i].z * 0.8 * sqrt(max(1.0 - rr, 0.0));
    }
    return h;
  }

  // The main bubble: a sphere drawn per pixel. rgb: colour, a: coverage.
  vec4 bubble(vec2 p, vec3 dish, out float rho) {
    vec2 q = surf(p, uCentre);
    float d = length(q);
    float th = atan(q.y, q.x + 1e-6);
    vec2 dir = q / max(d, 1e-5);
    float wobble = uInstability * (
        0.06 * sin(2.0 * th + uTime * 5.3)
      + 0.04 * sin(3.0 * th - uTime * 7.1 + 1.3)
      + 0.025 * sin(5.0 * th + uTime * 11.0 + 0.4)
      + 0.03 * sin(uTime * 6.0))
      + 0.03 * (noise(dir * 1.7 + uTime * 0.8) - 0.5);
    float r = max(uRadius * (1.0 + wobble), 1e-4);
    rho = d / r;
    float alpha = uForm * uLayers.x * (1.0 - smoothstep(1.0 - 1.5 * uPx / r, 1.0, rho));

    vec2 qn = q / r;
    float z = sqrt(max(1.0 - rho * rho, 0.0));
    vec3 n = normalize(vec3(qn, z * 0.85 + 1e-4));

    // The film tears open from one side and its edge races across.
    float torn = step(1e-3, uHole);
    float hd = length(qn - uHoleDir * 0.55);
    float holeR = uHole * 2.2 * (1.0 + 0.15 * (noise(qn * 4.0 + 7.0) - 0.5));
    float film = mix(1.0, smoothstep(holeR - 0.03, holeR + 0.03, hd), torn);
    float rim = torn * exp(-pow((hd - holeR) / 0.08, 2.0)) * step(rho, 1.0);

    // Through the film: the soup magnified and shifted, like a lens.
    vec2 lens = uCentre + (q * (0.86 + 0.14 * rho * rho) - n.xy * r * 0.07) * vec2(1.0, uFore);
    vec2 luv = toUv(lens);
    vec3 under = photoLod(luv, textureLod(uReveal, clamp(luv, 0.0, 1.0), 0.0).r);
    float graze = pow(1.0 - z, 2.0);
    vec3 body = mix(under * 0.9, under * 0.5 + dish * 0.45, 0.06 + 0.5 * graze);
    body *= 1.0 - 0.3 * pow(rho, 8.0);
    vec3 color = body * (1.0 - fresnel(n.z)) + uLayers.w * highlights(n, dish);
    color += rim * (dish * 0.7 + 0.45);
    alpha = max(alpha * film, rim * 0.85 * uForm * uLayers.x);
    return vec4(color, alpha);
  }

  void main() {
    vec2 p = vec2(vUv.x * uAspect, vUv.y);
    float h0 = heightAt(p);
    vec2 slope = vec2(heightAt(p + vec2(uPx, 0.0)) - h0, heightAt(p + vec2(0.0, uPx)) - h0) / uPx;
    vec3 n = normalize(vec3(-slope, 1.0));

    vec2 uv = clamp(toUv(p - slope * uRefract), 0.0, 1.0);
    float m = texture(uReveal, uv).r;
    vec3 photo = mix(
      texture(uCurrent, uCoverCurrent.zw + uv * uCoverCurrent.xy).rgb,
      texture(uNext, uCoverNext.zw + uv * uCoverNext.xy).rgb,
      m);
    vec3 dish = mix(uTintCurrent, uTintNext, m);

    // The front is liquid running over the old dish: a soft shadow just
    // ahead of it, and the new dish seen through a film just behind it.
    float e = 0.025;
    float around = 0.25 * (
        texture(uReveal, vUv + vec2(e / uAspect, 0.0)).r + texture(uReveal, vUv - vec2(e / uAspect, 0.0)).r
      + texture(uReveal, vUv + vec2(0.0, e)).r + texture(uReveal, vUv - vec2(0.0, e)).r);
    photo *= 1.0 - 0.35 * uRidge * max(around - m, 0.0);
    photo = mix(photo, photo * 0.8 + dish * 0.2, 0.5 * uRidge * m * (1.0 - m) * 4.0);

    // Lighting relative to a flat surface, so still liquid is the photo.
    vec3 lit = normalize(vec3(-slope * 0.8, 1.0));
    float diffuse = dot(lit, uLight) - uLight.z;
    vec3 still = vec3(0.0, 0.0, 1.0);
    vec3 color = photo * (1.0 + diffuse * 0.8 * uLayers.w)
      + 0.7 * uLayers.w * max(highlights(lit, dish) - highlights(still, dish), vec3(-0.1));

    float rho;
    vec4 b = bubble(p, dish, rho);
    // The dark crease where the bubble meets the liquid.
    color *= 1.0 - 0.15 * uForm * uLayers.x * exp(-pow((rho - 1.0) / 0.035, 2.0)) * step(1.0, rho);
    color = mix(color, b.rgb, b.a);

    if (uView == 1) color = vec3(0.5 + texture(uWave, vUv).r * 0.15);
    if (uView == 2) color = vec3(texture(uReveal, vUv).r);
    if (uView == 3) color = texture(uCurrent, uCoverCurrent.zw + vUv * uCoverCurrent.xy).rgb;
    if (uView == 4) color = texture(uNext, uCoverNext.zw + vUv * uCoverNext.xy).rgb;
    gl_FragColor = vec4(color, 1.0);
  }
`;

// Droplets and their shadows: instanced quads, each drawn as a sphere.
const DROP_VERTEX = /* glsl */ `
  attribute vec4 aDrop;    // xy: centre, z: radius, w: opacity
  attribute vec3 aStretch; // xy: direction of travel, z: elongation
  uniform float uAspect;
  varying vec2 vLocal;
  varying vec4 vDrop;
  varying vec2 vDir;
  void main() {
    vLocal = position.xy * 2.4;
    vec2 dir = aStretch.xy;
    vec2 offset = (dir * vLocal.x * aStretch.z + vec2(-dir.y, dir.x) * vLocal.y) * aDrop.z;
    vec2 pos = aDrop.xy + offset;
    vDrop = aDrop;
    vDir = dir;
    gl_Position = vec4(pos.x / uAspect * 2.0 - 1.0, pos.y * 2.0 - 1.0, 0.0, 1.0);
  }
`;

const DROP_FRAGMENT = /* glsl */ `
  precision highp float;
  varying vec2 vLocal;
  varying vec4 vDrop;
  varying vec2 vDir;
  ${PHOTO}
  ${LIGHTING}

  void main() {
    float rho = length(vLocal);
    float alpha = vDrop.w * (1.0 - smoothstep(1.0 - 1.5 * uPx / vDrop.z, 1.0, rho));
    if (alpha <= 0.0) discard;
    float z = sqrt(max(1.0 - rho * rho, 0.0));
    vec2 nxy = vDir * vLocal.x + vec2(-vDir.y, vDir.x) * vLocal.y;
    vec3 n = normalize(vec3(nxy, z));
    // A drop is a tiny lens: the soup behind it, upside down.
    vec2 uv = toUv(vDrop.xy - nxy * vDrop.z * 3.0);
    float m = textureLod(uReveal, clamp(uv, 0.0, 1.0), 0.0).r;
    vec3 dish = mix(uTintCurrent, uTintNext, m);
    // Thin in the middle, so clear; thick at the rim, so tinted and dark.
    vec3 body = mix(photoLod(uv, m) * 1.05, dish, 0.08 + 0.3 * rho * rho) * (1.0 - 0.2 * pow(rho, 6.0));
    // Light focused through the drop, on the side away from the light.
    body += dish * 0.6 * (1.0 - smoothstep(0.0, 0.45, length(nxy - uLight.xy * 0.55)));
    gl_FragColor = vec4(body * (1.0 - fresnel(n.z)) + highlights(n, dish), alpha);
  }
`;

const SHADOW_VERTEX = /* glsl */ `
  attribute vec4 aShadow; // xy: centre, z: radius, w: opacity
  uniform float uAspect;
  varying vec2 vLocal;
  varying float vOpacity;
  void main() {
    vLocal = position.xy * 2.0;
    vOpacity = aShadow.w;
    vec2 pos = aShadow.xy + vLocal * aShadow.z;
    gl_Position = vec4(pos.x / uAspect * 2.0 - 1.0, pos.y * 2.0 - 1.0, 0.0, 1.0);
  }
`;

const SHADOW_FRAGMENT = /* glsl */ `
  precision highp float;
  varying vec2 vLocal;
  varying float vOpacity;
  void main() {
    float a = vOpacity * (1.0 - smoothstep(0.1, 1.0, length(vLocal)));
    gl_FragColor = vec4(0.06, 0.03, 0.02, a);
  }
`;

// Deterministic randomness, so a seed always gives the same splash.
function random(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// Scale/offset that crops a texture like CSS object-cover in a box.
function coverFrame(textureAspect: number, boxAspect: number): Vector4 {
  if (textureAspect > boxAspect) {
    const scale = boxAspect / textureAspect;
    return new Vector4(scale, 1, (1 - scale) / 2, 0);
  }
  const scale = textureAspect / boxAspect;
  return new Vector4(1, scale, 0, (1 - scale) / 2);
}

function saturation(r: number, g: number, b: number) {
  const max = Math.max(r, g, b);
  return max > 0 ? (max - Math.min(r, g, b)) / max : 0;
}

function analyse(image: CanvasImageSource): Pick<SplashTexture, "tint" | "thumb"> {
  const canvas = document.createElement("canvas");
  canvas.width = THUMB;
  canvas.height = THUMB;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return { tint: [0.6, 0.4, 0.25], thumb: new Uint8ClampedArray(THUMB * THUMB * 4) };
  context.drawImage(image, 0, 0, THUMB, THUMB);
  const thumb = context.getImageData(0, 0, THUMB, THUMB).data;
  const sum = [0, 0, 0];
  let weight = 0;
  for (let i = 0; i < thumb.length; i += 4) {
    const [r, g, b] = [thumb[i] / 255, thumb[i + 1] / 255, thumb[i + 2] / 255];
    const w = 0.05 + saturation(r, g, b) ** 2;
    sum[0] += r * w;
    sum[1] += g * w;
    sum[2] += b * w;
    weight += w;
  }
  return { tint: [sum[0] / weight, sum[1] / weight, sum[2] / weight], thumb };
}

// The photo's colour at box uv (u, v), from its thumbnail.
function colourAt(source: SplashTexture, frame: Vector4, u: number, v: number): [number, number, number] {
  const px = Math.max(0, Math.min(THUMB - 1, Math.floor((frame.z + u * frame.x) * THUMB)));
  const py = Math.max(0, Math.min(THUMB - 1, Math.floor((1 - (frame.w + v * frame.y)) * THUMB)));
  const i = (py * THUMB + px) * 4;
  return [source.thumb[i] / 255, source.thumb[i + 1] / 255, source.thumb[i + 2] / 255];
}

// Where the bubble rises: a bright, saturated, food-coloured spot near the
// middle, away from the plate, table and edges. Returns screen coords.
function pickCentre(
  source: SplashTexture,
  frame: Vector4,
  aspect: number,
  area: SplashArea,
  rand: () => number,
): [number, number] {
  const candidates: { x: number; y: number; score: number }[] = [];
  const grid = 16;
  const [midU, midV] = [(area.left + area.right) / 2, (area.bottom + area.top) / 2];
  for (let gy = 0; gy <= grid; gy++) {
    for (let gx = 0; gx <= grid; gx++) {
      const u = area.left + ((area.right - area.left) * gx) / grid;
      const v = area.bottom + ((area.top - area.bottom) * gy) / grid;
      const [r, g, b] = colourAt(source, frame, u, v);
      const centred = Math.exp(-((u - midU) ** 2 + (v - midV) ** 2) / 0.08);
      candidates.push({ x: u * aspect, y: v, score: saturation(r, g, b) * Math.max(r, g, b) ** 2 * centred });
    }
  }
  candidates.sort((a, b) => b.score - a.score);
  const pick = candidates[Math.floor(rand() * Math.min(6, candidates.length))];
  return [pick.x, pick.y];
}

// x/y: surface offset from the burst, z: height. `ligament` is how
// stretched it is when it pinches off; `phase` staggers its wobble.
type Drop = {
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  r: number;
  age: number;
  ligament: number;
  phase: number;
};
type SimBubble = { x: number; y: number; r: number; age: number; life: number };

export function createSplashRenderer(canvas: HTMLCanvasElement): SplashRenderer {
  const renderer = new WebGLRenderer({ canvas, antialias: false, alpha: false, powerPreference: "high-performance" });
  const coarse = window.matchMedia("(pointer: coarse)").matches;
  const maxPixelRatio = Math.min(window.devicePixelRatio || 1, coarse ? 1.25 : 1.5);
  const pixelBudget = coarse ? COARSE_PIXEL_BUDGET : PIXEL_BUDGET;
  renderer.autoClear = false;
  const gl = renderer.getContext();
  if (!gl.getExtension("EXT_color_buffer_float") && !gl.getExtension("EXT_color_buffer_half_float")) {
    renderer.dispose();
    throw new Error("No float render targets");
  }

  const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const quadGeometry = new PlaneGeometry(2, 2);

  const noiseRand = random(7);
  const noiseData = new Uint8Array(256 * 256 * 4).map(() => Math.floor(noiseRand() * 256));
  const noiseTexture = new DataTexture(noiseData, 256, 256, RGBAFormat, UnsignedByteType);
  noiseTexture.wrapS = noiseTexture.wrapT = RepeatWrapping;
  noiseTexture.magFilter = noiseTexture.minFilter = LinearFilter;
  noiseTexture.needsUpdate = true;

  const targetOptions = { type: HalfFloatType, minFilter: LinearFilter, magFilter: LinearFilter, depthBuffer: false };
  const wave = [new WebGLRenderTarget(1, 1, targetOptions), new WebGLRenderTarget(1, 1, targetOptions)];
  const reveal = [new WebGLRenderTarget(1, 1, targetOptions), new WebGLRenderTarget(1, 1, targetOptions)];
  let waveIndex = 0;
  let revealIndex = 0;

  // Shared by every pass that reads them.
  const shared = {
    uNoise: { value: noiseTexture },
    uAspect: { value: 1 },
    uFore: { value: 1 },
    uCentre: { value: new Vector2() },
    uCrownK: { value: new Vector3() },
    uCrownA: { value: new Vector3() },
    uCrownPhi: { value: new Vector3() },
    uWave: { value: wave[0].texture as Texture },
    uReveal: { value: reveal[0].texture as Texture },
    uCurrent: { value: null as Texture | null },
    uNext: { value: null as Texture | null },
    uCoverCurrent: { value: new Vector4(1, 1, 0, 0) },
    uCoverNext: { value: new Vector4(1, 1, 0, 0) },
    uTintCurrent: { value: new Vector3() },
    uTintNext: { value: new Vector3() },
    uPx: { value: 0.001 },
    uTime: { value: 0 },
    uLight: { value: LIGHT },
  };

  const impulseSlots = Array.from({ length: MAX_IMPULSES }, () => new Vector4());
  const waveUniforms = {
    uState: { value: wave[0].texture as Texture },
    uTexel: { value: new Vector2() },
    uAspect: shared.uAspect,
    uDamping: { value: 0.995 },
    uImpulses: { value: impulseSlots },
    uImpulseCount: { value: 0 },
  };
  const waveMaterial = new ShaderMaterial({ uniforms: waveUniforms, vertexShader: QUAD_VERTEX, fragmentShader: WAVE_FRAGMENT });

  const stampSlots = Array.from({ length: MAX_STAMPS }, () => new Vector4());
  const revealUniforms = {
    ...shared,
    uPrevious: { value: reveal[0].texture as Texture },
    uFront: { value: 0 },
    uFloor: { value: 0 },
    uStamps: { value: stampSlots },
    uStampCount: { value: 0 },
  };
  const revealMaterial = new ShaderMaterial({ uniforms: revealUniforms, vertexShader: QUAD_VERTEX, fragmentShader: REVEAL_FRAGMENT });

  const bubbleSlots = Array.from({ length: MAX_BUBBLES }, () => new Vector4());
  const compositeUniforms = {
    ...shared,
    uWaveScale: { value: 0.004 },
    uWaveAmp: { value: 1 },
    uSimmer: { value: 0 },
    uRefract: { value: 0.022 },
    uBulge: { value: 0 },
    uForm: { value: 0 },
    uRadius: { value: 0.01 },
    uInstability: { value: 0 },
    uHole: { value: 0 },
    uHoleDir: { value: new Vector2(1, 0) },
    uCrater: { value: 0 },
    uCrownAmp: { value: 0 },
    uCrownRadius: { value: 0 },
    uCrownWidth: { value: 0.01 },
    uRidge: { value: 0 },
    uBubbles: { value: bubbleSlots },
    uBubbleCount: { value: 0 },
    uLayers: { value: new Vector4(1, 1, 1, 1) },
    uView: { value: 0 },
  };
  const compositeMaterial = new ShaderMaterial({
    uniforms: compositeUniforms,
    vertexShader: QUAD_VERTEX,
    fragmentShader: COMPOSITE_FRAGMENT,
  });

  const quad = new Mesh(quadGeometry, compositeMaterial);
  const quadScene = new Scene();
  quadScene.add(quad);
  quad.frustumCulled = false;

  // Droplets: one instanced quad per drop, plus one for its shadow.
  const plane = new PlaneGeometry(1, 1);
  const dropGeometry = new InstancedBufferGeometry();
  dropGeometry.index = plane.index;
  dropGeometry.setAttribute("position", plane.getAttribute("position"));
  const dropAttribute = new InstancedBufferAttribute(new Float32Array(MAX_DROPS * 4), 4).setUsage(DynamicDrawUsage);
  const stretchAttribute = new InstancedBufferAttribute(new Float32Array(MAX_DROPS * 3), 3).setUsage(DynamicDrawUsage);
  const shadowAttribute = new InstancedBufferAttribute(new Float32Array(MAX_DROPS * 4), 4).setUsage(DynamicDrawUsage);
  dropGeometry.setAttribute("aDrop", dropAttribute);
  dropGeometry.setAttribute("aStretch", stretchAttribute);
  dropGeometry.setAttribute("aShadow", shadowAttribute);
  const dropMaterial = new ShaderMaterial({
    uniforms: shared,
    vertexShader: DROP_VERTEX,
    fragmentShader: DROP_FRAGMENT,
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  const shadowMaterial = new ShaderMaterial({
    uniforms: { uAspect: shared.uAspect },
    vertexShader: SHADOW_VERTEX,
    fragmentShader: SHADOW_FRAGMENT,
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  const shadowMesh = new Mesh(dropGeometry, shadowMaterial);
  const dropMesh = new Mesh(dropGeometry, dropMaterial);
  shadowMesh.frustumCulled = dropMesh.frustumCulled = false;
  shadowMesh.renderOrder = 0;
  dropMesh.renderOrder = 1;
  const dropScene = new Scene();
  dropScene.add(shadowMesh, dropMesh);

  const loader = new TextureLoader();
  loader.setCrossOrigin("anonymous");
  const cache = new Map<string, Promise<SplashTexture>>();

  const loadTexture = (src: string) => {
    let pending = cache.get(src);
    if (!pending) {
      // A photo cached before the API sent CORS headers (it's served
      // `immutable`) fails as a texture; a distinct URL skips that entry.
      const fresh = `${src}${src.includes("?") ? "&" : "?"}cors=1`;
      pending = loader.loadAsync(src).catch(() => loader.loadAsync(fresh)).then((texture) => {
        texture.minFilter = LinearMipmapLinearFilter;
        texture.generateMipmaps = true;
        const image = texture.image as HTMLImageElement;
        return { texture, width: image.naturalWidth, height: image.naturalHeight, ...analyse(image) };
      });
      // A failed load shouldn't stick: let the next attempt retry it.
      pending.catch(() => cache.delete(src));
      cache.set(src, pending);
    }
    return pending;
  };

  // ---- one transition's state ----
  type Params = {
    simmer: number;
    bulge: number;
    form: number;
    radius: number;
    instability: number;
    hole: number;
    crater: number;
    crownAmp: number;
    crownRadius: number;
    crownWidth: number;
    front: number;
    ridge: number;
    waveAmp: number;
    damping: number;
    floor: number;
  };
  const params: Params = {
    simmer: 0,
    bulge: 0,
    form: 0,
    radius: 0,
    instability: 0,
    hole: 0,
    crater: 0,
    crownAmp: 0,
    crownRadius: 0,
    crownWidth: 0.01,
    front: 0,
    ridge: 0,
    waveAmp: 1,
    damping: 0.995,
    floor: 0,
  };
  let timeline: gsap.core.Timeline | null = null;
  let events: { at: number; fire: () => void }[] = [];
  let eventIndex = 0;
  let rand = random(1);
  let seed = 1;
  let simTime = 0;
  let lastRendered = -1;
  let ready = false;
  let aspect = 1;
  let fore = 1;
  let lift = 0;
  let scale = 1;
  let spawnDebt = 0;
  let currentSource: SplashTexture | null = null;
  let currentFrame = new Vector4(1, 1, 0, 0);
  const centre = new Vector2();
  const crown = { k: [0, 0, 0], a: [0, 0, 0], phi: [0, 0, 0] };
  const drops: Drop[] = [];
  const bubbles: SimBubble[] = [];
  const impulses: Vector4[] = [];
  const stamps: Vector4[] = [];
  let layers: SplashLayers = { bubble: true, crown: true, droplets: true, waves: true, lighting: true };

  const crownShape = (th: number) =>
    1 + crown.a[0] * Math.sin(crown.k[0] * th + crown.phi[0]) +
    crown.a[1] * Math.sin(crown.k[1] * th + crown.phi[1]) +
    crown.a[2] * Math.sin(crown.k[2] * th + crown.phi[2]);

  // Surface offset from the centre → screen (aspect) coords.
  const screenX = (d: Drop) => centre.x + d.x;
  const screenY = (d: Drop) => centre.y + d.y * fore + d.z * lift;

  const between = (min: number, max: number) => min + (max - min) * rand();

  const spawnDrop = (x: number, y: number, vx: number, vy: number, vz: number, r: number, ligament: number) => {
    if (drops.length >= MAX_DROPS) return;
    drops.push({ x, y, z: 0.002, vx, vy, vz, r, age: 0, ligament, phase: between(0, 6.28) });
  };

  // A drop thrown outwards from `from` (distance from the burst) at `angle`.
  const throwDrop = (angle: number, from: number, speed: number, up: number, r: number, ligament: number) => {
    const [c, s] = [Math.cos(angle), Math.sin(angle)];
    spawnDrop(c * from, s * from, c * speed, s * speed, up, r, ligament);
  };

  const buildTimeline = (burstAt: number, r1: number, r2: number, frontMax: number) => {
    timeline?.kill();
    Object.assign(params, {
      simmer: 0, bulge: 0, form: 0, radius: r1 * 0.3, instability: 0, hole: 0, crater: 0, crownAmp: 0,
      crownRadius: r2, crownWidth: r2 * 0.22, front: 0, ridge: 0, waveAmp: 1, damping: 0.995, floor: 0,
    });
    const tb = burstAt;
    // Several tweens per property, so none may apply its start values
    // before its own turn.
    const tl = gsap.timeline({ paused: true, defaults: { immediateRender: false } });
    // Idle: a simmer.
    tl.fromTo(params, { simmer: 0 }, { simmer: 1, duration: 0.6, ease: "sine.out" }, 0);
    // Formation: the surface swells and a bubble emerges.
    tl.fromTo(params, { bulge: 0 }, { bulge: 1, duration: 0.6, ease: "sine.inOut" }, 0.8);
    tl.fromTo(params, { radius: r1 * 0.3 }, { radius: r1, duration: 0.7, ease: "power1.out" }, 0.8);
    tl.fromTo(params, { form: 0 }, { form: 1, duration: 0.35, ease: "sine.out" }, 1.05);
    // Expansion: it grows and turns unstable.
    tl.fromTo(params, { radius: r1 }, { radius: r2, duration: tb - 1.5, ease: "power2.in" }, 1.5);
    tl.fromTo(params, { instability: 0 }, { instability: 1, duration: tb - 1.5, ease: "power3.in" }, 1.5);
    // Burst: the film tears across, the dome collapses into a crater.
    tl.fromTo(params, { hole: 0 }, { hole: 1, duration: 0.09, ease: "power1.in" }, tb);
    tl.set(params, { form: 0, bulge: 0 }, tb + 0.1);
    tl.fromTo(params, { crater: 0 }, { crater: 1, duration: 0.07, ease: "power2.out" }, tb + 0.02);
    tl.fromTo(params, { crater: 1 }, { crater: 0, duration: 0.55, ease: "sine.inOut" }, tb + 0.12);
    // Splash: a crown rises and races out, then sinks into ripples.
    tl.fromTo(params, { crownRadius: r2 * 0.9 }, { crownRadius: r2 * 3.4, duration: 0.95, ease: "power2.out" }, tb + 0.03);
    tl.fromTo(params, { crownWidth: r2 * 0.22 }, { crownWidth: r2 * 0.6, duration: 0.95, ease: "power1.out" }, tb + 0.03);
    tl.fromTo(params, { crownAmp: 0 }, { crownAmp: 1, duration: 0.14, ease: "power2.out" }, tb + 0.03);
    tl.fromTo(params, { crownAmp: 1 }, { crownAmp: 0, duration: 0.7, ease: "power2.in" }, tb + 0.25);
    // Reveal: the next dish spreads out right behind the liquid front.
    tl.fromTo(params, { front: 0 }, { front: r2 * 2.2, duration: 1.0, ease: "power2.out" }, tb + 0.06);
    tl.fromTo(params, { front: r2 * 2.2 }, { front: frontMax, duration: 1.2, ease: "sine.inOut" }, tb + 1.06);
    tl.fromTo(params, { ridge: 0 }, { ridge: 1, duration: 0.1 }, tb + 0.06);
    tl.fromTo(params, { ridge: 1 }, { ridge: 0, duration: 0.5, ease: "sine.in" }, 3.95);
    // Settling: the boil dies down and the new dish goes still.
    tl.fromTo(params, { simmer: 1 }, { simmer: 0, duration: 0.6, ease: "sine.inOut" }, 3.6);
    tl.fromTo(params, { damping: 0.995 }, { damping: 0.94, duration: 0.6, ease: "sine.in" }, 3.8);
    tl.fromTo(params, { waveAmp: 1 }, { waveAmp: 0, duration: 0.5, ease: "sine.inOut" }, 4.1);
    tl.fromTo(params, { floor: 0 }, { floor: 1, duration: 0.35, ease: "sine.in" }, 4.2);
    timeline = tl;
  };

  const buildEvents = (tb: number, r2: number, holeAngle: number) => {
    events = [];
    const at = (time: number, fire: () => void) => events.push({ at: time, fire });
    // The film's rim sheds a spray as it retracts round the bubble.
    for (let i = 0; i < 26; i++) {
      const offset = (rand() * 2 - 1) * Math.PI;
      at(tb + 0.09 * Math.abs(offset) / Math.PI, () =>
        throwDrop(holeAngle + offset, r2 * 0.9, between(0.25, 0.6) * scale, between(0.3, 0.75) * scale, (0.005 + 0.008 * rand() ** 2) * scale, 1.5),
      );
    }
    at(tb, () => impulses.push(new Vector4(centre.x, centre.y, r2 * 0.9, -5)));
    // The crown's peaks pinch off into the big drops.
    const peaks: number[] = [];
    const samples = 720;
    for (let i = 0; i < samples; i++) {
      const th = (i / samples) * Math.PI * 2 - Math.PI;
      const prev = crownShape(th - (Math.PI * 2) / samples);
      const value = crownShape(th);
      const nextValue = crownShape(th + (Math.PI * 2) / samples);
      if (value > prev && value >= nextValue && value > 1.05) peaks.push(th);
    }
    for (const th of peaks) {
      const height = crownShape(th);
      const count = 1 + Math.floor(rand() * 2);
      for (let i = 0; i < count; i++) {
        const when = tb + between(0.1, 0.22);
        const angle = th + between(-0.06, 0.06);
        const speed = between(0.18, 0.42) * height * scale;
        const up = between(0.55, 0.95) * scale;
        const r = (0.012 + 0.02 * rand() ** 2) * Math.min(height, 1.4) * scale;
        // A long strand pinches off the peak, leaving a satellite behind it.
        at(when, () => throwDrop(angle, params.crownRadius, speed, up, r, 2.4));
        at(when + 0.03, () => throwDrop(angle, params.crownRadius, speed * 0.85, up * 0.9, r * 0.3, 1.6));
      }
    }
    // Fine spray all round.
    for (let i = 0; i < 28; i++) {
      at(tb + between(0.05, 0.3), () =>
        throwDrop(between(-Math.PI, Math.PI), params.crownRadius, between(0.3, 0.75) * scale, between(0.4, 0.9) * scale, (0.003 + 0.006 * rand() ** 2) * scale, 1.8),
      );
    }
    // The collapsing crater shoots up a jet that breaks into a few drops.
    for (let i = 0; i < 3; i++) {
      at(tb + 0.28 + i * 0.05, () =>
        throwDrop(between(-Math.PI, Math.PI), 0.004, between(0.01, 0.06) * scale, (1.25 - i * 0.12) * scale, (0.03 - i * 0.006) * scale, 2.8),
      );
    }
    // The crown sinks back, setting off the ripple that follows it.
    at(tb + 0.55, () => {
      for (let i = 0; i < 12; i++) {
        const th = (i / 12) * Math.PI * 2;
        impulses.push(
          new Vector4(centre.x + Math.cos(th) * params.crownRadius, centre.y + Math.sin(th) * params.crownRadius * fore, params.crownWidth, 1.2 * crownShape(th)),
        );
      }
    });
    events.sort((a, b) => a.at - b.at);
  };

  const clearTarget = (target: WebGLRenderTarget) => {
    renderer.setRenderTarget(target);
    renderer.setClearColor(0x000000, 1);
    renderer.clear();
  };

  const reset = () => {
    rand = random(seed);
    simTime = 0;
    eventIndex = 0;
    spawnDebt = 0;
    drops.length = 0;
    bubbles.length = 0;
    impulses.length = 0;
    stamps.length = 0;
    [...wave, ...reveal].forEach(clearTarget);
    renderer.setRenderTarget(null);
    timeline?.time(0, true);
  };

  // Simmering bubbles cluster round where the big one will rise, and only
  // on the food: never on the plate, container or table.
  const spawnIdleBubble = () => {
    const angle = between(-Math.PI, Math.PI);
    const distance = Math.abs(between(-1, 1) * between(0, 1)) * 0.32;
    const x = Math.min(aspect - 0.03, Math.max(0.03, centre.x + Math.cos(angle) * distance));
    const y = Math.min(0.97, Math.max(0.03, centre.y + Math.sin(angle) * distance * fore));
    if (!currentSource || bubbles.length >= MAX_BUBBLES) return;
    const [r, g, b] = colourAt(currentSource, currentFrame, x / aspect, y);
    if (saturation(r, g, b) > 0.35) {
      bubbles.push({ x, y, r: (0.004 + rand() ** 2 * 0.012) * scale, age: 0, life: between(0.35, 0.9) });
    }
  };

  const runWave = () => {
    waveUniforms.uDamping.value = params.damping;
    for (let step = 0; step < WAVE_SUBSTEPS; step++) {
      const batch = step === 0 ? impulses.splice(0, MAX_IMPULSES) : [];
      batch.forEach((impulse, i) => impulseSlots[i].copy(impulse));
      waveUniforms.uImpulseCount.value = batch.length;
      waveUniforms.uState.value = wave[waveIndex].texture;
      waveIndex = 1 - waveIndex;
      quad.material = waveMaterial;
      renderer.setRenderTarget(wave[waveIndex]);
      renderer.render(quadScene, camera);
    }
    shared.uWave.value = wave[waveIndex].texture;
  };

  const runReveal = () => {
    const batch = stamps.splice(0, MAX_STAMPS);
    batch.forEach((stamp, i) => stampSlots[i].copy(stamp));
    revealUniforms.uStampCount.value = batch.length;
    revealUniforms.uFront.value = params.front;
    revealUniforms.uFloor.value = params.floor;
    revealUniforms.uPrevious.value = reveal[revealIndex].texture;
    revealIndex = 1 - revealIndex;
    quad.material = revealMaterial;
    renderer.setRenderTarget(reveal[revealIndex]);
    renderer.render(quadScene, camera);
    shared.uReveal.value = reveal[revealIndex].texture;
  };

  const step = () => {
    timeline?.time(simTime, true);
    while (eventIndex < events.length && events[eventIndex].at <= simTime) events[eventIndex++].fire();

    if (params.simmer > 0.05) {
      spawnDebt += params.simmer * IDLE_BUBBLES_PER_S * STEP_S * (1 + 1.5 * params.bulge);
      while (spawnDebt >= 1) {
        spawnDebt -= 1;
        spawnIdleBubble();
      }
    }
    for (let i = bubbles.length - 1; i >= 0; i--) {
      const b = bubbles[i];
      b.age += STEP_S;
      if (b.age >= b.life) {
        impulses.push(new Vector4(b.x, b.y, b.r * 1.6, -1.6 * (b.r / (0.01 * scale))));
        bubbles.splice(i, 1);
      }
    }

    for (let i = drops.length - 1; i >= 0; i--) {
      const d = drops[i];
      d.vz -= GRAVITY * scale * STEP_S;
      const drag = 1 - 0.5 * STEP_S;
      d.vx *= drag;
      d.vy *= drag;
      d.x += d.vx * STEP_S;
      d.y += d.vy * STEP_S;
      d.z += d.vz * STEP_S;
      d.age += STEP_S;
      if (d.z <= 0 && d.vz < 0) {
        const x = centre.x + d.x;
        const y = centre.y + d.y * fore;
        impulses.push(new Vector4(x, y, d.r * 2.5, -Math.min(3, 1.2 * (d.r / (0.008 * scale)))));
        stamps.push(new Vector4(x, y, d.r * (1.6 + 1.4 * rand()), Math.atan2(d.vy * fore, d.vx)));
        drops.splice(i, 1);
        // A big drop kicks up a little crown of its own.
        if (d.r > 0.012 * scale) {
          const heading = Math.atan2(d.vy, d.vx);
          for (let k = 0; k < 3; k++) {
            const a = heading + between(-1.2, 1.2);
            const speed = between(0.08, 0.22) * scale;
            spawnDrop(d.x, d.y, Math.cos(a) * speed, Math.sin(a) * speed, between(0.15, 0.3) * scale, d.r * between(0.15, 0.25), 1.3);
          }
        }
      }
    }

    runWave();
    runReveal();
    simTime += STEP_S;
  };

  const writeDrops = () => {
    const drop = dropAttribute.array as Float32Array;
    const stretch = stretchAttribute.array as Float32Array;
    const shadow = shadowAttribute.array as Float32Array;
    const fade = params.waveAmp;
    drops.forEach((d, i) => {
      const grow = 1 + d.z * 1.2; // nearer the camera
      const r = d.r * grow;
      drop.set([screenX(d), screenY(d), r, Math.min(1, d.age / 0.03) * fade], i * 4);
      const sx = d.vx;
      const sy = d.vy * fore + d.vz * lift;
      const speed = Math.hypot(sx, sy) || 1;
      // Stretched as it pinches off, then wobbling round under surface
      // tension.
      const pinch = (d.ligament - 1) * Math.exp(-d.age / 0.045);
      const wobble = 0.16 * Math.sin(d.age * 45 + d.phase) * Math.exp(-d.age / 0.18);
      stretch.set([sx / speed, sy / speed, 1 + pinch + wobble + 0.12 * Math.min(speed / scale, 1)], i * 3);
      // Shadow on the surface, cast away from the light.
      const gx = centre.x + d.x - LIGHT.x * d.z * 0.6;
      const gy = centre.y + d.y * fore - LIGHT.y * d.z * 0.6;
      shadow.set([gx, gy, r * (1.3 + d.z * 4), 0.35 * Math.exp(-d.z * 3) * fade], i * 4);
    });
    dropGeometry.instanceCount = layers.droplets ? drops.length : 0;
    dropAttribute.needsUpdate = stretchAttribute.needsUpdate = shadowAttribute.needsUpdate = true;
  };

  const drawFrame = () => {
    timeline?.time(simTime, true);
    const u = compositeUniforms;
    shared.uTime.value = simTime;
    u.uSimmer.value = params.simmer;
    u.uBulge.value = params.bulge;
    u.uForm.value = params.form;
    u.uRadius.value = params.radius;
    u.uInstability.value = params.instability;
    u.uHole.value = params.hole;
    u.uCrater.value = params.crater;
    u.uCrownAmp.value = params.crownAmp;
    u.uCrownRadius.value = params.crownRadius;
    u.uCrownWidth.value = params.crownWidth;
    u.uRidge.value = params.ridge;
    u.uWaveAmp.value = params.waveAmp;
    bubbles.forEach((b, i) => {
      const grow = Math.min(1, b.age / (b.life * 0.35));
      bubbleSlots[i].set(b.x, b.y, b.r * (0.4 + 0.6 * grow), 0);
    });
    u.uBubbleCount.value = params.waveAmp > 0 ? bubbles.length : 0;

    quad.material = compositeMaterial;
    renderer.setRenderTarget(null);
    renderer.render(quadScene, camera);
    writeDrops();
    if (layers.droplets && drops.length) renderer.render(dropScene, camera);
  };

  const renderAt = (t: number) => {
    if (!ready || t === lastRendered) return;
    lastRendered = t;
    if (t < simTime - STEP_S * 0.5) reset();
    while (simTime + STEP_S <= t + 1e-6) step();
    drawFrame();
  };

  const duration = () => (timeline ? Math.max(timeline.duration(), 4.6) : 0);

  // Reads the canvas straight after drawing (no preserveDrawingBuffer).
  const readCanvas = () => {
    const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
    gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    return pixels;
  };

  const difference = (a: Uint8Array, b: Uint8Array): SplashDifference => {
    let sum = 0;
    let max = 0;
    for (let i = 0; i < a.length; i += 4) {
      for (let c = 0; c < 3; c++) {
        const d = Math.abs(a[i + c] - b[i + c]);
        sum += d;
        max = Math.max(max, d);
      }
    }
    return { mean: sum / ((a.length / 4) * 3), max };
  };

  const plainFrame = (view: number) => {
    const previous = compositeUniforms.uView.value;
    compositeUniforms.uView.value = view;
    drawFrame();
    const pixels = readCanvas();
    compositeUniforms.uView.value = previous;
    return pixels;
  };

  return {
    loadTexture,
    prepare(current, next, size, options) {
      renderer.setPixelRatio(Math.min(maxPixelRatio, Math.sqrt(pixelBudget / (size.width * size.height))));
      renderer.setSize(size.width, size.height, false);
      aspect = size.width / size.height;
      fore = Math.cos(options.tilt);
      lift = Math.sin(options.tilt);
      scale = Math.min(aspect, 1);
      seed = options.seed;

      const gridW = aspect >= 1 ? WAVE_GRID : Math.round(WAVE_GRID * aspect);
      const gridH = aspect >= 1 ? Math.round(WAVE_GRID / aspect) : WAVE_GRID;
      wave.forEach((target) => target.setSize(gridW, gridH));
      reveal.forEach((target) => target.setSize(gridW * 2, gridH * 2));
      waveUniforms.uTexel.value.set(1 / gridW, 1 / gridH);

      shared.uAspect.value = aspect;
      shared.uFore.value = fore;
      shared.uPx.value = 1 / (size.height * renderer.getPixelRatio());
      shared.uCurrent.value = current.texture;
      shared.uNext.value = next.texture;
      const coverCurrent = coverFrame(current.width / current.height, aspect);
      shared.uCoverCurrent.value = coverCurrent;
      currentSource = current;
      currentFrame = coverCurrent;
      shared.uCoverNext.value = coverFrame(next.width / next.height, aspect);
      shared.uTintCurrent.value.set(...current.tint);
      shared.uTintNext.value.set(...next.tint);

      // Everything random about this splash comes from the seed.
      rand = random(seed);
      const [cx, cy] = pickCentre(current, coverCurrent, aspect, options.area ?? MIDDLE, rand);
      centre.set(cx, cy);
      shared.uCentre.value.copy(centre);
      const k1 = 5 + Math.floor(rand() * 4);
      crown.k = [k1, k1 + 2 + Math.floor(rand() * 3), 11 + Math.floor(rand() * 7)];
      crown.a = [between(0.28, 0.42), between(0.15, 0.25), between(0.08, 0.14)];
      crown.phi = [between(0, 6.28), between(0, 6.28), between(0, 6.28)];
      shared.uCrownK.value.set(crown.k[0], crown.k[1], crown.k[2]);
      shared.uCrownA.value.set(crown.a[0], crown.a[1], crown.a[2]);
      shared.uCrownPhi.value.set(crown.phi[0], crown.phi[1], crown.phi[2]);
      const holeAngle = between(-Math.PI, Math.PI);
      compositeUniforms.uHoleDir.value.set(Math.cos(holeAngle), Math.sin(holeAngle));

      const burstAt = 2.05 + between(-0.08, 0.08);
      const r1 = 0.09 * scale;
      const r2 = 0.17 * scale;
      const corners = [[0, 0], [aspect, 0], [0, 1], [aspect, 1]];
      const frontMax = Math.max(...corners.map(([x, y]) => Math.hypot(x - cx, (y - cy) / fore))) * 1.25 + 0.1;
      buildTimeline(burstAt, r1, r2, frontMax);
      buildEvents(burstAt, r2, holeAngle);
      ready = true;
      lastRendered = -1;
      reset();
      drawFrame();
    },
    duration,
    setLayers(next) {
      layers = next;
      compositeUniforms.uLayers.value.set(+next.bubble, +next.crown, +next.waves, +next.lighting);
      lastRendered = -1;
    },
    setView(view) {
      compositeUniforms.uView.value = view === "wave" ? 1 : view === "reveal" ? 2 : 0;
      lastRendered = -1;
    },
    render: renderAt,
    measureHandoff() {
      lastRendered = -1;
      renderAt(0);
      const first = difference(readCanvas(), plainFrame(3));
      renderAt(duration());
      const last = difference(readCanvas(), plainFrame(4));
      lastRendered = -1;
      return { first, last };
    },
    dispose() {
      ready = false;
      timeline?.kill();
      cache.forEach((pending) => pending.then(({ texture }) => texture.dispose()).catch(() => {}));
      [...wave, ...reveal].forEach((target) => target.dispose());
      [waveMaterial, revealMaterial, compositeMaterial, dropMaterial, shadowMaterial].forEach((m) => m.dispose());
      [quadGeometry, plane, dropGeometry].forEach((g) => g.dispose());
      noiseTexture.dispose();
      renderer.dispose();
    },
  };
}
