// The only module that talks to Rust. Payload types are generated from Rust by ts-rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AppState } from "./bindings/AppState";
import type { CommandError } from "./bindings/CommandError";
import type { CoreVersion } from "./bindings/CoreVersion";
import type { DisplayList } from "./bindings/DisplayList";
import type { ElementId } from "./bindings/ElementId";
import type { Mesh } from "./bindings/Mesh";
import type { OpeningPreview } from "./bindings/OpeningPreview";
import type { RoomPreview } from "./bindings/RoomPreview";
import type { PropertySheet } from "./bindings/PropertySheet";
import type { ProjectStatus } from "./bindings/ProjectStatus";
import type { Pt } from "./bindings/Pt";
import type { SnapResult } from "./bindings/SnapResult";
import type { ViewInfo } from "./bindings/ViewInfo";

export type {
  AppState,
  CommandError,
  CoreVersion,
  DisplayList,
  ElementId,
  Mesh,
  OpeningPreview,
  PropertySheet,
  RoomPreview,
  ProjectStatus,
  Pt,
  SnapResult,
  ViewInfo,
};

const PROJECT_FILTER = [{ name: "Rufplan Studio project", extensions: ["rfproj"] }];

type S = Promise<AppState | null>;

export const ipc = {
  coreVersion: () => invoke<CoreVersion>("core_version"),
  appState: (): S => invoke("app_state"),
  projectNew: (): S => invoke("project_new"),
  /** A new unsaved project with a sample two-storey house. */
  projectSample: (): S => invoke("project_sample"),
  projectOpen: (path: string): S => invoke("project_open", { path }),
  /** Omit `path` to save to the project's current location. */
  projectSave: (path?: string): S => invoke("project_save", { path: path ?? null }),

  displayList: (view: ElementId) => invoke<DisplayList | null>("view_display_list", { view }),
  meshes: () => invoke<Mesh[]>("view_meshes"),
  pick: (view: ElementId, point: Pt, tol: number) =>
    invoke<ElementId | null>("pick", { view, point, tol }),
  snap: (view: ElementId, point: Pt, from: Pt | null, tol: number) =>
    invoke<SnapResult>("snap", { view, point, from, tol }),

  createWall: (view: ElementId, typeId: ElementId, start: Pt, end: Pt): S =>
    invoke("create_wall", { view, typeId, start, end }),
  createGrid: (start: Pt, end: Pt): S => invoke("create_grid", { start, end }),
  createLevel: (elevation: number): S => invoke("create_level", { elevation }),
  /** An empty boundary means "from the outer faces of this level's walls". */
  createFloor: (view: ElementId, typeId: ElementId, boundary: Pt[]): S =>
    invoke("create_floor", { view, typeId, boundary }),
  /** Either a sketched boundary, or `inside` a room enclosed by walls. */
  createCeiling: (view: ElementId, typeId: ElementId, boundary: Pt[], inside: Pt | null): S =>
    invoke("create_ceiling", { view, typeId, boundary, inside }),
  /** Where a door/window of `typeId` would go for the cursor at `point` (plan views). */
  /** The enclosed area a room at `point` would fill (floor plans). */
  roomPreview: (view: ElementId, point: Pt) =>
    invoke<RoomPreview | null>("room_preview", { view, point }),
  createRoom: (view: ElementId, point: Pt): S => invoke("create_room", { view, point }),
  /** Moves elements by `delta` mm; joined walls stretch to follow. */
  moveElements: (ids: ElementId[], delta: Pt): S => invoke("move_elements", { ids, delta }),
  openingPreview: (view: ElementId, typeId: ElementId, point: Pt, tol: number) =>
    invoke<OpeningPreview | null>("opening_preview", { view, typeId, point, tol }),
  createOpening: (typeId: ElementId, host: ElementId, offset: number, flipFacing: boolean): S =>
    invoke("create_opening", { typeId, host, offset, flipFacing }),
  deleteElements: (ids: ElementId[]): S => invoke("delete_elements", { ids }),
  properties: (id: ElementId) => invoke<PropertySheet>("properties", { id }),
  setProperty: (id: ElementId, key: string, value: string): S =>
    invoke("set_property", { id, key, value }),
  undo: (): S => invoke("undo"),
  redo: (): S => invoke("redo"),
  /** Returns false (and stays open) if there are unsaved changes and `force` is false. */
  exit: (force: boolean) => invoke<boolean>("app_exit", { force }),

  /** Native menu clicks, forwarded by Rust as the menu item id. */
  onMenu: (handler: (id: string) => void): Promise<UnlistenFn> =>
    listen<string>("menu", (event) => handler(event.payload)),
};

export const dialogs = {
  /** Resolves to the chosen path, or null if cancelled. */
  pickProjectToOpen: async (): Promise<string | null> => {
    const picked = await open({ multiple: false, directory: false, filters: PROJECT_FILTER });
    return typeof picked === "string" ? picked : null;
  },
  pickProjectSaveLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.rfproj`, filters: PROJECT_FILTER }),
};

/** Turns anything a command rejects with into a user-facing message. */
export function errorMessage(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as CommandError).message);
  }
  return String(err);
}
