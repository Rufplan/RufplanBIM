// A guided denoiser for finished path traces (ADR-102), in the spirit of OIDN's and
// OptiX's albedo- and normal-guided filters, written here rather than added as a
// dependency.
// - A G-buffer rasterized in the render's own WebGL context: each surface's albedo
//   (textures and vertex colours, leaves cut out by their alpha) and its view-space
//   normal and depth. Glass is left out, so the room behind it guides.
// - The image is divided by the albedo (demodulated): what's left is lighting, which is
//   smooth, so the filter can blur it hard without touching texture. Wood grain, paver
//   joints and leaf detail come back when it's multiplied by the albedo again.
// - Five passes of an edge-stopping à-trous wavelet (Dammertz et al. 2010, as in SVGF),
//   at steps 1, 2, 4, 8 and 16 px, weighted by the normal, the depth and the lighting's
//   own (log) brightness, so edges, contact shadows and the sun's terminator stay sharp.
import * as THREE from "three";
import { FullScreenQuad } from "three/examples/jsm/postprocessing/Pass.js";

export interface DenoiseSettings {
  /** How far lighting differences are smoothed (log brightness; 0.6 keeps more). */
  strength: number;
  /** Passes (each doubles the reach): 5 reaches about 60 px. */
  passes: number;
  /** Aerial perspective (ADR-102): the distance (mm) at which 63% of a surface gives way
   * to the horizon's colour; 0 for none. */
  hazeDistance?: number;
  /** The horizon's radiance (linear), what distance fades to. */
  hazeColor?: [number, number, number];
}

export const DEFAULT_DENOISE: DenoiseSettings = { strength: 1, passes: 5 };

const GBUFFER_VERT = /* glsl */ `
  varying vec2 vUv;
  varying vec3 vNormal;
  varying float vDepth;
  #ifdef GB_COLOR
  attribute vec4 color;
  varying vec4 vColor;
  #endif
  void main() {
    vUv = uv;
    #ifdef GB_COLOR
    vColor = color;
    #endif
    vNormal = normalize(normalMatrix * normal);
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    vDepth = -mv.z;
    gl_Position = projectionMatrix * mv;
  }
`;

const GBUFFER_FRAG = /* glsl */ `
  uniform vec3 color;
  uniform sampler2D map;
  uniform bool useMap;
  uniform float alphaTest;
  uniform int mode;
  uniform float weight;
  varying vec2 vUv;
  varying vec3 vNormal;
  varying float vDepth;
  #ifdef GB_COLOR
  varying vec4 vColor;
  #endif
  void main() {
    vec4 c = vec4(color, 1.0);
    if (useMap) c *= texture2D(map, vUv);
    #ifdef GB_COLOR
    c.rgb *= vColor.rgb;
    #endif
    if (c.a < alphaTest) discard;
    if (mode == 0) {
      gl_FragColor = vec4(c.rgb, 1.0) * weight;
    } else {
      vec3 n = normalize(vNormal);
      // Facing the camera, so two-sided cards agree with themselves.
      if (!gl_FrontFacing) n = -n;
      gl_FragColor = vec4(n, vDepth);
    }
  }
`;

const ATROUS_FRAG = /* glsl */ `
  uniform sampler2D lightTex;
  uniform sampler2D normalTex;
  uniform float stepPx;
  uniform float sigmaL;
  uniform float noiseScale;
  varying vec2 vUv;
  float lum(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }
  void main() {
    ivec2 p = ivec2(gl_FragCoord.xy);
    vec4 c0 = texelFetch(lightTex, p, 0);
    // The lighting's own noise here (VARIANCE_FRAG, in alpha) widens the brightness test:
    // noise is smoothed, real edges (converged, so quiet) are kept (SVGF, ADR-118).
    float sl = sigmaL + 1.5 * noiseScale * c0.a;
    vec4 g0 = texelFetch(normalTex, p, 0);
    // Background (sky): nothing to filter.
    if (g0.w <= 0.0) { gl_FragColor = c0; return; }
    float l0 = log(lum(c0.rgb) + 1e-3);
    const float k[3] = float[3](0.375, 0.25, 0.0625);
    vec4 sum = vec4(0.0);
    float wsum = 0.0;
    ivec2 size = textureSize(lightTex, 0);
    for (int j = -2; j <= 2; j++) {
      for (int i = -2; i <= 2; i++) {
        ivec2 q = clamp(p + ivec2(i, j) * int(stepPx), ivec2(0), size - 1);
        vec4 g = texelFetch(normalTex, q, 0);
        if (g.w <= 0.0) continue;
        vec4 c = texelFetch(lightTex, q, 0);
        float wn = pow(max(0.0, dot(g0.xyz, g.xyz)), 64.0);
        // Depth: relative, so far and near surfaces stop alike.
        float wz = exp(-abs(g.w - g0.w) / (0.012 * g0.w * (1.0 + 0.25 * stepPx)));
        float wl = exp(-abs(log(lum(c.rgb) + 1e-3) - l0) / sl);
        float w = k[abs(i)] * k[abs(j)] * wn * wz * wl;
        sum += c * w;
        wsum += w;
      }
    }
    gl_FragColor = wsum > 0.0 ? sum / wsum : c0;
  }
`;

// The lighting's noise: the spread of its log brightness over 5×5 pixels of the same
// surface (normal and depth alike), kept in alpha for the filter (ADR-118).
const VARIANCE_FRAG = /* glsl */ `
  uniform sampler2D lightTex;
  uniform sampler2D normalTex;
  varying vec2 vUv;
  float lum(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }
  void main() {
    ivec2 p = ivec2(gl_FragCoord.xy);
    ivec2 size = textureSize(lightTex, 0);
    vec4 c0 = texelFetch(lightTex, p, 0);
    vec4 g0 = texelFetch(normalTex, p, 0);
    if (g0.w <= 0.0) { gl_FragColor = vec4(c0.rgb, 0.0); return; }
    float n = 0.0, m1 = 0.0, m2 = 0.0;
    for (int j = -2; j <= 2; j++)
      for (int i = -2; i <= 2; i++) {
        ivec2 q = clamp(p + ivec2(i, j), ivec2(0), size - 1);
        vec4 g = texelFetch(normalTex, q, 0);
        if (g.w <= 0.0 || dot(g.xyz, g0.xyz) < 0.9 || abs(g.w - g0.w) > 0.03 * g0.w) continue;
        float l = log(lum(texelFetch(lightTex, q, 0).rgb) + 1e-3);
        n += 1.0;
        m1 += l;
        m2 += l * l;
      }
    float mean = m1 / max(n, 1.0);
    float sd = sqrt(max(m2 / max(n, 1.0) - mean * mean, 0.0));
    gl_FragColor = vec4(c0.rgb, n > 3.0 ? sd : 0.0);
  }
`;

// Fireflies: a pixel far brighter than every neighbour is one stray path; brought down to
// its neighbours' brightest, so the filter doesn't smear it into a blotch.
const FIREFLY_FRAG = /* glsl */ `
  uniform sampler2D lightTex;
  varying vec2 vUv;
  float lum(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }
  void main() {
    ivec2 p = ivec2(gl_FragCoord.xy);
    ivec2 size = textureSize(lightTex, 0);
    vec4 c = texelFetch(lightTex, p, 0);
    float most = 0.0;
    for (int j = -1; j <= 1; j++)
      for (int i = -1; i <= 1; i++) {
        if (i == 0 && j == 0) continue;
        most = max(most, lum(texelFetch(lightTex, clamp(p + ivec2(i, j), ivec2(0), size - 1), 0).rgb));
      }
    float l = lum(c.rgb);
    float cap = most * 1.25 + 1e-4;
    gl_FragColor = l > cap ? vec4(c.rgb * (cap / l), c.a) : c;
  }
`;

const DEMOD_FRAG = /* glsl */ `
  uniform sampler2D colorTex;
  uniform sampler2D albedoTex;
  uniform bool remodulate;
  varying vec2 vUv;
  void main() {
    ivec2 p = ivec2(gl_FragCoord.xy);
    vec4 c = texelFetch(colorTex, p, 0);
    vec3 a = max(texelFetch(albedoTex, p, 0).rgb, vec3(0.02));
    gl_FragColor = remodulate ? vec4(c.rgb * a, c.a) : vec4(c.rgb / a, c.a);
  }
`;

const OUTPUT_FRAG = /* glsl */ `
  uniform sampler2D map;
  uniform sampler2D original;
  uniform sampler2D normalTex;
  uniform float hazeDistance;
  uniform vec3 hazeColor;
  varying vec2 vUv;
  void main() {
    ivec2 p = ivec2(gl_FragCoord.xy);
    vec4 o = texelFetch(original, p, 0);
    float depth = texelFetch(normalTex, p, 0).w;
    // Only where there's a surface; the alpha (and the sky) is the trace's own.
    vec4 c = depth > 0.0 ? vec4(texelFetch(map, p, 0).rgb, o.a) : o;
    if (depth > 0.0 && hazeDistance > 0.0)
      c.rgb = mix(c.rgb, hazeColor, 1.0 - exp(-depth / hazeDistance));
    gl_FragColor = c;
    #include <tonemapping_fragment>
    #include <colorspace_fragment>
    #include <premultiplied_alpha_fragment>
  }
`;

const QUAD_VERT = /* glsl */ `
  varying vec2 vUv;
  void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }
`;

/** The Halton sequence's `i`th value in `base`. */
export function halton(i: number, base: number): number {
  let f = 1;
  let r = 0;
  for (let n = i; n > 0; n = Math.floor(n / base)) {
    f /= base;
    r += f * (n % base);
  }
  return r;
}

/** The G-buffer material for one of the scene's materials. */
function gMaterial(src: THREE.Material, useColor: boolean, mode: number): THREE.ShaderMaterial {
  const m = src as THREE.MeshPhysicalMaterial;
  const map = m.map ?? null;
  return new THREE.ShaderMaterial({
    vertexShader: GBUFFER_VERT,
    fragmentShader: GBUFFER_FRAG,
    defines: useColor ? { GB_COLOR: "" } : {},
    uniforms: {
      color: { value: (m.color ?? new THREE.Color(0.7, 0.7, 0.7)).clone() },
      map: { value: map },
      useMap: { value: !!map },
      alphaTest: { value: m.alphaTest ?? 0 },
      mode: { value: mode },
      weight: { value: 1 },
    },
    side: THREE.DoubleSide,
  });
}

/** Jittered albedo passes: the guide antialiased as the trace is, so demodulating doesn't
 * leave fringes at edges (ADR-118). */
const ALBEDO_PASSES = 8;

const target = (w: number, h: number) =>
  new THREE.WebGLRenderTarget(w, h, {
    type: THREE.HalfFloatType,
    format: THREE.RGBAFormat,
    minFilter: THREE.NearestFilter,
    magFilter: THREE.NearestFilter,
    depthBuffer: true,
  });

/** Draws the denoised trace of `scene` from `camera` to the canvas, tone mapped as the
 * renderer is (ADR-102). `color` is the trace's linear target. */
export function guidedDenoise(
  renderer: THREE.WebGLRenderer,
  color: THREE.Texture,
  scene: THREE.Scene,
  camera: THREE.Camera,
  o: DenoiseSettings = DEFAULT_DENOISE,
  /** Where the result goes: linear, for the finishing pass (ADR-118), or the canvas. */
  out: THREE.WebGLRenderTarget | null = null,
) {
  const { width: w, height: h } = color.image as { width: number; height: number };
  const albedo = target(w, h);
  const normal = target(w, h);
  const ping = target(w, h);
  const pong = target(w, h);
  const made: THREE.Material[] = [];
  const quads: FullScreenQuad[] = [];
  const saved = new Map<THREE.Mesh, THREE.Material | THREE.Material[]>();
  const hidden: THREE.Object3D[] = [];
  const prevTarget = renderer.getRenderTarget();
  const prevClear = renderer.getClearColor(new THREE.Color());
  const prevAlpha = renderer.getClearAlpha();
  const prevBg = scene.background;
  try {
    scene.background = null;
    renderer.setClearColor(0x000000, 0);
    // ---- the G-buffer, both passes
    const meshes: THREE.Mesh[] = [];
    scene.traverse((o) => {
      if (!(o instanceof THREE.Mesh) || !o.visible) return;
      const m = (Array.isArray(o.material) ? o.material[0] : o.material) as
        THREE.MeshPhysicalMaterial | undefined;
      // Glass: the room behind it guides.
      if (m && (m.transmission ?? 0) > 0.9) {
        o.visible = false;
        hidden.push(o);
        return;
      }
      meshes.push(o);
      saved.set(o, o.material);
    });
    for (const [mode, rt] of [
      [0, albedo],
      [1, normal],
    ] as const) {
      for (const mesh of meshes) {
        const src = (saved.get(mesh) as THREE.Material[] | THREE.Material) ?? mesh.material;
        const first = Array.isArray(src) ? src[0]! : src;
        const useColor =
          !!mesh.geometry.getAttribute("color") &&
          !!(first as THREE.MeshPhysicalMaterial).vertexColors;
        const gm = gMaterial(first, useColor, mode);
        made.push(gm);
        mesh.material = gm;
      }
      renderer.setRenderTarget(rt);
      renderer.clear();
      const cam = camera as THREE.PerspectiveCamera;
      if (mode === 0 && cam.isPerspectiveCamera) {
        // Halton (2, 3) offsets within the pixel. Each pass drawn whole (nearest surface only), then added into the average: blending
        // while drawing would sum every surface behind the nearest too.
        const one = target(w, h);
        const add = new THREE.ShaderMaterial({
          vertexShader: QUAD_VERT,
          fragmentShader:
            "uniform sampler2D src; uniform float k; varying vec2 vUv; void main() { gl_FragColor = texture2D(src, vUv) * k; }",
          uniforms: { src: { value: one.texture }, k: { value: 1 / ALBEDO_PASSES } },
          depthTest: false,
          depthWrite: false,
          blending: THREE.AdditiveBlending,
          transparent: true,
        });
        const addQuad = new FullScreenQuad(add);
        made.push(add);
        quads.push(addQuad);
        const autoClear = renderer.autoClear;
        renderer.autoClear = false;
        // On top of the camera's own lens shift (two-point perspective), if it has one.
        const v = cam.view && cam.view.enabled ? { ...cam.view } : null;
        const fw = v?.fullWidth ?? w;
        const fh = v?.fullHeight ?? h;
        for (let s = 0; s < ALBEDO_PASSES; s++) {
          // A pixel is the visible window over the image's width.
          const jx = ((halton(s + 1, 2) - 0.5) * (v?.width ?? fw)) / w;
          const jy = ((halton(s + 1, 3) - 0.5) * (v?.height ?? fh)) / h;
          cam.setViewOffset(
            fw,
            fh,
            (v?.offsetX ?? 0) + jx,
            (v?.offsetY ?? 0) + jy,
            v?.width ?? fw,
            v?.height ?? fh,
          );
          renderer.setRenderTarget(one);
          renderer.clear();
          renderer.render(scene, cam);
          renderer.setRenderTarget(rt);
          addQuad.render(renderer);
        }
        one.dispose();
        renderer.autoClear = autoClear;
        if (v)
          cam.setViewOffset(v.fullWidth, v.fullHeight, v.offsetX, v.offsetY, v.width, v.height);
        else cam.clearViewOffset();
      } else {
        renderer.render(scene, camera);
      }
    }
    for (const [mesh, m] of saved) mesh.material = m;
    saved.clear();
    // ---- demodulate, filter, remodulate
    const pass = (
      frag: string,
      uniforms: Record<string, THREE.IUniform>,
      out: THREE.WebGLRenderTarget | null,
    ) => {
      const mat = new THREE.ShaderMaterial({
        vertexShader: QUAD_VERT,
        fragmentShader: frag,
        uniforms,
        depthTest: false,
        depthWrite: false,
        blending: THREE.NoBlending,
        toneMapped: out === null,
        premultipliedAlpha: out === null && renderer.getContextAttributes().premultipliedAlpha,
      });
      made.push(mat);
      const q = new FullScreenQuad(mat);
      quads.push(q);
      renderer.setRenderTarget(out);
      q.render(renderer);
    };
    pass(
      DEMOD_FRAG,
      {
        colorTex: { value: color },
        albedoTex: { value: albedo.texture },
        remodulate: { value: false },
      },
      ping,
    );
    pass(FIREFLY_FRAG, { lightTex: { value: ping.texture } }, pong);
    pass(
      VARIANCE_FRAG,
      { lightTex: { value: pong.texture }, normalTex: { value: normal.texture } },
      ping,
    );
    let src = ping;
    let dst = pong;
    for (let i = 0; i < o.passes; i++) {
      pass(
        ATROUS_FRAG,
        {
          lightTex: { value: src.texture },
          normalTex: { value: normal.texture },
          stepPx: { value: 1 << i },
          // Tighter each pass, as SVGF's variance falls.
          sigmaL: { value: (0.9 * o.strength) / Math.pow(1.6, i) },
          // The noise falls as the passes average it.
          noiseScale: { value: o.strength / Math.pow(1.5, i) },
        },
        dst,
      );
      [src, dst] = [dst, src];
    }
    pass(
      DEMOD_FRAG,
      {
        colorTex: { value: src.texture },
        albedoTex: { value: albedo.texture },
        remodulate: { value: true },
      },
      dst,
    );
    pass(
      OUTPUT_FRAG,
      {
        map: { value: dst.texture },
        original: { value: color },
        normalTex: { value: normal.texture },
        hazeDistance: { value: o.hazeDistance ?? 0 },
        hazeColor: { value: new THREE.Vector3(...(o.hazeColor ?? [0, 0, 0])) },
      },
      out,
    );
  } finally {
    for (const [mesh, m] of saved) mesh.material = m;
    for (const o of hidden) o.visible = true;
    scene.background = prevBg;
    renderer.setClearColor(prevClear, prevAlpha);
    renderer.setRenderTarget(prevTarget);
    for (const q of quads) q.dispose();
    for (const m of made) m.dispose();
    for (const t of [albedo, normal, ping, pong]) t.dispose();
  }
}
