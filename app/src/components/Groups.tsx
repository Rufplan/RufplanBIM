import { useState } from "react";
import type { AppState } from "../bindings/AppState";
import type { ElementId } from "../bindings/ElementId";
import type { GroupKind } from "../bindings/GroupKind";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";
import { Icons } from "./Icons";
import { CHECK, CROSS } from "./SketchRibbon";
export { groupOf, toGroups, withMembers } from "../groups";

// Model and detail groups (ADR-087), as in Revit: Create Group (GP), Place Group, Edit
// Group (Add, Remove, Finish repeats the edit in every instance, Cancel), Ungroup, and the
// Groups branch of the Project Browser. Rust owns the groups (studio-core `groups`); this
// module shows them and sends the commands.

/** "Modify | Model Groups" when only groups are selected. */
export function groupsLabel(app: AppState | null, selection: ElementId[]): string | null {
  if (!app || selection.length === 0) return null;
  const kinds = selection.map((id) => app.groups.find((g) => g.id === id)?.kind);
  if (kinds.some((k) => !k)) return null;
  const set = new Set(kinds);
  if (set.size > 1) return "Modify | Multi-Select";
  return set.has("Detail") ? "Modify | Detail Groups" : "Modify | Model Groups";
}

export async function editGroup(id: ElementId) {
  const s = useAppStore.getState();
  s.select([]);
  return apply(() => ipc.groupEdit(id));
}

/** Place Group: the tool with a type (the first of its kind when none given). */
export function placeGroup(kind: GroupKind, typeId?: ElementId) {
  const s = useAppStore.getState();
  const t = typeId ?? s.app?.groupTypes.find((x) => x.kind === kind && x.instances > 0)?.id ?? null;
  if (!t) {
    s.setError(
      kind === "Model"
        ? "There's no model group yet: select elements and use Create Group (GP)."
        : "There's no detail group yet: select a view's detail lines or text and use Create Group (GP).",
    );
    return;
  }
  s.setToolType("group", t);
  s.setTool("placeGroup");
}

/** Create Group (GP): names the new group, then selects it. */
export function CreateGroupDialog({ onClose }: { onClose: () => void }) {
  const selection = useAppStore((s) => s.selection);
  const app = useAppStore((s) => s.app);
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const taken = new Set(app?.groupTypes.map((t) => t.name) ?? []);
  const ok = async () => {
    try {
      const [made, state] = await ipc.groupCreate(selection, name);
      if (state) useAppStore.getState().setApp(state);
      useAppStore.getState().select(made);
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    }
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Create Group">
      <div className="modal drafting-dialog">
        <h2>Create Group</h2>
        <p className="group-note">
          {selection.length} selected. Model elements make a model group; a view&apos;s detail
          lines, text and regions make a detail group (both, if you chose both).
        </p>
        <label className="inplace-name">
          Name
          <input
            aria-label="Group name"
            autoFocus
            placeholder="Group 1"
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              setError("");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") void ok();
              if (e.key === "Escape") onClose();
            }}
          />
        </label>
        {taken.has(name.trim()) && (
          <div className="group-error">There&apos;s already a group named {name.trim()}.</div>
        )}
        {error && <div className="group-error">{error}</div>}
        <div className="mb-actions">
          <button className="btn-cyan" disabled={taken.has(name.trim())} onClick={() => void ok()}>
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

/** The contextual tab while editing a group (Revit's Edit Group panel). */
export function GroupEditRibbon() {
  const app = useAppStore((s) => s.app);
  const selection = useAppStore((s) => s.selection);
  const g = app?.groups.find((x) => x.id === app.editingGroup);
  if (!g) return null;
  const members = selection.filter((id) => g.members.includes(id));
  const others = selection.filter((id) => !g.members.includes(id));
  return (
    <div className="ribbon group-ribbon" role="toolbar" aria-label="Tools">
      <div className="rb-tabs" role="tablist" aria-label="Ribbon tabs">
        <button role="tab" aria-selected className="rb-tab active contextual">
          {`Edit Group | ${g.kind === "Detail" ? "Detail" : "Model"} : ${g.name}`}
        </button>
      </div>
      <div className="rb-body">
        <div className="rb-group">
          <div className="rb-items">
            <button
              className="rb-btn"
              disabled={others.length === 0}
              title="Add the selected elements to the group"
              onClick={() => void apply(() => ipc.groupAdd(others))}
            >
              {Icons.add}
              <span>Add</span>
            </button>
            <button
              className="rb-btn"
              disabled={members.length === 0}
              title="Take the selected elements out of the group (they stay in the model)"
              onClick={() => void apply(() => ipc.groupRemove(members))}
            >
              {Icons.del}
              <span>Remove</span>
            </button>
          </div>
          <div className="rb-title">Edit Group</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            <button
              className="rb-btn"
              title="Finish: the change is made in every instance of the group"
              onClick={() => void apply(() => ipc.groupFinish())}
            >
              {CHECK}
              <span>Finish</span>
            </button>
            <button
              className="rb-btn"
              title="Cancel: undo everything done since Edit Group"
              onClick={() => void apply(() => ipc.groupCancel())}
            >
              {CROSS}
              <span>Cancel</span>
            </button>
          </div>
          <div className="rb-title">Mode</div>
        </div>
        <div className="rb-group">
          <div className="rb-items group-status">
            <span>
              <b>{g.members.length}</b> elements in the group ·{" "}
              {app!.groupTypes.find((t) => t.id === g.typeId)?.instances ?? 1} instances
            </span>
            <span className="rb-hint">
              Draw new elements to add them; Finish updates every instance.
            </span>
          </div>
          <div className="rb-title">Group</div>
        </div>
      </div>
    </div>
  );
}

/** Tools for selected groups in the contextual Modify tab. */
export function GroupPanels() {
  const app = useAppStore((s) => s.app);
  const selection = useAppStore((s) => s.selection);
  const groups = selection
    .map((id) => app?.groups.find((g) => g.id === id))
    .filter((g): g is NonNullable<typeof g> => !!g);
  const grouped = groups.length > 0;
  return (
    <div className="rb-group">
      <div className="rb-items">
        {grouped ? (
          <>
            <button
              className="rb-btn"
              disabled={groups.length !== 1}
              title="Edit Group: change this instance; Finish repeats it in every instance (or double-click)"
              onClick={() => void editGroup(groups[0]!.id)}
            >
              {Icons.edit}
              <span>Edit Group</span>
            </button>
            <button
              className="rb-btn"
              title="Ungroup: the elements stay, the group instance goes"
              onClick={() => void apply(() => ipc.groupUngroup(groups.map((g) => g.id)))}
            >
              {Icons.ungroup}
              <span>Ungroup</span>
            </button>
            <button
              className="rb-btn"
              disabled={groups.length !== 1}
              title="Place another instance of this group"
              onClick={() => placeGroup(groups[0]!.kind, groups[0]!.typeId)}
            >
              {Icons.group}
              <span>Place</span>
            </button>
          </>
        ) : (
          <button
            className="rb-btn"
            disabled={selection.length === 0}
            title="Create Group (GP): group the selection so copies stay alike"
            onClick={() => useAppStore.getState().setUi({ viewDialog: "createGroup" })}
          >
            {Icons.group}
            <span>Create Group</span>
          </button>
        )}
      </div>
      <div className="rb-title">Group</div>
    </div>
  );
}
