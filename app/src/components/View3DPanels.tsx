import { useEffect, useRef, useState } from "react";
import type { SunPosition } from "../bindings/SunPosition";
import type { SunSettings } from "../bindings/SunSettings";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import type { NavMode } from "../render/navigate";
import { useAppStore } from "../store";
import { clock, SUN_PRESETS } from "./LightingDialogs";

// The 3D view's top-left panels (ADR-063): the sun, as Enscape's and D5's time-of-day
// slider, and Enscape's Orbit / Walk / Fly navigation.

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DEFAULT_SUN: SunSettings = {
  mode: "Still",
  month: 6,
  day: 21,
  hour: 15,
  azimuth: 225,
  altitude: 35,
};

const SunIcon = () => (
  <svg
    viewBox="0 0 24 24"
    width="16"
    height="16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    aria-hidden
  >
    <circle cx="12" cy="12" r="4" fill="currentColor" />
    <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
  </svg>
);

/** The sun: time of day, date, a Lighting study's azimuth and altitude, and exposure. It
 * previews as it's dragged and saves to Sun Settings when left alone for a moment. */
export function SunPanel() {
  const saved = useAppStore((s) => s.app?.sun ?? DEFAULT_SUN);
  const preview = useAppStore((s) => s.sunPreview);
  const setPreview = useAppStore((s) => s.setSunPreview);
  const exposure = useAppStore((s) => s.exposure3d);
  const setExposure = useAppStore((s) => s.setExposure3d);
  const [open, setOpen] = useState(false);
  const [sun, setSun] = useState<SunPosition | null>(null);
  const s = preview ?? saved;
  const timer = useRef(0);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    let live = true;
    ipc.sunFor(s).then(
      (p) => live && setSun(p),
      () => live && setSun(null),
    );
    return () => {
      live = false;
    };
  }, [s]);
  const change = (patch: Partial<SunSettings>) => {
    const next = { ...s, ...patch };
    setPreview(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      void apply(() => ipc.setSunSettings(next)).then(() => {
        if (useAppStore.getState().sunPreview === next) setPreview(null);
      });
    }, 700);
  };
  const label =
    s.mode === "Still"
      ? `${clock(s.hour)} · ${MONTHS[s.month - 1]} ${s.day}`
      : `${Math.round(s.azimuth)}° · ${Math.round(s.altitude)}° up`;
  return (
    <div className={`sun3d${open ? " open" : ""}`}>
      <button
        className="sun3d-toggle"
        aria-expanded={open}
        aria-label="Sun"
        title="Sun: time of day, date and exposure"
        onClick={() => setOpen(!open)}
      >
        <SunIcon />
        <span>{label}</span>
      </button>
      {open && (
        <div className="sun3d-panel" role="group" aria-label="Sun settings">
          {s.mode === "Still" ? (
            <>
              <label className="sun3d-row">
                <span>Time</span>
                <input
                  aria-label="Time of day"
                  type="range"
                  min={4}
                  max={22}
                  step={0.25}
                  value={s.hour}
                  onChange={(e) => change({ hour: Number(e.target.value) })}
                />
                <b>{clock(s.hour)}</b>
              </label>
              <label className="sun3d-row">
                <span>Date</span>
                <select
                  aria-label="Month"
                  value={s.month}
                  onChange={(e) => change({ month: Number(e.target.value) })}
                >
                  {MONTHS.map((m, i) => (
                    <option key={m} value={i + 1}>
                      {m}
                    </option>
                  ))}
                </select>
                <input
                  aria-label="Day"
                  type="number"
                  min={1}
                  max={31}
                  value={s.day}
                  onChange={(e) =>
                    change({ day: Math.max(1, Math.min(31, Number(e.target.value) || 1)) })
                  }
                />
              </label>
            </>
          ) : (
            <>
              <label className="sun3d-row">
                <span>Azimuth</span>
                <input
                  aria-label="Azimuth"
                  type="range"
                  min={0}
                  max={359}
                  value={s.azimuth}
                  onChange={(e) => change({ azimuth: Number(e.target.value) })}
                />
                <b>{Math.round(s.azimuth)}°</b>
              </label>
              <label className="sun3d-row">
                <span>Altitude</span>
                <input
                  aria-label="Altitude"
                  type="range"
                  min={1}
                  max={90}
                  value={s.altitude}
                  onChange={(e) => change({ altitude: Number(e.target.value) })}
                />
                <b>{Math.round(s.altitude)}°</b>
              </label>
            </>
          )}
          <label className="sun3d-row">
            <span>Exposure</span>
            <input
              aria-label="Exposure"
              type="range"
              min={0.3}
              max={2.5}
              step={0.05}
              value={exposure}
              onChange={(e) => setExposure(Number(e.target.value))}
            />
            <b>{exposure.toFixed(2)}</b>
          </label>
          <div className="sun3d-row">
            <span>Study</span>
            <select
              aria-label="Presets"
              value=""
              onChange={(e) => {
                const p = SUN_PRESETS.find((x) => x.name === e.target.value);
                if (p) change(p.patch);
              }}
            >
              <option value="">
                {s.mode === "Still" ? "Still (date and time)" : "Lighting (azimuth)"}…
              </option>
              {SUN_PRESETS.map((p) => (
                <option key={p.name} value={p.name}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>
          <p className="sun3d-note">
            {sun
              ? sun.altitude > 0
                ? `Sun ${sun.altitude.toFixed(0)}° up, ${sun.azimuth.toFixed(0)}° from north`
                : "The sun is down: the building's lights are on."
              : ""}
          </p>
        </div>
      )}
    </div>
  );
}

const MODES: { id: NavMode; label: string; title: string; path: string }[] = [
  {
    id: "orbit",
    label: "Orbit",
    title: "Orbit: drag to turn about the model",
    path: "M12 5a7 7 0 1 1-6.3 4M4 5v4h4",
  },
  {
    id: "walk",
    label: "Walk",
    title:
      "Walk: W A S D to move at eye height, up stairs, stopped by walls; drag to look; Shift to hurry; Space for Fly",
    path: "M13 4a1.5 1.5 0 1 1 0 .01M11 8l-2 5 3 2v5M11 8l3 3 3 1M9 13l-2 7",
  },
  {
    id: "fly",
    label: "Fly",
    title:
      "Fly: W A S D to move where you look, Q / E down and up; drag to look; Shift to hurry; Space for Walk",
    path: "M3 13l8-2 5-7 2 1-3 7 5 1 1 2-6 1-3 4-2-1 1-4-7-1z",
  },
];

/** Enscape's navigation: Orbit, Walk and Fly. */
export function NavBar() {
  const nav = useAppStore((s) => s.nav3d);
  const setNav = useAppStore((s) => s.setNav3d);
  return (
    <div className="nav3d" role="radiogroup" aria-label="Navigation">
      {MODES.map((m) => (
        <button
          key={m.id}
          role="radio"
          aria-checked={nav === m.id}
          className={nav === m.id ? "on" : undefined}
          title={m.title}
          onClick={() => setNav(m.id)}
        >
          <svg
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
            aria-hidden
          >
            <path d={m.path} />
          </svg>
          <span>{m.label}</span>
        </button>
      ))}
      {nav !== "orbit" && (
        <span className="nav3d-keys">
          W A S D move · {nav === "fly" ? "Q E down/up · " : ""}drag to look · Shift faster · Space{" "}
          {nav === "fly" ? "walk" : "fly"} · Esc orbit
        </span>
      )}
    </div>
  );
}
