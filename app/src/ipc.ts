import { useAppStore } from "./store";
// The only module that talks to Rust. Payload types are generated from Rust by ts-rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppState } from "./bindings/AppState";
import type { Category } from "./bindings/Category";
import type { Handles } from "./bindings/Handles";
import type { ImageryFrame } from "./bindings/ImageryFrame";
import type { CameraPose } from "./bindings/CameraPose";
import type { SunPosition } from "./bindings/SunPosition";
import type { OffsetPreview } from "./bindings/OffsetPreview";
import type { ParamDef } from "./bindings/ParamDef";
import type { RefLine } from "./bindings/RefLine";
import type { CloudStatus } from "./bindings/CloudStatus";
import type { CommandError } from "./bindings/CommandError";
import type { CoreVersion } from "./bindings/CoreVersion";
import type { DisplayList } from "./bindings/DisplayList";
import type { ElementId } from "./bindings/ElementId";
import type { Mesh } from "./bindings/Mesh";
import type { OpeningPreview } from "./bindings/OpeningPreview";
import type { RoomPreview } from "./bindings/RoomPreview";
import type { Table } from "./bindings/Table";
import type { DimensionPreview } from "./bindings/DimensionPreview";
import type { PropertySheet } from "./bindings/PropertySheet";
import type { ProjectStatus } from "./bindings/ProjectStatus";
import type { PublishOptions } from "./bindings/PublishOptions";
import type { PublishResult } from "./bindings/PublishResult";
import type { RufplanLink } from "./bindings/RufplanLink";
import type { Pt } from "./bindings/Pt";
import type { SnapResult } from "./bindings/SnapResult";
import type { SnapKind } from "./bindings/SnapKind";
import type { ViewInfo } from "./bindings/ViewInfo";
import type { DrawOptions } from "./bindings/DrawOptions";
import type { DrawTool } from "./bindings/DrawTool";
import type { SketchKind } from "./bindings/SketchKind";
import type { OpeningPreview3d } from "./bindings/OpeningPreview3d";
import type { ParcelHit } from "./bindings/ParcelHit";
import type { SiteKeys } from "./bindings/SiteKeys";

export type {
  AppState,
  Category,
  Handles,
  OffsetPreview,
  ParamDef,
  RefLine,
  CloudStatus,
  CommandError,
  CoreVersion,
  DisplayList,
  ElementId,
  Mesh,
  OpeningPreview,
  PropertySheet,
  RoomPreview,
  Table,
  DimensionPreview,
  ProjectStatus,
  PublishOptions,
  PublishResult,
  RufplanLink,
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
  meshes: (view: ElementId | null = null) => invoke<Mesh[]>("view_meshes", { view }),
  pick: (view: ElementId, point: Pt, tol: number) =>
    invoke<ElementId | null>("pick", { view, point, tol }),
  /** Snaps a point; a pending one-pick snap override (SE, SM…) applies unless given. */
  snap: (view: ElementId, point: Pt, from: Pt | null, tol: number, only?: SnapKind | null) =>
    invoke<SnapResult>("snap", {
      view,
      point,
      from,
      tol,
      only: only === undefined ? useAppStore.getState().snapOverride : only,
    }),

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
  createSection: (start: Pt, end: Pt): S => invoke("create_section", { start, end }),
  /** The dimension a→b with its line through `cursor`. */
  dimensionPreview: (view: ElementId, a: Pt, b: Pt, cursor: Pt) =>
    invoke<DimensionPreview | null>("dimension_preview", { view, a, b, cursor }),
  createDimension: (view: ElementId, a: Pt, b: Pt, offset: number): S =>
    invoke("create_dimension", { view, a, b, offset }),
  createText: (view: ElementId, at: Pt, text: string): S =>
    invoke("create_text", { view, at, text }),
  createSheet: (name: string, tabloid: boolean): S => invoke("create_sheet", { name, tabloid }),
  /** Places `view` in the middle of `sheet`. */
  placeView: (sheet: ElementId, view: ElementId): S => invoke("place_view", { sheet, view }),
  scheduleTable: (view: ElementId) => invoke<Table | null>("schedule_table", { view }),
  /** Writes every sheet to a PDF; resolves to the number of sheets. */
  exportPdf: (path: string) => invoke<number>("export_pdf", { path }),
  exportIfc: (path: string) => invoke<string>("export_ifc", { path }),
  /** Tags every untagged door, window and room in a floor plan. */
  tagAll: (view: ElementId): S => invoke("tag_all", { view }),
  /** Records an issuance of the current stage's sheet set and writes it to a PDF. */
  issueSet: (name: string, path: string): S => invoke("issue_set", { name, path }),
  /** The enclosed area a room at `point` would fill (floor plans). */
  roomPreview: (view: ElementId, point: Pt) =>
    invoke<RoomPreview | null>("room_preview", { view, point }),
  createRoom: (view: ElementId, point: Pt): S => invoke("create_room", { view, point }),
  /** Moves elements by `delta` mm; joined walls stretch to follow. */
  moveElements: (ids: ElementId[], delta: Pt): S => invoke("move_elements", { ids, delta }),
  /** Where a door/window of `typeId` would go for the cursor at `point` (plan views). */
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

  // Rufplan.io account and publishing (ADR-016).
  /** Signed-in state; the first call signs in again from the stored token. */
  cloudStatus: () => invoke<CloudStatus>("cloud_status"),
  cloudSignIn: (email: string, password: string) =>
    invoke<CloudStatus>("cloud_sign_in", { email, password }),
  /** Opens the system browser and resolves when it comes back. */
  cloudSignInGoogle: () => invoke<CloudStatus>("cloud_sign_in_google"),
  cloudSignOut: () => invoke<CloudStatus>("cloud_sign_out"),
  /** Rufplan projects the user owns. */
  cloudProjects: () => invoke<RufplanLink[]>("cloud_projects"),
  /** `null` unlinks. */
  linkRufplan: (link: RufplanLink | null): S => invoke("link_rufplan", { link }),
  publishOptions: () => invoke<PublishOptions>("publish_options"),
  publishToRufplan: (name: string, deliverable: string) =>
    invoke<PublishResult>("publish_to_rufplan", { name, deliverable }),
  /** Opens a rufplan.io page in the browser. */
  openRufplan: (url: string) => openUrl(url),

  // Modify tools, grips and temporary dimensions (ADR-017).
  /** A typed length in mm, or null if the text isn't one. */
  parseLength: (text: string) => invoke<number | null>("parse_length", { text }),
  handles: (view: ElementId, ids: ElementId[]) => invoke<Handles>("handles", { view, ids }),
  dragHandle: (id: ElementId, key: string, to: Pt): S => invoke("drag_handle", { id, key, to }),
  setTempDimension: (id: ElementId, key: string, value: string): S =>
    invoke("set_temp_dimension", { id, key, value }),
  /** Copy (count 1) or Array (count copies, each `delta` further). */
  copyElements: (ids: ElementId[], delta: Pt, count: number): S =>
    invoke("copy_elements", { ids, delta, count }),
  rotateElements: (ids: ElementId[], center: Pt, angle: number, copy: boolean): S =>
    invoke("rotate_elements", { ids, center, angle, copy }),
  mirrorElements: (ids: ElementId[], a: Pt, b: Pt, copy: boolean): S =>
    invoke("mirror_elements", { ids, a, b, copy }),
  trimExtend: (a: ElementId, aPick: Pt, b: ElementId, bPick: Pt): S =>
    invoke("trim_extend", { a, aPick, b, bPick }),
  offsetPreview: (view: ElementId, point: Pt, tol: number, distance: string) =>
    invoke<OffsetPreview | null>("offset_preview", { view, point, tol, distance }),
  offsetElement: (view: ElementId, point: Pt, tol: number, distance: string): S =>
    invoke("offset_element", { view, point, tol, distance }),
  splitWall: (id: ElementId, at: Pt): S => invoke("split_wall", { id, at }),
  flipSelection: (ids: ElementId[]): S => invoke("flip_selection", { ids }),
  refLine: (view: ElementId, point: Pt, tol: number, skip: ElementId | null) =>
    invoke<RefLine | null>("ref_line", { view, point, tol, skip }),
  align: (view: ElementId, reference: Pt, target: Pt, tol: number): S =>
    invoke("align", { view, reference, target, tol }),
  // Roofs, stairs and project parameters (ADR-018).
  createRoof: (view: ElementId, typeId: ElementId | null): S =>
    invoke("create_roof", { view, typeId }),
  createStair: (view: ElementId, start: Pt, toward: Pt, shape: string | null = null): S =>
    invoke("create_stair", { view, start, toward, shape }),
  // Structure, railings and wall options (ADR-019).
  createColumn: (view: ElementId, typeId: ElementId | null, at: Pt): S =>
    invoke("create_column", { view, typeId, at, rotation: null }),
  columnsAtGrids: (view: ElementId, typeId: ElementId | null): S =>
    invoke("columns_at_grids", { view, typeId }),
  createBeam: (view: ElementId, typeId: ElementId | null, start: Pt, end: Pt): S =>
    invoke("create_beam", { view, typeId, start, end }),
  createRailing: (view: ElementId, typeId: ElementId | null, path: Pt[]): S =>
    invoke("create_railing", { view, typeId, path }),
  attachWallTops: (ids: ElementId[], attach: boolean): S =>
    invoke("attach_wall_tops", { ids, attach }),
  createWallLocated: (
    view: ElementId,
    typeId: ElementId,
    start: Pt,
    end: Pt,
    location: string,
  ): S => invoke("create_wall_located", { view, typeId, start, end, location }),
  // Room separators, callouts, the section box and materials (ADR-020).
  createRoomSeparator: (view: ElementId, start: Pt, end: Pt): S =>
    invoke("create_room_separator", { view, start, end }),
  createCallout: (view: ElementId, a: Pt, b: Pt): S => invoke("create_callout", { view, a, b }),
  setSectionBox: (view: ElementId, min: number[], max: number[]): S =>
    invoke("set_section_box", { view, min, max }),
  createMaterial: (from: ElementId | null): S => invoke("create_material", { from }),
  // Boundary sketch mode (ADR-021).
  sketchBegin: (
    view: ElementId,
    kind: SketchKind,
    target: ElementId | null,
    typeId: ElementId | null,
    level: ElementId | null = null,
  ): S => invoke("sketch_begin", { view, kind, target, typeId, level }),
  sketchDraw: (tool: DrawTool, pts: Pt[], options: DrawOptions): S =>
    invoke("sketch_draw", { tool, pts, options }),
  sketchPickWalls: (cursor: Pt, tol: number, chain: boolean, core: boolean, offset: number): S =>
    invoke("sketch_pick_walls", { cursor, tol, chain, core, offset }),
  sketchPickLine: (cursor: Pt, tol: number, offset: number, lockToWall: boolean): S =>
    invoke("sketch_pick_line", { cursor, tol, offset, lockToWall }),
  sketchHit: (p: Pt, tol: number) => invoke<number | null>("sketch_hit", { p, tol }),
  sketchFillet: (a: number, b: number, radius: number): S =>
    invoke("sketch_fillet", { a, b, radius }),
  sketchTrim: (a: number, aPick: Pt, b: number, bPick: Pt): S =>
    invoke("sketch_trim", { a, aPick, b, bPick }),
  sketchDelete: (indices: number[]): S => invoke("sketch_delete", { indices }),
  sketchMoveVertex: (from: Pt, to: Pt): S => invoke("sketch_move_vertex", { from, to }),
  sketchFlip: (indices: number[]): S => invoke("sketch_flip", { indices }),
  sketchUndo: (redo: boolean): S => invoke("sketch_undo", { redo }),
  sketchSetType: (typeId: ElementId): S => invoke("sketch_set_type", { typeId }),
  sketchFinish: (): S => invoke("sketch_finish"),
  sketchCancel: (): S => invoke("sketch_cancel"),
  sketchPreview: (
    mode: string,
    pts: Pt[],
    cursor: Pt,
    options: DrawOptions,
    tol: number,
    chain: boolean,
    core: boolean,
  ) => invoke<Pt[][]>("sketch_preview", { mode, pts, cursor, options, tol, chain, core }),
  createElevationMarker: (
    view: ElementId,
    at: Pt,
    interior: boolean,
    typeId: ElementId | null,
  ): S => invoke("create_elevation_marker", { view, at, interior, typeId }),
  // Everyday commands (ADR-024).
  setPinned: (ids: ElementId[], pinned: boolean): S => invoke("set_pinned", { ids, pinned }),
  selectAllInstances: (id: ElementId) => invoke<ElementId[]>("select_all_instances", { id }),
  tagElement: (view: ElementId, target: ElementId): S => invoke("tag_element", { view, target }),
  hideElements: (view: ElementId, ids: ElementId[]): S => invoke("hide_elements", { view, ids }),
  setCategoryVisible: (view: ElementId, categories: Category[], visible: boolean): S =>
    invoke("set_category_visible", { view, categories, visible }),
  unhideAll: (view: ElementId): S => invoke("unhide_all", { view }),
  viewCategories: (view: ElementId) => invoke<[ElementId, Category][]>("view_categories", { view }),
  // Site (ADR-023).
  siteKeys: () => invoke<SiteKeys>("site_keys"),
  siteSetKeys: (google: string | null, regrid: string | null) =>
    invoke<SiteKeys>("site_set_keys", { google, regrid }),
  siteParcel: (lat: number, lon: number) => invoke<ParcelHit>("site_parcel", { lat, lon }),
  siteSetLot: (
    ring: number[][],
    apn: string,
    owner: string,
    address: string,
    acres: number,
    source: string,
  ): S => invoke("site_set_lot", { ring, apn, owner, address, acres, source }),
  /** Topography `spacing` apart over the lot or `extent` (2–4) times around it (ADR-026). */
  siteFetchTopo: (spacing: number, margin: number, extent = 1): S =>
    invoke("site_fetch_topo", { spacing, margin, extent }),
  siteImageryFrame: () => invoke<ImageryFrame>("site_imagery_frame"),
  // Cameras and renderings (ADR-027).
  createCamera: (view: ElementId, eye: Pt, target: Pt, height: number): S =>
    invoke("create_camera", { view, eye, target, height }),
  setCameraPose: (view: ElementId, pose: CameraPose): S =>
    invoke("set_camera_pose", { view, pose }),
  sunPosition: (month: number, day: number, hour: number) =>
    invoke<SunPosition>("sun_position", { month, day, hour }),
  /** Writes an image file; the bytes go as the raw request body. */
  saveRender: (path: string, bytes: Uint8Array) =>
    invoke<void>("save_render", bytes, { headers: { path: encodeURIComponent(path) } }),
  saveImageDialog: (name: string) =>
    save({ defaultPath: name, filters: [{ name: "PNG image", extensions: ["png"] }] }),
  /** The satellite image's bytes (JPEG). */
  siteImagery: (frame: ImageryFrame) => invoke<ArrayBuffer>("site_imagery", { frame }),
  openingPreview3d: (typeId: ElementId, host: ElementId, p: Pt) =>
    invoke<OpeningPreview3d | null>("opening_preview_3d", { typeId, host, p }),
  /** Location lines and stair shapes as [id, label] pairs. */
  drawingOptions: () => invoke<[[string, string][], [string, string][]]>("drawing_options"),
  addProjectParameter: (
    label: string,
    kind: string,
    typeScope: boolean,
    categories: Category[],
  ): S => invoke("add_project_parameter", { label, kind, typeScope, categories }),
  removeProjectParameter: (key: string): S => invoke("remove_project_parameter", { key }),

  /** Native menu clicks, forwarded by Rust as the menu item id. */
  onMenu: (handler: (id: string) => void): Promise<UnlistenFn> =>
    listen<string>("menu", (event) => handler(event.payload)),
  /** Get Topography's progress: USGS batches done, of how many. */
  onTopoProgress: (handler: (done: number, total: number) => void): Promise<UnlistenFn> =>
    listen<[number, number]>("topo-progress", (event) => handler(...event.payload)),
};

export const dialogs = {
  /** Resolves to the chosen path, or null if cancelled. */
  pickProjectToOpen: async (): Promise<string | null> => {
    const picked = await open({ multiple: false, directory: false, filters: PROJECT_FILTER });
    return typeof picked === "string" ? picked : null;
  },
  pickProjectSaveLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.rfproj`, filters: PROJECT_FILTER }),
  pickPdfLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] }),
  pickIfcLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.ifc`, filters: [{ name: "IFC", extensions: ["ifc"] }] }),
};

/** Turns anything a command rejects with into a user-facing message. */
export function errorMessage(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as CommandError).message);
  }
  return String(err);
}
