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
