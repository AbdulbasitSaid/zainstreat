// The homepage hero's boil-out, drawn in WebGL (three.js). Only
// BoilingSlideshow imports this, dynamically, so three.js stays out of the
// initial bundle.
import {
  LinearFilter,
  Mesh,
  OrthographicCamera,
  PlaneGeometry,
  Scene,
  ShaderMaterial,
  type Texture,
  TextureLoader,
  Vector4,
  WebGLRenderer,
} from "three";

export type BoilTexture = { texture: Texture; width: number; height: number };

const VERTEX_SHADER = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;

// The photo becomes the surface of a pot of soup. Its height field is
// slow rolling swells plus convection ridges (the soup bubbling up), both
// made of the photo itself: the surface only bends the picture under it
// (refraction) and shades it from its own colours — nothing is drawn on
// top. The boil swells from a simmer to a rolling boil and back to still,
// while the old dish thins away all over in soft drifting patches until
// only the next dish is left.
const FRAGMENT_SHADER = /* glsl */ `
  precision highp float;

  uniform sampler2D uCurrent;
  uniform sampler2D uNext;
  uniform vec4 uCoverCurrent; // xy: scale, zw: offset (object-cover framing)
  uniform vec4 uCoverNext;
  uniform float uAspect;
  uniform float uTime;     // seconds into the boil
  uniform float uProgress; // 0 → 1 over the boil

  varying vec2 vUv;

  float hash(vec3 p) {
    p = fract(p * 0.3183099 + 0.1);
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
  }

  float noise(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
      mix(mix(hash(i), hash(i + vec3(1.0, 0.0, 0.0)), f.x),
          mix(hash(i + vec3(0.0, 1.0, 0.0)), hash(i + vec3(1.0, 1.0, 0.0)), f.x), f.y),
      mix(mix(hash(i + vec3(0.0, 0.0, 1.0)), hash(i + vec3(1.0, 0.0, 1.0)), f.x),
          mix(hash(i + vec3(0.0, 1.0, 1.0)), hash(i + vec3(1.0, 1.0, 1.0)), f.x), f.y),
      f.z);
  }

  float fbm(vec3 p) {
    float value = 0.0;
    float amplitude = 0.5;
    for (int i = 0; i < 4; i++) {
      value += amplitude * noise(p);
      p *= 2.03;
      amplitude *= 0.5;
    }
    return value / 0.9375;
  }

  // The soup's surface height at p.
  float surface(vec2 p) {
    float swell = fbm(vec3(p * 3.0, uTime * 0.5));
    float ridge = 1.0 - abs(2.0 * noise(vec3(p * 7.0, uTime * 1.3)) - 1.0);
    return swell * 0.6 + ridge * ridge * 0.4;
  }

  vec3 cover(sampler2D tex, vec4 frame, vec2 uv) {
    return texture2D(tex, frame.zw + clamp(uv, 0.0, 1.0) * frame.xy).rgb;
  }

  void main() {
    float progress = clamp(uProgress, 0.0, 1.0);
    // Simmer → rolling boil (around 60%) → still again by the end.
    float boil = smoothstep(0.0, 0.45, progress) * (1.0 - smoothstep(0.7, 1.0, progress));
    vec2 p = vec2(vUv.x * uAspect, vUv.y);

    // Surface slope, for bending and shading the picture.
    float e = 0.004;
    float h = surface(p);
    vec2 slope = vec2(surface(p + vec2(e, 0.0)) - h, surface(p + vec2(0.0, e)) - h) / e;
    vec2 bend = slope * boil * 0.003;
    vec2 uv = vUv + vec2(bend.x / uAspect, bend.y);

    vec3 normal = normalize(vec3(-slope * boil * 0.35, 1.0));
    vec3 light = normalize(vec3(-0.45, 0.55, 0.7));
    vec3 view = vec3(0.0, 0.0, 1.0);
    // Both relative to a flat surface, so a calm pot is the photo as is.
    float diffuse = dot(normal, light) - light.z;
    float sheen = pow(max(dot(reflect(-light, normal), view), 0.0), 28.0)
      - pow(max(dot(reflect(-light, view), view), 0.0), 28.0);

    // The old dish boils away all over, in soft drifting patches.
    float threshold = mix(-0.45, 1.45, smoothstep(0.12, 0.92, progress));
    float patches = fbm(vec3(p * 2.2, 11.0 + uTime * 0.25));
    float reveal = 1.0 - smoothstep(threshold - 0.35, threshold + 0.35, patches);

    vec3 color = mix(cover(uCurrent, uCoverCurrent, uv), cover(uNext, uCoverNext, uv), reveal);
    color *= 1.0 + clamp(diffuse, -0.2, 0.2) * 0.6;
    // Highlights brighten the soup's own colour, never white.
    color += color * max(sheen, 0.0) * 0.5;
    gl_FragColor = vec4(color, 1.0);
  }
`;

// Scale/offset that crops a texture like CSS object-cover in a box of the
// given aspect.
function coverFrame(textureAspect: number, boxAspect: number): Vector4 {
  if (textureAspect > boxAspect) {
    const scale = boxAspect / textureAspect;
    return new Vector4(scale, 1, (1 - scale) / 2, 0);
  }
  const scale = textureAspect / boxAspect;
  return new Vector4(1, scale, 0, (1 - scale) / 2);
}

export type BoilRenderer = {
  loadTexture: (src: string) => Promise<BoilTexture>;
  // Sizes the canvas, loads one boil and renders its first (calm) frame.
  prepare: (current: BoilTexture, next: BoilTexture, width: number, height: number) => void;
  // Runs the boil on three's animation loop; `onFrame` gets the seconds
  // since start and returns the progress (0–1). Stops itself at 1.
  start: (onFrame: (elapsedS: number) => number, onDone: () => void) => void;
  stop: () => void;
  dispose: () => void;
};

export function createBoilRenderer(canvas: HTMLCanvasElement): BoilRenderer {
  const renderer = new WebGLRenderer({ canvas, antialias: false, alpha: false, powerPreference: "low-power" });
  // fbm per pixel: a 1.5× cap keeps it light on retina screens.
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));

  const uniforms = {
    uCurrent: { value: null as Texture | null },
    uNext: { value: null as Texture | null },
    uCoverCurrent: { value: new Vector4(1, 1, 0, 0) },
    uCoverNext: { value: new Vector4(1, 1, 0, 0) },
    uAspect: { value: 1 },
    uTime: { value: 0 },
    uProgress: { value: 0 },
  };
  const material = new ShaderMaterial({ uniforms, vertexShader: VERTEX_SHADER, fragmentShader: FRAGMENT_SHADER });
  const geometry = new PlaneGeometry(2, 2);
  const scene = new Scene();
  scene.add(new Mesh(geometry, material));
  const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);

  const loader = new TextureLoader();
  loader.setCrossOrigin("anonymous");
  const cache = new Map<string, Promise<BoilTexture>>();

  const loadTexture = (src: string) => {
    let pending = cache.get(src);
    if (!pending) {
      pending = loader.loadAsync(src).then((texture) => {
        texture.minFilter = LinearFilter;
        texture.generateMipmaps = false;
        const image = texture.image as HTMLImageElement;
        return { texture, width: image.naturalWidth, height: image.naturalHeight };
      });
      // A failed load shouldn't stick: let the next boil retry it.
      pending.catch(() => cache.delete(src));
      cache.set(src, pending);
    }
    return pending;
  };

  const stop = () => renderer.setAnimationLoop(null);

  return {
    loadTexture,
    prepare(current, next, width, height) {
      renderer.setSize(width, height, false);
      const aspect = width / height;
      uniforms.uAspect.value = aspect;
      uniforms.uCurrent.value = current.texture;
      uniforms.uNext.value = next.texture;
      uniforms.uCoverCurrent.value = coverFrame(current.width / current.height, aspect);
      uniforms.uCoverNext.value = coverFrame(next.width / next.height, aspect);
      uniforms.uTime.value = 0;
      uniforms.uProgress.value = 0;
      renderer.render(scene, camera);
    },
    start(onFrame, onDone) {
      let startedAt: number | null = null;
      renderer.setAnimationLoop((now) => {
        startedAt ??= now;
        const elapsed = (now - startedAt) / 1000;
        const progress = onFrame(elapsed);
        uniforms.uTime.value = elapsed;
        uniforms.uProgress.value = progress;
        renderer.render(scene, camera);
        if (progress >= 1) {
          stop();
          onDone();
        }
      });
    },
    stop,
    dispose() {
      stop();
      cache.forEach((pending) => pending.then(({ texture }) => texture.dispose()).catch(() => {}));
      material.dispose();
      geometry.dispose();
      renderer.dispose();
    },
  };
}
