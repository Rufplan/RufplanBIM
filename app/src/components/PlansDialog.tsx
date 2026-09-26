import { useEffect, useRef, useState } from "react";
import type { PlanReport } from "../bindings/PlanReport";
import type { PlansInputs } from "../bindings/PlansInputs";
import type { PlansProgress } from "../bindings/PlansProgress";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { guessLevel, readPlanFiles, type PlanPage } from "../render/planImages";
import { useAppStore } from "../store";

// Plans to 3D (ADR-036): floor plans in (images or PDF pages); Claude traces each sheet's
// walls, doors, windows, rooms and slabs; Rufplan Studio builds the model.

const MAX_SHEETS = 12;

interface Sheet extends PlanPage {
  level: string;
  /** Floor elevation in feet, as typed; blank lets Claude read or guess it. */
  elevation: string;
}

export function PlansDialog({ onClose }: { onClose: () => void }) {
  const [keySet, setKeySet] = useState<boolean | null>(null);
  const [keyText, setKeyText] = useState("");
  const [name, setName] = useState("");
  const [sheets, setSheets] = useState<Sheet[]>([]);
  const [notes, setNotes] = useState("");
  const [model, setModel] = useState("claude-opus-5-5");
  const [reading, setReading] = useState(false);
  const [over, setOver] = useState(false);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<PlansProgress | null>(null);
  const [result, setResult] = useState<{ report: PlanReport; summary: string } | null>(null);
  const started = useRef(0);
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    let live = true;
    ipc.claudeKeySet().then(
      (k) => live && setKeySet(k),
      () => live && setKeySet(false),
    );
    return () => {
      live = false;
    };
  }, []);
  useEffect(() => {
    if (!running) return;
    const t = setInterval(() => setElapsed((performance.now() - started.current) / 1000), 500);
    return () => clearInterval(t);
  }, [running]);

  const saveKey = async () => {
    try {
      setKeySet(await ipc.claudeSetKey(keyText));
      setKeyText("");
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    }
  };

  const addFiles = async (files: FileList | File[] | null) => {
    if (!files) return;
    setReading(true);
    try {
      const pages = await readPlanFiles([...files], MAX_SHEETS - sheets.length);
      setSheets((cur) =>
        [
          ...cur,
          ...pages.map((p, i) => ({
            ...p,
            level: guessLevel(p.name, cur.length + i),
            elevation: "",
          })),
        ].slice(0, MAX_SHEETS),
      );
    } catch (e) {
      useAppStore.getState().setError(`Couldn't read the plans: ${errorMessage(e)}`);
    } finally {
      setReading(false);
    }
  };

  const edit = (i: number, patch: Partial<Sheet>) =>
    setSheets((cur) => cur.map((s, k) => (k === i ? { ...s, ...patch } : s)));

  const build = async () => {
    const inputs: PlansInputs = {
      name: name.trim(),
      sheets: sheets.map((s) => ({
        mediaType: s.mediaType,
        data: s.data,
        width: s.width,
        height: s.height,
        level: s.level.trim(),
        elevation: s.elevation.trim() === "" ? null : Number(s.elevation) || 0,
      })),
      notes,
      model,
    };
    setRunning(true);
    setResult(null);
    setProgress(null);
    started.current = performance.now();
    setElapsed(0);
    const stop = await ipc.onPlansProgress((p) => setProgress(p)).catch(() => null);
    let out: { report: PlanReport; summary: string } | null = null;
    await apply(async () => {
      const r = await ipc.plansToModel(inputs);
      out = { report: r.report, summary: r.summary };
      return r.state;
    });
    stop?.();
    setRunning(false);
    if (out) setResult(out);
  };

  const open3d = () => {
    const s = useAppStore.getState();
    const v = s.app?.views.find((x) => x.viewType === "ThreeD" && !x.camera);
    if (v) s.openView(v.id);
    onClose();
  };

  const secs = `${Math.floor(elapsed / 60)}:${String(Math.floor(elapsed % 60)).padStart(2, "0")}`;
  const r = result?.report;
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Plans to 3D">
      <div className="modal generate-dialog">
        <div className="gen-head">
          <h2>Plans to 3D</h2>
          <button className="btn-ghost" onClick={onClose} aria-label="Close" disabled={running}>
            ×
          </button>
        </div>
        <div className="gen-body">
          {keySet === false && (
            <div className="gen-key">
              <p>
                Claude traces the plans. Add your Anthropic API key (from console.anthropic.com);
                it&apos;s kept in Windows Credential Manager and only sent to Anthropic.
              </p>
              <div className="row">
                <input
                  aria-label="Claude API key"
                  type="password"
                  placeholder="sk-ant-…"
                  value={keyText}
                  onChange={(e) => setKeyText(e.target.value)}
                />
                <button
                  className="btn-cyan"
                  onClick={() => void saveKey()}
                  disabled={!keyText.trim()}
                >
                  Save Key
                </button>
              </div>
            </div>
          )}
          {!result && (
            <fieldset className="gen-form" disabled={running}>
              <label className="field">
                Project name (optional)
                <input
                  aria-label="Project name"
                  placeholder="e.g. Fallingwater"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <div className="plans-sheets">
                {sheets.map((s, i) => (
                  <div className="plans-sheet" key={s.url}>
                    <img src={s.url} alt={s.name} />
                    <label className="field grow">
                      Level
                      <input
                        aria-label={`Level of sheet ${i + 1}`}
                        value={s.level}
                        onChange={(e) => edit(i, { level: e.target.value })}
                      />
                    </label>
                    <label className="field small">
                      Elevation (ft)
                      <input
                        aria-label={`Elevation of sheet ${i + 1}`}
                        placeholder="auto"
                        inputMode="decimal"
                        value={s.elevation}
                        onChange={(e) => edit(i, { elevation: e.target.value })}
                      />
                    </label>
                    <button
                      className="btn-ghost"
                      aria-label={`Remove ${s.name}`}
                      onClick={() => setSheets((cur) => cur.filter((_, k) => k !== i))}
                    >
                      ×
                    </button>
                  </div>
                ))}
                {sheets.length < MAX_SHEETS && (
                  <label
                    className={`plans-drop${over ? " over" : ""}`}
                    onDragOver={(e) => {
                      e.preventDefault();
                      setOver(true);
                    }}
                    onDragLeave={() => setOver(false)}
                    onDrop={(e) => {
                      e.preventDefault();
                      setOver(false);
                      void addFiles(e.dataTransfer.files);
                    }}
                  >
                    {reading
                      ? "Reading…"
                      : sheets.length
                        ? "+ Add more plans"
                        : "Drop floor plans here, or click to choose: images (JPG, PNG) or a PDF, one floor per page"}
                    <input
                      aria-label="Add plans"
                      type="file"
                      accept="image/*,application/pdf,.pdf"
                      multiple
                      onChange={(e) => void addFiles(e.target.files)}
                    />
                  </label>
                )}
              </div>
              <label className="field">
                Notes
                <textarea
                  aria-label="Notes"
                  rows={3}
                  placeholder='Anything that helps read them: the scale (1/4" = 1&apos;-0"), ceiling heights, which way is north, walls of stone…'
                  value={notes}
                  onChange={(e) => setNotes(e.target.value)}
                />
              </label>
              <label className="field">
                Model
                <select aria-label="Model" value={model} onChange={(e) => setModel(e.target.value)}>
                  <option value="claude-opus-5-5">Claude Opus 5.5 (best reading)</option>
                  <option value="claude-sonnet-5">Claude Sonnet 5 (faster)</option>
                </select>
              </label>
              <p className="muted">
                Building replaces the building in this project. Undo (Ctrl+Z) brings it back in one
                step. Clear drawings at a known scale read best; a dimensioned plan helps.
              </p>
            </fieldset>
          )}
          {running && (
            <div className="gen-progress" role="status">
              <div className="gen-spinner" aria-hidden />
              {progress?.phase === "building" ? (
                <p>
                  Building the model: {progress.sheets} {progress.sheets === 1 ? "floor" : "floors"}
                  , {progress.walls} walls…
                </p>
              ) : (
                <p>
                  Claude is tracing the plans
                  {progress
                    ? `: ${progress.sheets} ${progress.sheets === 1 ? "sheet" : "sheets"}, ${progress.walls} walls so far`
                    : "…"}{" "}
                  · {secs}
                </p>
              )}
            </div>
          )}
          {r && result && (
            <div className="gen-result" role="status">
              <h3>{r.name}</h3>
              <p>{result.summary}</p>
              <p className="gen-counts">
                {r.levels} levels · {r.walls} walls · {r.doors} doors · {r.windows} windows ·{" "}
                {r.rooms} rooms · {r.floors} floors · {r.roofs} roofs
              </p>
              {r.warnings.length > 0 && (
                <ul className="gen-warnings">
                  {r.warnings.map((w) => (
                    <li key={w}>{w}</li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
        <div className="gen-foot">
          {result ? (
            <>
              <button className="btn-cyan" onClick={open3d}>
                Open 3D View
              </button>
              <button className="btn-outline" onClick={() => setResult(null)}>
                Adjust &amp; Build Again
              </button>
              <button className="btn-ghost" onClick={onClose}>
                Close
              </button>
            </>
          ) : (
            <button
              className="btn-cyan"
              onClick={() => void build()}
              disabled={running || reading || !keySet || sheets.length === 0}
              title={
                !keySet
                  ? "Add your Claude API key first"
                  : sheets.length === 0
                    ? "Add at least one plan"
                    : "Claude traces the plans, then the model is built"
              }
            >
              {running ? "Building…" : "Build Model"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
