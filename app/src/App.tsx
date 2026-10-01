import { SiteDialog } from "./components/SiteDialog";
import { ViewDialogs } from "./components/ViewDialogs";
import { runAction, startTool } from "./actions";
import { deleteSketchSelection, flipSketchSelection, startSketch } from "./sketch";
import { useEffect, useRef } from "react";
import { errorMessage, ipc } from "./ipc";
import {
  apply,
  deleteSelection,
  handleMenu,
  newProject,
  openProject,
  redo,
  importIfc,
  sampleProject,
  undo,
} from "./fileActions";
import { REFERENCE_TOOLS, useAppStore } from "./store";
import { shortcut, startsTypedValue } from "./tools";
import { TopBar } from "./components/TopBar";
import { Ribbon } from "./components/Ribbon";
import { OptionsBar } from "./components/OptionsBar";
import { PaintChip } from "./components/PaintChip";
import { IfcReport } from "./components/IfcReport";
import { ProjectBrowser } from "./components/ProjectBrowser";
import { PropertiesPanel } from "./components/PropertiesPanel";
import { StatusBar, Workspace } from "./components/Workspace";
import {
  StandardsBrowser,
  StandardsProperties,
  StandardsStatus,
  StandardsView,
} from "./components/Standards";
import {
  ProjectInfoBrowser,
  ProjectInfoStatus,
  ProjectInfoSummary,
  ProjectInfoView,
} from "./components/ProjectInfo";
import {
  SpecsBrowser,
  SpecsDialogs,
  SpecsProperties,
  SpecsStatus,
  SpecsView,
} from "./components/Specs";
import { QaFixDialog, QaPanel } from "./components/Qa";
import { StandardChoicesDialog } from "./components/StandardChoices";
import { EditModelDialog } from "./components/EditModel";
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
          <button className="btn-outline light" onClick={() => void importIfc()}>
            Open IFC (Revit)
          </button>
          <button
            className="btn-ghost light"
            onClick={() => void sampleProject("modern")}
            title="A furnished two-story modern house with landscape, lighting and a drawing set"
          >
            Sample: Modern House
          </button>
          <button
            className="btn-ghost light"
            onClick={() => void sampleProject("basic")}
            title="A simple two-story box house with its drawing set"
          >
            Sample: Basic House
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
  const propsHidden = useAppStore((s) => s.propsHidden);
  // The Standards tab replaces the browser, views and properties (ADR-047).
  const standards = useAppStore((s) => s.ribbonTab === "Standards" && !s.app?.sketch);
  // So does the Project Info tab (ADR-084).
  const projectTab = useAppStore((s) => s.ribbonTab === "Project Info" && !s.app?.sketch);
  // And the Specifications tab (ADR-085).
  const specsTab = useAppStore((s) => s.ribbonTab === "Specifications" && !s.app?.sketch);
  const tool = useAppStore((s) => s.tool);

  useEffect(() => {
    ipc.appState().then(
      (s) => setApp(s, true),
      (err) => setError(errorMessage(err)),
    );
    // Development aid (ADR-095): render a camera view and save it, unattended.
    void ipc
      .autoRender()
      .then(async (auto) => {
        if (!auto) return;
        for (let i = 0; i < 100 && !useAppStore.getState().app; i++)
          await new Promise((r) => setTimeout(r, 100));
        const s = useAppStore.getState();
        const v = s.app?.views.find((x) => x.name === auto.view);
        if (!v) {
          s.setError(`Auto render: no view named ${auto.view}`);
          return;
        }
        s.openView(v.id);
        useAppStore.setState({ autoRender: auto });
        s.setUi({ viewDialog: "render" });
      })
      .catch(() => {});
    const unlisten = ipc.onMenu((id) => void handleMenu(id));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [setApp, setError]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ui = useAppStore.getState();
      if (
        isTyping(e.target) ||
        ui.confirm ||
        ui.rufplan ||
        ui.paramsOpen ||
        ui.siteDialog ||
        ui.viewDialog
      )
        return;
      const ctrl = e.ctrlKey || e.metaKey;
      if (ui.app?.sketch) {
        // Sketch mode keys (ADR-021): its own undo, Delete, Space (flip), Tab (chain).
        if (ctrl && (e.key.toLowerCase() === "z" || e.key.toLowerCase() === "y")) {
          e.preventDefault();
          const redo = e.key.toLowerCase() === "y" || e.shiftKey;
          void apply(() => ipc.sketchUndo(redo));
          return;
        }
        if (e.key === "Delete" || e.key === "Backspace") {
          deleteSketchSelection();
          return;
        }
        if (e.key === " ") {
          e.preventDefault();
          flipSketchSelection();
          return;
        }
        if (e.key === "Tab" && ui.sketchUi.mode === "PickWalls") {
          e.preventDefault();
          ui.setSketchUi({ tab: !ui.sketchUi.tab });
          return;
        }
        if (e.key === "Escape") {
          window.dispatchEvent(new Event("tool-cancel"));
          return;
        }
        if (!ctrl && !e.altKey && startsTypedValue(e.key)) {
          window.dispatchEvent(new CustomEvent("typed-value", { detail: e.key }));
          return;
        }
        if ((keys.current + e.key).toUpperCase().endsWith("ZF")) {
          keys.current = "";
          window.dispatchEvent(new Event("view-fit"));
          return;
        }
        keys.current = (keys.current + e.key).slice(-1);
        return;
      }
      if (e.key === "Tab" && REFERENCE_TOOLS.includes(ui.tool)) {
        // Tab steps to the next reference under the cursor, as in Revit.
        e.preventDefault();
        window.dispatchEvent(new Event("dimension-tab"));
        return;
      }
      if (e.key === "Tab" && ui.tool === "select") {
        // Tab (Shift+Tab back) steps through what's under the cursor, as in Revit (ADR-056).
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("select-tab", { detail: e.shiftKey }));
        return;
      }
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
        // With nothing in progress, Enter repeats the last command, as in Revit.
        if (ui.tool === "select") void runAction("repeat");
        else window.dispatchEvent(new Event("tool-finish"));
      } else if (e.key === " " && (ui.tool === "door" || ui.tool === "window")) {
        // Placing a door: the spacebar turns its swing through all four ways (Revit's).
        e.preventDefault();
        ui.turnOpening();
        window.dispatchEvent(new Event("opening-turn"));
      } else if (e.key === " " && ui.tool === "select" && ui.selection.length > 0) {
        // Spacebar flips the selected walls and windows, and turns doors through their
        // four swings, as in Revit.
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
        if (r.action) void runAction(r.action);
        else if (r.tool === "floor") void startSketch("Floor");
        else if (r.tool === "ceiling") void startSketch("Ceiling");
        else if (r.tool) void startTool(r.tool);
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
          {specsTab ? (
            <>
              <div className="main std-main">
                <SpecsBrowser />
                <SpecsView />
                <SpecsProperties />
              </div>
              <SpecsStatus />
              <SpecsDialogs />
            </>
          ) : projectTab ? (
            <>
              <div className="main std-main">
                <ProjectInfoBrowser />
                <ProjectInfoView />
                <ProjectInfoSummary />
              </div>
              <ProjectInfoStatus />
            </>
          ) : standards ? (
            <>
              <div className="main std-main">
                <StandardsBrowser />
                <StandardsView />
                <StandardsProperties />
              </div>
              <StandardsStatus />
              <StandardChoicesDialog />
            </>
          ) : (
            <>
              <OptionsBar />
              <div className={`main${tool === "paint" ? " painting" : ""}`}>
                <ProjectBrowser />
                <Workspace />
                <PaintChip />
                {!propsHidden && <PropertiesPanel />}
              </div>
              <StatusBar />
            </>
          )}
        </>
      ) : (
        <Welcome />
      )}
      <EditModelDialog />
      <ConfirmDialog />
      <RufplanDialog />
      <ParamsDialog />
      <SiteDialog />
      <ViewDialogs />
      <QaPanel />
      <QaFixDialog />
      <IfcReport />
    </div>
  );
}
