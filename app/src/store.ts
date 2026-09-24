import { create } from "zustand";
import type { AppState, ElementId } from "./ipc";

// UI state only. The model lives in Rust; `app` mirrors the last snapshot it returned.

export type Tool =
  "select" | "wall" | "grid" | "floor" | "floorAuto" | "ceiling" | "ceilingAuto" | "level";

export const TOOL_LABELS: Record<Tool, string> = {
  select: "Select",
  wall: "Wall",
  grid: "Grid",
  floor: "Floor: Sketch",
  floorAuto: "Floor: Pick Walls",
  ceiling: "Ceiling: Sketch",
  ceilingAuto: "Ceiling: Auto Room",
  level: "Level",
};

/** Which element types the active tools place, by category. */
export interface ToolTypes {
  wall: ElementId | null;
  floor: ElementId | null;
  ceiling: ElementId | null;
}

/** A pending Save / Don't Save / Cancel question and what to do after it. */
export interface Confirm {
  message: string;
  resolve: (choice: "save" | "discard" | "cancel") => void;
}

interface UiState {
  app: AppState | null;
  error: string | null;
  openViews: ElementId[];
  activeView: ElementId | null;
  /** When nothing is selected, the properties panel shows the active view. */
  selection: ElementId[];
  tool: Tool;
  toolTypes: ToolTypes;
  /** Prompt shown in the status bar by the active tool. */
  prompt: string;
  cursor: string;
  confirm: Confirm | null;

  /** `fresh` = a different project was just created or opened. */
  setApp: (app: AppState | null, fresh?: boolean) => void;
  setError: (error: string | null) => void;
  openView: (id: ElementId) => void;
  closeView: (id: ElementId) => void;
  select: (ids: ElementId[]) => void;
  setTool: (tool: Tool) => void;
  setToolType: (kind: keyof ToolTypes, id: ElementId) => void;
  setPrompt: (prompt: string) => void;
  setCursor: (cursor: string) => void;
  setConfirm: (confirm: Confirm | null) => void;
}

const firstId = (items: { id: ElementId }[] | undefined, current: ElementId | null) =>
  current && items?.some((i) => i.id === current) ? current : (items?.[0]?.id ?? null);

export const useAppStore = create<UiState>((set, get) => ({
  app: null,
  error: null,
  openViews: [],
  activeView: null,
  selection: [],
  tool: "select",
  toolTypes: { wall: null, floor: null, ceiling: null },
  prompt: "",
  cursor: "",
  confirm: null,

  setApp: (app, fresh = false) => {
    const s = get();
    if (!app) {
      set({ app: null, openViews: [], activeView: null, selection: [] });
      return;
    }
    const exists = new Set(app.views.map((v) => v.id));
    let openViews = s.openViews.filter((v) => exists.has(v));
    // A different project (or first load): open its first floor plan.
    const sameProject = !fresh && s.app !== null;
    if (!sameProject || openViews.length === 0) {
      const first = app.views[0]?.id;
      openViews = first ? [first] : [];
    }
    const activeView =
      s.activeView && openViews.includes(s.activeView) ? s.activeView : (openViews[0] ?? null);
    set({
      app,
      error: null,
      openViews,
      activeView,
      selection: sameProject ? s.selection : [],
      toolTypes: {
        wall: firstId(app.wallTypes, s.toolTypes.wall),
        floor: firstId(app.floorTypes, s.toolTypes.floor),
        ceiling: firstId(app.ceilingTypes, s.toolTypes.ceiling),
      },
    });
  },
  setError: (error) => set({ error }),
  openView: (id) =>
    set((s) => ({
      openViews: s.openViews.includes(id) ? s.openViews : [...s.openViews, id],
      activeView: id,
      selection: [],
      tool: "select",
    })),
  closeView: (id) =>
    set((s) => {
      const openViews = s.openViews.filter((v) => v !== id);
      const activeView =
        s.activeView === id ? (openViews[openViews.length - 1] ?? null) : s.activeView;
      return { openViews, activeView };
    }),
  select: (selection) => set({ selection }),
  setTool: (tool) => set({ tool, selection: tool === "select" ? get().selection : [] }),
  setToolType: (kind, id) => set((s) => ({ toolTypes: { ...s.toolTypes, [kind]: id } })),
  setPrompt: (prompt) => set({ prompt }),
  setCursor: (cursor) => set({ cursor }),
  setConfirm: (confirm) => set({ confirm }),
}));

/** Info about the active view, if any. */
export function activeViewInfo(s: UiState) {
  return s.app?.views.find((v) => v.id === s.activeView) ?? null;
}
