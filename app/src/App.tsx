import { useEffect, useRef } from "react";
import { errorMessage, ipc } from "./ipc";
import {
  apply,
  deleteSelection,
  handleMenu,
  newProject,
  openProject,
  redo,
  sampleProject,
  undo,
} from "./fileActions";
import { useAppStore } from "./store";
import { shortcut, startsTypedValue } from "./tools";
import { TopBar } from "./components/TopBar";
import { Ribbon } from "./components/Ribbon";
import { OptionsBar } from "./components/OptionsBar";
import { ProjectBrowser } from "./components/ProjectBrowser";
import { PropertiesPanel } from "./components/PropertiesPanel";
import { StatusBar, Workspace } from "./components/Workspace";
import { RufplanDialog } from "./components/RufplanDialog";
import { ParamsDialog } from "./components/ParamsDialog";
import logo from "./assets/rufplan-logo-white.svg";

function isTyping(target: EventTarget | null) {
  const el = target as HTMLElement | null;
  return (
    !!el &&
    (el.tagName === "INPUT" ||
      el.tagName === "SELECT" ||
      el.tagName === "TEXTAREA" ||
      el.isContentEditable)
  );
}

function ConfirmDialog() {
  const confirm = useAppStore((s) => s.confirm);
  if (!confirm) return null;
  return (
    <div className="modal-backdrop" role="dialog" aria-modal aria-label="Unsaved changes">
      <div className="modal">
        <div className="modal-kicker">Unsaved changes</div>
        <p className="modal-message">{confirm.message}</p>
        <div className="modal-actions">
          <button className="btn-cyan" autoFocus onClick={() => confirm.resolve("save")}>
            Save
          </button>
          <button className="btn-outline" onClick={() => confirm.resolve("discard")}>
            Don&apos;t Save
          </button>
          <button className="btn-ghost" onClick={() => confirm.resolve("cancel")}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

function Welcome() {
  return (
    <main className="welcome">
      <div className="welcome-card">
        <div className="welcome-mark">
          <img src={logo} alt="Rufplan" />
          <span>Studio</span>
        </div>
        <h1>Model it once. Draw it everywhere.</h1>
        <p>
          Levels, grids, walls, floors and ceilings — with plans, ceiling plans, elevations and 3D
          generated from one model.
        </p>
        <div className="welcome-actions">
          <button className="btn-cyan" onClick={() => void newProject()}>
            New Project
          </button>
          <button className="btn-outline light" onClick={() => void openProject()}>
            Open Project
          </button>
          <button className="btn-ghost light" onClick={() => void sampleProject()}>
            Sample Project
          </button>
        </div>
      </div>
    </main>
  );
}

export function App() {
  const app = useAppStore((s) => s.app);
  const error = useAppStore((s) => s.error);
  const setApp = useAppStore((s) => s.setApp);
  const setError = useAppStore((s) => s.setError);
  const keys = useRef("");

  useEffect(() => {
    ipc.appState().then(
      (s) => setApp(s, true),
      (err) => setError(errorMessage(err)),
    );
    const unlisten = ipc.onMenu((id) => void handleMenu(id));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [setApp, setError]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ui = useAppStore.getState();
      if (isTyping(e.target) || ui.confirm || ui.rufplan || ui.paramsOpen) return;
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.key.toLowerCase() === "z") {
        e.preventDefault();
        void (e.shiftKey ? redo() : undo());
      } else if (ctrl && e.key.toLowerCase() === "y") {
        e.preventDefault();
        void redo();
      } else if (e.key === "Delete" || e.key === "Backspace") {
        void deleteSelection();
      } else if (e.key === "Escape") {
        window.dispatchEvent(new Event("tool-cancel"));
      } else if (e.key === "Enter") {
        window.dispatchEvent(new Event("tool-finish"));
      } else if (e.key === " " && ui.tool === "select" && ui.selection.length > 0) {
        // Spacebar flips the selected walls, doors and windows, as in Revit.
        e.preventDefault();
        void apply(() => ipc.flipSelection(ui.selection));
      } else if (!ctrl && !e.altKey && startsTypedValue(e.key) && ui.app) {
        keys.current = "";
        window.dispatchEvent(new CustomEvent("typed-value", { detail: e.key }));
      } else if (!ctrl && !e.altKey && useAppStore.getState().app) {
        if ((keys.current + e.key).toUpperCase().endsWith("ZF")) {
          keys.current = "";
          window.dispatchEvent(new Event("view-fit"));
          return;
        }
        const r = shortcut(keys.current, e.key);
        keys.current = r.buffer;
        if (r.tool) useAppStore.getState().setTool(r.tool);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="shell">
      <TopBar />
      {error && (
        <div role="alert" className="error-bar">
          <span>{error}</span>
          <button onClick={() => setError(null)} aria-label="Dismiss">
            ×
          </button>
        </div>
      )}
      {app ? (
        <>
          <Ribbon />
          <OptionsBar />
          <div className="main">
            <ProjectBrowser />
            <Workspace />
            <PropertiesPanel />
          </div>
          <StatusBar />
        </>
      ) : (
        <Welcome />
      )}
      <ConfirmDialog />
      <RufplanDialog />
      <ParamsDialog />
    </div>
  );
}
