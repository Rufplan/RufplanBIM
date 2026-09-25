import { useEffect, useRef, useState } from "react";
import type { SunPosition } from "../bindings/SunPosition";
import { siteImagery } from "../imagery";
import { errorMessage, ipc } from "../ipc";
import type { RenderJob } from "../render/pathtrace";
import { activeViewInfo, useAppStore } from "../store";
import { liveCameras } from "./View3D";

// Render (RR): a path-traced image of the active 3D or camera view (ADR-027).

const SIZES: [number, number][] = [
  [1280, 720],
  [1920, 1080],
  [2560, 1440],
  [3840, 2160],
];
const QUALITY: [string, number][] = [
  ["Draft", 32],
  ["Medium", 128],
  ["High", 512],
  ["Best", 2048],
];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** 15.25 → "3:15 PM". */
export function clock(hour: number): string {
  const h = Math.floor(hour);
  const m = Math.round((hour - h) * 60);
  const h12 = ((h + 11) % 12) + 1;
  return `${h12}:${String(m).padStart(2, "0")} ${h < 12 ? "AM" : "PM"}`;
}

export function RenderDialog({ onClose }: { onClose: () => void }) {
  const view = useAppStore((s) => activeViewInfo(s));
  const satellite = useAppStore((s) => s.satellite && !!s.app?.site);
  const [size, setSize] = useState(1);
  const [quality, setQuality] = useState(1);
  const [month, setMonth] = useState(6);
  const [day, setDay] = useState(21);
  const [hour, setHour] = useState(15);
  const [background, setBackground] = useState<"sky" | "white">("sky");
  const [exposure, setExposure] = useState(1);
  const [sun, setSun] = useState<SunPosition | null>(null);
  const [status, setStatus] = useState("");
  const [progress, setProgress] = useState(0);
  const [running, setRunning] = useState(false);
  const [done, setDone] = useState(false);
  const stage = useRef<HTMLDivElement>(null);
  const job = useRef<RenderJob | null>(null);

  // The sun at the site for the chosen date and time.
  useEffect(() => {
    let live = true;
    ipc.sunPosition(month, day, hour).then(
      (s) => live && setSun(s),
      () => live && setSun(null),
    );
    return () => {
      live = false;
    };
  }, [month, day, hour]);

  useEffect(() => () => job.current?.dispose(), []);

  const [w, h] = SIZES[size]!;
  const samples = QUALITY[quality]![1];

  const render = async () => {
    if (!view) return;
    const pose = liveCameras.get(view.id) ?? view.camera;
    if (!pose) {
      useAppStore.getState().setError("Orbit the 3D view once, or render a camera view.");
      return;
    }
    job.current?.dispose();
    job.current = null;
    stage.current?.replaceChildren();
    setDone(false);
    setRunning(true);
    setStatus("Preparing the model…");
    try {
      // The path tracer loads on first use.
      const { buildScene, cameraFor, RenderJob } = await import("../render/pathtrace");
      const meshes = await ipc.meshes(view.id);
      const imagery = satellite ? await siteImagery().catch(() => null) : null;
      const levels = useAppStore.getState().app?.levelElevations ?? [0];
      const scene = buildScene(
        meshes,
        { sun: sun && sun.altitude > 0 ? sun : null, background },
        imagery,
        Math.min(0, ...levels),
      );
      const j = new RenderJob({ width: w, height: h, samples, sun, background, exposure });
      job.current = j;
      stage.current?.replaceChildren(j.canvas);
      await j.start(scene, cameraFor(pose, w / h), samples, (n, secs, phase) => {
        setProgress(n / samples);
        const t = `${Math.floor(secs / 60)}:${String(Math.floor(secs % 60)).padStart(2, "0")}`;
        if (phase === "preparing") setStatus("Building the scene…");
        else if (phase === "done") {
          setStatus(`Done: ${n} samples in ${t}.`);
          setRunning(false);
          setDone(true);
        } else setStatus(`Rendering: ${n} / ${samples} samples · ${t}`);
      });
    } catch (e) {
      setRunning(false);
      setStatus("");
      useAppStore.getState().setError(`Render failed: ${errorMessage(e)}`);
    }
  };

  const stop = () => {
    job.current?.stop();
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
      await ipc.saveRender(path, await j.png());
      setStatus(`Saved ${path}`);
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    }
  };

  if (!view) return null;
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Render">
      <div className="modal render-dialog">
        <div className="render-side">
          <h2>Render — {view.name}</h2>
          <label className="field">
            Output size
            <select
              aria-label="Output size"
              value={size}
              onChange={(e) => setSize(Number(e.target.value))}
              disabled={running}
            >
              {SIZES.map(([a, b], i) => (
                <option key={a} value={i}>
                  {a} × {b}
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
              {QUALITY.map(([name, n], i) => (
                <option key={name} value={i}>
                  {name} ({n} samples)
                </option>
              ))}
            </select>
          </label>
          <h3>Sun</h3>
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
                onChange={(e) => setDay(Math.max(1, Math.min(31, Number(e.target.value) || 1)))}
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
          <label className="field">
            Background
            <select
              aria-label="Background"
              value={background}
              onChange={(e) => setBackground(e.target.value as "sky" | "white")}
              disabled={running}
            >
              <option value="sky">Sky</option>
              <option value="white">White</option>
            </select>
          </label>
          <label className="field">
            Exposure: {exposure.toFixed(1)}
            <input
              aria-label="Exposure"
              type="range"
              min={0.3}
              max={2.5}
              step={0.1}
              value={exposure}
              onChange={(e) => setExposure(Number(e.target.value))}
              disabled={running}
            />
          </label>
          {satellite && <p className="muted">The satellite image drapes the ground.</p>}
          <div className="render-bar" aria-hidden>
            <div style={{ width: `${Math.round(progress * 100)}%` }} />
          </div>
          <div className="render-progress" role="status">
            {status}
          </div>
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
            <button className="btn-ghost" onClick={onClose}>
              Close
            </button>
          </div>
        </div>
        <div className="render-stage" ref={stage}>
          <div className="render-empty">
            Path-traced from this view&apos;s camera. The image sharpens as samples add up; stop
            whenever it looks good.
          </div>
        </div>
      </div>
    </div>
  );
}
