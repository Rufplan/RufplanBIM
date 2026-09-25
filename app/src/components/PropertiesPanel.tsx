import { useEffect, useState } from "react";

// Inputs are keyed by their value, so a new value from Rust remounts them fresh.
import type { NamedItem } from "../bindings/NamedItem";
import type { Property } from "../bindings/Property";
import { apply } from "../fileActions";
import { errorMessage, ipc, type PropertySheet } from "../ipc";
import { useAppStore, TOOL_LABELS } from "../store";

function TextInput({ prop, onCommit }: { prop: Property; onCommit: (v: string) => void }) {
  const [value, setValue] = useState(prop.value);
  const commit = () => {
    if (value !== prop.value) onCommit(value);
  };
  return (
    <input
      className="prop-input"
      value={value}
      aria-label={prop.label}
      onChange={(e) => setValue(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        if (e.key === "Escape") {
          setValue(prop.value);
          (e.target as HTMLInputElement).blur();
        }
      }}
    />
  );
}

function TypeSelector({
  value,
  options,
  onChange,
}: {
  value: string | null;
  options: NamedItem[];
  onChange: (id: string) => void;
}) {
  return (
    <select
      className="type-select"
      aria-label="Type"
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((o) => (
        <option key={o.id} value={o.id}>
          {o.name}
        </option>
      ))}
    </select>
  );
}

export function PropertiesPanel() {
  const app = useAppStore((s) => s.app);
  const selection = useAppStore((s) => s.selection);
  const activeView = useAppStore((s) => s.activeView);
  const tool = useAppStore((s) => s.tool);
  const toolTypes = useAppStore((s) => s.toolTypes);
  const setToolType = useAppStore((s) => s.setToolType);
  const select = useAppStore((s) => s.select);
  const [loaded, setSheet] = useState<PropertySheet | null>(null);
  const target =
    selection.length === 1 ? selection[0]! : selection.length === 0 ? activeView : null;

  useEffect(() => {
    let live = true;
    if (!target) return;
    ipc.properties(target).then(
      (s) => live && setSheet(s),
      (e) => {
        if (!live) return;
        setSheet(null);
        // A deleted element can linger in the selection for a moment.
        if (!String(errorMessage(e)).includes("not found"))
          useAppStore.getState().setError(errorMessage(e));
      },
    );
    return () => {
      live = false;
    };
  }, [target, app?.revision]);

  if (!app) return null;
  const sheet = loaded && loaded.id === target ? loaded : null;

  // While a placement tool is active, the panel shows which type it will place (like Revit).
  const sketching = app?.sketch ?? null;
  const toolKind: keyof typeof toolTypes | null = sketching
    ? sketching.kind === "Floor"
      ? "floor"
      : "ceiling"
    : tool === "wall" ||
        tool === "door" ||
        tool === "window" ||
        tool === "roof" ||
        tool === "column" ||
        tool === "beam" ||
        tool === "railing"
      ? tool
      : tool.startsWith("floor")
        ? "floor"
        : tool.startsWith("ceiling")
          ? "ceiling"
          : null;
  const typesByKind = {
    wall: app.wallTypes,
    floor: app.floorTypes,
    ceiling: app.ceilingTypes,
    door: app.doorTypes,
    window: app.windowTypes,
    roof: app.roofTypes,
    column: app.columnTypes,
    beam: app.beamTypes,
    railing: app.railingTypes,
  };
  const toolOptions = toolKind ? typesByKind[toolKind] : [];

  const categoryKind: Partial<Record<string, keyof typeof typesByKind>> = {
    Wall: "wall",
    Floor: "floor",
    Ceiling: "ceiling",
    Door: "door",
    Window: "window",
    Roof: "roof",
    Column: "column",
    Beam: "beam",
    Railing: "railing",
  };
  const instanceKind = sheet ? categoryKind[sheet.category] : undefined;
  const instanceTypes = instanceKind ? typesByKind[instanceKind] : null;

  const groups = new Map<string, Property[]>();
  for (const p of sheet?.properties ?? []) groups.set(p.group, [...(groups.get(p.group) ?? []), p]);

  const set = (key: string, value: string) =>
    sheet && void apply(() => ipc.setProperty(sheet.id, key, value));

  return (
    <aside className="panel properties" aria-label="Properties">
      <div className="panel-title">Properties</div>
      <div className="panel-body">
        {toolKind ? (
          <div className="prop-type">
            <div className="prop-kicker">
              {sketching ? (sketching.kind === "Floor" ? "Floor" : "Ceiling") : TOOL_LABELS[tool]} —
              Type
            </div>
            <TypeSelector
              value={sketching ? sketching.typeId : toolTypes[toolKind]}
              options={toolOptions}
              onChange={(id) =>
                sketching ? void apply(() => ipc.sketchSetType(id)) : setToolType(toolKind, id)
              }
            />
          </div>
        ) : selection.length > 1 ? (
          <div className="prop-empty">{selection.length} elements selected</div>
        ) : sheet ? (
          <>
            <div className="prop-type">
              <div className="prop-kicker">
                {sheet.category.replace(/([a-z])([A-Z])/g, "$1 $2")}
              </div>
              {instanceTypes && sheet.typeId ? (
                <>
                  <TypeSelector
                    value={sheet.typeId}
                    options={instanceTypes}
                    onChange={(id) => set("type", id)}
                  />
                  <button className="link-btn" onClick={() => select([sheet.typeId!])}>
                    Edit Type
                  </button>
                </>
              ) : (
                <div className="prop-title">{sheet.title}</div>
              )}
            </div>
            {[...groups.entries()].map(([group, props]) => (
              <div key={group} className="prop-group">
                <div className="prop-group-title">{group}</div>
                {props.map((p) => (
                  <div key={p.key} className="prop-row">
                    <label className="prop-label">{p.label}</label>
                    <div className="prop-value">
                      {p.kind === "ReadOnly" ? (
                        <span className="prop-ro">{p.value}</span>
                      ) : p.kind === "Action" ? (
                        <button className="prop-action" onClick={() => set(p.key, "")}>
                          {p.value}
                        </button>
                      ) : p.kind === "Choice" ? (
                        <select
                          className="prop-input"
                          aria-label={p.label}
                          value={p.value}
                          onChange={(e) => set(p.key, e.target.value)}
                        >
                          {p.options.map((o) => (
                            <option key={o.id} value={o.id}>
                              {o.label}
                            </option>
                          ))}
                        </select>
                      ) : (
                        <TextInput
                          key={`${sheet.id}:${p.key}:${p.value}`}
                          prop={p}
                          onCommit={(v) => set(p.key, v)}
                        />
                      )}
                    </div>
                  </div>
                ))}
              </div>
            ))}
          </>
        ) : (
          <div className="prop-empty">Select an element or open a view.</div>
        )}
      </div>
    </aside>
  );
}
