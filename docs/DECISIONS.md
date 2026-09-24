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
