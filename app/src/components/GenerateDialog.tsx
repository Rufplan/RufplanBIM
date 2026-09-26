import { useEffect, useRef, useState } from "react";
import type { BuildReport } from "../bindings/BuildReport";
import type { GenerateInputs } from "../bindings/GenerateInputs";
import type { GenerateProgress } from "../bindings/GenerateProgress";
import type { ReferenceImage } from "../bindings/ReferenceImage";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// Generate with Claude (ADR-030): describe a building; Claude plans it and Rufplan Studio
// builds the model.

interface BuildingType {
  label: string;
  stories: number;
  /** What the count field counts. */
  count: "Bedrooms" | "Units" | "Keys";
  defaultCount: number;
}

export const BUILDING_TYPES: BuildingType[] = [
  { label: "Single-family house", stories: 2, count: "Bedrooms", defaultCount: 3 },
  { label: "Duplex", stories: 2, count: "Units", defaultCount: 2 },
  { label: "Townhouses", stories: 3, count: "Units", defaultCount: 4 },
  { label: "Garden-style apartments", stories: 3, count: "Units", defaultCount: 24 },
  { label: "Mid-rise apartments", stories: 5, count: "Units", defaultCount: 60 },
  { label: "Mixed-use (retail + apartments)", stories: 4, count: "Units", defaultCount: 30 },
  { label: "Boutique hotel", stories: 4, count: "Keys", defaultCount: 40 },
  { label: "Select-service hotel", stories: 5, count: "Keys", defaultCount: 100 },
];

const STYLES = [
  "Modern",
  "Contemporary",
  "Modern farmhouse",
  "Craftsman",
  "Traditional",
  "Colonial",
  "Mediterranean",
  "Spanish revival",
  "Mid-century modern",
  "Industrial loft",
];

/** A reference image, scaled so its long side is at most 1568 px (what Claude uses), as
 * base64 JPEG. */
async function readImage(file: File): Promise<ReferenceImage & { name: string; url: string }> {
  const url = URL.createObjectURL(file);
  const img = new Image();
  img.src = url;
  await img.decode();
  const k = Math.min(1, 1568 / Math.max(img.width, img.height));
  const c = document.createElement("canvas");
  c.width = Math.round(img.width * k);
  c.height = Math.round(img.height * k);
  c.getContext("2d")!.drawImage(img, 0, 0, c.width, c.height);
  const data = c.toDataURL("image/jpeg", 0.85).split(",")[1] ?? "";
  return { mediaType: "image/jpeg", data, name: file.name, url };
}

export function GenerateDialog({ onClose }: { onClose: () => void }) {
  const hasSite = useAppStore((s) => !!s.app?.site);
  const [keySet, setKeySet] = useState<boolean | null>(null);
  const [keyText, setKeyText] = useState("");
  const [typeIndex, setTypeIndex] = useState(0);
  const type = BUILDING_TYPES[typeIndex]!;
  const [stories, setStories] = useState(type.stories);
  const [count, setCount] = useState(type.defaultCount);
  const [bathrooms, setBathrooms] = useState(2.5);
  const [area, setArea] = useState("");
  const [style, setStyle] = useState("Modern farmhouse");
  const [roof, setRoof] = useState("auto");
  const [fitLot, setFitLot] = useState(hasSite);
  const [setbacks, setSetbacks] = useState<[number, number, number]>([20, 5, 20]);
  const [references, setReferences] = useState("");
  const [prompt, setPrompt] = useState("");
  const [images, setImages] = useState<(ReferenceImage & { name: string; url: string })[]>([]);
  const [model, setModel] = useState("claude-opus-5-5");
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<GenerateProgress | null>(null);
  const [result, setResult] = useState<{ report: BuildReport; summary: string } | null>(null);
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
  // A clock while Claude plans.
  useEffect(() => {
    if (!running) return;
    const t = setInterval(() => setElapsed((performance.now() - started.current) / 1000), 500);
    return () => clearInterval(t);
  }, [running]);

  const pickType = (i: number) => {
    setTypeIndex(i);
    setStories(BUILDING_TYPES[i]!.stories);
    setCount(BUILDING_TYPES[i]!.defaultCount);
  };

  const saveKey = async () => {
    try {
      setKeySet(await ipc.claudeSetKey(keyText));
      setKeyText("");
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    }
  };

  const addImages = async (files: FileList | null) => {
    if (!files) return;
    const read = await Promise.all(
      [...files]
        .filter((f) => f.type.startsWith("image/"))
        .map((f) => readImage(f).catch(() => null)),
    );
    setImages((cur) =>
      [...cur, ...read.filter((x): x is NonNullable<typeof x> => !!x)].slice(0, 5),
    );
  };

  const generate = async () => {
    const inputs: GenerateInputs = {
      buildingType: type.label,
      stories,
      area: area.trim() ? Number(area.replace(/[^0-9.]/g, "")) || null : null,
      count: count > 0 ? count : null,
      bathrooms: type.count === "Bedrooms" ? bathrooms : null,
      style,
      roof,
      fitLot: fitLot && hasSite,
      setbacks,
      references,
      prompt,
      images: images.map(({ mediaType, data }) => ({ mediaType, data })),
      model,
    };
    setRunning(true);
    setResult(null);
    setProgress(null);
    started.current = performance.now();
    setElapsed(0);
    const stop = await ipc.onGenerateProgress((p) => setProgress(p)).catch(() => null);
    let out: { report: BuildReport; summary: string } | null = null;
    await apply(async () => {
      const r = await ipc.generateBuilding(inputs);
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
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Generate with Claude">
      <div className="modal generate-dialog">
        <div className="gen-head">
          <h2>Generate with Claude</h2>
          <button className="btn-ghost" onClick={onClose} aria-label="Close" disabled={running}>
            ×
          </button>
        </div>
        <div className="gen-body">
          {keySet === false && (
            <div className="gen-key">
              <p>
                Claude plans the building from your brief. Add your Anthropic API key (from
                console.anthropic.com); it&apos;s kept in Windows Credential Manager and only sent
                to Anthropic.
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
              <div className="row">
                <label className="field grow">
                  Building type
                  <select
                    aria-label="Building type"
                    value={typeIndex}
                    onChange={(e) => pickType(Number(e.target.value))}
                  >
                    {BUILDING_TYPES.map((t, i) => (
                      <option key={t.label} value={i}>
                        {t.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  Stories
                  <input
                    aria-label="Stories"
                    type="number"
                    min={1}
                    max={20}
                    value={stories}
                    onChange={(e) =>
                      setStories(Math.max(1, Math.min(20, Number(e.target.value) || 1)))
                    }
                  />
                </label>
                <label className="field">
                  {type.count}
                  <input
                    aria-label={type.count}
                    type="number"
                    min={0}
                    value={count}
                    onChange={(e) => setCount(Math.max(0, Number(e.target.value) || 0))}
                  />
                </label>
                {type.count === "Bedrooms" && (
                  <label className="field">
                    Baths
                    <input
                      aria-label="Bathrooms"
                      type="number"
                      min={1}
                      step={0.5}
                      value={bathrooms}
                      onChange={(e) => setBathrooms(Number(e.target.value) || 1)}
                    />
                  </label>
                )}
              </div>
              <div className="row">
                <label className="field">
                  Area (sf, optional)
                  <input
                    aria-label="Area"
                    placeholder="e.g. 2,800"
                    value={area}
                    onChange={(e) => setArea(e.target.value)}
                  />
                </label>
                <label className="field grow">
                  Style
                  <select
                    aria-label="Style"
                    value={style}
                    onChange={(e) => setStyle(e.target.value)}
                  >
                    {STYLES.map((s) => (
                      <option key={s}>{s}</option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  Roof
                  <select aria-label="Roof" value={roof} onChange={(e) => setRoof(e.target.value)}>
                    <option value="auto">Claude decides</option>
                    <option value="hip">Hip</option>
                    <option value="gable">Gable</option>
                    <option value="flat">Flat</option>
                  </select>
                </label>
              </div>
              <div className="row gen-lot">
                <label className="ob-check">
                  <input
                    type="checkbox"
                    checked={fitLot && hasSite}
                    disabled={!hasSite}
                    onChange={(e) => setFitLot(e.target.checked)}
                  />
                  Fit on the lot {hasSite ? "" : "(set a lot on the Site tab first)"}
                </label>
                {fitLot && hasSite && (
                  <>
                    {(["Front", "Sides", "Rear"] as const).map((label, i) => (
                      <label key={label} className="field small">
                        {label} setback (ft)
                        <input
                          aria-label={`${label} setback`}
                          type="number"
                          min={0}
                          value={setbacks[i]}
                          onChange={(e) => {
                            const next = [...setbacks] as [number, number, number];
                            next[i] = Math.max(0, Number(e.target.value) || 0);
                            setSetbacks(next);
                          }}
                        />
                      </label>
                    ))}
                  </>
                )}
              </div>
              <label className="field">
                References
                <textarea
                  aria-label="References"
                  rows={2}
                  placeholder="Buildings, architects or features to draw from: e.g. board-and-batten, black windows, deep porch like a Hudson Valley farmhouse"
                  value={references}
                  onChange={(e) => setReferences(e.target.value)}
                />
              </label>
              <div className="gen-images">
                {images.map((im, i) => (
                  <figure key={im.url}>
                    <img src={im.url} alt={im.name} />
                    <button
                      className="btn-ghost"
                      aria-label={`Remove ${im.name}`}
                      onClick={() => setImages((cur) => cur.filter((_, k) => k !== i))}
                    >
                      ×
                    </button>
                  </figure>
                ))}
                {images.length < 5 && (
                  <label className="gen-add">
                    + Reference images
                    <input
                      aria-label="Add reference images"
                      type="file"
                      accept="image/*"
                      multiple
                      onChange={(e) => void addImages(e.target.files)}
                    />
                  </label>
                )}
              </div>
              <label className="field">
                Prompt
                <textarea
                  aria-label="Prompt"
                  rows={4}
                  placeholder="Anything else: e.g. open kitchen facing the backyard, primary suite on the main floor, mudroom from the garage, rooftop amenity deck…"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                />
              </label>
              <label className="field">
                Model
                <select aria-label="Model" value={model} onChange={(e) => setModel(e.target.value)}>
                  <option value="claude-opus-5-5">Claude Opus 5.5 (best plans)</option>
                  <option value="claude-sonnet-5">Claude Sonnet 5 (faster)</option>
                </select>
              </label>
              <p className="muted">
                Generating replaces the building in this project (walls, floors, roofs, rooms…).
                Undo (Ctrl+Z) brings it back in one step.
              </p>
            </fieldset>
          )}
          {running && (
            <div className="gen-progress" role="status">
              <div className="gen-spinner" aria-hidden />
              {progress?.phase === "building" ? (
                <p>
                  Building <b>{progress.name}</b>: {progress.stories} stories, {progress.rooms}{" "}
                  rooms…
                </p>
              ) : (
                <p>
                  Claude is planning
                  {progress?.name ? (
                    <>
                      {" "}
                      <b>{progress.name}</b>
                    </>
                  ) : (
                    " the building"
                  )}
                  {progress
                    ? `: ${progress.stories} ${progress.stories === 1 ? "story" : "stories"}, ${progress.rooms} rooms so far`
                    : "…"}{" "}
                  · {secs}
                </p>
              )}
            </div>
          )}
          {result && (
            <div className="gen-result" role="status">
              <h3>{result.report.name}</h3>
              <p>{result.summary}</p>
              <p className="gen-counts">
                {result.report.levels} levels · {result.report.walls} walls · {result.report.doors}{" "}
                doors · {result.report.windows} windows · {result.report.rooms} rooms ·{" "}
                {result.report.stairs} stairs · {result.report.floors} floors ·{" "}
                {result.report.roofs} roofs
                {result.report.materials ? ` · ${result.report.materials} materials` : ""}
              </p>
              {result.report.warnings.length > 0 && (
                <ul className="gen-warnings">
                  {result.report.warnings.map((w) => (
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
                Revise &amp; Generate Again
              </button>
              <button className="btn-ghost" onClick={onClose}>
                Close
              </button>
            </>
          ) : (
            <button
              className="btn-cyan"
              onClick={() => void generate()}
              disabled={running || !keySet}
              title={
                keySet
                  ? "Claude plans the building, then it's built in the model"
                  : "Add your Claude API key first"
              }
            >
              {running ? "Generating…" : "Generate"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
