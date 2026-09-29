import { useState } from "react";
import type { FormDraft } from "../bindings/FormDraft";
import type { FormKind } from "../bindings/FormKind";
import { ipc } from "../ipc";
import { ftIn, setForm } from "../inplace";

// The settings of the in-place form being sketched (ADR-068), on the sketch tab: an
// extrusion's start and end, a blend's base and top, a sweep's profile and elevation.

interface LengthProps {
  label: string;
  mm: number;
  onChange: (mm: number) => void;
}

/** A length typed in feet and inches; it starts over when the value changes. */
function LengthField(props: LengthProps) {
  return <LengthInput key={props.mm} {...props} />;
}

function LengthInput({ label, mm, onChange }: LengthProps) {
  const [text, setText] = useState(ftIn(mm));
  const commit = async () => {
    const v = await ipc.parseLength(text).catch(() => null);
    if (v === null || v === undefined) setText(ftIn(mm));
    else if (Math.abs(v - mm) > 1e-6) onChange(v);
  };
  return (
    <label className="rb-field">
      <span>{label}</span>
      <input
        aria-label={label}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => void commit()}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
    </label>
  );
}

/** What the sketch tab calls a form's sketch, as Revit does. */
export function formTitle(form: FormDraft): string {
  const edit = form.index !== null;
  if ("Extrusion" in form.kind) {
    const what = form.void ? "Void Extrusion" : "Extrusion";
    return edit ? `Modify | ${what} > Edit Extrusion` : `Modify | Create ${what}`;
  }
  if ("Blend" in form.kind) {
    const end = form.top ? "Top" : "Base";
    return `Modify | ${edit ? "Blend > Edit" : "Create Blend"} ${end} Boundary`;
  }
  return `Modify | ${edit ? "Sweep > Edit" : "Create Sweep"} Path`;
}

export function FormPanel({ form }: { form: FormDraft }) {
  const k = form.kind;
  const set = (kind: FormKind) => void setForm(kind);
  let body: React.ReactNode;
  let hint: string;
  if ("Extrusion" in k) {
    const e = k.Extrusion;
    body = (
      <>
        <LengthField
          label="Extrusion Start"
          mm={e.start}
          onChange={(v) => set({ Extrusion: { ...e, start: v } })}
        />
        <LengthField
          label="Extrusion End"
          mm={e.end}
          onChange={(v) => set({ Extrusion: { ...e, end: v } })}
        />
      </>
    );
    hint = form.void ? "Sketch the loops to cut away" : "Sketch closed loops";
  } else if ("Blend" in k) {
    const b = k.Blend;
    body = (
      <>
        <LengthField
          label="Base (First End)"
          mm={b.base}
          onChange={(v) => set({ Blend: { ...b, base: v } })}
        />
        <LengthField
          label="Top (Second End)"
          mm={b.top}
          onChange={(v) => set({ Blend: { ...b, top: v } })}
        />
      </>
    );
    hint = form.top
      ? "Sketch the top loop, then Finish"
      : "Sketch the base loop; Finish goes on to the top";
  } else {
    const sw = k.Sweep;
    const p = sw.profile;
    const size = "Rectangle" in p ? p.Rectangle.height : p.Circle.diameter;
    body = (
      <>
        <label className="rb-field">
          <span>Profile</span>
          <select
            aria-label="Profile"
            value={"Rectangle" in p ? "Rectangle" : "Circle"}
            onChange={(e) =>
              set({
                Sweep: {
                  ...sw,
                  profile:
                    e.target.value === "Circle"
                      ? { Circle: { diameter: size } }
                      : { Rectangle: { width: size, height: size } },
                },
              })
            }
          >
            <option value="Rectangle">Rectangle</option>
            <option value="Circle">Circle</option>
          </select>
        </label>
        {"Rectangle" in p ? (
          <>
            <LengthField
              label="Width"
              mm={p.Rectangle.width}
              onChange={(v) =>
                set({ Sweep: { ...sw, profile: { Rectangle: { ...p.Rectangle, width: v } } } })
              }
            />
            <LengthField
              label="Height"
              mm={p.Rectangle.height}
              onChange={(v) =>
                set({ Sweep: { ...sw, profile: { Rectangle: { ...p.Rectangle, height: v } } } })
              }
            />
          </>
        ) : (
          <LengthField
            label="Diameter"
            mm={p.Circle.diameter}
            onChange={(v) => set({ Sweep: { ...sw, profile: { Circle: { diameter: v } } } })}
          />
        )}
        <LengthField
          label="Profile Elevation"
          mm={sw.elevation}
          onChange={(v) => set({ Sweep: { ...sw, elevation: v } })}
        />
      </>
    );
    hint = "Sketch the path (open or closed) the profile follows";
  }
  return (
    <div className="rb-group">
      <div className="rb-items rb-form">
        {body}
        <span className="rb-hint">{hint}</span>
      </div>
      <div className="rb-title">
        {"Extrusion" in k
          ? form.void
            ? "Void Extrusion"
            : "Extrusion"
          : "Blend" in k
            ? "Blend"
            : "Sweep"}
      </div>
    </div>
  );
}
