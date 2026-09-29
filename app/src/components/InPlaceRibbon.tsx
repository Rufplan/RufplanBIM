import { useAppStore } from "../store";
import {
  cancelModel,
  deleteForm,
  finishModel,
  formName,
  startForm,
  type FormName,
} from "../inplace";
import { Icons } from "./Icons";
import { CHECK, CROSS } from "./SketchRibbon";

// The In-Place Editor's tab (ADR-068), after Revit's family editor in the project: forms
// to sketch (Extrusion, Blend, Sweep and Void Extrusion), the model's forms to edit or
// delete, and Finish or Cancel Model.

const FORMS: [FormName, string, keyof typeof Icons, string][] = [
  ["Extrusion", "Extrusion", "extrusion", "Sketch closed loops and extrude them up"],
  ["Blend", "Blend", "blend", "Sketch a base loop and a top loop; the form blends between them"],
  ["Sweep", "Sweep", "sweep", "Sketch a path; a rectangle or round profile follows it"],
];

export function InPlaceRibbon() {
  const ip = useAppStore((s) => s.app?.inPlace ?? null);
  if (!ip) return null;
  const btn = (kind: FormName, label: string, icon: keyof typeof Icons, title: string) => (
    <button key={kind} className="rb-btn" onClick={() => void startForm(kind)} title={title}>
      {Icons[icon]}
      <span>{label}</span>
    </button>
  );
  return (
    <div className="ribbon inplace-ribbon" role="toolbar" aria-label="Tools">
      <div className="rb-tabs" role="tablist" aria-label="Ribbon tabs">
        <button role="tab" aria-selected className="rb-tab active contextual">
          {`In-Place Editor | ${ip.categoryLabel} : ${ip.name}`}
        </button>
      </div>
      <div className="rb-body">
        <div className="rb-group">
          <div className="rb-items">{FORMS.map((f) => btn(...f))}</div>
          <div className="rb-title">Forms</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            {btn(
              "VoidExtrusion",
              "Void Extrusion",
              "voidExtrusion",
              "Sketch loops to cut out of the solid extrusions",
            )}
          </div>
          <div className="rb-title">Void Forms</div>
        </div>
        <div className="rb-group">
          <div className="rb-items inplace-forms" role="list" aria-label="Forms in the model">
            {ip.forms.length === 0 && <span className="rb-hint">No forms yet</span>}
            {ip.forms.map((label, i) => (
              <div key={`${label}-${i}`} className="inplace-form" role="listitem">
                <span>{label}</span>
                <button
                  className="btn-mini"
                  aria-label={`Edit Sketch of ${label}`}
                  title="Edit Sketch"
                  onClick={() => void startForm(formName(label), i)}
                >
                  <svg
                    viewBox="0 0 24 24"
                    width="14"
                    height="14"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.8"
                    aria-hidden
                  >
                    <path d="M4 20l1-5L16 4l4 4L9 19z" />
                  </svg>
                </button>
                <button
                  className="btn-mini"
                  aria-label={`Delete ${label}`}
                  title="Delete this form"
                  onClick={() => void deleteForm(i)}
                >
                  {Icons.del}
                </button>
              </div>
            ))}
          </div>
          <div className="rb-title">Model</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            <button
              className="rb-btn"
              onClick={() => void finishModel()}
              title="Finish Model: leave the In-Place Editor (one undo step)"
            >
              {CHECK}
              <span>Finish Model</span>
            </button>
            <button
              className="rb-btn"
              onClick={() => void cancelModel()}
              title="Cancel Model: undo everything since the editor opened"
            >
              {CROSS}
              <span>Cancel Model</span>
            </button>
          </div>
          <div className="rb-title">In-Place Editor</div>
        </div>
      </div>
    </div>
  );
}
