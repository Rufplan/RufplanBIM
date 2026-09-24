# Architecture decision records

Format: context → decision → consequences. Append new ADRs; never rewrite accepted ones
(supersede them instead).

## ADR-001 Desktop shell: Tauri 2 — Accepted
Reuse Rufplan's React skills and components, native Rust backend, small installers.
Electron rejected (JS-only compute, large binaries). Qt kept as fallback if webview/native
integration becomes a blocker (see ADR-004).

## ADR-002 Core engine in Rust — Accepted
Memory safety, performance, good tooling, and clean integration with Tauri. C++ libraries
(OpenCascade, IfcOpenShell) are reached through FFI when needed.

## ADR-003 Project file is SQLite — Accepted
Transactional saves, incremental writes, queryable, and a natural base for a change log
and sync. Elements stored as MessagePack blobs with indexed header columns.

## ADR-004 Rendering: display lists computed in Rust, drawn in the webview — Accepted
Canvas 2D first, WebGL2 when needed; three.js for 3D. Keeps all geometry in Rust while
reusing the web UI. Revisit a native `wgpu` viewport if models of ~10k+ elements are
too slow after WebGL2 optimization.

## ADR-005 Internal units: millimeters (f64), UUID v7 IDs — Accepted
SI internally avoids imperial rounding bugs; display defaults to US architectural ft-in.

## ADR-006 Geometry kernel: native prismatic first, OpenCascade later — Accepted
v0.1 elements are all vertical extrusions, which a small native kernel handles exactly and
fast. A `GeometryKernel` trait isolates this so OCCT (via `opencascade-rs` or a `cxx`
bridge, LGPL, dynamically linked) can be introduced for roofs, sloped/curved geometry,
and exact hidden-line removal without rewriting callers.

## ADR-007 IFC4 as the interchange format; IFC export hand-written — Accepted
Keeps the build free of heavy C++ dependencies in v0.1. IfcOpenShell is used only in
tests to validate output. IFC import is deferred.

## ADR-008 DWG deferred — Accepted
ODA SDK requires paid membership; LibreDWG is GPL, incompatible with closed distribution.
Revisit after v0.1 based on user demand.

## ADR-009 M0 scaffold choices — Accepted (2026-09-23)
Context: the handoff left several M0-level choices open.
Decision:
- **IPC type generation: `ts-rs`.** Stable, and needs no changes to Tauri command
  registration. `tauri-specta` for Tauri 2 was still a release candidate. Payload structs
  derive `TS`; `cargo test` writes `app/src/bindings/`, and CI fails if they drift.
- **Save strategy.** The in-memory project is written to `<file>.rfproj.tmp` in a single
  SQLite transaction, then renamed over the target. The project file is never held open
  between saves. Autosave (from M1) will write `<file>.rfproj.autosave` the same way.
  Migrations apply on write, so opening an older file and saving it upgrades it.
- **Schema 1 = `meta` table only.** Element tables arrive in M1 as migration 0002.
- **Frontend pins.** React 18 as locked (new Tauri templates default to 19). TypeScript
  6.0.x, because typescript-eslint does not yet support TypeScript 7.
- **Menu.** A native Tauri menu forwards item ids to the UI as a `menu` event. The UI owns
  the file dialogs and calls the project commands. This keeps dialogs as UI and project
  state in Rust.
- **Bundle target.** NSIS installer only for now; MSI is not needed yet.
Consequences: IPC payload types must derive `TS` and be exported. Saves rewrite the whole
file, which is fine for v0.1 model sizes. Incremental writes (ADR-003) can come later
without a format change.

## ADR-010 Design stages (SD / DD / CD …) as project data — Accepted (2026-09-23)
Context: the owner wants the delivery phases architects work in to be part of the project:
Pre-Design, Schematic Design, Design Development, Construction Documents, Bidding,
Construction Administration. This is not Revit-style construction phasing
(Existing / Demolished / New per element), which stays out of scope for now.
Decision:
- **Stages are elements** (`ProjectStage`): editable name, abbreviation, order, planned
  start/target dates. New projects get the six AIA-style defaults; users can rename,
  reorder, add (e.g. "Permit") or remove ones they don't use.
- **ProjectInfo holds `current_stage_id`** plus an append-only **stage history**
  (from → to, timestamp, note). Moving stage is a normal transaction, so it can be undone.
- **Downstream uses**, built when those features exist:
  - Sheets (and views, optionally) list the stages whose deliverable set they belong to,
    so the browser can filter "SD package" vs "CD package" (M4).
  - The title block shows the current stage name (M4).
  - Each issuance records the stage it was issued in (M4 locally, M5 in Supabase).
  - Publishing sends the stage to Rufplan, so a CD or Bidding issuance can drive
    marketplace bidding (M5; needs owner approval of the Supabase schema change).
Consequences: M1 adds two element kinds (ProjectStage, ProjectInfo) and a Project Info
panel. Nothing depends on stages before M4, so the list can evolve without breaking
features. Stages are referenced by ElementId, never by name, so renaming is safe.

## ADR-011 Revit-style prototype slice ahead of the milestone order — Accepted (2026-09-23)
Context: the owner asked for a working Revit-like prototype (grids, walls, floors,
ceilings, plans, elevations, 3D) in the Rufplan visual style before finishing M1–M3 in
order, so they can start iterating hands-on.
Decision: build a vertical slice across M1–M4 now, keeping the golden rules (Rust owns the
model and geometry, every edit is an undoable transaction, mm internally, UUID v7). Take
these shortcuts on purpose; each is listed in the roadmap as remaining work:
- **Full regeneration** of derived geometry on every request instead of the dependency
  graph. Fine for small models; the M2 performance target (500 walls < 50 ms) needs the graph.
- **Typed struct fields** for element data instead of the generic `ParamValue` map.
  Properties are exposed through `ops::properties` / `ops::set_property`, so the UI won't
  change when storage moves to a parameter map.
- **JSON display lists** over IPC (binary transfer comes with larger models).
- **Ceilings added** as element kinds (`CeilingType`, `Ceiling`), with reflected ceiling
  plans. The ACT grid pattern is chosen by the type name containing "ACT" until types get
  a pattern parameter.
- **Walls: centerline location line only.** Two-wall corners are mitered; T and cross
  junctions are cleaned by polygon union of the cut footprints.
- **Polygon union grows inputs by 0.01 mm** (`tol::LINEAR`), because the boolean engine
  can leave exactly-touching mitered footprints unmerged.
- **Elevations use painter's-order hidden lines**, per ADR-006.
- **File format schema 2** adds the element tables from DATA_MODEL.md. Schema-1 files
  still open and are upgraded on save.
- **New dependencies:** `geo` (polygon booleans, allowed by the locked stack), `earcutr`
  (small triangulator for 3D caps), `uuid` and `rmp-serde` (per DATA_MODEL.md), `three`
  (locked stack), and `@fontsource/barlow` + `barlow-condensed`, which bundle Rufplan's
  fonts so the app works offline.
- **Rufplan look:** tokens from `Rufplan/packages/theme` (ink #0a0a0a, cyan #3ECFF7,
  paper #f8f8f6, borders #e8e8e8, Barlow / Barlow Condensed uppercase labels, square
  buttons) and the Rufplan wordmark.
Consequences: the milestone checklists are partly done out of order; ROADMAP.md marks
exactly what exists. Regeneration and parameter storage must be revisited before the M2
performance acceptance test.

## ADR-012 Doors and windows — Accepted (2026-09-23)
Context: M3 hosted elements, built on the ADR-011 prototype.
Decision:
- **Types carry the family.** `DoorType { family: DoorFamily, width, height }` and
  `WindowType { family: WindowFamily, width, height, sill }`, with code-defined
  families (Single Flush, Double Flush; Fixed, Casement). A separate `Family` element
  (DATA_MODEL.md) arrives with user-editable families; converting then is a migration.
- **Position = distance from the host wall's start to the opening's center** (mm), as in
  DATA_MODEL.md. Openings move with their wall and are deleted with it. When wall-endpoint
  dragging lands, the drag operation will recompute offsets so openings keep their
  real-world position (Revit behaviour).
- **Validation on every commit:** each opening must lie within its wall's length and
  height and must not overlap another opening in the same wall. Edits that break this
  (shortening a wall, lowering a level, widening a type) are rejected with a message.
- **Geometry:** walls are split into prism pieces around openings (full-height pieces,
  sill and head pieces). Plans cut through the pieces, so openings become gaps in the
  poché. Door leaves and swings, window sills and glass are view symbols. Elevations draw
  whole walls with openings on top; 3D gets real holes plus door-leaf and glass panels.
- **Placement:** Rust computes the preview (nearest cut wall, offset clamped into the
  wall, rounded to whole inches from the wall start, snapped to the wall's center). The
  side of the wall under the cursor sets the facing, as in Revit.
- **Marks** are the next free number per category ("1", "2", …).
Consequences: no new file schema version is needed, because new element kinds
deserialize as new enum variants and old files simply have none. Files saved before this
get the built-in door and window types when opened.

## ADR-013 Rooms and Move — Accepted (2026-09-24)
Context: completing M3 rooms. The M3 acceptance test ("moving a wall updates room
areas, door positions and the 3D view") also needs a Move operation.
Decision:
- **Rooms store only level, placement point, name and number.** The boundary is derived
  every regeneration: the smallest enclosed area (a hole in the union of the level's wall
  footprints) containing the point. That makes it the inside faces of the walls, with
  doors not breaking enclosure. If walls change so the point is no longer enclosed, the
  room reports "Not Enclosed" and area 0, as in Revit. Room separation lines come later.
- **One room per enclosed area**, enforced when placing (the preview says which room
  already occupies it). Numbers are the next free integer.
- **Area, perimeter and status are derived properties.** The app layer appends them to
  the core property sheet because they need regeneration.
- **Move (MV) follows Revit:** moved walls drag the ends of corner-joined walls with
  them, and walls T-joined into a moved wall keep their end on its line. Doors and
  windows in a stretched wall keep their real-world position (offsets recomputed); a
  selected door or window slides along its host. Every move is one transaction, and
  moves that would push an opening out of its wall are rejected.
Consequences: floors and ceilings keep their own sketched boundaries and don't follow
moved walls yet. Revit's locked "pick walls" boundary lines are future work.

## ADR-014 Documents: annotations, sections, schedules, sheets, PDF — Accepted (2026-09-24)
Context: M4, whose acceptance test is a 4-sheet vector PDF that measures true to scale.
Decision:
- **Door, window and room tags are drawn automatically** in floor plans from each element's
  mark / name / number / area. There are no tag elements yet, so tags can't be moved or
  hidden individually; tag elements come when that matters.
- **Dimensions are point-based** (`Dimension { view, a, b, offset }`), placed with snapping
  in plans, elevations and sections. They are not yet associated with the wall faces they
  measure, so they don't follow edits (Revit's reference-based dimensions are future work).
  Text is upright and uses feet-inches. `TextNote` holds free text in a view.
- **Sections** are views: `ViewKind::Section { start, end, depth }`, looking to the left
  of the line. They share the elevation generator, generalized with a cut plane and far
  clip. Elements crossing the plane draw as poché from their prism pieces (so door heads
  and window sills cut correctly); elements beyond draw in painter's order.
- **Schedules** are views listing doors, windows, rooms or sheets; the tables are computed
  from the model on every request. New projects get all four; older files gain them on
  open.
- **Sheets and viewports** are elements. A viewport maps a view's display list into paper
  mm by its scale; the same sheet display list draws on screen and in the PDF. A drawing
  view can be on one sheet only; schedules can repeat. The title block is code-defined in
  Rufplan style (cyan bar, wordmark, project info, current design stage, date, sheet
  name/number) in ARCH D and Tabloid.
- **PDF via krilla** (locked stack): 1 paper mm = 72/25.4 pt, real pen widths
  (0.13–0.7 mm), dash patterns in paper mm, and Barlow Condensed SemiBold embedded
  (OFL-licensed; the font and its license live in `crates/studio-sheets/fonts`). Text is
  measured with `ttf-parser`, already in the dependency tree via krilla.
Consequences: a sample 4-sheet set (A0.0 cover + indexes, A1.0 plans, A2.0 elevations,
A3.0 section + door/window schedules) exports from the sample project; tests check the
page sizes, font embedding and true-scale geometry (a 10'-0" wall prints 63.5 mm at 1/4").
Open: key plan, revisions, issuances, sheet sets by stage (M4 checklist), view crop regions.

## ADR-015 Completing M4: associative dimensions, tag elements, stage sets, issuances — Accepted (2026-09-24)
- **Dimension ends attach** to what they're placed on, in plan views: a wall (fraction
  along its location line and signed side offset, so both faces and the centerline work)
  or a grid. Ends re-evaluate on every draw, so dimensions follow moves and stretches.
  The stored points are the fallback if the element is deleted. Moving an attached
  dimension slides its line. Snapping gains wall face corners and faces.
- **Tags are elements** (`Tag { view, target, offset }`), created in every floor plan of
  the target's level when a door, window or room is placed (Revit's tag on placement).
  Each tag moves or deletes independently of its target; Tag All restores missing tags in
  a plan. Files from before this get tagged once when opened.
- **Stage sets:** `Sheet.stages` lists the design stages whose deliverable set includes
  the sheet (a Yes/No property per stage). The browser filters sheets by stage.
- **Issuances:** `Issuance { name, stage, date, sheets }`. Issue Set records the current
  stage's set (every sheet, if none are assigned yet) and exports it to PDF. Title blocks
  list every issue that included the sheet (date, stage, name) as a revision-style block.
- **Sheet notes:** text notes can sit on sheets, with printed sizes from 3/32" to 1".
  Title blocks gain a key plan (the lowest level's wall outline) and a north arrow.

## ADR-016 M5: IFC4 export and publishing to Rufplan without schema changes — Accepted (2026-09-24)
- **IFC4 export** is hand-written STEP in `studio-io::ifc` (ADR-007): project, site, building,
  storeys; walls as extruded footprints with `IfcOpeningElement` voids that doors and windows
  fill; floors as `IfcSlab .FLOOR.`, ceilings as `IfcCovering .CEILING.`, rooms as `IfcSpace`
  with `Qto_SpaceBaseQuantities.NetFloorArea`; materials by type; `Pset_WallCommon`,
  `Pset_DoorCommon`, `Pset_WindowCommon`. Lengths in mm. GlobalIds are the element UUID in
  IFC base-64; derived objects (openings, relationships, psets) use UUID v5 of the element id,
  so re-exports are byte-stable apart from the header timestamp. CI writes the sample model
  and validates it with IfcOpenShell (`tools/validate_ifc.py`); locally it was also checked
  with web-ifc (all 31 products produce geometry and walls are cut).
- **No new tables.** Instead of the proposed `studio_projects` / `studio_issuances`, Publish
  writes into what Rufplan's own "Upload Design Set" flow already uses: files go to the
  `project-media` bucket at `{uid}/deliverables/{project}/{phase}/{deliverable}/{stamp}-{file}`
  and each gets a `project_deliverables` row (PDF in slot `drawings` with `sheet_count`, IFC in
  discipline slot `A`). RLS already allows the owner (and visible-project members) to do
  this. Design stages map to Rufplan's phase tabs: PD/SD → `sd`, DD → `dd`, CD/BN/CA → `cd`;
  the user picks a deliverable from Rufplan's catalog (sd30…, dd-owner, permit, ifc…).
  The issue is also recorded locally as an `Issuance`, like Issue Set.
- **Known limit:** the `project-media` bucket's `allowed_mime_types` only permits PDF, image
  and video types, so the IFC upload is refused until the owner adds `application/x-step`
  (a bucket-config change, not made here). The IFC is therefore optional: the PDF publishes
  and the result says why the IFC was skipped.
- **Link:** `ProjectInfo.rufplan: Option<RufplanLink { id, name, slug }>` (serde default, so
  older files load unchanged); set from the user's `open_projects` (`owner_business_id =
  auth.uid()`); undoable.
- **Auth:** `studio-sync` is a small blocking Supabase client (ureq + rustls) behind an
  `Http` trait so it is tested offline. Email/password sign-in (same accounts as
  rufplan.io) plus Google via PKCE in the system browser. Deviation from SYNC_AND_RUFPLAN:
  the redirect is an RFC 8252 **loopback** listener, `http://127.0.0.1:53682/auth/callback`,
  instead of a `rufplan-studio://` deep link. This needs no deep-link or single-instance
  plugins. The owner must add that URL to Supabase → Authentication → URL Configuration →
  Redirect URLs for Google sign-in to work. The refresh token is stored in the OS credential
  store (`keyring`) and the access token is kept in memory.
- **Keys:** only the anon key, read at build time from `RUFPLAN_SUPABASE_ANON_KEY` or the
  gitignored `app/.env.local`. Builds without it (e.g. CI) run with sign-in disabled.
- **New dependencies:** ureq, sha2, base64, keyring, tauri-plugin-opener (+ npm
  `@tauri-apps/plugin-opener`, allowed only for `https://rufplan.io/*`).

## ADR-017 Revit-style editing, incremental regeneration, parameter map — Accepted (2026-09-24)
- **Modify tools** (studio-core `edit`, one transaction each): Copy (CO, with Multiple),
  Array (AR, linear, count in the options bar), Rotate (RO, click center / from / to, or
  type degrees; optional copy), Mirror (MM, draw the axis; copies by default like Revit),
  Align (AL: reference line, then the line to move — wall faces, centerlines, grids),
  Trim/Extend to Corner (TR), Offset (OF, distance in the options bar, preview on hover),
  Split (SL) and Flip (Space). Walls carry their hosted doors and windows. A mirrored wall
  runs the other way so its exterior stays outside, and mirrored doors change hand. New
  copies continue mark, room-number and grid-name sequences and get tags in plans.
- **Grips and temporary dimensions** (studio-views `handles`): wall and grid ends,
  dimension lines and crop-region edges drag; dragging a wall end moves the joint (corner-
  joined walls follow, T-joined walls stay on the line, openings keep their world
  position). A selected wall shows its length, and a door or window its clear distances
  to the wall ends; clicking the value lets the user type a new one.
- **Typed lengths while drawing:** after the first click of Wall, Grid, Move, Copy, Array
  or Stair, typing digits opens a length box; Enter places the point that far toward the
  cursor (Rotate takes degrees).
- **Incremental regeneration:** documents get a content stamp and a derived-data cache.
  `regenerate` memoizes each expensive step on exactly its inputs (a wall's footprint on
  its ends, width and miter partners; its pieces on footprint, heights and openings; a
  level's room regions on its walls' footprints) — the dependency graph without
  hand-maintained edges, and correct through undo and redo. Display lists (views and
  sheets) are cached by stamp, so hover picking and snapping no longer regenerate.
  Polygon unions cluster touching inputs and merge in a balanced tree. Measured in a
  release build: a wall-type edit on 535 connected walls redraws the plan in ~6.5 ms;
  hover pick + snap takes ~0.6 ms (M2's target is 50 ms). A property test checks that
  incremental results equal a full rebuild over random edit / undo / redo sequences.
- **Category index** in the element store (`Document::of` no longer scans).
- **Parameters:** `ParamValue` / `ParamKind` / `ParamDef` (DATA_MODEL.md). Built-in
  parameters remain typed fields of the element data. User-defined **project parameters**
  (Manage ▸ Project Parameters: name, type, instance or type, categories) are stored in each
  element's parameter map, persisted in the existing `params` column (no schema change),
  shown under "Other" in Properties, undoable, and removed from every element together with
  their definition.
- **Deferred:** binary IPC for display lists. With caching, JSON is only sent after an
  edit and is fast enough; revisit with large models.

## ADR-018 Layered walls, roofs, stairs, crop regions — Accepted (2026-09-24)
- **Compound wall types:** `WallType.layers` (material, thickness, function), exterior
  first; the exterior face is the wall's left looking from start to end (clockwise-drawn
  buildings face out, as in Revit). The built-in types got layer builds that add up to their
  nominal widths; typing a new Width resizes the structure layer. Layers are edited in
  Properties (material, function, thickness, move up, delete, add). Plans at 1/4" and larger
  draw layer lines on a lighter cut fill (Revit's medium detail); IFC exports an
  `IfcMaterialLayerSet` per type.
- **Roof by footprint:** `Roof { boundary, level, offset, slope, sloped[edge] }` with a
  `RoofType` (thickness). Each sloped edge rises inward and owns the part of the footprint
  where it is the nearest sloped edge line (half-plane clipping). This gives exact hips,
  ridges and gables for convex footprints; non-convex footprints should be split into convex
  roofs for now (no straight skeleton yet). The Roof tool covers the view level's walls with
  an 18" overhang at 6/12, bearing on the walls' tops (on the level at that height when there
  is one). Plans show the roof plan (dashed when above the cut), elevations draw sloped
  faces and fascias as polygons, sections cut the slopes, and 3D and IFC (`IfcRoof`,
  triangulated) get the full solid.
- **Stairs:** straight runs from a level to the one above; the riser count is the smallest
  that keeps risers at or under the maximum (7"), with 11" treads and a 3'-6" width. Plans
  show treads up to the cut, a break line, dashed treads beyond and an UP arrow; the upper
  level shows DN. Elevations, sections, 3D and IFC (`IfcStair` with `Pset_StairCommon`).
  Floors don't get stair openings yet.
- **Crop regions:** `View.crop` / `show_crop`; Crop View in Properties, and edge grips when
  the view is selected. Lines, fills and text are clipped to the region; the boundary never
  prints on sheets, and viewports size to the cropped view.
- The sample gained a Roof level with a hip roof, a stair from Level 1 to Level 2, and
  clockwise exterior walls so the layered exteriors face out.
