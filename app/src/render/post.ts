// The finishing pass of a rendering (ADR-118), on the linear HDR image as Corona's and
// V-Ray's frame buffers work (not on the 8-bit picture, as CSS filters did):
//   exposure → white balance (Kelvin) → bloom (added in linear) → tone curve → the backdrop
//   composited under → vignette (cos⁴) → saturation and contrast → sRGB → dither.
// Bloom is Jimenez's (Call of Duty: Advanced Warfare, SIGGRAPH 2014): a chain of half-size
// 13-tap downsamples, Karis-averaged on the first so a single bright pixel doesn't flare,
// summed back up with a 3×3 tent, and mixed in at a few percent with no threshold. Only
// light far brighter than white (the sun in glass, glints, lamps) visibly blooms, as a
// lens's veiling glare does.
import * as THREE from "three";
import { FullScreenQuad } from "three/examples/jsm/postprocessing/Pass.js";
import type { Tone } from "./pathtrace";
import { FOG_GLSL, fogUniforms, type FogSettings } from "./fog";

export interface FinishSettings {
  /** Multiplies the scene's radiance (the camera's exposure). */
  exposure: number;
  tone: Tone;
  /** Glare: the share of the image that is bloom (0 none, about 0.04 a lens's). */
  bloom: number;
  /** Vignette strength (0 none, 1 a full cos⁴ fall-off at the corners). */
  vignette: number;
  /** The camera's white balance (K): 6500 neutral; above, the image warms. */
  kelvin: number;
  /** Display-space saturation and contrast (1 unchanged; D5's look is about 1.14, 1.06). */
  saturation: number;
  contrast: number;
  /** White balance's green-magenta tint (0 none; ADR-120). */
  tint?: number;
  /** The look's grade (ADR-120); each is "none" when left out. */
  grade?: Partial<Grade>;
}

/** A colourist's grade beyond saturation and contrast (ADR-120): what sets MIR's, The
 * Boundary's or an editorial photographer's images apart. */
export interface Grade {
  /** Lifted blacks (a matte print): the darkest the image goes, 0–0.1. */
  fade: number;
  /** Tints added in the shadows and the highlights (display units, ±0.05). */
  shadows: [number, number, number];
  highlights: [number, number, number];
  /** Greens' saturation (1 unchanged; MIR's olive foliage about 0.7). */
  greens: number;
  /** Film grain (0 none; 0.03 visible, 0.06 strong). */
  grain: number;
  /** Lateral chromatic aberration at the corners (px). */
  aberration: number;
  /** Output sharpening (an unsharp mask's amount, 0–0.6). */
  sharpen: number;
}

export const NO_GRADE: Grade = {
  fade: 0,
  shadows: [0, 0, 0],
  highlights: [0, 0, 0],
  greens: 1,
  grain: 0,
  aberration: 0,
  sharpen: 0,
};

/** The atmosphere for [`finish`]: the fog pass's target and the camera's matrices. */
export interface FinishFog {
  settings: FogSettings;
  share: THREE.Texture;
  camera: THREE.PerspectiveCamera;
}

export const DEFAULT_FINISH: FinishSettings = {
  exposure: 1,
  tone: "neutral",
  bloom: 0.025,
  vignette: 0.35,
  kelvin: 6500,
  saturation: 1,
  contrast: 1,
};

/** Linear sRGB of a black body at `k` kelvin (Tanner Helland's fit), luminance 1. */
export function blackbody(k: number): [number, number, number] {
  const t = Math.min(Math.max(k, 1000), 40000) / 100;
  const r = t <= 66 ? 255 : 329.698727446 * Math.pow(t - 60, -0.1332047592);
  const g =
    t <= 66
      ? 99.4708025861 * Math.log(t) - 161.1195681661
      : 288.1221695283 * Math.pow(t - 60, -0.0755148492);
  const b = t >= 66 ? 255 : t <= 19 ? 0 : 138.5177312231 * Math.log(t - 10) - 305.0447927307;
  const lin = (v: number) => {
    const c = Math.min(Math.max(v, 0), 255) / 255;
    return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  const rgb: [number, number, number] = [lin(r), lin(g), lin(b)];
  const y = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
  return [rgb[0] / y, rgb[1] / y, rgb[2] / y];
}

/** The per-channel gain of a white balance: a camera balanced for `kelvin` sees 6500 K
 * light as this colour, so a setting above 6500 warms the image (V-Ray's and Corona's
 * convention). Luminance is kept. */
export function whiteBalance(kelvin: number, tint = 0): [number, number, number] {
  const w = blackbody(kelvin);
  const d = blackbody(6500);
  // It expects light of its own temperature and neutralises that: 6500 K light, bluer than
  // an 8000 K setting expects, comes out warmer. The tint is a camera's green-magenta axis
  // (Lightroom's Tint): above 0, greener (twilight's ozone magenta taken out).
  const g: [number, number, number] = [d[0] / w[0], (d[1] / w[1]) * (1 + tint), d[2] / w[2]];
  const y = 0.2126 * g[0] + 0.7152 * g[1] + 0.0722 * g[2];
  return [g[0] / y, g[1] / y, g[2] / y];
}

const VERT = /* glsl */ `
  varying vec2 vUv;
  void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }
`;

// 13 taps around the texel (Jimenez's pattern), the first level Karis-averaged.
const DOWN_FRAG = /* glsl */ `
  uniform sampler2D src;
  uniform vec2 texel;
  uniform bool karis;
  uniform float scale;
  varying vec2 vUv;
  vec3 tap(vec2 o) { return texture2D(src, vUv + o * texel).rgb * scale; }
  float kw(vec3 c) { return 1.0 / (1.0 + dot(c, vec3(0.2126, 0.7152, 0.0722))); }
  void main() {
    vec3 a = tap(vec2(-2.0, 2.0)), b = tap(vec2(0.0, 2.0)), c = tap(vec2(2.0, 2.0));
    vec3 d = tap(vec2(-2.0, 0.0)), e = tap(vec2(0.0, 0.0)), f = tap(vec2(2.0, 0.0));
    vec3 g = tap(vec2(-2.0, -2.0)), h = tap(vec2(0.0, -2.0)), i = tap(vec2(2.0, -2.0));
    vec3 j = tap(vec2(-1.0, 1.0)), k = tap(vec2(1.0, 1.0));
    vec3 l = tap(vec2(-1.0, -1.0)), m = tap(vec2(1.0, -1.0));
    vec3 o;
    if (karis) {
      // Five 2×2 groups, each weighted by 1 / (1 + luma), so fireflies don't bloom.
      vec3 g0 = (a + b + d + e) * 0.25, g1 = (b + c + e + f) * 0.25;
      vec3 g2 = (d + e + g + h) * 0.25, g3 = (e + f + h + i) * 0.25;
      vec3 g4 = (j + k + l + m) * 0.25;
      float w0 = kw(g0) * 0.125, w1 = kw(g1) * 0.125, w2 = kw(g2) * 0.125, w3 = kw(g3) * 0.125;
      float w4 = kw(g4) * 0.5;
      o = (g0 * w0 + g1 * w1 + g2 * w2 + g3 * w3 + g4 * w4) / (w0 + w1 + w2 + w3 + w4);
    } else {
      o = e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
    }
    gl_FragColor = vec4(o, 1.0);
  }
`;

// A 3×3 tent of the coarser level added onto this one.
const UP_FRAG = /* glsl */ `
  uniform sampler2D src;
  uniform sampler2D base;
  uniform vec2 texel;
  varying vec2 vUv;
  void main() {
    vec3 s = texture2D(src, vUv + vec2(-1.0, 1.0) * texel).rgb
      + 2.0 * texture2D(src, vUv + vec2(0.0, 1.0) * texel).rgb
      + texture2D(src, vUv + vec2(1.0, 1.0) * texel).rgb
      + 2.0 * texture2D(src, vUv + vec2(-1.0, 0.0) * texel).rgb
      + 4.0 * texture2D(src, vUv).rgb
      + 2.0 * texture2D(src, vUv + vec2(1.0, 0.0) * texel).rgb
      + texture2D(src, vUv + vec2(-1.0, -1.0) * texel).rgb
      + 2.0 * texture2D(src, vUv + vec2(0.0, -1.0) * texel).rgb
      + texture2D(src, vUv + vec2(1.0, -1.0) * texel).rgb;
    gl_FragColor = vec4(texture2D(base, vUv).rgb + s / 16.0, 1.0);
  }
`;

const FINAL_FRAG = /* glsl */ `
  #include <tonemapping_pars_fragment>
  ${FOG_GLSL}
  uniform sampler2D hdr;
  uniform sampler2D bloomTex;
  uniform sampler2D backdrop;
  uniform bool hasBackdrop;
  uniform float exposure;
  uniform vec3 balance;
  uniform float bloom;
  uniform float levels;
  uniform float vignette;
  uniform float aspect;
  uniform float saturation;
  uniform float contrast;
  uniform int tone;
  // Atmosphere (fog.ts): the fog's share of the model per pixel, its colour, the sun glow.
  uniform bool hasFog;
  uniform sampler2D fogTex;
  uniform vec3 fogColor;
  uniform vec3 fogSun;
  uniform vec3 sunDir;
  uniform float glow;
  uniform mat4 projInv;
  uniform mat4 camWorld;
  // The look (ADR-120).
  uniform float sharpen;
  uniform float aberration;
  uniform float fade;
  uniform vec3 shadows;
  uniform vec3 highlights;
  uniform float greens;
  uniform float grain;
  uniform vec2 size;
  varying vec2 vUv;
  float srgb(float c) { return c <= 0.0031308 ? 12.92 * c : 1.055 * pow(c, 1.0 / 2.4) - 0.055; }
  float lin(float c) { return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4); }
  float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
  float luma(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }
  vec3 tonemap(vec3 c) {
    return tone == 0 ? NeutralToneMapping(c) : tone == 1 ? AgXToneMapping(c) : ACESFilmicToneMapping(c);
  }
  // The world direction through a point of the image.
  vec3 rayDir(vec2 uv) {
    vec4 v = projInv * vec4(uv * 2.0 - 1.0, 1.0, 1.0);
    return normalize((camWorld * vec4(v.xyz / v.w, 0.0)).xyz);
  }
  // The haze's colour along a direction: the horizon's, glowing toward the sun
  // (Henyey-Greenstein, g 0.4, mean 1 over the sphere) in the sun's colour.
  vec3 hazeColor(vec3 d) {
    float c = dot(d, sunDir);
    float hg = 0.84 / pow(1.16 - 0.8 * c, 1.5);
    return fogColor * (1.0 - glow) + fogSun * glow * hg;
  }
  // The image at uv, toned and over its backdrop, in linear display light.
  vec3 compose(vec2 uv, out float alpha) {
    vec4 c = texture2D(hdr, uv);
    // The trace's colour is straight (the sky's own light where the model isn't), its
    // alpha the model's coverage.
    vec3 col = c.rgb;
    if (sharpen > 0.0) {
      // An unsharp mask on the linear image, as a photographer's output sharpening.
      vec2 px = 1.0 / vec2(textureSize(hdr, 0));
      vec3 n = texture2D(hdr, uv + vec2(px.x, 0.0)).rgb + texture2D(hdr, uv - vec2(px.x, 0.0)).rgb
        + texture2D(hdr, uv + vec2(0.0, px.y)).rgb + texture2D(hdr, uv - vec2(0.0, px.y)).rgb;
      col = max(col + sharpen * (col - 0.25 * n), vec3(0.0));
    }
    col *= exposure * balance;
    float a = c.a;
    vec3 d = rayDir(uv);
    vec3 haze = hazeColor(d) * exposure * balance;
    float skyFog = 0.0;
    if (hasFog) {
      vec4 f = texture2D(fogTex, uv);
      col = mix(col, haze, f.a > 1e-3 ? clamp(f.r / f.a, 0.0, 1.0) : 0.0);
      skyFog = fogShare(d, 0.0, true);
    }
    // Bloom (its levels summed, divided back to an average) replaces a share of the image,
    // in linear light before the tone curve.
    vec3 bl = texture2D(bloomTex, uv).rgb * balance / levels;
    vec3 m = tonemap(mix(col, bl, bloom));
    if (hasBackdrop) {
      // The backdrop (already in display colour) under the model, sunk into the mist, with
      // the glare over it.
      vec3 bd = texture2D(backdrop, uv).rgb;
      bd = vec3(lin(bd.r), lin(bd.g), lin(bd.b));
      // The camera's white balance holds for the sky too (ADR-120).
      bd *= balance;
      bd = mix(bd, tonemap(haze), skyFog);
      bd = mix(bd, tonemap(bl), bloom);
      alpha = 1.0;
      return m * a + bd * (1.0 - a);
    }
    alpha = a;
    return m;
  }
  void main() {
    float outa;
    vec3 outc = compose(vUv, outa);
    if (aberration > 0.0) {
      // Lateral chromatic aberration: red a little outward, blue inward, growing to
      // \`aberration\` px at the corners, as a real lens fringes.
      vec2 r = (vUv - 0.5);
      vec2 off = r * aberration / (0.5 * length(size));
      float ar, ab;
      outc.r = compose(vUv + off, ar).r;
      outc.b = compose(vUv - off, ab).b;
    }
    // A lens's natural fall-off (cos⁴ of the angle off axis), blended to its strength.
    vec2 d = (vUv - 0.5) * vec2(aspect, 1.0);
    float r2 = dot(d, d) / (0.25 * (aspect * aspect + 1.0));
    float cos2 = 1.0 / (1.0 + r2 * 0.6);
    outc *= mix(1.0, cos2 * cos2, vignette);
    // The grade, in display space as a colourist's: saturation, greens, split toning,
    // an S-curve's contrast, lifted blacks, grain; then a dither against banding.
    vec3 s = clamp(vec3(srgb(outc.r), srgb(outc.g), srgb(outc.b)), 0.0, 1.0);
    float y = luma(s);
    s = mix(vec3(y), s, saturation);
    // Greens muted toward olive (MIR's foliage) below 1, richer above.
    float gm = smoothstep(0.0, 0.06, s.g - max(s.r, s.b));
    vec3 muted = mix(vec3(y), s, greens);
    muted.r += (muted.g - muted.r) * max(0.0, 1.0 - greens) * 0.35;
    s = mix(s, muted, gm);
    // Split toning: tints added in the shadows and the highlights.
    s += shadows * (1.0 - y) * (1.0 - y) + highlights * y * y;
    s = clamp(s, 0.0, 1.0);
    // Contrast as an S-curve about mid grey: never clips, unlike a straight stretch.
    vec3 lo = 0.5 * pow(2.0 * s, vec3(contrast));
    vec3 hi = 1.0 - 0.5 * pow(2.0 - 2.0 * s, vec3(contrast));
    s = mix(lo, hi, step(0.5, s));
    // A matte: blacks lifted to \`fade\`, white kept.
    s = fade + s * (1.0 - fade);
    if (grain > 0.0) {
      // Film grain: monochrome, strongest in the mid-tones, about a pixel and a half.
      vec2 g = floor(gl_FragCoord.xy / 1.5);
      float n = hash(g) + hash(g + 17.31) + hash(g + 43.7) - 1.5;
      s += grain * n * (0.35 + 2.6 * y * (1.0 - y));
    }
    s += (hash(gl_FragCoord.xy) - 0.5) / 255.0;
    gl_FragColor = vec4(clamp(s, 0.0, 1.0) * outa, outa);
  }
`;

function rt(w: number, h: number) {
  return new THREE.WebGLRenderTarget(Math.max(1, w), Math.max(1, h), {
    type: THREE.HalfFloatType,
    format: THREE.RGBAFormat,
    minFilter: THREE.LinearFilter,
    magFilter: THREE.LinearFilter,
    depthBuffer: false,
  });
}

/** Draws the finished image to the renderer's canvas: `hdr` (premultiplied linear, the
 * trace or its denoised copy) with `backdrop` (a display-colour canvas, or none for a
 * transparent cut-out) behind it. */
export function finish(
  renderer: THREE.WebGLRenderer,
  hdr: THREE.Texture,
  backdrop: HTMLCanvasElement | null,
  o: FinishSettings,
  /** Haze and mist (ADR-120), or none. */
  fog: FinishFog | null = null,
) {
  const { width: w, height: h } = hdr.image as { width: number; height: number };
  const made: (THREE.Material | THREE.WebGLRenderTarget | THREE.Texture)[] = [];
  const quads: FullScreenQuad[] = [];
  const prev = renderer.getRenderTarget();
  const prevExposure = renderer.toneMappingExposure;
  const run = (
    frag: string,
    uniforms: Record<string, THREE.IUniform>,
    out: THREE.WebGLRenderTarget | null,
  ) => {
    const m = new THREE.ShaderMaterial({
      vertexShader: VERT,
      fragmentShader: frag,
      uniforms,
      depthTest: false,
      depthWrite: false,
      blending: THREE.NoBlending,
      toneMapped: false,
      transparent: out === null,
      premultipliedAlpha: true,
    });
    made.push(m);
    const q = new FullScreenQuad(m);
    quads.push(q);
    renderer.setRenderTarget(out);
    q.render(renderer);
  };
  try {
    renderer.toneMappingExposure = 1;
    // The bloom chain: six halvings, summed back up.
    const chain: THREE.WebGLRenderTarget[] = [];
    let cw = w;
    let ch = h;
    let src: THREE.Texture = hdr;
    let sw = w;
    let sh = h;
    for (let i = 0; i < 6 && cw > 8 && ch > 8; i++) {
      cw = Math.ceil(cw / 2);
      ch = Math.ceil(ch / 2);
      const t = rt(cw, ch);
      made.push(t);
      run(
        DOWN_FRAG,
        {
          src: { value: src },
          texel: { value: new THREE.Vector2(1 / sw, 1 / sh) },
          karis: { value: i === 0 },
          scale: { value: i === 0 ? o.exposure : 1 },
        },
        t,
      );
      chain.push(t);
      src = t.texture;
      sw = cw;
      sh = ch;
    }
    for (let i = chain.length - 1; i > 0; i--) {
      const coarse = chain[i]!;
      const fine = chain[i - 1]!;
      const t = rt(fine.width, fine.height);
      made.push(t);
      run(
        UP_FRAG,
        {
          src: { value: coarse.texture },
          base: { value: fine.texture },
          texel: { value: new THREE.Vector2(1 / coarse.width, 1 / coarse.height) },
        },
        t,
      );
      chain[i - 1] = t;
    }
    const bd = backdrop ? new THREE.CanvasTexture(backdrop) : null;
    if (bd) {
      bd.colorSpace = THREE.NoColorSpace;
      bd.flipY = true;
      made.push(bd);
    }
    const wb = whiteBalance(o.kelvin, o.tint ?? 0);
    const g = { ...NO_GRADE, ...o.grade };
    const f = fog?.settings;
    const cam = fog?.camera;
    const sun = f?.sunDir ?? [0, 1, 0];
    // The glow's colour: the haze's brightness in the sun's colour.
    const hl = f ? 0.2126 * f.color[0] + 0.7152 * f.color[1] + 0.0722 * f.color[2] : 0;
    const sc = f?.sunColor ?? [1, 1, 1];
    const sl = 0.2126 * sc[0] + 0.7152 * sc[1] + 0.0722 * sc[2] || 1;
    run(
      FINAL_FRAG,
      {
        hdr: { value: hdr },
        // Bloom is relative to the exposed image; divide the exposure back out here.
        bloomTex: { value: chain[0]?.texture ?? hdr },
        backdrop: { value: bd },
        hasBackdrop: { value: !!bd },
        exposure: { value: o.exposure },
        balance: { value: new THREE.Vector3(...wb) },
        bloom: { value: chain.length ? o.bloom : 0 },
        levels: { value: Math.max(1, chain.length) },
        vignette: { value: o.vignette },
        aspect: { value: w / h },
        saturation: { value: o.saturation },
        contrast: { value: o.contrast },
        tone: { value: o.tone === "neutral" ? 0 : o.tone === "filmic" ? 1 : 2 },
        toneMappingExposure: { value: 1 },
        ...fogUniforms(
          f ?? {
            visibility: 0,
            mistVisibility: 0,
            mistHeight: 0,
            ground: 0,
            color: [0, 0, 0],
            sunDir: null,
            glow: 0,
            sunColor: [1, 1, 1],
          },
          cam?.position ?? new THREE.Vector3(),
        ),
        hasFog: { value: !!fog },
        fogTex: { value: fog?.share ?? null },
        fogColor: { value: new THREE.Vector3(...(f?.color ?? [0, 0, 0])) },
        fogSun: { value: new THREE.Vector3(...sc.map((c) => (c / sl) * hl)) },
        sunDir: { value: new THREE.Vector3(...sun).normalize() },
        glow: { value: f?.sunDir ? f.glow : 0 },
        projInv: { value: cam ? cam.projectionMatrixInverse.clone() : new THREE.Matrix4() },
        camWorld: { value: cam ? cam.matrixWorld.clone() : new THREE.Matrix4() },
        sharpen: { value: g.sharpen },
        aberration: { value: g.aberration },
        fade: { value: g.fade },
        shadows: { value: new THREE.Vector3(...g.shadows) },
        highlights: { value: new THREE.Vector3(...g.highlights) },
        greens: { value: g.greens },
        grain: { value: g.grain },
        size: { value: new THREE.Vector2(w, h) },
      },
      null,
    );
  } finally {
    renderer.setRenderTarget(prev);
    renderer.toneMappingExposure = prevExposure;
    for (const q of quads) q.dispose();
    for (const m of made) m.dispose();
  }
}

const KEY_FRAG = /* glsl */ `
  uniform sampler2D hdr;
  varying vec2 vUv;
  void main() {
    // A 4×4 box of the full image for each texel of the small target.
    vec2 px = 1.0 / vec2(textureSize(hdr, 0));
    vec3 s = vec3(0.0);
    for (int j = 0; j < 4; j++)
      for (int i = 0; i < 4; i++)
        s += texture2D(hdr, vUv + (vec2(i, j) - 1.5) * px * 4.0).rgb;
    gl_FragColor = vec4(s / 16.0, 1.0);
  }
`;

/** The scene's key (ADR-118): the log-average luminance of the linear image, as a camera's
 * meter reads it, over a small grid of it (the brightest 2% left out, as the sun is). */
export function sceneKey(renderer: THREE.WebGLRenderer, hdr: THREE.Texture): number {
  const w = 96;
  const h = 54;
  const t = new THREE.WebGLRenderTarget(w, h, {
    type: THREE.FloatType,
    format: THREE.RGBAFormat,
    depthBuffer: false,
  });
  const m = new THREE.ShaderMaterial({
    vertexShader: VERT,
    fragmentShader: KEY_FRAG,
    uniforms: { hdr: { value: hdr } },
    depthTest: false,
    depthWrite: false,
    blending: THREE.NoBlending,
    toneMapped: false,
  });
  const q = new FullScreenQuad(m);
  const prev = renderer.getRenderTarget();
  try {
    renderer.setRenderTarget(t);
    q.render(renderer);
    const px = new Float32Array(w * h * 4);
    renderer.readRenderTargetPixels(t, 0, 0, w, h, px);
    const lum: number[] = [];
    for (let i = 0; i < px.length; i += 4) {
      const l = 0.2126 * px[i]! + 0.7152 * px[i + 1]! + 0.0722 * px[i + 2]!;
      if (Number.isFinite(l)) lum.push(Math.max(l, 1e-5));
    }
    return logAverage(lum, 0.02);
  } finally {
    renderer.setRenderTarget(prev);
    q.dispose();
    m.dispose();
    t.dispose();
  }
}

/** The geometric mean of `values`, the brightest `trim` share left out. */
export function logAverage(values: number[], trim = 0): number {
  if (!values.length) return 0;
  const v = [...values].sort((a, b) => a - b);
  const keep = v.slice(0, Math.max(1, Math.ceil(v.length * (1 - trim))));
  return Math.exp(keep.reduce((s, x) => s + Math.log(x), 0) / keep.length);
}

/** Auto exposure (ADR-118), as D5's, Enscape's and Lumion's: a scene darker than a daylit
 * exterior (`target`, the key our exposures were calibrated on) is opened up toward it, at
 * most `most` stops; brighter scenes are left as they are. Returns the multiplier. */
export function autoExposure(key: number, target: number, most = 2.5): number {
  if (!(key > 0) || key >= target) return 1;
  return Math.min(Math.pow(2, most), target / key);
}
