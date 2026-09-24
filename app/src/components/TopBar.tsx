import logo from "../assets/rufplan-logo-white.svg";
import { apply, newProject, openProject, redo, saveProject, undo } from "../fileActions";
import { ipc } from "../ipc";
import { useAppStore } from "../store";
import { Icons } from "./Icons";

export function TopBar() {
  const app = useAppStore((s) => s.app);
  const info = app?.projectInfo;
  return (
    <header className="topbar">
      <div className="brand">
        <img src={logo} alt="Rufplan" className="brand-logo" />
        <span className="brand-product">Studio</span>
      </div>
      <nav className="topbar-file">
        <button onClick={() => void newProject()}>New</button>
        <button onClick={() => void openProject()}>Open</button>
        <button onClick={() => void saveProject()} disabled={!app}>
          Save
        </button>
      </nav>
      <div className="topbar-center">
        {app && (
          <>
            <span className="project-name" title={app.project.path ?? "Not saved yet"}>
              {app.projectName || app.project.name}
              {app.project.dirty && <span className="dirty-dot" title="Unsaved changes" />}
            </span>
            <span className="project-file">
              {app.project.path ? app.project.name + ".rfproj" : "Unsaved"}
            </span>
          </>
        )}
      </div>
      {app && info && (
        <label className="stage-picker" title="Current design stage">
          <span className="stage-label">Stage</span>
          <select
            aria-label="Design stage"
            value={app.currentStage ?? ""}
            onChange={(e) =>
              void apply(() => ipc.setProperty(info, "current_stage", e.target.value))
            }
          >
            {app.stages.map((s) => (
              <option key={s.id} value={s.id}>
                {s.abbreviation} — {s.name}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="topbar-actions">
        <button
          className="icon-btn"
          onClick={() => void undo()}
          disabled={!app?.undo}
          title={app?.undo ? `Undo ${app.undo} (Ctrl+Z)` : "Nothing to undo"}
        >
          {Icons.undo}
        </button>
        <button
          className="icon-btn"
          onClick={() => void redo()}
          disabled={!app?.redo}
          title={app?.redo ? `Redo ${app.redo} (Ctrl+Y)` : "Nothing to redo"}
        >
          {Icons.redo}
        </button>
      </div>
    </header>
  );
}
