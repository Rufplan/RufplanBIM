import { useEffect, useRef, useState } from "react";
import type { SunPosition } from "../bindings/SunPosition";
import { siteImagery } from "../imagery";
import { errorMessage, ipc } from "../ipc";
import {
  BACKGROUNDS,
  libraryId,
  skyBackground,
  skyLibrary,
  type Background,
  type BackgroundId,
} from "../render/backgrounds";
import type { SkyPreset } from "../bindings/SkyPreset";
import type { RenderJob } from "../render/pathtrace";
import { activeViewInfo, useAppStore } from "../store";
import { apply } from "../fileActions";
import { D5_CLOUDS, liveCameras } from "./View3D";
import type { FogSettings } from "../render/fog";
import { plantLoader } from "./AssetLibrary";
import type { Mesh } from "../bindings/Mesh";
import {
  distanceLabel,
  gradeAt,
  hazeVisibility,
  hourAtAltitude,
  lookOf,
  LOOKS,
  mistOf,
  type LookId,
} from "../render/looks";

// Render (RR): a path-traced image of the active 3D or camera view (ADR-027, ADR-028).

const SIZES: [number, number][] = [
  [1280, 720],
  [1920, 1080],
  [2560, 1440],
  [3840, 2160],
  // A film frame, 2.39:1 (the Cinematic look, ADR-120).
  [1920, 804],
  [2560, 1072],
  [3840, 1608],
];
/** Depth of field (ADR-120): the far distance's spread as a share of the image's width. */
const DOF: [string, number][] = [
  ["Off", 0],
  ["Subtle", 0.0025],
  ["Shallow", 0.006],
  ["Strong", 0.012],
];
/** (name, samples, supersampling): Final and Best trace larger and draw it down, so grass,
 * siding lines and window frames don't step (ADR-102). */
const QUALITY: [string, number, number][] = [
  ["Draft", 32, 1],
  ["Medium", 128, 1],
  ["High", 512, 1],
  ["Final", 512, 1.5],
  ["Best", 1024, 2],
];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** 15.25 → "3:15 PM". */
export function clock(hour: number): string {
  const h = Math.floor(hour);
  const m = Math.round((hour - h) * 60);
  const h12 = ((h + 11) % 12) + 1;
  return `${h12}:${String(m).padStart(2, "0")} ${h < 12 ? "AM" : "PM"}`;
}

export type Lighting = "sunsky" | "dome";

/** The key (log-average luminance after exposure) of a daylit exterior at the default
 * exposure: auto exposure opens dimmer scenes up toward it (ADR-118). */
export const AUTO_KEY = 0.15;

/** Revit's Lighting Schemes, and the exposure each starts from: interiors and night
 * scenes need more than daylight exteriors (Revit's exposure control). */
export const SCHEMES = [
  ["Exterior: Sun only", 1],
  ["Exterior: Sun and Artificial", 1],
  ["Exterior: Artificial only", 12],
  ["Interior: Sun only", 2.5],
  ["Interior: Sun and Artificial", 2.5],
  ["Interior: Artificial only", 25],
] as const;
export type Scheme = (typeof SCHEMES)[number][0];

export function RenderDialog({ onClose }: { onClose: () => void }) {
  const projectSun = useAppStore((s) => s.app?.sun ?? null);
  // An unattended render (ADR-095): its settings, then save and (by default) quit.
  const auto = useAppStore((s) => s.autoRender);
  const [scheme, setScheme] = useState<Scheme>(
    (useAppStore.getState().autoRender?.scheme as Scheme | undefined) ?? "Exterior: Sun only",
  );
  const view = useAppStore((s) => activeViewInfo(s));
  const satellite = useAppStore((s) => s.satellite && !!s.app?.site);
  const [size, setSize] = useState(1);
  const [quality, setQuality] = useState(1);
  // The date and time start from the project's Sun Settings (ADR-057).
  const [month, setMonth] = useState(auto?.month ?? projectSun?.month ?? 6);
  const [day, setDay] = useState(auto?.day ?? projectSun?.day ?? 21);
  const [hour, setHour] = useState(auto?.hour ?? projectSun?.hour ?? 15);
  const lightingMode = projectSun?.mode === "Lighting";
  const sunOn = !scheme.endsWith("Artificial only");
  const artificial = scheme.includes("Artificial");
  const baseExposure = SCHEMES.find(([s]) => s === scheme)![1];
  // D5's default: its physical sky with clouds, lit by the site's sun (ADR-065).
  const [background, setBackground] = useState<BackgroundId>(
    (auto?.background as BackgroundId | undefined) ?? "physical",
  );
  const [lighting, setLighting] = useState<Lighting>(auto?.lighting ?? "sunsky");
  // The Sky Library (ADR-101): photographed skies that light the scene.
  const [skies, setSkies] = useState<SkyPreset[]>([]);
  const [matchSun, setMatchSun] = useState(auto?.matchSun ?? true);
  const [preview, setPreview] = useState<{ id: string; url: string } | null>(null);
  useEffect(() => {
    let live = true;
    void skyLibrary().then((s) => live && setSkies(s ?? []));
    return () => {
      live = false;
    };
  }, []);
  const [rotation, setRotation] = useState(auto?.rotation ?? 0);
  // The Look (ADR-120): a studio's atmosphere, light, lens and grade in one choice.
  const [lookId, setLookId] = useState<LookId>(
    auto?.look ?? (auto?.d5 === false ? "plain" : "natural"),
  );
  const look = lookOf(lookId);
  const [strength, setStrength] = useState(auto?.lookStrength ?? 1);
  const [haze, setHaze] = useState(
    auto?.hazeLevel ??
      (auto?.haze !== undefined
        ? auto.haze > 0
          ? Math.log((auto.haze * 3.912) / 30_000) / Math.log(0.01)
          : 0
        : look.haze),
  );
  const [mist, setMist] = useState(auto?.mist ?? look.mist);
  const [overcast, setOvercast] = useState(auto?.overcast ?? look.overcast);
  const [dof, setDof] = useState(auto?.dof ?? look.dof);
  // A camera's exposure in stops (ADR-102); the multiplier is 2^EV.
  const [exposure, setExposure] = useState(
    auto?.ev !== undefined ? Math.pow(2, auto.ev) : (auto?.exposure ?? Math.pow(2, look.ev)),
  );
  const stops = Math.log2(exposure);
  // Corona's and V-Ray's look (ADR-063): filmic highlights, a touch of glare and vignette.
  const [tone, setTone] = useState<"contrast" | "filmic" | "neutral">(auto?.tone ?? look.tone);
  const [glare, setGlare] = useState(auto?.glare ?? true);
  const [vignette, setVignette] = useState(auto?.vignette ?? true);
  const [denoise, setDenoise] = useState(auto?.denoise ?? true);
  // Verticals vertical (ADR-118): a level camera with its lens shifted, as architectural
  // photographs are taken.
  const [twoPoint, setTwoPoint] = useState(auto?.twoPoint ?? true);
  // Opens up dim scenes (dusk, deep shade) as D5's and Enscape's cameras do (ADR-118).
  const [autoExposureOn, setAutoExposureOn] = useState(auto?.autoExposure ?? true);
  const [withBackground, setWithBackground] = useState(true);
  const [sun, setSun] = useState<SunPosition | null>(null);
  const [status, setStatus] = useState("");
  const [progress, setProgress] = useState(0);
  const [running, setRunning] = useState(false);
  const [done, setDone] = useState(false);
  const stage = useRef<HTMLDivElement>(null);
  // Zoom and pan the render (the wheel zooms toward the cursor; drag pans; double-click
  // fits). z 1 is fitted to the stage; x, y the offset in screen px.
  const [view3, setView3] = useState({ z: 1, x: 0, y: 0 });
  const pan = useRef<{ x: number; y: number; ox: number; oy: number } | null>(null);
  useEffect(() => {
    const c = stage.current?.querySelector("canvas");
    if (c) {
      c.style.transform = `translate(${view3.x}px, ${view3.y}px) scale(${view3.z})`;
      // Close up, show the pixels rather than blur them.
      c.style.imageRendering = view3.z > 2 ? "pixelated" : "auto";
    }
  });
  /** Zooms by `k` keeping the stage point (mx, my) (from its centre) still. */
  const zoomAt = (k: number, mx = 0, my = 0) =>
    setView3((v) => {
      const z = Math.min(16, Math.max(1, v.z * k));
      const f = z / v.z;
      return z === 1 ? { z, x: 0, y: 0 } : { z, x: mx - (mx - v.x) * f, y: my - (my - v.y) * f };
    });
  const zoomBy = (k: number) => zoomAt(k);
  const zoomTo = (z: number) => zoomAt(z / view3.z);
  /** The zoom that shows the render pixel for pixel. */
  const actual = () => {
    const c = stage.current?.querySelector("canvas");
    return c && c.offsetWidth ? Math.max(1, c.width / c.offsetWidth) : 1;
  };
  const onWheel = (e: React.WheelEvent) => {
    const r = stage.current?.getBoundingClientRect();
    if (!r) return;
    zoomAt(
      Math.exp(-e.deltaY * 0.0015),
      e.clientX - (r.left + r.width / 2),
      e.clientY - (r.top + r.height / 2),
    );
  };
  const onPanStart = (e: React.PointerEvent) => {
    if (e.button !== 0 || view3.z === 1) return;
    pan.current = { x: e.clientX, y: e.clientY, ox: view3.x, oy: view3.y };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  };
  const onPanMove = (e: React.PointerEvent) => {
    const p = pan.current;
    if (p) setView3((v) => ({ ...v, x: p.ox + e.clientX - p.x, y: p.oy + e.clientY - p.y }));
  };
  const onPanEnd = () => {
    pan.current = null;
  };
  const job = useRef<RenderJob | null>(null);
  const display = useRef<HTMLCanvasElement | null>(null);
  const backdrop = useRef<HTMLCanvasElement | null>(null);
  const finish = useRef<() => void>(() => {});
  // The building alone, for a transparent PNG.
  const cut = useRef<() => HTMLCanvasElement | null>(() => null);

  const libSky = libraryId(background);
  const bg: Background =
    BACKGROUNDS.find((b) => b.id === background) ??
    skyBackground(
      skies.find((s) => s.id === libSky) ?? {
        id: libSky ?? "",
        name: libSky ?? "Sky",
        mood: "",
        note: "",
      },
    );
  // The chosen library sky's preview.
  const thumb = preview && preview.id === libSky ? preview.url : null;
  useEffect(() => {
    if (!libSky) return;
    let live = true;
    let url: string | null = null;
    ipc.skyFile(libSky, "thumb").then(
      (buf) => {
        if (!live || !buf) return;
        url = URL.createObjectURL(new Blob([buf], { type: "image/png" }));
        setPreview({ id: libSky, url });
      },
      () => {},
    );
    return () => {
      live = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [libSky]);
  // A dome light needs a photo to light by.
  const lightingUsed: Lighting = bg.photo ? lighting : "sunsky";

  // The sun at the site for the chosen date and time.
  useEffect(() => {
    let live = true;
    // Sun Settings' Lighting study: its own azimuth and altitude.
    (lightingMode ? ipc.sunNow() : ipc.sunPosition(month, day, hour)).then(
      (s) => live && setSun(s),
      () => live && setSun(null),
    );
    return () => {
      live = false;
    };
  }, [month, day, hour, lightingMode]);

  useEffect(() => () => job.current?.dispose(), []);

  const [w, h] = auto?.width
    ? [auto.width, auto.height ?? Math.round((auto.width * 9) / 16)]
    : SIZES[size]!;
  const samples = auto?.samples ?? QUALITY[quality]![1];
  const supersample = auto?.supersample ?? QUALITY[quality]![2];

  /** Applies a Look's settings (ADR-120); each can then be changed. The golden and blue
   * hours move the sun to theirs for the chosen day. */
  const chooseLook = (id: LookId) => {
    const l = lookOf(id);
    setLookId(id);
    setHaze(l.haze);
    setMist(l.mist);
    setOvercast(l.overcast);
    setDof(l.dof);
    setTone(l.tone);
    setExposure(Math.pow(2, l.ev));
    if (l.artificial && !artificial)
      setScheme(
        scheme.startsWith("Interior")
          ? "Interior: Sun and Artificial"
          : "Exterior: Sun and Artificial",
      );
    const [sw] = SIZES[size]!;
    const cinema = SIZES[size]![0] / SIZES[size]![1] > 2;
    if (l.cinema && !cinema)
      setSize(SIZES.findIndex(([a, b]) => a === Math.max(sw, 1920) && a / b > 2));
    if (!l.cinema && cinema) setSize(SIZES.findIndex(([a, b]) => a === sw && a / b < 2));
    if (l.time && !lightingMode)
      void hourAtAltitude(
        (h) => ipc.sunPosition(month, day, h).then((p) => p.altitude),
        l.time === "golden" ? 7 : -4,
      ).then((h) => h !== null && setHour(Math.min(21, Math.max(5, h))));
  };

  const render = async () => {
    if (!view) return;
    const pose =
      auto?.eye && auto.target
        ? { eye: auto.eye, target: auto.target, fov: auto.fov ?? 50 }
        : (liveCameras.get(view.id) ?? view.camera);
    if (!pose) {
      useAppStore.getState().setError("Orbit the 3D view once, or render a camera view.");
      return;
    }
    job.current?.dispose();
    job.current = null;
    stage.current?.replaceChildren();
    setDone(false);
    setRunning(true);
    setProgress(0);
    setStatus("Preparing the model…");
    try {
      // The path tracer loads on first use.
      const [pt, sky, bgs] = await Promise.all([
        import("../render/pathtrace"),
        import("../render/sky"),
        import("../render/backgrounds"),
      ]);
      // A development aid: translucent foliage off, to compare.
      (await import("../render/ptPatch")).foliage.translucent = auto?.translucent ?? true;
      const {
        buildScene,
        cameraFor,
        composite,
        horizontalIrradiance,
        renderBackdrop,
        RenderJob,
        toYUp,
      } = pt;
      const meshes = await ipc.meshes(view.id);
      const imagery = satellite ? await siteImagery().catch(() => null) : null;
      const levels = useAppStore.getState().app?.levelElevations ?? [0];
      let rot = (rotation * Math.PI) / 180;
      setStatus("Preparing the sky…");
      const sunUp = sunOn && sun && sun.altitude > 0 ? sun : null;
      // Twilight (ADR-120): the sun just below the horizon still lights the sky from where it is.
      const sunSky = sunOn && sun && sun.altitude > -12 ? sun : null;
      let lights = artificial
        ? (await ipc.lights(view.id)).filter((l) => l.on && l.lumens > 0)
        : [];
      // Every light added thins the samples each gets: keep the ones that matter, nearest
      // what the camera looks at (ADR-095).
      const most = auto?.maxLights;
      if (most && lights.length > most) {
        const [tx, ty, tz] = pose.target;
        const d2 = (l: (typeof lights)[number]) =>
          (l.at[0] - tx!) ** 2 + (l.at[1] - ty!) ** 2 + (l.at[2] - tz!) ** 2;
        lights = [...lights].sort((a, b) => d2(a) - d2(b)).slice(0, most);
      }
      const ev = exposure * baseExposure;
      const physical = () =>
        sky.physicalSky({
          sunDir: sunSky ? toYUp(sunSky.dir) : toYUp([0, -1, -0.2]),
          altitude: sunSky ? sunSky.altitude : -6,
          clouds: sunUp ? (auto?.clouds ?? D5_CLOUDS) : 0,
          overcast,
          turbidity: auto?.turbidity,
          // Sunlit to skylit (ADR-095): lower lifts the shadows, as a hazier sky does.
          sunToSky: auto?.sunToSky,
          // 1.5 times the real sun (V-Ray's size multiplier): crisp shadows with a little
          // softening, now that the sun fits the tracer's range (ADR-118).
          sunRadius: auto?.sunRadius ?? 0.4,
          width: 2048,
          height: 1024,
        });
      let env: import("three").DataTexture;
      let intensity = 1;
      if (lightingUsed === "dome") {
        if (bg.library) setStatus("Getting the sky (about 25 MB, the first time only)…");
        env = await bgs.backgroundHdr(bg.id);
        // A library sky's sun, brighter or softer against its sky (ADR-102).
        const sunK = bg.library ? (auto?.sunScale ?? 1) : 1;
        if (sunK !== 1) env = bgs.scaleSun(env, sunK);
        // The sky's own sun turned to where the site's sun is (ADR-101).
        const hs = bg.library && matchSun && sun ? bgs.hdrSun(env) : null;
        if (hs && sun) rot = bgs.matchSunRotation(hs.azimuth, sun.dir);
        if (hs && auto)
          console.warn(
            `Render: sky sun at ${((hs.azimuth * 180) / Math.PI).toFixed(1)}°, ${((hs.elevation * 180) / Math.PI).toFixed(1)}° up; site sun ${sun?.azimuth}°, ${sun?.altitude}° up; rotation ${((rot * 180) / Math.PI).toFixed(1)}°`,
          );
        // A photo HDR is brought to the brightness of a clear afternoon sun & sky, so the
        // exposure means the same in both modes.
        const reference = sky.physicalSky({
          sunDir: toYUp([0.5, -0.5, 0.7]),
          altitude: 45,
          width: 256,
          height: 128,
        });
        intensity = horizontalIrradiance(reference) / Math.max(horizontalIrradiance(env), 1e-6);
      } else {
        env = physical();
      }
      const camera = cameraFor(pose, w / h, twoPoint);
      const photo = bg.photo ? await bgs.backgroundPhoto(bg.id) : null;
      const light = horizontalIrradiance(env) * intensity;
      setStatus("Preparing materials…");
      const materials = await ipc.renderMaterials();
      const groundId = useAppStore.getState().app?.ground ?? null;
      const materialOf = await pt.prepareMaterials(
        groundId ? [...meshes, { material: groundId } as Mesh] : meshes,
        materials,
        (set, map) => ipc.materialTexture(set, map),
        (done, total) => total > 0 && setStatus(`Loading material textures… ${done} of ${total}`),
      );
      const scene = buildScene(meshes, {
        materialOf,
        environment: env,
        environmentIntensity: intensity,
        // Daylit exteriors: the rooms behind the glass read lit (ADR-101).
        // A look's lit rooms (the blue hour) glow against the sky's own light (ADR-120).
        interiorGlow:
          sunOn && scheme.startsWith("Exterior")
            ? (auto?.interiorGlow ?? (look.interiorGlow ? look.interiorGlow * light : 1))
            : 0,
        rotation: lightingUsed === "dome" ? rot : 0,
        ground: bg.ground,
        projection:
          photo && bg.project
            ? {
                photo,
                rotation: rot,
                center: camera.position.clone(),
                height: 1700,
                albedoScale: Math.PI / Math.max(light, 1e-6),
              }
            : null,
        imagery,
        groundZ: Math.min(0, ...levels),
        lights,
        groundMaterial: groundId ? materialOf?.({ material: groundId } as Mesh) : null,
      });
      // Plants and Enscape's grass (ADR-064).
      setStatus("Growing the plants…");
      const [{ loadPlantEntries, plantMeshesYUp }, { grassMeshesYUp }, THREE] = await Promise.all([
        import("../render/plants"),
        import("../render/grass"),
        import("three"),
      ]);
      const instances = await ipc.plantInstances(view.id).catch(() => []);
      const patchSpec = new Map(
        (await ipc.grassPatches(view.id).catch(() => [])).map((p) => [p.el, p.spec]),
      );
      for (const m of plantMeshesYUp(await loadPlantEntries(instances, plantLoader))) scene.add(m);
      // Depth of field (ADR-120), focused where the frame's middle meets the model.
      if (dof > 0) {
        const focus = auto?.focus ?? pt.focusDistance(scene, camera);
        if (focus) pt.setDepthOfField(camera, dof, focus);
        if (auto) console.warn(`Render: depth of field ${dof}, focus ${focus?.toFixed(0)} mm`);
      }
      const byId = new Map(materials.map((m) => [m.id, m]));
      const box = new THREE.Box3();
      const v = new THREE.Vector3();
      let hasSite = false;
      const surfaces: import("../render/grass").GrassSurface[] = [];
      // Topography renders a little low (ADR-118); its grass grows from where it is drawn.
      const grown = (m: Mesh) =>
        m.category === "Site"
          ? m.positions.map((v, i) => (i % 3 === 2 ? v - pt.SITE_DROP : v))
          : m.positions;
      for (const m of meshes) {
        if (m.category === "Site") hasSite = true;
        else
          for (let i = 0; i < m.positions.length; i += 3)
            box.expandByPoint(v.set(m.positions[i]!, m.positions[i + 1]!, m.positions[i + 2]!));
        const painted = patchSpec.get(m.el);
        if (painted) {
          surfaces.push({
            positions: grown(m),
            grass: { height: painted.height, variation: painted.variation },
            color: painted.color,
            kind: painted.kind,
            density: painted.density,
          });
          continue;
        }
        const mat = m.material ? byId.get(m.material) : undefined;
        const cones = mat?.appearance.texture === "gen:pine-straw";
        if (mat && (mat.appearance.grass || cones))
          surfaces.push({
            positions: grown(m),
            grass: mat.appearance.grass ?? null,
            color: mat.color,
            cones,
          });
      }
      const center = box.isEmpty() ? new THREE.Vector3() : box.getCenter(new THREE.Vector3());
      const radius = box.isEmpty() ? 10_000 : box.getSize(new THREE.Vector3()).length() / 2;
      const base = groundId ? byId.get(groundId) : undefined;
      if (
        !hasSite &&
        base &&
        (base.appearance.grass || base.appearance.texture === "gen:pine-straw")
      ) {
        const z = Math.min(0, ...levels) - 2;
        const r = Math.max(radius * 3, 45_000);
        const [x0, y0, x1, y1] = [center.x - r, center.y - r, center.x + r, center.y + r];
        surfaces.push({
          positions: [x0, y0, z, x1, y0, z, x1, y1, z, x0, y0, z, x1, y1, z, x0, y1, z],
          grass: base.appearance.grass ?? null,
          color: base.color,
          cones: base.appearance.texture === "gen:pine-straw",
        });
      }
      const blockers = meshes
        .filter((m) => {
          const mat = m.material ? byId.get(m.material) : undefined;
          const grows = !!mat?.appearance.grass || mat?.appearance.texture === "gen:pine-straw";
          return (
            !grows &&
            m.category !== "Site" &&
            m.category !== "Planting" &&
            m.category !== "GrassPatch"
          );
        })
        .map((m) => m.positions);
      for (const m of grassMeshesYUp(
        surfaces,
        center,
        radius,
        { x: pose.eye[0]!, y: pose.eye[1]! },
        blockers,
        (() => {
          const dx = pose.target[0]! - pose.eye[0]!;
          const dy = pose.target[1]! - pose.eye[1]!;
          const l = Math.hypot(dx, dy) || 1;
          return { x: dx / l, y: dy / l };
        })(),
        // Inside, the lawn shows only through the windows (ADR-118).
        auto?.grass ?? (scheme.startsWith("Interior") ? 40_000 : 120_000),
      )) {
        m.userData.grass = true;
        if (auto)
          console.warn(
            `Render: grass mesh ${(m.geometry.index?.count ?? m.geometry.getAttribute("position").count) / 3} triangles; surfaces ${surfaces.map((x) => `${x.kind ?? "material"} ${x.grass?.height}mm ${x.positions.length / 9}`).join(", ")}`,
          );
        scene.add(m);
      }
      // The backdrop at an exposure `k` times the render's (auto exposure redraws it).
      const makeBackdrop = (k: number) =>
        photo
          ? renderBackdrop(w, h, camera, {
              texture: photo,
              rotation: rot,
              exposure: ev * k * (auto?.skyExposure ?? 1),
              tone,
              // A library sky's photo is already toned: shown as photographed (ADR-101).
              raw: !!bg.library,
              gain: k * (auto?.skyExposure ?? 1),
            })
          : bg.id === "physical"
            ? renderBackdrop(w, h, camera, {
                texture: lightingUsed === "sunsky" ? env : physical(),
                rotation: 0,
                exposure: ev * k * (auto?.skyExposure ?? 1),
                tone,
              })
            : renderBackdrop(w, h, camera, null);
      backdrop.current = makeBackdrop(1);
      // Supersampled: traced larger, drawn down to the image (ADR-102).
      const j = new RenderJob({
        width: Math.round(w * supersample),
        height: Math.round(h * supersample),
        samples,
        exposure: ev,
        tone,
        interior: scheme.startsWith("Interior"),
        bounces: auto?.bounces,
      });
      job.current = j;
      const shown = document.createElement("canvas");
      shown.width = w;
      shown.height = h;
      display.current = shown;
      stage.current?.replaceChildren(shown);
      setView3({ z: 1, x: 0, y: 0 });
      let last = 0;
      // The finished image already holds its backdrop.
      let finished = false;
      const show = () => composite(shown, finished ? null : backdrop.current, j.canvas);
      // The look's grade at its strength (ADR-120).
      const graded = gradeAt(look, strength);
      const finishing = {
        exposure: ev,
        tone,
        bloom: glare ? (auto?.bloom ?? look.bloom) : 0,
        vignette: vignette ? look.vignette : 0,
        // Warmth as a camera's white balance, not a sepia filter (ADR-118).
        kelvin: auto?.kelvin ?? graded.kelvin + 6000 * (auto?.warm ?? 0),
        tint: graded.tint,
        saturation: graded.saturation,
        contrast: graded.contrast,
        grade: graded.grade,
      };
      // The atmosphere (ADR-120): haze and mist by distance, glowing toward the sun. Inside,
      // no mist, and the haze four times as clear (a room's air, not the valley's).
      const inside = scheme.startsWith("Interior");
      const mistIs = inside ? { visibility: 0, height: 0 } : mistOf(mist);
      const fogSettings: FogSettings = {
        visibility: hazeVisibility(haze) * (inside ? 4 : 1),
        start: inside ? 0 : (look.fogStart ?? 0),
        mistVisibility: mistIs.visibility,
        mistHeight: mistIs.height,
        ground: Math.min(0, ...levels),
        color: bgs.horizonColor(env).map((v) => v * intensity) as [number, number, number],
        sunDir: sunUp ? (toYUp(sunUp.dir).normalize().toArray() as [number, number, number]) : null,
        glow: look.glow * (1 - overcast),
        sunColor: sky.sunColor(sunUp?.altitude ?? 5),
      };
      if (auto)
        console.warn(
          `Render: look ${look.id} at ${strength}; haze ${distanceLabel(fogSettings.visibility)}, mist ${distanceLabel(fogSettings.mistVisibility)} ${(fogSettings.mistHeight / 1000).toFixed(0)} m deep, overcast ${overcast}; light ${light.toFixed(3)}; sun ${sun?.altitude.toFixed(1)}° up`,
        );
      cut.current = () => {
        j.finish(null, finishing);
        return pt.cutout(j.canvas, scene, camera);
      };
      finish.current = () => {
        // The guided denoiser (ADR-102); "blur" is the old bilateral one, for comparison.
        if (denoise) {
          if ((auto?.denoiser ?? "guided") === "guided")
            j.guidedDenoise({
              strength: auto?.denoiseStrength ?? 1,
              passes: 5,
            });
        }
        // Aerial perspective now comes with the atmosphere (ADR-120), denoised or not.
        j.atmosphere(fogSettings);
        // Auto exposure (ADR-118): a dusk or dim scene is opened up toward a daylit one's key.
        const key = j.key() * ev;
        const lift = autoExposureOn ? pt.autoExposure(key, AUTO_KEY, 4) : 1;
        if (auto)
          console.warn(`Render: scene key ${key.toFixed(4)}, auto exposure ×${lift.toFixed(2)}`);
        finishing.exposure = ev * lift;
        if (lift !== 1) backdrop.current = makeBackdrop(lift);
        // The finishing pass on the linear image (ADR-118): Corona's and V-Ray's frame buffer
        // order, rather than CSS filters on the 8-bit picture.
        j.finish(backdrop.current, finishing);
        finished = true;
        show();
      };
      await j.start(scene, camera, samples, (n, secs, phase) => {
        setProgress(n / samples);
        const t = `${Math.floor(secs / 60)}:${String(Math.floor(secs % 60)).padStart(2, "0")}`;
        if (phase === "preparing") setStatus("Building the scene…");
        else if (phase === "done") {
          finish.current();
          setStatus(`Done: ${n} samples in ${t}${denoise ? ", denoised" : ""} on ${j.gpu()}.`);
          console.warn(`Render: ${n} samples in ${t} on ${j.gpu()}`);
          setRunning(false);
          setDone(true);
          if (auto && display.current) void finishAuto(display.current);
        } else {
          const now = performance.now();
          if (now - last > 250) {
            last = now;
            show();
          }
          setStatus(`Rendering: ${n} / ${samples} samples · ${t}`);
        }
      });
    } catch (e) {
      setRunning(false);
      setStatus("");
      useAppStore.getState().setError(`Render failed: ${errorMessage(e)}`);
    }
  };

  const finishAuto = async (canvas: HTMLCanvasElement) => {
    if (!auto) return;
    try {
      // A .jpg out is written as a JPEG (the sample's bundled rendering).
      const bytes = /.jpe?g$/i.test(auto.out)
        ? await new Promise<Uint8Array>((ok, fail) =>
            canvas.toBlob(
              (b) => (b ? void b.arrayBuffer().then((a) => ok(new Uint8Array(a))) : fail()),
              "image/jpeg",
              0.9,
            ),
          )
        : await (await import("../render/pathtrace")).pngOf(canvas);
      await ipc.saveRender(auto.out, bytes);
    } finally {
      useAppStore.setState({ autoRender: null });
      if (auto.quit !== false) await ipc.quitApp();
    }
  };
  // Starts by itself once the sun is known.
  const started = useRef(false);
  useEffect(() => {
    if (!auto || started.current || (!sun && !lightingMode)) return;
    started.current = true;
    void render();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [auto, sun]);

  const stop = () => {
    job.current?.stop();
    finish.current();
    setRunning(false);
    setDone(true);
    setStatus((s) => s.replace("Rendering", "Stopped at"));
  };

  const saveImage = async () => {
    const j = job.current;
    if (!j || !view) return;
    const path = await ipc.saveImageDialog(`${view.name}.png`);
    if (!path) return;
    try {
      const { pngOf } = await import("../render/pathtrace");
      // Without the background: the building alone, on transparency.
      const canvas = withBackground && display.current ? display.current : cut.current();
      if (!canvas) return;
      await ipc.saveRender(path, await pngOf(canvas));
      setStatus(`Saved ${path}${withBackground ? "" : " (transparent background)"}`);
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    }
  };

  // Revit's Save to Project (ADR-095): the image, as shown, kept as a Rendering view.
  const saveToProject = async () => {
    const canvas = display.current;
    if (!canvas || !view) return;
    const url = canvas.toDataURL("image/jpeg", 0.92);
    const data = url.slice(url.indexOf(",") + 1);
    const name = `${view.name} - Rendering`;
    if (await apply(() => ipc.saveRendering(name, "image/jpeg", data, canvas.width, canvas.height)))
      setStatus(`Saved to the project as ${name} (Renderings)`);
  };

  if (!view) return null;
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Render">
      <div className="modal render-dialog">
        <div className="render-side">
          <div className="render-title">
            <h2>Render — {view.name}</h2>
            <button className="btn-ghost" onClick={onClose} aria-label="Close">
              ×
            </button>
          </div>
          {/* Settings scroll; the footer (progress, save options, buttons) always shows. */}
          <div className="render-settings">
            <div className="row">
              <label className="field">
                Output size
                <select
                  aria-label="Output size"
                  value={size}
                  onChange={(e) => setSize(Number(e.target.value))}
                  disabled={running}
                >
                  {SIZES.map(([a, b], i) => (
                    <option key={`${a}x${b}`} value={i}>
                      {a} × {b}
                      {a / b > 2 ? " (2.39:1 film)" : ""}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                Quality
                <select
                  aria-label="Quality"
                  value={quality}
                  onChange={(e) => setQuality(Number(e.target.value))}
                  disabled={running}
                >
                  {QUALITY.map(([name, n, ss], i) => (
                    <option key={name} value={i}>
                      {name} ({n}
                      {ss > 1 ? `, ${ss}× supersampled` : ""})
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <h3>Look</h3>
            <label className="field">
              Look
              <select
                aria-label="Look"
                value={lookId}
                onChange={(e) => chooseLook(e.target.value as LookId)}
                disabled={running}
              >
                {LOOKS.map((l) => (
                  <option key={l.id} value={l.id}>
                    {l.label}
                  </option>
                ))}
              </select>
            </label>
            <p className="muted">
              {look.note}
              {look.time === "golden"
                ? " (sets the time to the golden hour)"
                : look.time === "blue"
                  ? " (sets the time to just after sunset)"
                  : ""}
              .
            </p>
            <label className="field">
              Grade strength: {Math.round(strength * 100)}%
              <input
                aria-label="Grade strength"
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={strength}
                onChange={(e) => setStrength(Number(e.target.value))}
                disabled={running}
              />
            </label>
            <h3>Atmosphere</h3>
            <label className="field">
              Haze: {haze > 0 ? `${distanceLabel(hazeVisibility(haze))} visibility` : "none"}
              <input
                aria-label="Haze"
                type="range"
                min={0}
                max={1}
                step={0.02}
                value={haze}
                onChange={(e) => setHaze(Number(e.target.value))}
                disabled={running}
              />
            </label>
            <label className="field">
              Ground mist:{" "}
              {mist > 0
                ? `${distanceLabel(mistOf(mist).visibility)} at the ground, ${Math.round(mistOf(mist).height / 1000)} m deep`
                : "none"}
              <input
                aria-label="Ground mist"
                type="range"
                min={0}
                max={1}
                step={0.02}
                value={mist}
                onChange={(e) => setMist(Number(e.target.value))}
                disabled={running}
              />
            </label>
            <label className="field">
              Overcast: {overcast > 0 ? `${Math.round(overcast * 100)}%` : "clear"}
              <input
                aria-label="Overcast"
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={overcast}
                onChange={(e) => setOvercast(Number(e.target.value))}
                disabled={running || lightingUsed === "dome"}
              />
            </label>
            <label className="field">
              Depth of field
              <select
                aria-label="Depth of field"
                value={DOF.reduce(
                  (b, [, v], i) => (Math.abs(v - dof) < Math.abs(DOF[b]![1] - dof) ? i : b),
                  0,
                )}
                onChange={(e) => setDof(DOF[Number(e.target.value)]![1])}
                disabled={running}
              >
                {DOF.map(([name], i) => (
                  <option key={name} value={i}>
                    {name}
                    {i > 0 ? " (focused on the middle of the frame)" : ""}
                  </option>
                ))}
              </select>
            </label>
            <h3>Background</h3>
            <label className="field">
              Background
              <select
                aria-label="Background"
                value={background}
                onChange={(e) => {
                  const id = e.target.value as BackgroundId;
                  setBackground(id);
                  // A library sky lights the scene by its own sun and sky.
                  if (libraryId(id)) setLighting("dome");
                }}
                disabled={running}
              >
                {BACKGROUNDS.map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.label}
                  </option>
                ))}
                {[...new Set(skies.map((s) => s.mood))].map((mood) => (
                  <optgroup key={mood} label={`Sky Library: ${mood}`}>
                    {skies
                      .filter((s) => s.mood === mood)
                      .map((s) => (
                        <option key={s.id} value={`sky:${s.id}`}>
                          {s.name}: {s.note}
                        </option>
                      ))}
                  </optgroup>
                ))}
              </select>
            </label>
            {bg.library && (
              <div className="sky-preview">
                {thumb ? <img src={thumb} alt={bg.label} /> : <div className="sky-thumb-wait" />}
                <label className="ob-check">
                  <input
                    type="checkbox"
                    aria-label="Turn the sky to the site's sun"
                    checked={matchSun}
                    onChange={(e) => setMatchSun(e.target.checked)}
                    disabled={running}
                  />
                  Turn the sky so its sun is the site&apos;s sun
                </label>
                <span className="muted">Poly Haven, CC0 · downloaded once, then offline</span>
              </div>
            )}
            {bg.photo && (
              <label className="field">
                Rotate background: {rotation}°
                <input
                  aria-label="Background rotation"
                  type="range"
                  min={0}
                  max={359}
                  step={1}
                  value={rotation}
                  onChange={(e) => setRotation(Number(e.target.value))}
                  disabled={running}
                />
              </label>
            )}
            <h3>Lighting</h3>
            <label className="field">
              Lighting Scheme
              <select
                aria-label="Lighting scheme"
                value={scheme}
                onChange={(e) => setScheme(e.target.value as Scheme)}
                disabled={running}
              >
                {SCHEMES.map(([s]) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Light by
              <select
                aria-label="Lighting"
                value={lightingUsed}
                onChange={(e) => setLighting(e.target.value as Lighting)}
                disabled={running || !bg.photo}
              >
                <option value="sunsky">Sun &amp; Sky (site, date and time)</option>
                <option value="dome">Background photo (dome light)</option>
              </select>
            </label>
            {lightingUsed === "sunsky" && lightingMode && (
              <p className="muted">
                Sun Settings: Lighting, {projectSun?.azimuth}° azimuth, {projectSun?.altitude}°
                altitude (Lighting tab &gt; Sun Settings).
              </p>
            )}
            {lightingUsed === "sunsky" && !lightingMode && (
              <>
                <div className="row">
                  <label className="field">
                    Month
                    <select
                      aria-label="Month"
                      value={month}
                      onChange={(e) => setMonth(Number(e.target.value))}
                      disabled={running}
                    >
                      {MONTHS.map((m, i) => (
                        <option key={m} value={i + 1}>
                          {m}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="field">
                    Day
                    <input
                      aria-label="Day"
                      type="number"
                      min={1}
                      max={31}
                      value={day}
                      onChange={(e) =>
                        setDay(Math.max(1, Math.min(31, Number(e.target.value) || 1)))
                      }
                      disabled={running}
                    />
                  </label>
                </div>
                <label className="field">
                  Time: {clock(hour)}
                  <input
                    aria-label="Time of day"
                    type="range"
                    min={5}
                    max={21}
                    step={0.25}
                    value={hour}
                    onChange={(e) => setHour(Number(e.target.value))}
                    disabled={running}
                  />
                </label>
                <p className="muted">
                  {sun
                    ? sun.altitude > 0
                      ? `Sun ${sun.altitude.toFixed(0)}° up, ${sun.azimuth.toFixed(0)}° from north (at the site${useAppStore.getState().app?.site ? "" : ": none set, central USA"}).`
                      : "The sun is down: sky light only."
                    : ""}
                </p>
              </>
            )}
            {lightingUsed === "dome" && (
              <p className="muted">
                Lit by the photo&apos;s own sun and sky; rotate the background to move them.
              </p>
            )}
            <label className="field">
              Exposure: {stops >= 0 ? "+" : "−"}
              {Math.abs(stops).toFixed(1)} EV
              <input
                aria-label="Exposure"
                type="range"
                min={-2}
                max={2}
                step={1 / 3}
                value={stops}
                onChange={(e) => setExposure(Math.pow(2, Number(e.target.value)))}
                disabled={running}
              />
            </label>
            <label className="field">
              Tone
              <select
                aria-label="Tone"
                value={tone}
                onChange={(e) => setTone(e.target.value as "contrast" | "filmic" | "neutral")}
                disabled={running}
              >
                <option value="neutral">Neutral (true whites and greens, no clipping)</option>
                <option value="contrast">Contrast (punchy, V-Ray style)</option>
                <option value="filmic">Filmic (soft highlights)</option>
              </select>
            </label>
            <label
              className="ob-check"
              title="Open up dusk and dim scenes, as D5's and Enscape's cameras do"
            >
              <input
                type="checkbox"
                checked={autoExposureOn}
                onChange={(e) => setAutoExposureOn(e.target.checked)}
                disabled={running}
              />
              Auto exposure
            </label>
            <label
              className="ob-check"
              title="Keep verticals vertical, as an architectural photographer's shift lens does"
            >
              <input
                type="checkbox"
                checked={twoPoint}
                onChange={(e) => setTwoPoint(e.target.checked)}
                disabled={running}
              />
              Two-point perspective
            </label>
            <label className="ob-check">
              <input
                type="checkbox"
                checked={denoise}
                onChange={(e) => setDenoise(e.target.checked)}
                disabled={running}
              />
              Denoise when finished
            </label>
            <label className="ob-check">
              <input
                type="checkbox"
                checked={glare}
                onChange={(e) => setGlare(e.target.checked)}
                disabled={running}
              />
              Lens glare
            </label>
            <label className="ob-check">
              <input
                type="checkbox"
                checked={vignette}
                onChange={(e) => setVignette(e.target.checked)}
                disabled={running}
              />
              Vignette
            </label>
            {satellite && <p className="muted">The satellite image drapes the ground.</p>}
            {bg.source && <p className="muted render-credit">Background: {bg.source}</p>}
          </div>
          <div className="render-footer">
            <div className="render-bar" aria-hidden>
              <div style={{ width: `${Math.round(progress * 100)}%` }} />
            </div>
            <div className="render-progress" role="status">
              {status}
            </div>
            <label className="ob-check">
              <input
                type="checkbox"
                aria-label="Include background"
                checked={withBackground}
                onChange={(e) => setWithBackground(e.target.checked)}
              />
              Save with the background (off: transparent PNG)
            </label>
            <div className="modal-actions">
              {running ? (
                <button className="btn-outline" onClick={stop}>
                  Stop
                </button>
              ) : (
                <button className="btn-cyan" onClick={() => void render()}>
                  Render
                </button>
              )}
              <button className="btn-outline" onClick={() => void saveImage()} disabled={!done}>
                Save Image…
              </button>
              <button className="btn-outline" onClick={() => void saveToProject()} disabled={!done}>
                Save to Project
              </button>
            </div>
          </div>
        </div>
        <div className="render-stage-wrap">
          <div
            className={`render-stage${view3.z !== 1 ? " zoomed" : ""}`}
            ref={stage}
            onWheel={onWheel}
            onPointerDown={onPanStart}
            onPointerMove={onPanMove}
            onPointerUp={onPanEnd}
            onPointerCancel={onPanEnd}
            onDoubleClick={() => setView3({ z: 1, x: 0, y: 0 })}
          >
            <div className="render-empty">
              Path-traced from this view&apos;s camera. The image sharpens as samples add up; stop
              whenever it looks good.
            </div>
          </div>
          <div className="render-zoom" role="toolbar" aria-label="Zoom">
            <button aria-label="Zoom out" onClick={() => zoomBy(1 / 1.5)}>
              −
            </button>
            <button
              onClick={() => setView3({ z: 1, x: 0, y: 0 })}
              title="Fit the image (double-click)"
            >
              Fit
            </button>
            <button onClick={() => zoomTo(actual())} title="Actual pixels">
              100%
            </button>
            <button aria-label="Zoom in" onClick={() => zoomBy(1.5)}>
              +
            </button>
            <span aria-label="Zoom level">{Math.round(view3.z * 100)}%</span>
          </div>
        </div>
      </div>
    </div>
  );
}
