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
export function whiteBalance(kelvin: number): [number, number, number] {
  const w = blackbody(kelvin);
  const d = blackbody(6500);
  // It expects light of its own temperature and neutralises that: 6500 K light, bluer than
  // an 8000 K setting expects, comes out warmer.
  const g: [number, number, number] = [d[0] / w[0], d[1] / w[1], d[2] / w[2]];
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
  varying vec2 vUv;
  float srgb(float c) { return c <= 0.0031308 ? 12.92 * c : 1.055 * pow(c, 1.0 / 2.4) - 0.055; }
  float lin(float c) { return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4); }
  float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
  vec3 tonemap(vec3 c) {
    return tone == 0 ? NeutralToneMapping(c) : tone == 1 ? AgXToneMapping(c) : ACESFilmicToneMapping(c);
  }
  void main() {
    vec4 c = texture2D(hdr, vUv);
    // The trace's colour is straight (the sky's own light where the model isn't), its
    // alpha the model's coverage.
    vec3 col = c.rgb * exposure * balance;
    float a = c.a;
    // Bloom (its levels summed, divided back to an average) replaces a share of the image,
    // in linear light before the tone curve.
    vec3 bl = texture2D(bloomTex, vUv).rgb * balance / levels;
    vec3 m = tonemap(mix(col, bl, bloom));
    vec3 outc;
    float outa;
    if (hasBackdrop) {
      // The backdrop (already in display colour) under the model, with the glare over it.
      vec3 bd = texture2D(backdrop, vUv).rgb;
      bd = vec3(lin(bd.r), lin(bd.g), lin(bd.b));
      bd = mix(bd, tonemap(bl), bloom);
      outc = m * a + bd * (1.0 - a);
      outa = 1.0;
    } else {
      outc = m;
      outa = a;
    }
    // A lens's natural fall-off (cos⁴ of the angle off axis), blended to its strength.
    vec2 d = (vUv - 0.5) * vec2(aspect, 1.0);
    float r2 = dot(d, d) / (0.25 * (aspect * aspect + 1.0));
    float cos2 = 1.0 / (1.0 + r2 * 0.6);
    outc *= mix(1.0, cos2 * cos2, vignette);
    // Display-space grade, the screen's encoding, a dither against banding in skies.
    vec3 s = vec3(srgb(outc.r), srgb(outc.g), srgb(outc.b));
    float y = dot(s, vec3(0.2126, 0.7152, 0.0722));
    s = mix(vec3(y), s, saturation);
    s = (s - 0.5) * contrast + 0.5;
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
    const wb = whiteBalance(o.kelvin);
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
