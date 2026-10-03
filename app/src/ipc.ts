import { useAppStore } from "./store";
// The only module that talks to Rust. Payload types are generated from Rust by ts-rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppState } from "./bindings/AppState";
import type { CategoryChoice } from "./bindings/CategoryChoice";
import type { SheetCopy } from "./bindings/SheetCopy";
import type { RefShape } from "./bindings/RefShape";
import type { RefTarget } from "./bindings/RefTarget";
import type { WorksetInfo } from "./bindings/WorksetInfo";
import type { Keynote } from "./bindings/Keynote";
import type { Discipline } from "./bindings/Discipline";
import type { MepClimate } from "./bindings/MepClimate";
import type { MepProposal } from "./bindings/MepProposal";
import type { MepSettings } from "./bindings/MepSettings";
import type { KeynoteNumbering } from "./bindings/KeynoteNumbering";
import type { KeynoteSource } from "./bindings/KeynoteSource";
import type { KeynoteStyle } from "./bindings/KeynoteStyle";
import type { KeynoteTableInfo } from "./bindings/KeynoteTableInfo";
import type { KeynoteTarget } from "./bindings/KeynoteTarget";
import type { Assignable } from "./bindings/Assignable";
import type { Seismic } from "./bindings/Seismic";
import type { StructuralProposal } from "./bindings/StructuralProposal";
import type { SchemeSettings } from "./bindings/SchemeSettings";
import type { StructuralLayer } from "./bindings/StructuralLayer";
import type { OverlayPrim } from "./bindings/OverlayPrim";
import type { OverlayMesh } from "./bindings/OverlayMesh";
import type { OverlayInfo } from "./bindings/OverlayInfo";
import type { ComponentTypeInfo } from "./bindings/ComponentTypeInfo";
import type { Leader } from "./bindings/Leader";
import type { TextAlign } from "./bindings/TextAlign";
import type { TextNoteInfo } from "./bindings/TextNoteInfo";
import type { DetailInfo } from "./bindings/DetailInfo";
import type { FillPattern } from "./bindings/FillPattern";
import type { FormKind } from "./bindings/FormKind";
import type { Category } from "./bindings/Category";
import type { Handles } from "./bindings/Handles";
import type { ImageryFrame } from "./bindings/ImageryFrame";
import type { CameraPose } from "./bindings/CameraPose";
import type { Preset } from "./bindings/Preset";
import type { BuildingTypeOption } from "./bindings/BuildingTypeOption";
import type { SetOptions } from "./bindings/SetOptions";
import type { SetPlan } from "./bindings/SetPlan";
import type { SetsCreated } from "./bindings/SetsCreated";
import type { SetsExported } from "./bindings/SetsExported";
import type { LoadedWindows } from "./bindings/LoadedWindows";
import type { IfcImported } from "./bindings/IfcImported";
import type { SpecPreview } from "./bindings/SpecPreview";
import type { WindowLibrary } from "./bindings/WindowLibrary";
import type { WindowSpec } from "./bindings/WindowSpec";
import type { DoorLibrary } from "./bindings/DoorLibrary";
import type { DoorSpec } from "./bindings/DoorSpec";
import type { DoorSpecPreview } from "./bindings/DoorSpecPreview";
import type { OpeningThumb } from "./bindings/OpeningThumb";
import type { ThumbSource } from "./bindings/ThumbSource";
import type { GenerateInputs } from "./bindings/GenerateInputs";
import type { GenerateProgress } from "./bindings/GenerateProgress";
import type { Precedent } from "./bindings/Precedent";
import type { SkyPreset } from "./bindings/SkyPreset";
import type { GenerateResult } from "./bindings/GenerateResult";
import type { PlansInputs } from "./bindings/PlansInputs";
import type { Cap } from "./bindings/Cap";
import type { Terrain } from "./bindings/Terrain";
import type { ProjectDetails } from "./bindings/ProjectDetails";
import type { ProjectInfoState } from "./bindings/ProjectInfoState";
import type { QaFixAction } from "./bindings/QaFixAction";
import type { QaFixPlan } from "./bindings/QaFixPlan";
import type { QaSuggestion } from "./bindings/QaSuggestion";
import type { QaOptions } from "./bindings/QaOptions";
import type { QaReport } from "./bindings/QaReport";
import type { SpecEdit } from "./bindings/SpecEdit";
import type { SpecEditPlan } from "./bindings/SpecEditPlan";
import type { SpecSection } from "./bindings/SpecSection";
import type { SpecState } from "./bindings/SpecState";
import type { SpecUpdate } from "./bindings/SpecUpdate";
import type { Standards } from "./bindings/Standards";
import type { StandardChoice } from "./bindings/StandardChoice";
import type { LineStyle } from "./bindings/LineStyle";
import type { EditPlan } from "./bindings/EditPlan";
import type { ModelPlan } from "./bindings/ModelPlan";
import type { Reference } from "./bindings/Reference";
import type { Prefer } from "./bindings/Prefer";
import type { DimKind } from "./bindings/DimKind";
import type { ViewportInfo } from "./bindings/ViewportInfo";
import type { PlansProgress } from "./bindings/PlansProgress";
import type { PlansResult } from "./bindings/PlansResult";
import type { RenderMaterial } from "./bindings/RenderMaterial";
import type { TextureMap } from "./bindings/TextureMap";
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
import type { OpeningFlip } from "./bindings/OpeningFlip";
import type { FasciaSpec } from "./bindings/FasciaSpec";
import type { RoomPreview } from "./bindings/RoomPreview";
import type { Table } from "./bindings/Table";
import type { DimensionPreview } from "./bindings/DimensionPreview";
import type { PropertySheet } from "./bindings/PropertySheet";
import type { PickCandidate } from "./bindings/PickCandidate";
import type { LightLibrary } from "./bindings/LightLibrary";
import type { FixtureSource } from "./bindings/FixtureSource";
import type { FixtureThumb } from "./bindings/FixtureThumb";
import type { FfeClass } from "./bindings/FfeClass";
import type { FfeLibrary } from "./bindings/FfeLibrary";
import type { FfeSource } from "./bindings/FfeSource";
import type { FfeThumb } from "./bindings/FfeThumb";
import type { LightInfo } from "./bindings/LightInfo";
import type { PlantLibrary } from "./bindings/PlantLibrary";
import type { PlantSource } from "./bindings/PlantSource";
import type { PlantModel } from "./bindings/PlantModel";
import type { PlantMap } from "./bindings/PlantMap";
import type { PlantInstance } from "./bindings/PlantInstance";
import type { PlantAt } from "./bindings/PlantAt";
import type { GroundChoice } from "./bindings/GroundChoice";
import type { GrassKindInfo } from "./bindings/GrassKindInfo";
import type { GrassPatchInfo } from "./bindings/GrassPatchInfo";
import type { GrassSpec } from "./bindings/GrassSpec";
import type { SunSettings } from "./bindings/SunSettings";
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

// Projects save as .ruf (ADR-092); Open also takes the earlier .rfproj.
const PROJECT_FILTER = [{ name: "Rufplan Studio project", extensions: ["ruf"] }];
const OPEN_FILTER = [{ name: "Rufplan Studio project", extensions: ["ruf", "rfproj"] }];

type S = Promise<AppState | null>;

/** An unattended render (ADR-095): which camera view, the settings, and where to save. */
export interface AutoRender {
  out: string;
  view: string;
  width?: number;
  height?: number;
  samples?: number;
  month?: number;
  day?: number;
  hour?: number;
  exposure?: number;
  background?: string;
  /** "sunsky" or "dome" (light by the background photo's HDR), and its rotation. */
  lighting?: "sunsky" | "dome";
  rotation?: number;
  /** A camera instead of the view's (model mm, z up), for close-ups. */
  eye?: [number, number, number];
  target?: [number, number, number];
  fov?: number;
  /** A warm finishing grade (0 none, 0.08 typical). */
  warm?: number;
  /** The most artificial lights to render (the nearest the camera's target). */
  maxLights?: number;
  /** The physical sky's haze (2 clear … 6 hazy) and cloud cover (0–0.9). */
  turbidity?: number;
  clouds?: number;
  /** The backdrop photo's brightness relative to the scene (default 1). */
  skyExposure?: number;
  /** Ceilings' glow in exteriors, so interiors read lit (default 1, ADR-101). */
  interiorGlow?: number;
  /** A Sky Library sky's sun against its sky (default 1, ADR-102). */
  sunScale?: number;
  /** A Sky Library sky's sun turned to the site's sun (default on, ADR-101). */
  matchSun?: boolean;
  /** Sunlit to skylit light (default 6): lower lifts the shadows. */
  sunToSky?: number;
  /** Smooth the remaining noise at the end (default on). */
  denoise?: boolean;
  /** "guided" (albedo/normal/depth à-trous, ADR-102) or "blur" (the old bilateral). */
  denoiser?: "guided" | "blur";
  denoiseStrength?: number;
  /** Aerial haze: metres to fade 63% toward the horizon (default 2500; 0 off, ADR-102). */
  haze?: number;
  /** A Lighting Scheme name ("Exterior: Sun and Artificial"…). */
  scheme?: string;
  /** Grass clumps in the render (default 120,000). */
  grass?: number;
  tone?: "contrast" | "filmic" | "neutral";
  /** Exposure in stops (ADR-102), in place of the `exposure` multiplier. */
  ev?: number;
  glare?: boolean;
  vignette?: boolean;
  d5?: boolean;
  quit?: boolean;
}

export const ipc = {
  coreVersion: () => invoke<CoreVersion>("core_version"),
  appState: (): S => invoke("app_state"),
  projectNew: (): S => invoke("project_new"),
  /** A new unsaved project with a sample two-storey house. */
  projectSample: (kind: "modern" | "basic" = "modern"): S => invoke("project_sample", { kind }),
  /** Development aid (ADR-095): the --autorender settings the app was started with. */
  autoRender: () => invoke<AutoRender | null>("auto_render"),
  quitApp: () => invoke<void>("quit_app"),
  /** Save to Project (ADR-095): a rendered image kept as a Rendering view. */
  saveRendering: (name: string, mime: string, data: string, width: number, height: number): S =>
    invoke("save_rendering", { name, mime, data, width, height }),
  /** A saved rendering's image as a data URL. */
  renderImage: (id: ElementId) => invoke<string>("render_image", { id }),
  projectOpen: (path: string): S => invoke("project_open", { path }),
  /** Omit `path` to save to the project's current location. */
  projectSave: (path?: string): S => invoke("project_save", { path: path ?? null }),

  displayList: (view: ElementId) => invoke<DisplayList | null>("view_display_list", { view }),
  meshes: (view: ElementId | null = null) => invoke<Mesh[]>("view_meshes", { view }),
  /** The section box's cuts through the model (ADR-044). */
  sectionCaps: (view: ElementId) => invoke<Cap[]>("section_caps", { view }),
  /** The site's terrain for 3D (ADR-045). */
  siteTerrain: () => invoke<Terrain | null>("site_terrain"),
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
  /** Dimension references under the cursor, best first (ADR-040). */
  dimensionReferences: (
    view: ElementId,
    cursor: Pt,
    tol: number,
    prefer: Prefer,
    snapped: Pt | null,
  ) => invoke<Reference[]>("dimension_references", { view, cursor, tol, prefer, snapped }),
  dimensionStringPreview: (view: ElementId, refs: Reference[], cursor: Pt, kind: DimKind) =>
    invoke<DimensionPreview | null>("dimension_string_preview", { view, refs, cursor, kind }),
  createDimensionString: (view: ElementId, refs: Reference[], cursor: Pt, kind: DimKind): S =>
    invoke("create_dimension_string", { view, refs, cursor, kind }),
  angularPreview: (view: ElementId, first: Reference, second: Reference, cursor: Pt) =>
    invoke<DimensionPreview | null>("angular_preview", { view, first, second, cursor }),
  createAngularDimension: (view: ElementId, first: Reference, second: Reference, cursor: Pt): S =>
    invoke("create_angular_dimension", { view, first, second, cursor }),
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
  openingPreview: (view: ElementId, typeId: ElementId, point: Pt, tol: number, turns = 0) =>
    invoke<OpeningPreview | null>("opening_preview", { view, typeId, point, tol, turns }),
  createOpening: (
    typeId: ElementId,
    host: ElementId,
    offset: number,
    flipFacing: boolean,
    flipHand = false,
  ): S => invoke("create_opening", { typeId, host, offset, flipFacing, flipHand }),
  /** Fascia (ADR-095): the 20 profiles, and giving roofs one (every roof when ids is null). */
  fasciaCatalog: () => invoke<FasciaSpec[]>("fascia_catalog"),
  setFascia: (ids: ElementId[] | null, name: string | null): S =>
    invoke("set_fascia", { ids, name }),
  /** Revit's flip controls: a door's hand (left/right) or facing (up/down). */
  flipOpening: (id: ElementId, flip: OpeningFlip): S => invoke("flip_opening", { id, flip }),
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
  /** A viewport's sheet, view and center, to activate it (ADR-039). */
  viewportInfo: (id: ElementId) => invoke<ViewportInfo | null>("viewport_info", { id }),
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
  /** Align by picked references (ADR-042). */
  alignReferences: (view: ElementId, reference: Reference, target: Reference): S =>
    invoke("align_references", { view, reference, target }),
  /** Align in 3D (ADR-097): moves `target` by `delta` (x, y, z mm). */
  align3d: (target: ElementId, delta: number[]): S => invoke("align_3d", { target, delta }),
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
  // MEPT (ADR-082).
  mepSuggest: (discipline: Discipline, climate: MepClimate) =>
    invoke<MepProposal>("mep_suggest", { discipline, climate }),
  mepGenerate: (settings: MepSettings): S => invoke("mep_generate", { settings }),
  mepOverlay2d: (view: ElementId, disciplines: Discipline[]) =>
    invoke<OverlayPrim[]>("mep_overlay_2d", { view, disciplines }),
  mepOverlay3d: (disciplines: Discipline[]) =>
    invoke<OverlayMesh[]>("mep_overlay_3d", { disciplines }),
  mepPick: (view: ElementId, at: Pt, tol: number, disciplines: Discipline[]) =>
    invoke<OverlayInfo | null>("mep_pick", { view, at, tol, disciplines }),
  mepInfo: (item: number | null, flag: number | null) =>
    invoke<OverlayInfo | null>("mep_info", { item, flag }),
  mepExportJson: (discipline: Discipline, path: string) =>
    invoke<string>("mep_export_json", { discipline, path }),
  mepEditRules: () => invoke<string>("mep_edit_rules"),
  // Keynotes (ADR-081).
  keynoteTable: () => invoke<KeynoteTableInfo>("keynote_table"),
  keynoteSave: (oldKey: string | null, entry: Keynote): S =>
    invoke("keynote_save", { oldKey, entry }),
  keynoteDelete: (key: string): S => invoke("keynote_delete", { key }),
  keynoteSetNumbering: (numbering: KeynoteNumbering): S =>
    invoke("keynote_set_numbering", { numbering }),
  keynoteImport: (path: string, replace: boolean) =>
    invoke<[number, AppState | null]>("keynote_import", { path, replace }),
  keynoteExport: (path: string) => invoke<string>("keynote_export", { path }),
  keynoteAssign: (ids: ElementId[], key: string | null): S =>
    invoke("keynote_assign", { ids, key }),
  keynoteAssignables: () => invoke<Assignable[]>("keynote_assignables"),
  keynoteTarget: (id: ElementId) => invoke<KeynoteTarget | null>("keynote_target", { id }),
  keynotePlace: (
    view: ElementId,
    source: KeynoteSource,
    arrow: Pt | null,
    at: Pt,
    style: KeynoteStyle,
  ): S => invoke("keynote_place", { view, source, arrow, at, style }),
  keynoteLegend: () => invoke<[ElementId, AppState | null]>("keynote_legend"),
  // Suggest Structure and the structural overlay (ADR-080).
  structuralSuggest: (seismic: Seismic) =>
    invoke<StructuralProposal>("structural_suggest", { seismic }),
  structuralGenerate: (settings: SchemeSettings): S => invoke("structural_generate", { settings }),
  structuralLayer: () => invoke<StructuralLayer | null>("structural_layer"),
  structuralOverlay2d: (view: ElementId) =>
    invoke<OverlayPrim[]>("structural_overlay_2d", { view }),
  structuralOverlay3d: () => invoke<OverlayMesh[]>("structural_overlay_3d"),
  structuralPick: (view: ElementId, at: Pt, tol: number) =>
    invoke<OverlayInfo | null>("structural_pick", { view, at, tol }),
  structuralInfo: (member: number | null, flag: number | null) =>
    invoke<OverlayInfo | null>("structural_info", { member, flag }),
  structuralExportJson: (path: string) => invoke<string>("structural_export_json", { path }),
  structuralExportIfc: (path: string) => invoke<string>("structural_export_ifc", { path }),
  structuralEditRules: () => invoke<string>("structural_edit_rules"),
  // Worksets (ADR-079).
  worksetsList: () => invoke<WorksetInfo[]>("worksets_list"),
  setActiveWorkset: (ws: ElementId): S => invoke("set_active_workset", { ws }),
  createWorkset: (name: string, visible: boolean): S => invoke("create_workset", { name, visible }),
  renameWorkset: (ws: ElementId, name: string): S => invoke("rename_workset", { ws, name }),
  deleteWorkset: (ws: ElementId, moveTo: ElementId): S => invoke("delete_workset", { ws, moveTo }),
  setWorksetVisibleInAllViews: (ws: ElementId, visible: boolean): S =>
    invoke("set_workset_visible_in_all_views", { ws, visible }),
  setWorksetVisibleInView: (view: ElementId, ws: ElementId, visible: boolean): S =>
    invoke("set_workset_visible_in_view", { view, ws, visible }),
  setElementsWorkset: (ids: ElementId[], ws: ElementId): S =>
    invoke("set_elements_workset", { ids, ws }),
  elementWorksets: () => invoke<[ElementId, ElementId][]>("element_worksets"),
  // Reference sections and callouts (ADR-076).
  referenceTargets: (view: ElementId, callout: boolean) =>
    invoke<RefTarget[]>("reference_targets", { view, callout }),
  createReference: (view: ElementId, shape: RefShape, target: ElementId | null): S =>
    invoke("create_reference", { view, shape, target }),
  referenceTarget: (id: ElementId) => invoke<ElementId | null>("reference_target", { id }),
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
    host: ElementId | null = null,
    toward: Pt | null = null,
  ): S => invoke("sketch_begin", { view, kind, target, typeId, level, host, toward }),
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
  /** The options bar's settings for the in-place form being sketched (ADR-068). */
  sketchSetForm: (kind: FormKind): S => invoke("sketch_set_form", { kind }),
  // Model In-Place (ADR-068).
  // Detail components (ADR-071).
  detailComponentTypes: () => invoke<ComponentTypeInfo[]>("detail_component_types"),
  detailComponentPreview: (key: string, start: Pt, end: Pt, flip: boolean) =>
    invoke<Pt[][]>("detail_component_preview", { key, start, end, flip }),
  createDetailComponent: (view: ElementId, key: string, start: Pt, end: Pt, flip: boolean): S =>
    invoke("create_detail_component", { view, key, start, end, flip }),
  // The Details tab (ADR-069).
  detailLibrary: () => invoke<DetailInfo[]>("detail_library"),
  detailPreview: (id: string) => invoke<DisplayList>("detail_preview", { id }),
  detailInsert: (id: string) => invoke<[ElementId, AppState | null]>("detail_insert", { id }),
  createDraftingView: (name: string, scale: number) =>
    invoke<[ElementId, AppState | null]>("create_drafting_view", { name, scale }),
  sketchSetPattern: (pattern: FillPattern): S => invoke("sketch_set_pattern", { pattern }),
  // Views and sheets (ADR-074).
  createPlanView: (level: ElementId, ceiling: boolean) =>
    invoke<[ElementId, AppState | null]>("create_plan_view", { level, ceiling }),
  create3dView: () => invoke<[ElementId, AppState | null]>("create_3d_view"),
  duplicateView: (view: ElementId, detailing: boolean) =>
    invoke<[ElementId, AppState | null]>("duplicate_view", { view, detailing }),
  duplicateSheet: (sheet: ElementId, how: SheetCopy) =>
    invoke<[ElementId, AppState | null]>("duplicate_sheet", { sheet, how }),
  // Your details (ADR-073).
  detailSave: (view: ElementId, name: string, category: string, description: string) =>
    invoke<DetailInfo>("detail_save", { view, name, category, description }),
  detailDelete: (id: string) => invoke<void>("detail_delete", { id }),
  // Revit's Text (ADR-070).
  createTextNote: (
    view: ElementId,
    at: Pt,
    text: string,
    size: number,
    leaders: Leader[],
    align: TextAlign,
    width: number | null,
  ): S => invoke("create_text_note", { view, at, text, size, leaders, align, width }),
  textNoteInfo: (id: ElementId) => invoke<TextNoteInfo>("text_note_info", { id }),
  addTextLeader: (ids: ElementId[], left: boolean): S => invoke("add_text_leader", { ids, left }),
  removeTextLeader: (ids: ElementId[]): S => invoke("remove_text_leader", { ids }),
  inPlaceCategories: () => invoke<CategoryChoice[]>("in_place_categories"),
  inPlaceDefaultName: (category: Category) => invoke<string>("in_place_default_name", { category }),
  inPlaceOf: (ids: ElementId[]) => invoke<ElementId[]>("in_place_of", { ids }),
  inPlaceBegin: (
    view: ElementId,
    category: Category,
    name: string | null,
    level: ElementId | null,
  ): S => invoke("in_place_begin", { view, category, name, level }),
  inPlaceEdit: (id: ElementId): S => invoke("in_place_edit", { id }),
  inPlaceFinish: (): S => invoke("in_place_finish"),
  inPlaceCancel: (): S => invoke("in_place_cancel"),
  inPlaceFormBegin: (view: ElementId, kind: string, index: number | null): S =>
    invoke("in_place_form_begin", { view, kind, index }),
  inPlaceDeleteForm: (index: number): S => invoke("in_place_delete_form", { index }),
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
  tagRoomInView: (view: ElementId, at: Pt): S => invoke("tag_room_in_view", { view, at }),
  tagElements: (view: ElementId, targets: ElementId[]): S =>
    invoke("tag_elements", { view, targets }),
  pickCycle: (view: ElementId, point: Pt, tol: number) =>
    invoke<PickCandidate[]>("pick_cycle", { view, point, tol }),
  gripSnap: (id: ElementId, key: string, point: Pt, tol: number) =>
    invoke<SnapResult | null>("grip_snap", { id, key, point, tol }),
  pickCandidates: (ids: ElementId[]) => invoke<PickCandidate[]>("pick_candidates", { ids }),
  /** Revit's box selection: a window (all inside) or a crossing (touching). */
  pickInRect: (view: ElementId, a: Pt, b: Pt, crossing: boolean) =>
    invoke<ElementId[]>("pick_in_rect", { view, a, b, crossing }),
  elementCategories: (ids: ElementId[]) =>
    invoke<[ElementId, Category][]>("element_categories", { ids }),
  // Furniture and equipment (ADR-090).
  ffeLibrary: (cls: FfeClass) => invoke<FfeLibrary>("ffe_library", { class: cls }),
  loadFfeTypes: (names: string[]) => invoke<LoadedWindows>("load_ffe_types", { names }),
  ffeThumbnail: (source: FfeSource) => invoke<FfeThumb>("ffe_thumbnail", { source }),
  createFfe: (view: ElementId, typeId: ElementId, at: Pt, rotation: number | null = null): S =>
    invoke("create_ffe", { view, typeId, at, rotation }),
  // Lighting (ADR-057).
  lightingLibrary: () => invoke<LightLibrary>("lighting_library"),
  loadLightingTypes: (names: string[]) => invoke<LoadedWindows>("load_lighting_types", { names }),
  fixtureThumbnail: (source: FixtureSource) =>
    invoke<FixtureThumb>("fixture_thumbnail", { source }),
  createLightingFixture: (
    view: ElementId,
    typeId: ElementId | null,
    at: Pt,
    level: ElementId | null = null,
    elevation: number | null = null,
  ): S => invoke("create_lighting_fixture", { view, typeId, at, rotation: null, level, elevation }),
  setLights: (ids: ElementId[], on: boolean | null, dimming: number | null): S =>
    invoke("set_lights", { ids, on, dimming }),
  lights: (view: ElementId | null) => invoke<LightInfo[]>("lights", { view }),
  setSunSettings: (settings: SunSettings): S => invoke("set_sun_settings", { settings }),
  sunNow: () => invoke<SunPosition>("sun_now"),
  sunFor: (settings: SunSettings) => invoke<SunPosition>("sun_for", { settings }),
  // Vegetation (ADR-064).
  plantingLibrary: () => invoke<PlantLibrary>("planting_library"),
  loadPlantingTypes: (names: string[]) => invoke<LoadedWindows>("load_planting_types", { names }),
  plantModel: (source: PlantSource, variant: number) =>
    invoke<PlantModel>("plant_model", { source, variant }),
  /** A plant's foliage atlas or bark (PNG), made once and cached by Rust. */
  plantTexture: (source: PlantSource, map: PlantMap, size: number | null = null) =>
    invoke<ArrayBuffer>("plant_texture", { source, map, size }),
  plantInstances: (view: ElementId | null) => invoke<PlantInstance[]>("plant_instances", { view }),
  createPlants: (
    view: ElementId,
    typeId: ElementId,
    at: PlantAt[],
    level: ElementId | null = null,
  ): S => invoke("create_plants", { view, typeId, at, level }),
  setBaseGround: (material: ElementId | null, preset: string | null = null): S =>
    invoke("set_base_ground", { material, preset }),
  groundLibrary: () => invoke<GroundChoice[]>("ground_library"),
  // Grass Brush (ADR-065).
  grassKinds: () => invoke<GrassKindInfo[]>("grass_kinds"),
  paintGrass: (
    level: ElementId | null,
    dabs: [number, number, number, number][],
    spec: GrassSpec,
  ): S => invoke("paint_grass", { level, dabs, spec }),
  eraseGrass: (dabs: [number, number, number, number][]): S => invoke("erase_grass", { dabs }),
  grassPatches: (view: ElementId | null) => invoke<GrassPatchInfo[]>("grass_patches", { view }),
  selectionCategories: (ids: ElementId[]) => invoke<Category[]>("selection_categories", { ids }),
  hideElements: (view: ElementId, ids: ElementId[]): S => invoke("hide_elements", { view, ids }),
  setCategoryVisible: (view: ElementId, categories: Category[], visible: boolean): S =>
    invoke("set_category_visible", { view, categories, visible }),
  unhideAll: (view: ElementId): S => invoke("unhide_all", { view }),
  viewCategories: (view: ElementId) => invoke<[ElementId, Category][]>("view_categories", { view }),
  // Detail and model lines (ADR-054).
  createLines: (
    view: ElementId,
    model: boolean,
    tool: DrawTool,
    pts: Pt[],
    options: DrawOptions,
    style: LineStyle,
  ): S => invoke("create_lines", { view, model, tool, pts, options, style }),
  linesPreview: (tool: DrawTool, pts: Pt[], cursor: Pt, options: DrawOptions) =>
    invoke<Pt[][]>("lines_preview", { tool, pts, cursor, options }),
  // Edit Model with Claude (ADR-050).
  modelEditPreview: (prompt: string, view: ElementId, selection: ElementId[]) =>
    invoke<EditPlan>("model_edit_preview", { prompt, view, selection }),
  modelEditApply: (editPlan: ModelPlan, view: ElementId, selection: ElementId[]): S =>
    invoke("model_edit_apply", { editPlan, view, selection }),
  // QA/QC (ADR-088).
  qaReview: (options: QaOptions) => invoke<QaReport>("qa_review", { options }),
  qaClaude: (report: QaReport) => invoke<QaReport>("qa_claude", { report }),
  qaExportPdf: (path: string, report: QaReport, resolved: string[]) =>
    invoke<string>("qa_export_pdf", { path, report, resolved }),
  // Fix Issues (ADR-089).
  qaFixPlan: (report: QaReport) => invoke<QaFixPlan>("qa_fix_plan", { report }),
  /** Fix with Claude (ADR-094): advice and checked fixes for findings Fix Issues can't plan. */
  qaFixClaude: (report: QaReport, ids: string[]) =>
    invoke<QaSuggestion[]>("qa_fix_claude", { report, ids }),
  qaFixApply: (actions: QaFixAction[], label: string) =>
    invoke<[number, string[], AppState | null]>("qa_fix_apply", { actions, label }),
  // Model and detail groups (ADR-087).
  groupCreate: (ids: ElementId[], name: string) =>
    invoke<[ElementId[], AppState | null]>("group_create", { ids, name }),
  groupUngroup: (ids: ElementId[]): S => invoke("group_ungroup", { ids }),
  groupPlace: (typeId: ElementId, at: { x: number; y: number }, view: ElementId): S =>
    invoke("group_place", { typeId, at, view }),
  groupEdit: (id: ElementId): S => invoke("group_edit", { id }),
  groupAdd: (ids: ElementId[]): S => invoke("group_add", { ids }),
  groupRemove: (ids: ElementId[]): S => invoke("group_remove", { ids }),
  groupFinish: (): S => invoke("group_finish"),
  groupCancel: (): S => invoke("group_cancel"),
  groupDeleteType: (typeId: ElementId): S => invoke("group_delete_type", { typeId }),
  // Specifications (ADR-085).
  specState: () => invoke<SpecState>("spec_state"),
  specGenerate: (styleId: string, issue: string, date: string): S =>
    invoke("spec_generate", { styleId, issue, date }),
  specUpdate: () => invoke<[SpecUpdate, AppState | null]>("spec_update"),
  specSetSection: (number: string, section: SpecSection): S =>
    invoke("spec_set_section", { number, section }),
  specLibrarySection: (number: string) =>
    invoke<SpecSection | null>("spec_library_section", { number }),
  specAddLibrary: (numbers: string[]): S => invoke("spec_add_library", { numbers }),
  specAddCustom: (number: string, title: string): S => invoke("spec_add_custom", { number, title }),
  specRemove: (numbers: string[]): S => invoke("spec_remove", { numbers }),
  specSetIncluded: (numbers: string[], included: boolean): S =>
    invoke("spec_set_included", { numbers, included }),
  specSetSettings: (styleId: string, issue: string, date: string): S =>
    invoke("spec_set_settings", { styleId, issue, date }),
  /** Writes the book as "pdf" or "docx"; returns the path written and the page count. */
  specExport: (path: string, format: "pdf" | "docx") =>
    invoke<[string, number]>("spec_export", { path, format }),
  specEditPreview: (prompt: string, focus: string | null) =>
    invoke<SpecEditPlan>("spec_edit_preview", { prompt, focus }),
  specEditApply: (edit: SpecEdit): S => invoke("spec_edit_apply", { edit }),
  // Project Info (ADR-084).
  projectInfoGet: () => invoke<ProjectInfoState>("project_info_get"),
  projectInfoSet: (name: string, number: string, details: ProjectDetails): S =>
    invoke("project_info_set", { name, number, details }),
  // Standards (ADR-047).
  standardsGet: () => invoke<Standards>("standards_get"),
  standardsLibraries: () => invoke<string[]>("standards_libraries"),
  /** A value (a non-empty one marks the standard defined) and/or its status. */
  standardsSet: (category: string, index: number, value: string | null, done: boolean | null): S =>
    invoke("standards_set", { category, index, value, done }),
  standardsLoadLibrary: (name: string): S => invoke("standards_load_library", { name }),
  /** A standard's choices, with previews of graphic ones (ADR-048). */
  standardsChoices: (category: string, index: number) =>
    invoke<StandardChoice[]>("standards_choices", { category, index }),
  // Symbols (ADR-048).
  createSpotElevation: (view: ElementId, at: Pt, leader: Pt): S =>
    invoke("create_spot_elevation", { view, at, leader }),
  createNorthArrow: (view: ElementId, at: Pt): S => invoke("create_north_arrow", { view, at }),
  /** A spot slope on a sloped roof, floor or the ground (ADR-049). */
  createSpotSlope: (view: ElementId, at: Pt): S => invoke("create_spot_slope", { view, at }),
  createGraphicScale: (view: ElementId, at: Pt): S => invoke("create_graphic_scale", { view, at }),
  createKeyPlan: (sheet: ElementId, at: Pt): S => invoke("create_key_plan", { sheet, at }),
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
  // Generate with Claude (ADR-030).
  /** The Sky Library (ADR-101), and one of a sky's files, downloaded once and cached. */
  skyLibrary: () => invoke<SkyPreset[]>("sky_library"),
  skyFile: (id: string, file: "light" | "photo" | "thumb") =>
    invoke<ArrayBuffer>("sky_file", { id, file }),
  claudeKeySet: () => invoke<boolean>("claude_key_set"),
  /** Architects and works Generate can design after (ADR-099). */
  generatePrecedents: () => invoke<Precedent[]>("generate_precedents"),
  claudeSetKey: (key: string) => invoke<boolean>("claude_set_key", { key }),
  generateBuilding: (inputs: GenerateInputs) =>
    invoke<GenerateResult>("generate_building", { inputs }),
  /** Plans to 3D (ADR-036): Claude reads plan sheets; the model is built from them. */
  plansToModel: (inputs: PlansInputs) => invoke<PlansResult>("plans_to_model", { inputs }),
  /** Opens an IFC file (from Revit…) as a new project (ADR-035). */
  projectImportIfc: (path: string) => invoke<IfcImported>("project_import_ifc", { path }),
  // Sheet sets (ADR-032).
  buildingTypes: () => invoke<BuildingTypeOption[]>("building_types"),
  sheetSetPlan: (options: SetOptions) => invoke<SetPlan>("sheet_set_plan", { options }),
  createSheetSets: (options: SetOptions) => invoke<SetsCreated>("create_sheet_sets", { options }),
  exportSheetSets: (phases: string[], folder: string, record: boolean) =>
    invoke<SetsExported>("export_sheet_sets", { phases, folder, record }),
  // Door Library and type thumbnails (ADR-033).
  doorLibrary: () => invoke<DoorLibrary>("door_library"),
  doorPreview: (spec: DoorSpec) => invoke<DoorSpecPreview>("door_preview", { spec }),
  loadDoorTypes: (specs: DoorSpec[]) => invoke<LoadedWindows>("load_door_types", { specs }),
  openingThumbnail: (source: ThumbSource) => invoke<OpeningThumb>("opening_thumbnail", { source }),
  // Window Library (ADR-031).
  windowLibrary: () => invoke<WindowLibrary>("window_library"),
  windowPreview: (spec: WindowSpec) => invoke<SpecPreview>("window_preview", { spec }),
  loadWindowTypes: (specs: WindowSpec[]) => invoke<LoadedWindows>("load_window_types", { specs }),
  // Material library (ADR-029).
  materialLibrary: () => invoke<Preset[]>("material_library"),
  addLibraryMaterial: (id: string): S => invoke("add_library_material", { id }),
  applyMaterial: (ids: ElementId[], material: ElementId): S =>
    invoke("apply_material", { ids, material }),
  renderMaterials: () => invoke<RenderMaterial[]>("render_materials"),
  /** Paints elements (ADR-034); a null material removes their paint. */
  /** Paints the face of `el` hit in 3D (z-up mm), only that face (ADR-096). */
  paintFace: (el: ElementId, point: number[], normal: number[], material: ElementId | null): S =>
    invoke("paint_face", { el, point, normal, material }),
  /** Paints the face of `el` seen at `point` in a plan, elevation or section (ADR-096). */
  paintInView: (view: ElementId, el: ElementId, point: Pt, material: ElementId | null): S =>
    invoke("paint_in_view", { view, el, point, material }),
  paintElements: (ids: ElementId[], material: ElementId | null): S =>
    invoke("paint_elements", { ids, material }),
  /** A library texture map (JPEG), downloaded once and cached by Rust. */
  materialTexture: (set: string, map: TextureMap) =>
    invoke<ArrayBuffer>("material_texture", { set, map }),
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
  openingPreview3d: (typeId: ElementId, host: ElementId, p: Pt, turns = 0) =>
    invoke<OpeningPreview3d | null>("opening_preview_3d", { typeId, host, p, turns }),
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
  /** Generate with Claude: planning and building progress. */
  onGenerateProgress: (handler: (p: GenerateProgress) => void): Promise<UnlistenFn> =>
    listen<GenerateProgress>("generate-progress", (event) => handler(event.payload)),
  /** Plans to 3D: reading and building progress. */
  onPlansProgress: (handler: (p: PlansProgress) => void): Promise<UnlistenFn> =>
    listen<PlansProgress>("plans-progress", (event) => handler(event.payload)),
  /** Get Topography's progress: USGS batches done, of how many. */
  onTopoProgress: (handler: (done: number, total: number) => void): Promise<UnlistenFn> =>
    listen<[number, number]>("topo-progress", (event) => handler(...event.payload)),
};

export const dialogs = {
  /** Resolves to the chosen path, or null if cancelled. */
  pickProjectToOpen: async (): Promise<string | null> => {
    const picked = await open({ multiple: false, directory: false, filters: OPEN_FILTER });
    return typeof picked === "string" ? picked : null;
  },
  pickProjectSaveLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.ruf`, filters: PROJECT_FILTER }),
  pickPdfLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] }),
  pickWordLocation: (defaultName: string): Promise<string | null> =>
    save({
      defaultPath: `${defaultName}.docx`,
      filters: [{ name: "Word document", extensions: ["docx"] }],
    }),
  /** A folder, for exporting several files. */
  pickFolder: async (): Promise<string | null> => {
    const picked = await open({ multiple: false, directory: true });
    return typeof picked === "string" ? picked : null;
  },
  pickIfcToOpen: async (): Promise<string | null> => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "IFC model", extensions: ["ifc"] }],
    });
    return typeof picked === "string" ? picked : null;
  },
  pickKeynoteFile: async (): Promise<string | null> => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Keynote file", extensions: ["txt"] }],
    });
    return typeof picked === "string" ? picked : null;
  },
  pickKeynoteSaveLocation: (defaultName: string): Promise<string | null> =>
    save({
      defaultPath: `${defaultName}.txt`,
      filters: [{ name: "Keynote file", extensions: ["txt"] }],
    }),
  pickJsonLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.json`, filters: [{ name: "JSON", extensions: ["json"] }] }),
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
