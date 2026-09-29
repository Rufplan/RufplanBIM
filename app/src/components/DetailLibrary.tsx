import { useEffect, useMemo, useState } from "react";
import type { DetailInfo } from "../bindings/DetailInfo";
import type { DisplayList } from "../bindings/DisplayList";
import { errorMessage, ipc } from "../ipc";
import { createDraftingView, insertDetail } from "../details";
import { useAppStore } from "../store";
import { DetailThumb } from "./DetailThumb";

// The Detail Library (ADR-069), laid out like the door and window libraries: categories,
// a grid of drawn thumbnails, and the chosen detail larger with its scale. Insert makes a
// drafting view of it at that scale.

let library: Promise<DetailInfo[]> | null = null;

/** Forgets the library so it's fetched again (after Save to Library or a delete). */
export function reloadLibrary() {
  library = null;
}
const previews = new Map<string, Promise<DisplayList>>();

function preview(id: string) {
  let p = previews.get(id);
  if (!p) {
    p = ipc.detailPreview(id);
    p.catch(() => previews.delete(id));
    previews.set(id, p);
  }
  return p;
}

function Preview({ id, name, big }: { id: string; name: string; big?: boolean }) {
  const [dl, setDl] = useState<{ id: string; dl: DisplayList } | null>(null);
  useEffect(() => {
    let live = true;
    preview(id).then(
      (d) => live && setDl({ id, dl: d }),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [id]);
  if (!dl || dl.id !== id)
    return <div className="dl-thumb dl-loading" role="img" aria-label={name} />;
  return <DetailThumb dl={dl.dl} label={name} text={big} />;
}

export function DetailLibrary({ onClose }: { onClose: () => void }) {
  const [details, setDetails] = useState<DetailInfo[]>([]);
  const [cat, setCat] = useState("All");
  const [query, setQuery] = useState("");
  const [current, setCurrent] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  useEffect(() => {
    library ??= ipc.detailLibrary();
    library.then(
      (d) => {
        setDetails(d);
        setCurrent((c) => c ?? d[0]?.id ?? null);
      },
      (e) => {
        library = null;
        useAppStore.getState().setError(errorMessage(e));
      },
    );
  }, []);
  const cats = useMemo(() => ["All", ...new Set(details.map((d) => d.category))], [details]);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return details.filter(
      (d) =>
        (cat === "All" || d.category === cat) &&
        (!q || `${d.name} ${d.description} ${d.category}`.toLowerCase().includes(q)),
    );
  }, [details, cat, query]);
  const chosen = details.find((d) => d.id === current) ?? null;
  const insert = async (id: string) => {
    if (await insertDetail(id)) onClose();
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Detail Library">
      <div className="modal material-browser detail-library">
        <div className="mb-head">
          <h2>Detail Library</h2>
          <input
            className="mb-search"
            aria-label="Search details"
            placeholder="Search details"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>
        <div className="mb-body">
          <nav className="mb-filters" aria-label="Categories">
            {cats.map((c) => (
              <button
                key={c}
                className={`mb-cat${cat === c ? " active" : ""}`}
                aria-pressed={cat === c}
                onClick={() => setCat(c)}
              >
                {c}
                <span className="mb-count">
                  {c === "All" ? details.length : details.filter((d) => d.category === c).length}
                </span>
              </button>
            ))}
          </nav>
          <div className="mb-grid dl-grid" role="list" aria-label="Details">
            {shown.map((d) => (
              <button
                key={d.id}
                role="listitem"
                className={`mb-card dl-card${current === d.id ? " active" : ""}`}
                onClick={() => setCurrent(d.id)}
                onDoubleClick={() => void insert(d.id)}
                title={d.description}
              >
                <Preview id={d.id} name={d.name} />
                <span className="mb-name">{d.name}</span>
                <span className="dl-scale">{d.scaleLabel}</span>
              </button>
            ))}
            {details.length > 0 && shown.length === 0 && <p className="muted">No details match.</p>}
          </div>
          <aside className="mb-detail dl-detail">
            {chosen ? (
              <>
                <div className="dl-big">
                  <Preview id={chosen.id} name={`${chosen.name} preview`} big />
                </div>
                <h3>{chosen.name}</h3>
                <p className="muted">{chosen.description}</p>
                <dl className="dl-facts">
                  <dt>Category</dt>
                  <dd>{chosen.category}</dd>
                  <dt>Scale</dt>
                  <dd>{chosen.scaleLabel}</dd>
                </dl>
                <p className="muted">
                  Inserting makes a drafting view at this scale: every line, filled region and note
                  is yours to edit, and it goes on a sheet like any view.
                </p>
                <div className="mb-actions">
                  <button className="btn-cyan" onClick={() => void insert(chosen.id)}>
                    Insert Detail
                  </button>
                  {chosen.user && (
                    <button
                      className="btn-outline"
                      onClick={() => {
                        if (deleting !== chosen.id) {
                          setDeleting(chosen.id);
                          return;
                        }
                        void ipc.detailDelete(chosen.id).then(
                          () => {
                            reloadLibrary();
                            setDeleting(null);
                            setDetails((d) => d.filter((x) => x.id !== chosen.id));
                            setCurrent(null);
                          },
                          (e) => useAppStore.getState().setError(errorMessage(e)),
                        );
                      }}
                    >
                      {deleting === chosen.id ? "Click again to delete" : "Delete from Library"}
                    </button>
                  )}
                </div>
              </>
            ) : (
              <p className="muted">Pick a detail.</p>
            )}
          </aside>
        </div>
      </div>
    </div>
  );
}

/** Save to Library (ADR-073): the active drafting view as one of your details. */
export function SaveDetailDialog({ onClose }: { onClose: () => void }) {
  const view = useAppStore((s) => s.app?.views.find((v) => v.id === s.activeView) ?? null);
  const [name, setName] = useState(view?.name ?? "");
  const [category, setCategory] = useState("My Details");
  const [description, setDescription] = useState("");
  const [cats, setCats] = useState<string[]>([]);
  useEffect(() => {
    library ??= ipc.detailLibrary();
    library.then(
      (d) => setCats([...new Set(["My Details", ...d.map((x) => x.category)])]),
      () => {},
    );
  }, []);
  const ok = async () => {
    const s = useAppStore.getState();
    if (!view || view.viewType !== "Drafting") {
      s.setError("Open the drafting view to save to the library.");
      return;
    }
    try {
      await ipc.detailSave(view.id, name, category, description);
      reloadLibrary();
      onClose();
    } catch (e) {
      s.setError(errorMessage(e));
    }
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Save to Library">
      <div className="modal drafting-dialog">
        <h2>Save to Library</h2>
        <p className="muted">
          Saves this drafting view&apos;s lines, regions, components, notes and dimensions as a
          detail on this computer, to insert in any project.
        </p>
        <label className="inplace-name">
          Name
          <input aria-label="Name" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="inplace-name">
          Category
          <input
            aria-label="Category"
            list="detail-categories"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          />
          <datalist id="detail-categories">
            {cats.map((c) => (
              <option key={c} value={c} />
            ))}
          </datalist>
        </label>
        <label className="inplace-name">
          Description
          <input
            aria-label="Description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </label>
        <div className="mb-actions">
          <button className="btn-cyan" onClick={() => void ok()}>
            Save
          </button>
          <button className="btn-outline" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

/** Revit's New Drafting View dialog: a name and a scale. */
export const DRAFTING_SCALES: [number, string][] = [
  [2, `6" = 1'-0"`],
  [4, `3" = 1'-0"`],
  [8, `1 1/2" = 1'-0"`],
  [12, `1" = 1'-0"`],
  [16, `3/4" = 1'-0"`],
  [24, `1/2" = 1'-0"`],
  [48, `1/4" = 1'-0"`],
];

export function DraftingViewDialog({ onClose }: { onClose: () => void }) {
  const [name, setName] = useState("");
  const [scale, setScale] = useState(4);
  const ok = async () => {
    if (await createDraftingView(name, scale)) onClose();
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="New Drafting View">
      <div className="modal drafting-dialog">
        <h2>New Drafting View</h2>
        <label className="inplace-name">
          Name
          <input
            aria-label="Name"
            value={name}
            placeholder="Drafting 1"
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void ok();
            }}
          />
        </label>
        <label className="inplace-name">
          Scale
          <select
            aria-label="Scale"
            value={scale}
            onChange={(e) => setScale(Number(e.target.value))}
          >
            {DRAFTING_SCALES.map(([s, l]) => (
              <option key={s} value={s}>
                {l}
              </option>
            ))}
          </select>
        </label>
        <div className="mb-actions">
          <button className="btn-cyan" onClick={() => void ok()}>
            OK
          </button>
          <button className="btn-outline" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
