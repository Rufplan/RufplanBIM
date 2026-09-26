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

## ADR-019 Structure, railings, finished envelope, assembly detail — Accepted (2026-09-24)
- **Columns:** `ColumnType { shape, structural }` with rectangular, round and wide-flange
  sections (five built-ins: concrete square and round, W10x33, 6x6 post, architectural
  round). `Column { base_level, base_offset, top, at, rotation }` rises to the level above
  by default. Placed one at a time (snaps to grid intersections and other column centers) or
  **At Grids**, which fills every free grid intersection on the level. Plans: structural
  columns in poché, architectural ones lighter; IFC `IfcColumn` with
  `Pset_ColumnCommon.LoadBearing`.
- **Beams:** `BeamType` (rectangular or wide-flange: W12x26, W8x18, glulam, concrete) and
  `Beam { level, offset, start, end }` with its top at the level. A beam drawn in a plan
  frames the floor above it (Revit's convention). Plans draw beams overhead dashed with a
  centerline, cut beams in poché; wide-flange beams are three prisms, so sections cut the
  flanges and web. IFC `IfcBeam`.
- **Railings:** `RailingType { height }` (42" guardrail, 36" handrail) and `Railing { level,
  offset, path }`, sketched as an open polyline. Resolved to a top rail, a bottom rail 4"
  up, balusters at 4" and end posts. Stairs with `railings` (on by default) get 36"
  handrails on both sides of each flight and on the landing's open edges. IFC
  `IfcRailing` (.GUARDRAIL. / .HANDRAIL.; a stair's rail gets a GUID derived from the stair).
- **L- and U-shaped stairs:** `Stair.shape` (straight, L or U, turning left or right) and
  `first_run` (risers before the landing; 0 = half). Two flights and a square landing one
  stair-width deep; plans draw the second flight dashed above the cut with the walking line
  through the landing. IFC `.QUARTER_TURN_STAIR.` / `.HALF_TURN_STAIR.`.
- **Stair openings:** a stair cuts its footprint (flights and landing) out of the floors on
  its top level and the ceilings it passes through. It's derived in regeneration, not stored,
  so it follows the stair. Slabs gain holes (or notches at an edge); sections break the cut
  slab at the opening; IFC uses `IfcArbitraryProfileDefWithVoids`, one `IfcSlab` per floor.
- **Walls attached to roofs:** `Wall.attach_top` (a flag, not a reference, so deleting the
  roof can't cascade). The top follows the underside of the lowest roof above each point
  of the centerline, sampled at the ends and at every roof edge crossing. Attach Top and
  Detach Top (Modify tab) act on the selection in one undo step. Elevations draw the sloped
  top; 3D and IFC tessellate the wall.
- **Roofs on L, T and U plans:** the Roof tool splits a right-angled footprint into maximal
  overlapping rectangles (`studio_geom::rect_cover`), one hipped roof per wing. Where roofs
  on the same level overlap, each face keeps only the part where it's the top surface
  (`roof::resolve_overlaps`), so plans and elevations show valleys, and fascias buried
  inside the other roof are hidden. Other non-convex shapes are refused with a message
  (sketch those roofs edge by edge). A straight skeleton is still the long-term answer.
- **Location line:** `Wall.location` (Wall Centerline, Core Centerline, Finish or Core Face
  Exterior/Interior) from the options bar while drawing. Walls are still stored by their
  centerline; the drawn line is offset into it, and Flip keeps the location line in place.
  Changing it in Properties keeps the wall where it is.
- **Layered floors and roofs:** `FloorType`, `CeilingType` and `RoofType` gained layers
  (hardwood/plywood/I-joist/gypsum 12" floor, 6" slab, shingle/plywood/rafter roof,
  membrane/cover board/insulation/deck flat roof), edited in Properties like wall layers.
  Sections draw their layer lines on the lighter cut fill; IFC exports a layer set per type.
- **Assembly detail in plan (1/4" and larger):** finish layers wrap free ends and door and
  window jambs (the core stops short, and a return line closes the finish). Cut patterns
  come from the layer's function and material name: batt insulation zigzag, masonry
  diagonals, concrete diagonals with aggregate. No new material library yet: patterns
  follow names such as "CMU", "Brick", "Concrete" and "Insulation".
- **File format:** new variants and `#[serde(default)]` fields only; files saved before this
  still open, and gain the built-in column, beam and railing types on open.
- The sample gained a guardrail around the stair opening on Level 2. Its floor is now cut
  by the stair, as is the Living room ceiling.

## ADR-020 Bound floors, room separators, section box, materials, detailing — Accepted (2026-09-24)
- **Floors and ceilings follow walls:** `Floor.bound` / `Ceiling.bound` (`SlabBound`: Sketch,
  Walls, Room { point }). Floor: Pick Walls stores `Walls`; the outline is taken from the
  level's regenerated wall regions (outer faces), so moving, adding or resizing walls
  reshapes the floor. Ceiling: Auto Room stores the room point and follows that room.
  The stored boundary is kept as a fallback when the walls go away. Properties show
  "Boundary: Follows Walls / Sketched". Detaching freezes the current outline (it needs
  the model, so it's done in `studio_regen::derived`, as is creating bound slabs).
- **Room separation lines:** `RoomSeparator { level, start, end }`, drawn as a chain (RS).
  Each adds a 1 mm strip to its level's room regions, so rooms split along it (the strip
  costs 0.5 mm × length of each room's area). Thin lines in floor plans; snaps.
- **3D section box:** `View.section_box` (min/max, model mm) on 3D views. Section Box in
  Properties starts it around the model with a 1' margin; the six faces are editable
  lengths there, and in 3D the box shows with a handle on each face to drag. Clipping is
  done by three.js clipping planes (a rendering concern); caps are not drawn.
- **Column cleanup:** architectural columns cut by the plan and touching a wall join its
  poché and outline; structural columns stay separate, drawn over the wall.
- **Materials:** `Material { name, cut, surface, color }` elements, 19 built in. Cut
  patterns (none, batt, rigid, masonry, concrete, wood, solid) drive plan hatching;
  surface patterns (lap, running bond, grid presets) draw in elevations on the face
  toward the viewer (walls: its finish layer; roofs: courses along the slope), aligned to
  the project origin and skipped when finer than 0.8 mm on paper; colors drive 3D. Layers
  (`WallLayer.material`) and column/beam types (`material`) reference materials; unlinked
  layers still resolve by name (so older files draw as before), and files gain the built-in
  materials, linked by name, on open. A material in use can't be deleted. Edited in
  Properties (select it in the browser's Materials section; Manage > New Material /
  Duplicate). IFC layer sets and column/beam materials use the material names.
- **Schedules:** Structural Column Schedule (location mark, type, base/top level, length),
  Structural Framing Schedule (type, reference level, length) and a Material Takeoff (area
  and volume per material across walls, floors, ceilings, roofs, columns and beams; walls
  by their material volume, so openings and joins count). Older files gain any missing
  standard schedule.
- **Structural tags and marks:** Column Location Mark (Revit's "B-2": the grid
  intersection within 3', letters first) in properties, schedules and column tags. Tag All
  also tags columns (their mark) and beams (their size along the beam, in the plan below
  the level they frame).
- **Callouts (detail views):** a View with `callout_of` (its parent) and a crop, created by
  drawing a rectangle (CA) in a plan, elevation or section, at 1 1/2" = 1'-0" (new scales
  1 1/2" and 3" were added). Callouts of sections and elevations follow their parent's cut.
  The parent shows a rounded boundary and a head with the sheet it's placed on; double-click
  opens it. Deleting the parent deletes its callouts. Callouts don't get tags automatically.
- **File format:** new element kinds and `#[serde(default)]` fields only.
- The sample's floors and ceilings are bound, and a callout of the southwest corner is
  placed on A3.0.

## ADR-021 Revit sketch mode for floor boundaries; Revit level and elevation symbols; interior elevations — Accepted (2026-09-24)
- **Sketch mode:** Floor (SB), Sketch Ceiling (CS), Edit Boundary (Modify tab) and
  double-clicking a floor or ceiling enter sketch mode. The sketch lives in the session
  (not the model) until Finish; the ribbon becomes Revit's contextual tab ("Modify | Create
  Floor Boundary") with Mode (Finish ✓ / Cancel ✗), Draw (the Boundary Line tools) and
  Modify groups; the model draws halftone and the element being edited is hidden; sketch
  lines are magenta. Sketch mode has its own undo/redo (Ctrl+Z / Ctrl+Y); Esc ends the
  current chain, then returns to Modify, and never leaves the sketch.
- **Draw tools:** Line (Chain, Offset, Radius: chained corners filleted), Rectangle
  (Offset, Radius: rounded corners), Inscribed and Circumscribed Polygon (Sides, Offset),
  Circle (Offset), Start-End-Radius Arc, Center-ends Arc (up to a half circle, toward the
  cursor), Fillet Arc (two lines, Radius), Pick Lines (wall faces, centerlines and grids;
  Offset toward the cursor; Lock keeps a wall line on its wall) and Pick Walls (the face on
  the cursor's side; Offset; Extend into wall (to core); **Tab** picks the whole chain of
  connected walls, all inside or all outside). Picked lines trim to each other at their
  corners. Typed lengths work for lines; the sketch's own ends and midpoints snap.
- **Modify in sketch:** select lines (Shift adds), drag a vertex (every line end there moves),
  Trim/Extend to Corner (keeping the clicked parts), Delete, and Flip (Space) for picked
  wall lines, moving them to the wall's other face with their corners.
- **Model:** `Floor.sketch` / `Ceiling.sketch` hold the boundary loops (`SketchCurve`: lines,
  optionally locked to a `WallRef` face with an offset, and arcs). Regeneration puts each
  locked line on its wall's current face and re-intersects it with its neighbors, so the
  floor follows moved walls, and changes of wall type thickness, like Revit's locked
  sketch lines. Loops inside other loops are openings; separate loops are separate pieces
  of one floor. Pick Walls floors made outside sketch mode (and the sample's) are the same
  locked perimeter sketch. "Boundary: Sketched" unlocks the lines where they are;
  "Follows Walls" re-picks the perimeter.
- **Finish** checks, with Revit's messages and the lines highlighted red: "Lines must be in
  closed loops. The highlighted lines are open on one end.", "Lines must not intersect",
  "Highlighted lines overlap. Lines may not overlap.", an empty sketch, zero-area loops.
  Finish creates the floor (or updates the edited one) as one undo step.
- **Not yet:** slope arrows, span direction, tangent-end arcs, splines, and the modify
  tools (move, copy, rotate, mirror, offset) on sketch lines.
- **Level heads:** Revit's Level Head – Circle: a datum target (two opposite quarters
  filled) at the line's end, with the name over the elevation (`10' - 0"`) above the line.
- **Elevation marks:** a round body with a filled arrowhead pointer per view (double-click a
  pointer to open its view; the body selects the marker). One view shows its detail
  number over its sheet number; several show each detail number by its pointer. Detail
  numbers are the order views were placed on their sheet. Callout heads use the same
  detail-over-sheet form.
- **Interior elevations:** `ElevationMarker { level, at, interior }` with `MarkerElevation
  { marker, facing }` views (deleting the marker deletes them). The Elevation tool (EL, with
  Interior / Building type in the options bar) places a marker looking at the nearest wall;
  the marker's Properties turn the North/East/South/West views on and off, named after the
  room ("Kitchen - North"). An interior view cuts through the marker, as wide as the room
  plus 1' each side (so side walls show cut) to the far wall, and crops floor to the level
  above (its own crop, if set, wins). Building markers draw a plain elevation. The sample's
  Kitchen has a four-view interior marker.
- **File format:** new element kinds, a new view kind and `#[serde(default)]` fields only.

## ADR-022 Elevation mark families, editing in 3D, 3D ground grid — Accepted (2026-09-24)
- **Elevation mark types:** `ElevationMarkerType { name, interior, style, size }`, listed under
  Families > Elevation Marks and edited in Properties (name, Interior, Symbol, body radius).
  Built in: Interior Elevation, Interior Elevation - Diamond, Building Elevation, Building
  Elevation - Half Circle, Building Elevation - Diamond. Symbols: Circle - Filled Arrow,
  Circle - Filled Half, Diamond - Filled Corners. A placed marker's type is picked in the
  type selector at the top of Properties, like any Revit family instance; switching between
  an interior and a building type changes its views (interior views crop to the room,
  building views don't). The four building elevations pick their mark in their view
  Properties (Elevation Mark). The Elevation tool's options bar picks the type to place.
- **Editing in 3D:** Wall and Column place on the work plane of the level chosen in the
  options bar (snapping as in that level's plan; walls chain until Esc or right-click);
  Door and Window place on the wall face under the cursor with a cyan ghost of the opening
  (red where it would overlap); Floor (Pick Walls), Ceiling (Auto Room), Roof (by
  footprint) and Room act on the level of the wall or floor clicked. Each goes through the
  same commands as the plan tools, via that level's floor plan. Meshes carry their level.
- **Ground grid:** a hairline Rufplan-cyan grid on the lowest level: 4' squares with a
  stronger line every 20', around the model. Toggle with the Ground Grid chip in the 3D
  view or the View tab. Not clipped by the section box; not pickable.

## ADR-023 Site tab: Google Maps, Regrid parcels, USGS 3DEP topography — Accepted (2026-09-24)
Owner decisions (2026-09-24): Google Maps for search and imagery (owner's API key), Regrid
for parcel boundaries (owner's token), USGS 3DEP for elevations.
- **Site tab** (first, before Architecture): Find Lot, Site Plan, Get/Refresh Topo, Site
  Settings, API Keys.
- **Keys:** the Google Maps key and Regrid token live in the OS credential store (service
  "Rufplan Studio"), never in the project file or the repository. The Google key is handed to
  the page because Google's map runs there (restrict it by API in Google Cloud); the Regrid
  token never leaves Rust.
- **Find Lot:** the Maps JavaScript API (hybrid imagery, Geocoder limited to the USA). A
  search or a click looks up the parcel at that point from Regrid (`/api/v2/parcels/point`);
  Use This Lot stores it.
- **Content Security Policy** widened for Google Maps only: scripts from maps.googleapis.com
  and maps.gstatic.com; images, fonts and connections to Google's map hosts; blob workers.
- **Model:** `ElementData::Site` (address, lat/lon of the lot's center, boundary in a local
  east/north frame in mm, parcel record, Offset and Angle to True North placing that frame in
  the project, Level 1 elevation (NAVD88), contour interval, and the topography grid). Local
  coordinates use WGS84 degree lengths at the site (mm-accurate over a lot). Setting the lot
  adds a "Site" plan (1" = 20'-0"; engineering scales 1"=10' to 1"=50' were added).
- **Topography:** a grid over the lot plus a margin (2'–20' spacing), sampled from the USGS
  3DEP ImageServer `getSamples` (1,000 points per request, bilinear, 1 m lidar where
  available). Gaps take the nearest sample. The first topo sets Level 1 to the ground at the
  lot's center. Preliminary; not a survey. No image library was needed (the owner had
  approved one): getSamples returns JSON.
- **Drawings:** site plans show contours (every fifth heavier and labeled in feet), the
  property line with bearings (N 12°30'00" E) and decimal-foot distances, and a north
  arrow; plans of the lowest level show the property line. Sections cut the ground; building
  elevations draw its highest line; 3D shows the ground surface. IFC writes the site's
  RefLatitude, RefLongitude and RefElevation.
- **Not yet:** tracing a lot by hand (no Regrid coverage), building pads/grading, easements
  and setbacks, and exporting the topo to IFC.


## ADR-024 Revit keyboard shortcuts and view visibility commands — Accepted (2026-09-24)
Owner request (2026-09-24): "add all the typical Revit shortcuts like align AL and trim TR".
- **Registry:** `app/src/shortcuts.ts` lists every command with Revit's default keys (tools
  Revit has no default for are marked "ours"). A key maps to a tool or to an action
  (`app/src/actions.ts`). Typing two letters in a view runs it; Enter with no tool running
  repeats the last command (RC), as in Revit.
- **Defaults changed to match Revit:** CL is Structural Column (it was Ceiling); SB is Floor;
  CS is Create Similar; MM is Mirror - Pick Axis and DM is Mirror - Draw Axis; SC is gone.
- **Keyboard Shortcuts (KS):** search and change keys. Clashing keys show in red; Reset to
  Revit Defaults clears changes. Overrides are a UI preference in this computer's local
  storage, not in the project file.
- **Modify:** Pin (PN) and Unpin (UP) set a hidden `__pinned` parameter; Move, Rotate,
  Mirror, Align, Trim/Extend, grips and Delete refuse pinned elements with a message, and
  Properties show "Pinned". Match Type Properties (MA) picks a source, then targets. Create
  Similar (CS) starts the selected element's tool and type. Select All Instances (SA)
  selects every instance of its type. Tag by Category (TG, RT) tags the clicked element.
- **Visibility:** Temporary Hide/Isolate (HH, HI, HC, IC, HR) is session state with Revit's
  cyan frame; nothing is saved. Hide in View (EH elements, VH category) and
  Visibility/Graphics (VV/VG) are stored on the view (`hidden`, `hidden_categories`, both
  `#[serde(default)]`, so older files open). Drawings, PDF and 3D omit what a view hides;
  Unhide All restores it (undoable).
- **Display:** Thin Lines (TL); 3D Wireframe (WF), Hidden Line (HL), Shaded (SD); Zoom to
  Fit (ZF/ZE/ZX/ZA), Zoom Out 2x (ZO), Zoom in Region (ZR), Previous Pan/Zoom (ZP);
  Properties (PP) toggles the panel.
- **Snap overrides:** SE, SM, SI, SP, SN keep only that snap for the next pick, SO turns
  snapping off for it, SS clears it. `snap` takes an `only` argument
  (`studio_views::snap_only`).
- **Not yet:** importing or exporting Revit's KeyboardShortcuts.xml, filters and graphic
  overrides in V/G, and hiding annotation subcategories.

## ADR-025 Sketching and modifying floors in 3D — Accepted (2026-09-25)
Owner request (2026-09-25): "allow to draw and sketch and modify a floor from 3D view".
- **Sketch mode from 3D:** Floor (SB) and Sketch Ceiling in a 3D view enter the same sketch
  mode as in plans (ADR-021), on the level chosen in the options bar. `sketch_begin` takes an
  optional `level`; outside a plan the sketch goes through that level's plan
  (`sketch::plan_for`: its floor plan for a floor, its ceiling plan for a ceiling) for
  snapping and Pick Lines, so the model and commands are unchanged.
- **Work plane:** the sketch is drawn at `sketch::work_plane_z`: the level for a floor, the
  ceiling's height above the level for a ceiling (9'-0" for a new one). Sketch lines are
  magenta over the model, selected lines cyan, invalid ones red.
- **Tools in 3D:** Line, Rectangle, polygons, circle and arcs click on the work plane with
  snaps and a live preview. Pick Walls and Pick Lines take the wall face under the cursor
  (Tab picks the chain). Modify selects lines (Shift adds) and drags their ends. Trim/Extend
  and Fillet Arc work, as do Delete, Flip, undo and Finish/Cancel on the ribbon. Esc ends a
  chain, then returns to Modify. Typed lengths stay a plan feature.
- **Modifying in 3D:** Edit Boundary (ribbon, or double-click a floor or ceiling) opens its
  sketch in 3D, and hides the element while it's being edited. Move (MV) and Copy (CO) take
  two points on the plane through the first one (on an element or the work plane), snapping
  through that level's plan. Properties edits (type, offsets, thickness) already applied.
- Also fixed: after a model change, the 3D view keeps Temporary Hide/Isolate and the visual
  style, and hidden meshes can no longer be picked.

## ADR-026 Satellite overlay on the topography, and wider topography — Accepted (2026-09-25)
Owner request (2026-09-25): "a toggle to have Google Maps overlay onto the topo, and expand the
topo and map overlay to 2-4 zoom out times from the lot".
- **Imagery:** one satellite image from Google's Maps Static API, using the owner's key from
  the credential store (fetched in Rust; the key is not exposed further). `site::imagery_frame`
  covers the topography, or the lot and 25' around it before there is one. It picks the
  closest Web Mercator zoom that fits in 640 px (fetched at scale 2) and gives the image's
  corners in project coordinates, so it follows the site's Offset and Angle to True North.
- **Not stored:** Google's terms don't allow caching its imagery, so the image lives only in
  the page's memory for the session. It is never in the project file, PDF or IFC. Views show
  "Imagery ©Google".
- **Toggle:** Satellite on the Site tab, and a Satellite chip in 3D (session setting, off by
  default).
  - In 3D the image is draped on the ground surface (the Site mesh's UVs come from the
    corners).
  - In Site plans it's drawn under the linework at 70% opacity.
- **Area:** Get Topography takes an Area setting: the lot (as before), or a square 2x, 3x or 4x
  the lot's longer side centered on it, plus Beyond the lot. A grid over 40,000 points doubles
  its spacing until it fits (`site::topo_grid`). The overlay covers the same area.
- **Owner setup:** the Google key needs the Maps Static API enabled (it already was).

## ADR-027 Rendering tab: cameras and path-traced renders — Accepted (2026-09-25)
Owner request (2026-09-25): "another tab after architecture called rendering, with camera
tools similar to Revit, and a separate button to render a view from that angle".
Owner decision (2026-09-25): photoreal path tracing with **three-gpu-pathtracer** (MIT, built
on our three.js; plus three-mesh-bvh), over an enhanced real-time render with no new
dependency.
- **Camera (Revit's tool):** in a floor plan, click the eye, then the target. The options bar
  has Perspective and Offset (eye height, 5'-6" by default). It makes a perspective 3D view
  named "3D View 1", "3D View 2"… and opens it.
- **Model:** views gain `camera: Option<ViewCamera>` (`#[serde(default)]`). A camera stores
  its level, the eye and target in plan, and their heights above the level (so it moves with
  the level), plus a vertical field of view.
  - Properties: Eye Elevation, Target Elevation, Field of View, Reference Level.
  - Orbiting or zooming a camera view saves its pose once navigation settles (undoable, "Move
    camera"). The default {3D} view keeps its own orbit as before.
- **In plans:** cameras show on floor plans of their level, as Revit's camera glyph with its
  view cone out to the target. Clicking one selects its view. Grips drag the eye and target.
  Cameras never print on sheets.
- **Render (RR):** on the Rendering tab, for the active 3D or camera view.
  - Settings: output size (720p to 4K), quality (32 to 2048 samples), sun date and time,
    background (sky or white), exposure.
  - The image sharpens progressively; Stop keeps it, and Save Image writes a PNG. The bytes go
    to Rust as the raw request body.
  - Materials by category: transmissive glass, metallic steel and railings, matte elsewhere,
    with each element's material color. The satellite image drapes the topography when that
    overlay is on. Without topography, a plain ground is added.
  - The sun is NOAA's solar position at the site's latitude and longitude (central USA
    without a site), turned by the Angle to True North (`camera::sun_position`). A
    procedural sky provides the fill light.
  - The scene is built synchronously: the library's async build needs a web worker, which our
    content security policy doesn't allow. The path tracer is loaded only when Render is
    first used.
- **Verified:** a headless software-WebGL render of a test house showed the sky, sun shadows
  and glass. Final quality depends on the GPU.
- **Not yet:** saving renders into the project (for sheets), artificial lights, material
  textures and bump maps, entourage (people, trees), depth of field, and camera crop regions.

## ADR-028 V-Ray-style lighting, photo backgrounds, transparent renders — Accepted (2026-09-25)
Owner requests (2026-09-25):
1. Placing a camera in a floor plan showed nothing.
2. "Make the lighting and rendering look like the V-Ray engine."
3. "Have the sky actually show a sky, add mountains, grass plain and city, and a checkbox
   to save with the background or as a transparent PNG."

Owner decision (2026-09-25): real photo backgrounds, not procedural ones.
- **Camera fix:** the Camera tool's clicks were never routed to point placement, so nothing
  happened. The point-placing tools are now one shared list (`POINT_TOOLS`), tested. While
  placing, the camera and its view cone follow the cursor, as in Revit.
- **Lighting like V-Ray's Sun & Sky:**
  - The sky is a Preetham physical sky (turbidity 3), computed for the site's sun.
  - The sun is part of the sky map: a disk of 1° radius (like raising V-Ray's sun size),
    balanced to about 6:1 sunlit to sky-lit. Importance-sampled, it gives soft, clean shadow
    edges; the library's directional light only gives hard shadows.
  - Or **Dome light:** the background photo's HDR lights the scene. It is normalised to a
    clear afternoon sun & sky, so exposure means the same in both modes, and it turns with
    the background.
- **Rendering:**
  - 8 diffuse bounces and 12 through glass.
  - Thin architectural glass (no refraction, no rays trapped in the window box). Glass
    doesn't block the sun's direct light.
  - V-Ray-like material settings per category.
  - Tone: Contrast (ACES, the default, punchy) or Filmic (AgX, soft highlights).
  - An edge-aware denoise at the end, stronger than the library's default.
- **Backgrounds:** Sky, Mountains, Grass Plain and City are Poly Haven CC0 panoramas
  (app/public/backgrounds, about 9.7 MB): a 3072 px JPEG as the backdrop, plus a 1k HDR for
  the dome light. The other options are Physical Sky (matching the sun) and White.
  - The backdrop is rendered from the camera with the same tone curve and composited behind
    the path-traced model, which renders on transparency.
- **Ground projection (V-Ray's dome ground projection):** for Mountains, Grass Plain and
  City, the photo's own ground is mapped onto a 50 m ground disc around the camera, as seen
  from where the photo was taken (1.7 m up). Its albedo is scaled so that it renders like the
  photo under our light. The building then casts shadows on the photo's grass or paving,
  which meets the backdrop at the horizon.
  - Past 50 m the backdrop shows its own view, so trees and buildings aren't smeared across
    the ground.
  - Sky, Physical Sky and White use a plain lawn, paving or grey ground in linear colour.
    A tiled texture banded toward the horizon.
- **Saving:** "Save with the background" (on by default) saves the composite. Off, it saves
  a transparent PNG of the building alone: the render is cut out by an antialiased raster
  mask of the model (ground and topography excluded).
- **Verified:** the smoke tests ran headless Chrome on the owner's GPU through the DevTools
  protocol. 256 samples at 480 × 270 take about 45 s; 1024 samples clear the noise in glass
  seen from afar.
- **Not yet:** a firefly clamp (the library has none), clouds in the physical sky,
  interior lights, textures and bump maps, and entourage.

### ADR-023 amendment (2026-09-25): resilient USGS requests
USGS 3DEP answered a Get Topography with 502, and was taking 20–30 s per request. One failed
batch used to abort the whole topography. Now:
- Batches are 500 points (was 1,000), so USGS's gateway times out less often.
- Three batches run at once.
- Each batch is retried up to four times, with waits of 3, 6 and 12 s, after a network
  error, a 5xx or 429, or a 200 carrying an error. Other 4xx answers are final.
- The Find Lot dialog shows "n of m batches".
- If USGS stays down, the message says it is busy and nothing was changed.

## ADR-029 Materials tab: V-Ray-style material library — Accepted (2026-09-25)
Owner request (2026-09-25): "a tab for materials between architecture and rendering… make
all the materials V-Ray-like high-res materials with presets typical for residential,
hospitality and multifamily buildings in the USA, high-end to typical, including metals,
wood, etc.… have the render preview thumbnail be off a cube with the full rendered material".
Owner decisions (2026-09-25):
- 2K photo textures are downloaded on first use and cached, not bundled.
- The file format changes: materials gain appearance settings.

- **Model:** `Material` gains `appearance` (`#[serde(default)]`, so older files open with
  defaults).
  - Fields, in V-Ray's terms: reflection glossiness (stored as roughness), reflection,
    metalness, refraction and IOR, bump, texture, real-world texture size, tint, texture
    colour on or off (for painted brick and coloured plaster), coat and sheen.
  - All of these are editable in Properties under Appearance.
  - The built-in materials get sensible defaults: steel is metallic, masonry matte.
- **Library** (`studio_core::library`): 78 presets in 12 categories: Wood, Stone, Tile,
  Masonry, Concrete, Plaster & Paint, Metal, Glass, Fabric & Leather, Roofing, Surfaces &
  Ceilings and Site.
  - Each is tagged Typical, Mid-range or High-end, and Residential, Hospitality and/or
    Multifamily, with a description of typical use.
  - Each has a shading colour, cut and surface patterns for drawings, and its appearance.
  - `add_preset` adds one to the project (named uniquely).
  - `apply_to` sets the outside finish of the selected elements' types: a wall's exterior
    layer, a slab's or roof's top layer, a column's or beam's material. As in Revit, every
    element of that type changes.
- **Textures:**
  - 34 presets use Poly Haven CC0 photo sets (colour, OpenGL normal, roughness) at their
    measured real-world size. A few are corrected: shingles and shakes were listed far
    larger than real, and the green-cast concrete set is used for relief only.
  - Downloading:
    - The app fetches the 2K JPEGs from Poly Haven the first time a material renders, then
      keeps them in the app's local data folder (`studio_sync::textures`).
    - Only URLs the library itself names can be fetched.
    - Half-written or error files are never cached.
    - Without internet, a material renders in its colour and gloss alone.
  - Procedural textures are generated in the page: subway, hex (on a tile holding whole
    rows, so it repeats seamlessly) and marble mosaic, CMU, acoustic ceiling tile, standing
    seam, loop and patterned carpet, Carrara and Calacatta veining, granite, quartz, brushed
    metal and grass.
- **Rendering:**
  - Meshes carry their element's finish material (`Mesh.material`).
  - The renderer builds a MeshPhysicalMaterial per material: glass is thin and doesn't
    block the sun; coat is clearcoat; sheen is fabric sheen.
  - Textures use real-world box mapping, as V-Ray's box UVW map at real-world scale: each
    face projects along its dominant axis, and tangents are set for the normal maps.
- **Previews:**
  - A path-traced 500 mm rounded cube on a studio floor. Lighting is Poly Haven's
    studio_small_09 HDRI (bundled, 1.6 MB) plus two softboxes, so metals and gloss show
    crisp highlights. ACES tone, light denoise.
  - The 78 library previews ship as 256 px WebP (about 1 MB in all). They were rendered by
    a throwaway page that drove the same `renderPreview` in headless Chrome on the GPU.
  - Edited project materials render their own preview on demand, one at a time, cached
    for the session.
- **UI:** a Materials tab between Architecture and Rendering, with Material Browser, New
  Material and Duplicate (moved from Manage), and Apply Material.
  - The browser has Library and In This Project views, filters (category, building type,
    finish level) and search, cube thumbnails, and a detail panel showing the V-Ray
    settings.
  - Actions: Add to Project, Add & Apply to Selection, Apply to Selection, Edit in
    Properties, Duplicate.
- **Not yet:** textures in the shaded 3D view (the renderer only), per-face material
  painting, interior finishes of walls in renders (the exterior layer dresses the whole
  wall), and user-imported textures.

## ADR-030 Generate with Claude — Accepted (2026-09-25)
Owner request (2026-09-25): "a Claude input in the program where you can input a prompt…
per type of building, how many stories, references, etc… then it builds a 3D model in
Rufplan Studio per those inputs."

- **Flow:**
  1. Architecture > Generate opens the Generate with Claude dialog.
  2. The owner sets the brief:
     - building type: single-family house, duplex, townhouses, garden-style or mid-rise
       apartments, mixed-use, boutique or select-service hotel;
     - stories, and bedrooms, units or keys (with baths for houses);
     - optional area, style and roof;
     - "Fit on the lot" with front, side and rear setbacks, when a site is set;
     - text references, up to five reference images (scaled to 1568 px, sent as JPEG), and
       a free prompt;
     - model: Opus 5.5, or Sonnet 5 for speed.
  3. Claude plans; the dialog shows live progress (building name, stories and rooms so far,
     elapsed time).
  4. The model is built, and the dialog reports what was built plus any warnings, with
     Open 3D View and Revise & Generate Again.
- **Claude** (`studio_sync::claude`): one Messages API call that streams, with the
  `build_model` tool offered with `tool_choice: auto` (current models refuse a forced tool
  choice) and the system prompt requiring it; a reply in prose is shown as an error.
  - The API key is stored in the OS credential store (`anthropic-api-key`) and sent only
    to api.anthropic.com, from Rust.
  - Errors are explained: a refused key, a busy service or rate limit, a plan cut off at the
    length limit.
- **The plan** (`studio_core::generate::BuildingSpec`) is rectangular rooms per story in
  feet, each with a kind (living, kitchen, bedroom, bath, corridor, stair, unit,
  guest_room, lobby, retail…), plus roof type and pitch, structure (wood or masonry), and
  library materials by id (ADR-029).
  - The system prompt teaches the rules the builder relies on: tiling without overlaps,
    open-plan kinds, doors needing 4 ft of shared edge, stacked stair rooms, two stairs for
    multifamily and hotels.
  - It also gives typical US sizes: rooms, corridor widths, unit and key sizes,
    floor-to-floor heights.
  - Larger buildings model each unit or key as one room.
- **Building from the plan** (`generate::build`, one undo step through the new
  `Document::merge_undo`):
  - It replaces the current building's walls, openings, slabs, roofs, rooms, stairs and
    structure, reusing and renaming levels (Level 1…N and Roof).
  - It centres the building on the lot, or on the origin without one.
  - **Walls:** room edges are split at every corner along each grid line. One room on a
    side makes an exterior wall; two rooms make an interior wall. Between open-plan kinds
    there's no wall, only a room separator, so each room keeps its own area.
  - **Doors:** a cheapest-route search from the street entries (ground floor) or stairs
    (upper floors), preferring circulation; baths, closets and storage are dead ends. Every
    stair also gets a door onto a non-dead-end room, so no stair is a sealed shaft. Doors
    are 30" for private rooms, 36" otherwise, double at lobbies and garages.
  - **Windows:** on exterior walls of habitable kinds about every 7 ft — casements in
    bedrooms and kitchens, larger units in living spaces and units, storefront at lobbies
    and retail — kept clear of doors.
  - **The rest:**
    - stairs: straight, or U-shaped when the room is short;
    - one floor slab per story from the union of its rooms;
    - the roof: hip by footprint (L/T/U handled), gable over a rectangle, or flat;
    - rooms named as planned;
    - materials applied to the exterior walls', interior walls', floors' and roofs' types.
  - Problems become warnings rather than failures: overlapping rooms, a room with no route
    in, no room for a door. A failed build leaves the model untouched.
- **Verified:** tests cover a two-story house and three-story garden apartments (exact
  door counts, stacked stairs, gable slope, one undo). The house's plans were drawn and
  checked by eye. The live API call can't be tested without the owner's key; the stream
  parser is tested on recorded events.
- **Not yet:** refining a generated building by chat, non-rectangular rooms, curtain walls
  and balconies, parking and site work, and cost estimates.

## ADR-031 Window families — Accepted (2026-09-26)
Owner request (2026-09-26): "create families for all most common window types used in
America and make those as presets in Rufplan Studio." Owner approved the file-format
change and "starter set + library" (2026-09-26).

- **Families** (`WindowFamily`, code-defined as before, ADR-012): Fixed/Picture,
  Casement, Double-Hung, Single-Hung, Awning, Hopper, Horizontal Slider (XO), 3-Lite
  End-Vent Slider (XOX), Picture with Flanking Casements, Picture over Awning, Bay (45°,
  picture flanked by double-hungs) and Storefront. Transoms are small fixed types.
- **File format:** `WindowType` gains `units` (mulled side by side, 1–4; default 1),
  `grille` (None, Colonial, Prairie, Craftsman) and `finish` (White, Almond, Dark
  Bronze, Black, Natural Wood), all with serde defaults, so older files open unchanged.
  Files that use the new families don't open in older builds.
- **Layouts** (`studio_core::windows::layout`) divide a type into frame (2"), sashes
  (1.75" stiles and rails) in one or two tracks, mullions and mulls, and grille bars, seen
  from outside. Colonial is about 9" x 12" lites (6-over-6 on a 3' double-hung),
  craftsman is 3-over-1, prairie is bars 3.5" in from the edges. A bay projects
  min(18", width/4). Storefront is 1.75" x 4.5" aluminum in lites of 5' or less, with a
  transom bar at 7'-0" when taller than 8'-6".
- **Drawings** (`studio_views::windows`):
  - Plan: wall faces, glass per sash track (hung and sliding sashes stagger), mullions,
    casement swings at 30°, and a bay's projecting outline.
  - Elevation: sash and glass outlines, grilles, and operation marks (casement and awning
    marks point to the hinge, sliders get an arrow). Mirrored when seen from inside.
  - 3D: two meshes a window, the frame, sashes and muntins in the finish colour, and the
    glass. The 3D view and renderer tell them apart by colour (glass has none). A bay adds
    seat and head boards.
  - IFC: IfcWindow's PartitioningType follows the family (single, double or triple panel,
    else USERDEFINED with the family name).
- **Presets:** a catalog of 77 standard US sizes (nominal rough openings, heads at 7'-0"
  to line up with doors, transoms at 86", storefront on a 6" curb).
  - New projects get a starter set of 17, one or two per family. The original three keep
    their names.
  - Architecture > Load Windows opens the Window Library: families, size thumbnails
    (drawn from Rust's elevation lines), grille and finish, a custom size, and Load,
    Load Checked, or Load & Place (which arms the Window tool with the new type). Types
    already in the project are reused by name.
  - Type properties gain Units Mulled, Grille Pattern and Frame Finish.
- **Generate with Claude (ADR-030)** now also picks a window family, grille and finish to
  suit the style (`BuildingSpec.windows`). Its punched, living-space and storefront
  types are loaded from the catalog.
- **Not yet:** arched and round tops, garden windows, glass block, skylights, casement
  hand flip per instance, windows above or below the plan cut shown dashed, and per-type
  frame materials in renderings (the finish is a colour).

## ADR-032 Deliverable sheet sets per phase and building type — Accepted (2026-09-26)
Owner request (2026-09-26): "generate a set of deliverable documents and their relevant
sheets per phase and building type." Owner chose: a feature in Studio, US National CAD
Standard numbering, and consultant placeholder sheets.

- **Where:** `studio_sheets::sets`; View > Sheets > Sheet Sets opens the dialog.
  - Pick a building type: single-family, duplex, townhouses, garden or mid-rise
    apartments, mixed-use, hotel.
  - Pick the phases (the project's design stages, ADR-010) and the sheet size.
  - The dialog previews every deliverable and the sheet index with each sheet's contents
    and phases, then offers Create / Update Sheets and Export PDFs.
- **Deliverables per phase** use Rufplan's catalog ids (ADR-016):
  - PD: Program & Site Analysis (`pd-program`).
  - SD: 100% Schematic Design (`sd100`).
  - DD: 100% Design Development (`dd100`).
  - CD: 100% Construction Documents (`cd100`) and Permit Set (`permit`).
  - BN: Bid Set (`bid`).
  - CA: IFC — Issued for Construction (`ifc`).
- **Sheets** are numbered to the NCS (`G-001`, `A-101`) and listed in discipline order:
  G, C, L, S, A, I, F, P, M, E. `ops::sheet_cmp` now orders every sheet list and the sheet
  index; other numbers such as A1.0 follow them. A sheet belongs to several phases through
  `Sheet.stages` (ADR-015), so one A-101 serves SD through CA.
  - **Every type:**
    - G-001 cover and sheet index; G-002 code summary (IRC) or analysis (IBC); G-005 energy
      compliance.
    - Structural: notes, foundation, framing per floor group, roof framing, details.
    - A-001 program (PD, SD); A-100 site plan; A-101… floor plans, then the roof plan;
      A-151… reflected ceiling plans (DD on); A-201… elevations; A-301… sections.
    - A-311 wall sections; A-401 enlarged plans (named per type); A-451 interior
      elevations; A-501 details; A-601… door and window schedules and the room finish
      schedule; A-901 3D views (SD only).
    - Plumbing, mechanical and electrical.
  - **IBC types add:** G-003 life safety; G-004 accessibility (FHA and A117.1 for
    multifamily, ADA for hotels); civil (notes, site and grading, utilities, erosion
    control); landscape; A-421 stairs; fire protection (NFPA 13R for garden apartments,
    NFPA 13 otherwise); MEP notes and per-floor plans; a roof mechanical plan.
  - **Hotels add** interior finish plans and schedule. **Townhouses** get civil and
    landscape too.
  - **Consultant floors:** a building over four floors groups the middle ones as "Typical
    Levels 2–N".
  - **Phase membership** follows US practice: plans, elevations and sections from SD;
    RCPs, wall sections, enlarged plans, schedules, code and consultant plans from DD;
    details, life safety, accessibility, energy, interiors, fire protection and consultant
    notes from CD. Bid and CA sets equal the CD set.
- **Filled from the model:**
  - **Views:** plans (levels with walls; the level above takes the roof plan), RCPs,
    exterior elevations (south, north, east, west), sections, plan callouts and interior
    elevation marks.
  - **Scale:** views are packed in rows on the drawing area, left of the title block, at
    1/4" for IRC types and 1/8" otherwise. Elevations and sections go one scale smaller
    when that puts two or more on a sheet. A view too big for a sheet steps down to 1/16"
    and is centred, with a warning. Site plans use 1/8" or engineering scales.
  - **Sections:** two building sections are made through the middle when the project has
    none.
  - **Placeholders:** drawings the model doesn't make yet (details, wall sections, code
    sheets) and consultants' sheets get titled placeholder notes saying who provides them.
  - **Cover:** the project name, building type and address, plus the sheet index,
    top-aligned for the largest set.
- **The sheet index** now lists the current design stage's set, so each deliverable's
  cover indexes its own sheets.
- **Re-running** updates by sheet number:
  - The chosen phases' flags are set on each sheet, and other phases are left alone.
  - Empty sheets are filled; sheets with content are left as the user arranged them.
  - A view already on a sheet outside the sets moves in (reported). Older sheets it leaves
    empty drop out of the chosen phases' sets (reported).
  - One undo step.
- **Export PDFs** writes one PDF per deliverable into a folder, named "{number} - {project}
  - {deliverable}.pdf".
  - Each prints from a copy of the project set to that stage, so the title block shows the
    deliverable's stage and the index its sheets.
  - "Record as issued" also records an Issuance per deliverable in its stage (ADR-015).
- **No file-format change:** placeholders are sheets with text notes.
- **Verified:** tests cover packing and scale fitting, a house permit set (numbers, order,
  phases, deliverables), a six-story mid-rise (typical floors, life safety, 1/8" plans),
  creation (views placed, stage sets, one undo, re-run without duplicates), moving views
  from older sheets, and missing stages. The sample house's SD and Permit PDFs were
  rendered and checked by eye.
- **Not yet:**
  - Typical-floor architectural plans (one plan for identical floors).
  - Enlarged plans made automatically from rooms.
  - Consultant PDFs merged in place of placeholders.
  - Per-deliverable sheet differences (for example, Permit without the bid forms).
  - Commercial office and retail building types.

## ADR-033 Door families and rendered type pickers — Accepted (2026-09-26)
Owner request (2026-09-26): "do the same thing for doors that you did for windows… the most
widely used door types in USA including glass doors", with a picker "like the materials has
… a rendered thumbnail of the door in a 3D position … to place that door or edit the current
door if a door is already selected", and the same grid for windows instead of a dropdown.
Owner approved the file-format change (2026-09-26).

- **Families** (`DoorFamily`): Single Swing, Double Swing, Entry with Sidelites, Sliding
  Glass Patio (OX, OXO, OXXO), Pocket (single or double), Barn (surface sliding), Bifold,
  Folding Glass Wall, Storefront (aluminum, single or pair), and Garage (sectional
  overhead). The variants `SingleFlush` and `DoubleFlush` keep their names for the single
  and double swing families, so older files open unchanged.
- **File format:** `DoorType` gains the following fields, all with serde defaults:
  - `leaf`: Flush, Six-Panel, Shaker, Five-Panel, Craftsman, Full Lite, Half Lite, 15-Lite
    French, Vision Lite, Louvered or Barn X-Brace;
  - `panels`: leaves, sidelites or sliding/folding panels, where 0 means the family's
    default;
  - `finish`: Painted White, Black or Gray, Stained Oak, Walnut, Clear Aluminum or Dark
    Bronze; None means the family's default.

  Type properties show Leaf Style, the panel count and Finish.
- **Layouts** (`studio_core::doors::layout`) break each door into its parts:
  - jambs (1.5"), casings (3.5" on both faces), and the aluminum or vinyl frames of glass
    doors;
  - leaves, with 4.5" stiles, a 9" bottom rail, recessed panels, lites, muntins, louvers
    and braces;
  - sliding panels in two tracks, and sidelites with mullions;
  - the garage door's four sections of raised panels.
- **Drawings** (`studio_views::doors`):
  - **Plan:** swings and arcs (unchanged for flush doors), sidelite glass, staggered
    sliding panels, the pocket dashed in the wall with the leaf half open, the barn leaf on
    the face with a dashed track, bifold and folding-wall zigzags, and a garage door open
    overhead, dashed.
  - **Elevation:** frame, leaves, panels with a bevel line, glass filled, muntins, louvers,
    X-braces and slide arrows; mirrored when seen from the other side.
  - **3D:** frame, casing and leaves in the finish (panels recessed, garage panels raised),
    plus separate glass. As with windows, the glass mesh is the one without a colour.
  - **IFC:** OperationType follows the family (single or double swing, sliding, folding,
    swing with fixed panel), USERDEFINED otherwise.
- **Catalog:** 65 standard US sizes, at 6'-8" and 7'-0" heights (garages 7' and 8').
  - New projects start with 17, including the original three by name.
  - Generate with Claude puts an entry with sidelites at entries, a storefront pair at
    lobbies and retail, and 16' or 9' garage doors at garages, falling back to narrower
    doors where they don't fit.
- **The type picker** replaces the Window Library.
  - **Opens from:** Door and Window, their DR and WN shortcuts, Load Doors and Load Windows,
    and Properties > Browse Types….
  - **In This Project:** the project's types as rendered thumbnails.
  - **Library:** families and sizes as rendered thumbnails, with leaf style, panels, grille
    and finish options, and a custom size.
  - **Actions:** Place, Load & Place, or Change Selected when doors or windows are
    selected.
  - **How thumbnails are made:** Rust sends the type's triangles in a short piece of wall
    (`studio_views::thumbs`). three.js draws them with a studio environment and a raking
    key light that casts soft shadows, one at a time, cached for the session.
- **Verified:** tests cover layouts (panels inside rails, French muntins, the latch-side
  vision lite, OXXO tracks, centred entry doors, bifold pairs, garage sections, barn
  overlaps), names, the catalog, loading, plan symbols per family, elevation glass, 3D
  casings and faces, thumbnails and the picker flows. Plans, elevations and thumbnails of
  every starter type were checked by eye.
- **Not yet:** Dutch and revolving doors, transoms over doors, hardware, fire ratings in the
  schedule, and choosing the hinge side from the picker.

## ADR-034 Paint tool — Accepted (2026-09-26)
Owner request (2026-09-26): "when applying a material could you have some sort of icon modern
sleek paint once the material is selected from the grid to apply it to the component (wall
most likely) right now it doesn't do anything after selecting a material." Owner chose
per-element paint, with Shift-click for the whole type.

- **What was wrong:** Apply to Selection only showed when something was selected before the
  Material Browser opened, and 3D meshes never said which material they were made of, so
  renderings ignored library materials. Meshes now carry their surface material: the
  element's paint, else its type's outside finish.
- **Paint** (`studio_core::paint`): a material on one wall, floor, ceiling, roof, column or
  beam, without changing its type, as Revit's Paint tool does.
  - Stored as the element's `rufplan.paint` parameter. Element data is unchanged, and older
    builds simply ignore it.
  - Paint wins over the type finish in 3D colour, renderings and the elevation surface
    pattern.
  - A deleted material leaves no paint behind.
  - Properties show the paint with Remove Paint.
  - Paint is one undo step.
- **The tool:**
  - **Start it:** in the Material Browser, pick a material and click Paint (a library
    material is added first), or use Materials > Paint or PT, which repeats the last
    material or opens the browser.
  - **The cursor** becomes a paint roller, and a chip at the top of the view shows the
    material's swatch and name, with Change and Done.
  - **Clicking:** a click paints the element under the cursor, in plans, elevations,
    sections and 3D. Shift-click puts the material on the element's type (every element of
    that type, as Apply Material did). Esc or Done finishes.
- **Not yet:** painting one face of a wall (paint covers the whole element), and paint in
  plan poché.

## ADR-035 Revit round-trip through IFC — Accepted (2026-09-26)
Owner request (2026-09-26): "create a feature to export or import a revit model to work on."
Revit's .rvt format is closed. The owner chose an IFC round-trip over a Revit add-in or
Autodesk's cloud translation.

- **Import** (`studio_io::ifc_import`, File > Open IFC, and Open IFC (Revit) on the welcome
  screen) opens an IFC2x3 or IFC4 file as a new, unsaved project.
- **The reader:** a small STEP reader (`studio_io::step`) handles the DATA section, typed
  values and string escapes. There is no new dependency.
- **Model setup:**
  - Units (metres, millimetres, feet) and plane angles are read.
  - Placements are composed.
  - Models far from the origin (site coordinates) are recentred.
- **Storeys** become levels, with their names and elevations, and their plans are renamed.
- **Walls:**
  - The line comes from the Axis representation (else the footprint's long direction).
  - Thickness and height come from the body extrusion, followed through boolean clippings
    and mapped items.
  - The base offset comes from the storey.
  - Wall types are made per Revit type name and thickness, and IsExternal sets the wall
    function.
  - Curved walls come in straight (reported).
- **Doors and windows** go in their host walls through IfcRelFillsElement and
  IfcRelVoidsElement.
  - They are placed where their opening's body sits along the wall, at OverallWidth and
    OverallHeight, with the window sill from the opening.
  - Their facing comes from their placement, and the door hand from the operation type.
  - Types are named from Revit's family and type ("M_Single-Flush: 0915 x 2134mm"). The
    family comes from the operation type or name (double, sliding, folding, garage;
    casement, double-hung, slider…).
- **Slabs** become floors (thickness from the body), and roof slabs become flat roofs over
  their outline.
- **The rest:**
  - Spaces become rooms, with Revit's number and name.
  - Grids come in with their tags.
  - Columns come in at their centroid.
- **Reported, not brought in:** stairs, railings, curtain walls, furniture, MEP and the
  like.
- **Door and window stacking:** openings may now share a stretch of wall one above another
  (a clerestory over a window, windows on two floors of a tall wall).
  - The overlap check compares heights too.
  - The wall is cut in stretches between the openings' edges, so each stretch is solid
    except where openings cross it.
- **Export back to Revit** is the IFC4 export Studio already has (ADR-016). In Revit, File >
  Open > IFC, or Insert > Link IFC. The import report explains both directions.
- **Verified:**
  - Studio's own export round-trips: walls, a door at its place, a window, a floor, a room.
  - buildingSMART's Revit 2011 Duplex (IFC2x3) came in with 4 levels, 57 walls, 14 of 14
    doors, 22 of 24 windows (the other two are roof skylights), 20 floors, a roof and 21
    named rooms, in 0.13 s. Its plan and 3D were checked by eye.
  - The Revit Clinic came in with 1,080 walls, 244 doors (the other 10 are curtain-wall
    doors), 58 windows and 269 rooms, in under 1 s.
- **Not yet:**
  - Sloped roofs and stairs.
  - Curtain walls, and room separators from spaces. Rooms Revit separates without walls
    share one area.
  - Materials and colours.
  - Merging an IFC into an open project.

## ADR-036 Plans to 3D — Accepted (2026-09-26)
Owner request (2026-09-26): "have a function to create a 3D Model from a set of floor plans you
input. you can test this with fallingwater". The owner chose images and PDFs as input, which
approves one new dependency: `pdfjs-dist` (Mozilla, Apache-2.0), loaded on first use.

- **Input:** Architecture > Generate > Plans to 3D.
  - Up to 12 sheets: JPG/PNG images, or PDF pages drawn with pdf.js.
  - Each sheet's level name is guessed from the file name ("second floor") and can be edited,
    with an optional floor elevation.
  - Notes (scale, heights, materials) and the model (Opus 5.5 or Sonnet 5) are also given.
- **Images** are sent with their long side at most 1568 px, the size Claude reads, so its pixel
  coordinates are the image's.
- **pdf.js worker:** the CSP allows workers from `blob:` only, so the worker's source is
  bundled as text (`?raw`) and started from a Blob.
- **The reading** (`plans_to_model`, tool `read_plans`, streamed like ADR-030 with
  progress). For each sheet, Claude returns:
  - the level, elevation and wall height;
  - the scale as `pixelsPerFoot` (from the scale bar, a dimension, or a known size);
  - an anchor point (pixel and feet) on a feature shared by every floor, so the floors stack;
  - wall centerlines with thickness and exterior/interior;
  - doors and windows (point, width, height, sill, kind);
  - room names at points, and slab outlines.
- **The build** (`studio_core::plans::build`) happens in one undo step:
  - Pixels become feet by the sheet's scale and anchor.
  - Walls are cleaned: squared within 3°, ends snapped within 9", and T joints closed.
  - Levels are made per sheet, plus a Roof level, and the model is centred.
  - Wall types are made by thickness and function.
  - Each opening is hosted in the nearest wall, searched in this order:
    1. a wall it lies along, within 3 ft;
    2. a gap between two lined-up wall ends, closed with a wall like its neighbour's (Claude
       often traces doorways as gaps);
    3. a wall end nearby.
  - Door and window types come from the ADR-031/033 catalogs, matched by kind and size.
  - Slabs become floors, as read, else the walls' outline.
  - Rooms are placed by point and named.
  - A roof goes over the top floor.
- **Verified** on the Historic American Buildings Survey drawings of Fallingwater (Library of
  Congress, public domain), sent as four plan sheets:
  - Opus read them in 77 s: 4 levels, 82 walls, 32 rooms, 8 floors and a roof.
  - The first read placed 5 doors; with gap closing, 8 doors and 6 windows.
  - The 4 doors not placed sit on terraces with no wall within 3 ft; each is reported.
  - The plans and 3D were checked by eye: the living room, kitchen, terraces, loggia and bridge
    are where they belong. The scale read (8.9 px/ft) matches a 34x44 sheet at 1/4".
- **Not yet:**
  - Curved walls, stairs, and sloped roofs read from the plans.
  - Sections and elevations as input, for heights.
  - Reading at more than 1568 px by tiling large sheets.
  - A trace overlay to correct Claude's reading before building.

## ADR-037 ViewCube — Accepted (2026-09-26)
Owner request (2026-09-26): "create a 3D navigation cube … for 3D views … design it after
Revit's".

- **Where:** every 3D view and camera view has the cube in the top-right corner. It turns with
  the model, and is half see-through until the pointer is over it. The Ground Grid and
  Satellite chips moved to the bottom-right to make room.
- **The cube:**
  - Grey faces labelled FRONT (south), BACK, LEFT (west), RIGHT, TOP and BOTTOM.
  - Each face has 9 hotspots: its middle, 4 edges and 4 corners, 26 in all. Hovering one
    highlights it in blue, with its name as a tooltip ("Top, Front, Right").
- **Clicking a hotspot** turns the view to look from there. The turn is animated, and the view
  fits the model (the section box when on), as Revit's defaults do.
  - Top puts north up the screen.
- **Dragging the cube** orbits the view.
- **The compass ring** under the cube carries N, E, S and W.
  - Dragging the ring turns the view about the vertical.
  - Clicking a letter looks from that side at the same height.
  - The ring fades out as the view comes level, where it would sit edge-on.
- **Straight at a face,** four arrows lead to the faces above, below, left and right of it on
  screen.
- **Home** (the house above the cube, and Go Home in the menu) goes to the view's home. That is
  the view's first fit, until Set Current View as Home saves one. Reset Home clears it.
  - Home is kept per view on this computer (local storage). The file format is unchanged.
- **The menu** (the arrow at the cube's corner, or a right-click) has:
  - Go Home, Set Current View as Home, Reset Home;
  - Fit to View, Orient to Top, Orient to Southeast.
- **ZF** (Zoom to Fit) now also fits in 3D.
- **How it's drawn:**
  - The cube is its own small three.js scene with an orthographic camera that copies the view
    camera's rotation.
  - The view's renderer draws it into its corner after the model, so there is no second
    WebGL context.
  - An HTML layer over the corner takes the pointer, so orbiting the model and using the cube
    don't clash.
- **Not yet** (Revit has these):
  - The roll arrows: the orbit keeps the scene upright.
  - Lock to Selection, Set Current View as Front, and perspective/orthographic switching.

## ADR-038 Visual styles, seamless walls, floors to the outside of walls — Accepted (2026-09-26)
Owner requests (2026-09-26):
1. "default the floor to extend to outside edge of wall instead of inside".
2. "when in 3D view you can see the joint lines after a window or door are placed … join the
   form so that these seams don't show".
3. "a toggle button between materials for 3D view (wireframe, hidden line, shaded,
   consistent colors, realistic) … in lower left corner". The owner handed off design 1F
   ("expands on hover") with its icons and specs.

- **Floors reach the outside of exterior walls.** In a floor sketch, Pick Walls on an
  exterior wall picks its outer face, whichever side the cursor is on
  (`sketch::pick_floor_walls`).
  - The outer face is the one outside the closed chain the wall belongs to, else its
    exterior face.
  - Interior walls still take the face on the cursor's side.
  - Ceilings are unchanged.
  - Floor by walls (click a wall) already followed the outer faces.
- **Walls without seams.** A wall's solid is split in pieces around its openings. Drawn as
  they were, the pieces showed seams in two ways: lines found from their triangles, and the
  faces between them flickering through the surface. Now:
  - `studio_views::edges::wall_triangles` leaves out every face where one piece meets
    another. Only the uncovered parts remain, so an opening's jambs stay.
  - `wall_edges` gives the wall's own lines: its outline, with no corner line where the
    outline runs straight on (a T join), and each opening's outline on both faces and through
    the wall.
  - `Mesh.edges` carries these lines (6 floats per segment). It is empty for other elements,
    whose lines still come from their triangles.
  - Stacked walls on two levels still show the line between them, as in Revit.
- **Visual styles**, per 3D view (Shaded until chosen; kept for the session, the file is
  unchanged):
  - **Wireframe:** lines only, hidden ones too. The faces aren't drawn but can still be
    picked, and selected lines turn blue.
  - **Hidden Line:** white faces with black lines.
  - **Shaded:** material colours, lit, with lines, as before.
  - **Consistent Colors:** the same colours unlit, with lines.
  - **Realistic:** the project's physical materials (ADR-029) with their textures, by
    real-world box mapping.
    - Lighting: ACES tone mapping, an image-based room environment, and a sun from the
      southeast casting soft shadows.
    - Setting: a sky, a matte ground where there's no topography, and the grid at a quarter.
    - No lines.
- **The toggle** (design 1F) sits in the 3D view's lower-left corner.
  - At rest it shows the current style's icon. Hovering or focusing it opens all five, from
    Wireframe to Realistic.
  - The chosen one is filled light grey, with no accent colour.
  - It is a radio group (arrow keys move between styles); on touch, a tap opens it.
  - The View tab's buttons and the WF, HL and SD shortcuts set the same style. Consistent
    Colors and Realistic are in the Keyboard Shortcuts dialog without keys, as in Revit.
  - The design's 1–5 keys were left out: digits already start a typed length.
- **Also moved:** the satellite credit moved right of the pill.
