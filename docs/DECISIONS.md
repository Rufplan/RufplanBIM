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

## ADR-039 Activate View on a sheet, stretchable view titles — Accepted (2026-09-26)
Owner requests (2026-09-26):
1. "double click within the view in the sheet and be able to access the view and draw in the
   view similar to how Revit works".
2. "have the view title line be stretchable … however long you want it on the sheet".

The owner approved the file-format change for (2).

- **Activate View:**
  - Double-clicking a plan, ceiling plan, elevation or section viewport on a sheet activates
    it. Double-clicking a schedule or 3D viewport opens its view.
  - The view is then worked in right there: selection, every drawing and editing tool, its
    properties and its ribbon.
  - The view stays where the sheet showed it, at the sheet's scale. The paper-to-model mapping
    is the one `studio_sheets::viewport_items` places views with.
  - The rest of the sheet shows in halftone. Panning and zooming move the sheet with the view.
  - It ends by double-clicking outside the view, with Deactivate View in the banner, or by
    opening another view.
  - The sheet's tab stays the active one. `activeViewInfo` gives the activated view, so the
    ribbon, the tools and the properties follow it.
  - A `viewport_info` command gives a viewport's sheet, view and center.
- **Stretchable view titles:**
  - A selected viewport shows a grip at the end of its title's heavy rule. Dragging it
    sets the rule's length (`Viewport.title_length`, paper mm, at least 12 mm).
  - The length applies on the sheet, in its PDF and in issued sets. Undo puts the length back.
  - Viewports without a length, including every file saved before, keep the rule fitted to
    the title.
  - The field is optional in the file (`serde(default)`, skipped when unset).
  - A grip sits at each end of the rule. Either one stretches the rule left or right only;
    the left grip carries the bubble, name and scale with the rule's start. With Shift held,
    a grip moves the whole title instead.
  - With the viewport selected, dragging the title itself (its bubble, name or rule) moves it
    anywhere on the sheet, keeping its length. A dashed outline shows where it goes.
  - Its position is `Viewport.title_offset`, paper mm from its place under the view, also
    optional.
- **Verified:**
  - A headless render of the sample house's A1.0: the activated Level 1 plan sits exactly where
    the sheet drew it, and the Level 2 plan and title block are faded.
  - Unit tests cover the camera mapping, the title's grip, stretch, least length and undo, and
    old viewports reading without the field.
- **Not yet** (Revit has these):
  - The activated view's own title hides while it's active.
  - Activate View from the View tab or a right-click.

## ADR-040 Revit-style dimensions — Accepted (2026-09-26)
Owner request (2026-09-26): "update dimensions to be more like Revit's where it has angled,
aligned, etc. and make them default snap to gridlines, face of wall, centerline of wall".

The owner approved the file-format additions and chose Wall centerlines as the default
Prefer.

- **References** (`studio_core::dimension::references_at`). In plans, the dimension tools
  pick:
  - a wall's exterior or interior face, its centerline, and on layered walls its core faces
    and core centerline;
  - grid lines;
  - points: endpoints, intersections and midpoints from the snaps.
- **Picking a reference:**
  - The nearest line within reach wins.
  - Inside a wall but at none of its lines, the **Prefer** option (Options Bar: Wall
    centerlines, Wall faces, Center of core, Faces of core) decides, as in Revit.
  - **Tab** steps through the other references under the cursor.
  - The reference under the cursor is highlighted cyan, the picked ones stay cyan, and its
    name shows in the status bar.
- **Aligned (DI):**
  - Pick references one after another; the preview string follows the cursor, and a click in
    empty space places it.
  - Lines must be parallel to the first (within 1°). The string then measures across them,
    one value per segment.
  - Picking only points measures point to point, as before.
- **Linear:** pick points or references. The dimension is horizontal or vertical: horizontal
  when the cursor is above or below the picks, vertical when beside them. It can be a string.
- **Angular:** pick two walls or grid lines, then click inside the angle to measure. The arc
  goes in the angle that holds the cursor, with extension lines out to the arc, a tick at each
  end, and the value in degrees (`45.00°`).
- **Following the model.** Every reference is anchored to its wall or grid, so the dimension
  follows it:
  - moving or stretching the wall or grid;
  - copying or mirroring it with its elements (the copy re-anchors to the copies);
  - splitting or flipping walls.
  - A grip moves a string's line; an angular dimension's grip moves its arc.
- **File (additive, optional):**
  - `Dimension.between` holds a string's middle references (`DimRef`).
  - `Dimension.along` is the fixed measuring direction.
  - `Dimension.kind` is Aligned or Linear.
  - The new `AngularDimension` element holds each line's point, direction and anchor, and a
    point on the arc.
  - Older dimensions are 2-point aligned ones and draw as before.
- **Properties:**
  - Value (the total), and the Segments of a string.
  - Type (Aligned, Linear horizontal or vertical, Angular).
  - How many references follow the model, and the line's offset.
- **Verified:** unit tests cover:
  - the references at faces, centerlines and grids, and Prefer from inside a wall;
  - parallel-only strings and their values, following a moved wall and a copy;
  - linear horizontal/vertical by the cursor;
  - angular's angle and supplementary side, following walls, and parallel lines refused;
  - one value per segment, and the degrees.
  - A headless render of the sample house showed a string across its walls, a vertical linear
    and an angular dimension.
- **Not yet** (Revit has these):
  - Radial, diameter and arc-length dimensions (no curved walls yet).
  - "Entire walls" picking (openings and intersecting walls in one pick).
  - EQ constraints, and text moved off crowded segments.
  - Line references in elevations and sections (points only there).

## ADR-041 Typing dimensions moves the selection — Accepted (2026-09-26)
Owner request (2026-09-26): "when you select a wall … allow to then select the dimension
numbering and then it updates per that dimension number you entered, just how Revit does it".

- **Temporary dimensions.** A selected wall or grid in a plan shows Revit's temporary
  dimensions to the nearest parallel wall or grid on each side that runs alongside it,
  centerline to centerline. They sit beside the wall's length dimension.
  - Click the value, type a distance, and the selection moves square to itself to that
    distance (`dimension::set_distance_to`).
  - Joined walls stretch to follow, as with Move.
- **Permanent dimensions.** Every value of a permanent dimension in the view whose segment
  ends on the selected wall or grid gets a box to type into.
  - Typing moves the selection along the dimension so the segment measures the new value; the
    segment's other end stays put (`dimension::set_dimension_segment`).
  - A segment with both ends on the selection is refused, with a hint to change the thickness.
    So is one with neither end on it.
- **Wiring.** Both go through `set_temp_dimension` with keys `to:<other>` and
  `dim:<dimension>:<segment>`. They are computed in `studio_views::handles`, which places
  the boxes over the dimension's own text (`segment_labels`).
- **Not yet** (Revit has these):
  - Moving a door or window by a permanent dimension.
  - Temporary dimensions to wall faces (Revit's Temporary Dimension settings).
  - Dragging a temporary dimension's witness line to another reference.

## ADR-042 Dimension points follow the crossing wall; Revit's Align — Accepted (2026-09-26)
Owner requests (2026-09-26):
1. "the dimension should only increase or decrease on the side of the dimension string the
   wall is on. right now it shifts the whole dimension over".
2. "the align tool … should act more like Revit's align tool and default to selecting to the
   face of something or the gridline … right now it has like a nearest point".

- **Cause of 1.** A dimension point (as the sample house's two-point dimensions have, and as a
  snapped point in a string does) was attached by `anchor_at` to whichever wall line it sat
  on, stored as a fraction along that wall. At a corner or a T that was often the wall
  running *along* the dimension. Moving or stretching that wall slid the point, so the
  dimension's other end moved too.
- **Fix: `dimension::anchor_across`.** A point now follows the wall line (face or
  centerline) or grid that crosses the dimension there, as Revit's references do.
  - Failing that, it follows a wall end it sits at.
  - Failing both, it stays put.
  - Two-point dimensions (`ops::create_dimension`) and the points of Aligned and Linear
    strings attach this way.
  - Typing a segment now moves only the selected wall's end of it.
  - Checked on the sample house: each wall's typed segment changes on its side only.
  - Dimensions already saved in a project keep their old attachment. Redraw one if it still
    shifts.
- **Align (AL)** now picks references as the dimension tools do (`references_at`):
  - Wall faces, centerlines and core lines, and grid lines, highlighted in cyan under the
    cursor, with Tab to cycle. The Nearest-point snap marker is gone.
  - Its **Prefer** option defaults to **Wall faces**.
  - The second pick offers only lines parallel to the reference on another element. The
    element moves so that line lies on the reference (`dimension::align`,
    `align_references`).
- **Not yet** (Revit has these): Align's lock padlock, Multiple Alignment, and aligning doors,
  windows or text.

## ADR-043 3D zooms toward the cursor — Accepted (2026-09-26)
Owner request (2026-09-26): "when you zoom in within the 3D view it just zooms from the center
point of the screen … update so it zooms from where your cursor is pointing or what's
selected".

- **Zoom toward the cursor.** OrbitControls' `zoomToCursor` is on, so the wheel zooms toward
  the cursor rather than the middle of the screen.
- **At the right depth.** On its own, zoomToCursor zooms at the depth of the orbit center,
  which crawls toward far objects and overshoots near ones. So before each wheel step (a
  capture-phase listener that runs before OrbitControls'), the orbit center slides along the
  line of sight to the depth of the model under the cursor (`pivotAt`).
  - The view doesn't move; the zoom heads straight at what is pointed at.
  - Geometry outside an active section box is ignored.
- **The selection.** With nothing under the cursor, the selection's center sets the depth.
- **Orbiting.** Pressing to orbit sets the orbit depth the same way, so the model turns about
  what is under the cursor.
- **Not yet** (Revit has this): Revit orbits exactly about the selected element, even off the
  line of sight. OrbitControls can only orbit about a point on it.

## ADR-044 Section box caps — Accepted (2026-09-26)
Owner request (2026-09-26): "when [the section box] is cutting through a building [it] has
these planes and blank spaces between the assemblies. could you fill those to look like it's
cutting through something? … mimic as much as possible how Revit does it".

- **The problem.** Clipping planes only hide what's outside the box, so a cut wall or floor
  was left hollow.
- **Computed in Rust** (geometry belongs there). For each face of the section box and each
  element crossing it:
  - its triangles are cut by the plane and the cuts are chained into loops
    (`studio_geom::mesh_section`);
  - the solid region (even-odd, so holes stay open) is clipped to the face's rectangle
    (`even_odd_in_rect`, geo booleans) and triangulated.
  - `studio_views::caps::section_caps` returns each cut's fill, its outline, and for walls
    the boundaries between their layers inside it.
- **Drawn as Revit draws them:**
  - The fill is a deep tone of the element's colour, reading as poché against its lit faces.
  - A heavy cut line (2.2 px) runs around each cut; layer boundaries are thin lines inside
    walls.
- **Per visual style:**
  - Hidden Line leaves the fill white; Consistent Colors shows it flat.
  - Wireframe shows only the cut lines.
  - A selected element's cut turns blue, and a hidden element's cut hides with it.
- **Updates.** Caps follow the model and the box. While a face of the box is dragged they hide,
  and they come back when it's set.
- **Verified:**
  - Unit tests cover a box's cross-section (area, clipping, a miss) and a section box through
    four layered walls: the right walls capped, a mitered cap's area, cut lines, layer lines,
    and vertical caps.
  - A headless render of the sample house cut at 4'-3" shows its wall tops, floor edge and
    stair landing filled with heavy cut lines.
- **Not yet** (Revit has these): material cut patterns (hatches) on the fill, and caps for
  topography.

## ADR-045 Terrain as a toposolid — Accepted (2026-09-26)
Owner request (2026-09-26): "the site topo that comes in is on a plane, could you make it so
there's a small icon that toggles … so the topo looks like it has depth below it like it's
cut out of the earth like Revit does. also add contour lines and be able to control the
frequency … as well as label them, but most important add the depth".

- **Earth depth (Revit's toposolid).** The ground becomes a block
  (`SiteSolid::skirt`).
  - Its sides drop straight down from every edge of the topography, to 10' below the lowest
    ground (`TERRAIN_DEPTH`), and it has a bottom.
  - The sides are soil in faint strata with a cut outline, so the terrain reads as cut out
    of the earth.
  - Section boxes cap the block like any solid (ADR-044).
- **Contours in 3D.**
  - They lie on the ground (lifted 25 mm so they don't flicker), from the site's contour
    interval.
  - Every fifth is heavier, as in the site plan.
  - Each contour's elevation labels the middle of its longest run: `412'`, or feet-inches
    when the interval isn't whole feet.
  - Labels keep a steady on-screen size, like annotation text. With more than 60 contours,
    only the heavier ones are labelled.
- **Toolbar.** Only when the project has a site, the 3D view shows a small pill above the
  Satellite and Ground Grid chips with:
  - an Earth depth toggle;
  - a Contours toggle and a Labels toggle;
  - the interval (6", 1', 2', 5', 10', 20').
  - Choosing an interval sets the Site's Contour Interval, so plan contours change too, and
    it can be undone.
  - The toggles are per session; earth depth, contours and labels start on.
- **Visual styles:** Hidden Line shows the block white with its outline; Wireframe shows only
  its outline and the contours.
- **Not yet** (Revit has these): a toposolid thickness per type, and subdivision or grading of
  the terrain.

## ADR-046 Site plan contours and a grids toggle — Accepted (2026-09-27)
Owner request (2026-09-27): "add the topography contours to the site plan as well and then
also make a toggle to turn on and off the gridlines in the site plan".

- **Contours as a survey draws them.** Site plans already drew contours (ADR-023), as thin
  lines with small labels on every fifth only. They were easy to miss, especially over the
  satellite image. Now (`site_plan::contours`):
  - Minor contours are thin and every fifth is heavier (pen 3).
  - Each contour is labelled with its elevation (`412'`, as in 3D, ADR-045), read along the
    contour and kept upright, in a paper-white gap that breaks the line around it.
  - With more than 25 contours, only the heavier ones are labelled.
  - The Site's Contour Interval (the 3D terrain toolbar's interval) sets their frequency, so
    plan and 3D match.
- **Grids toggle.** Site plans show a Grids chip in the lower right that hides or shows the
  gridlines in that view. It is the same as unchecking Grids in Visibility/Graphics, so it is
  saved with the view and can be undone.
- **Not yet:** Revit's Label Contours tool (a label where you pick, several along a contour).

## ADR-047 Standards tab — Accepted (2026-09-27)
Owner request (2026-09-27): "create a standards tab at the beginning of the tabs and
reference these documents to start" (handoff: README, `Rufplan Standards Tab.dc.html`, whose
seed has 14 categories and 90 standards).

- **What it is.** The office's drawing-set standards (sheet setup, symbols, tags, text,
  linework, material graphics, phasing, dimensioning, numbering, schedules, keynotes, scales,
  BIM, issuance). Each standard has an office value and a status (defined or open), so the tab
  is also a checklist of how complete the set is.
- **Saved in the project** (owner's choice). One `Standards` element (`ElementData::Standards`,
  category Standards) holds the library name and the categories. It is created on the first
  edit; until then the default is shown.
  - Every edit is a transaction, so it can be undone.
  - This adds an element kind to the .rfproj; older files open unchanged.
- **Libraries** (owner's choice: derive from the default). There are three: "Rufplan Default
  (NCS 6)", "Residential" (ARCH C sheets, 1/4" plans…) and "Preservation" (historic fabric,
  SOI treatments, HSR/SHPO issues…). The last two are the default with values changed.
  Loading one replaces the project's standards and can be undone.
- **UI**, following the handoff:
  - "Standards" is the first ribbon tab, with a button per category grouped as SHEETS,
    ANNOTATION, GRAPHICS, DATA, VIEWS and OUTPUT, and the LIBRARY group's Office Standard
    select.
  - The tab replaces the Project Browser, views and Properties with:
    - the Standards Browser (counts per category, SET COMPLETE);
    - the category's checklist (ALL / UNDEFINED filter; the checkbox toggles defined);
    - Properties (preset select plus a value box; a non-empty value defines the standard; a
      Status select; APPLIES TO).
  - The status bar shows "N of M standards defined".
  - Starting a tool (a shortcut such as WA) goes back to Architecture, since tools need the
    views.
  - A different project opens on Architecture.
- **Not yet:** standards don't drive anything. For example, Sheet Size doesn't yet set new
  sheets' title blocks. Office libraries can't be saved from a project to reuse.

## ADR-048 Standard choices, spot elevations, north arrows, graphic scales, key plans — Accepted (2026-09-27)
Owner request (2026-09-27): "create a pop up for each standard and to see choices of how to
change it, both list and grid versions controlled by a toggle in top right, also add spot
elevations, key plan and north arrow, and graphic scale". The owner chose Revit-style tools and
approved the new element kinds.

- **Choices.** `standards_catalog` offers 2–5 choices, each a label and a line on what it
  means, for every one of the 90 standards. Every default and library value is among them.
  - A standard's preset options now come from the catalog, not the file.
  - The pop-up opens by clicking a row's value, double-clicking the row, or Properties >
    Choices…
  - A toggle in its top right shows the choices as a grid (previews) or a list (small
    previews). The toggle is remembered on this computer.
  - It marks the current choice. Use this (or a double-click) saves the choice as the value,
    and it can be undone. Custom values are still typed in Properties.
- **Previews.** 30 graphic standards draw a preview of each choice in Rust
  (`standards_preview`, paper mm):
  - symbols, tags, dimension ticks, sheet sizes, the key plan and phasing;
  - spot elevations, north arrows and graphic scales use their real drawing code.
  - Other standards show their text.
- **Symbols (new element kinds; older files open unchanged).**
  - *Spot Elevation* (plans, ceiling plans, elevations, sections): click the point, then
    where the text goes (the same point: no leader). It reads the model and follows it:
    - in plans, the top of the floor there (at or below the 4'-0" cut plane), else the
      ground (site topography), else the level;
    - in ceiling plans, the ceiling;
    - in elevations and sections, the point's height.
  - *North Arrow* (plans and sheets). It shows project north and, from the Site's rotation,
    true north.
  - *Graphic Scale* (plans, elevations, sections). It is divided 0 · u · 2u · 4u, with u
    picked so the bar is at most 60 mm on paper (1/8" gives 0 · 4 · 8 · 16'). It follows the
    view's scale.
  - *Key Plan* (sheets). It shows the building's outline, with the area the sheet's plans
    show shaded (their crop boxes; an uncropped plan shades the whole building), a north
    arrow and a label.
  - Spot elevations, north arrows and graphic scales count in a view's extent (Zoom to Fit,
    viewports).
- **The standards drive the symbols.** Each symbol's style is the index of its standard's
  choice. An open or custom value uses the first choice.
  - Spot Elevations: triangle with the project elevation; target relative to 100'-0"; cross
    with the survey elevation (the site's datum, decimal feet); text only.
  - North Arrow: project north with a TN line; circle; half-filled; compass rose; true north
    only.
  - Graphic Scale: alternating bar; line with ticks; double checkered; bar with the scale
    written.
  - Key Plan & North Arrow: title block key plan with north arrow (as before); placed key
    plans (the title block leaves it out); a north arrow only; none.
- **Shortcuts.** Annotate > Symbol has the four tools, and they are in Keyboard Shortcuts.
  Revit's EL is already our Elevation (ADR-024), so Spot Elevation has no default key.
- **Not yet:** other standards (text, linework, tags…) don't drive drawing yet. There are no
  per-instance style overrides, and no spot coordinates or spot slopes.

## ADR-049 Spot slopes and sloped floors — Accepted (2026-09-27)
Owner request (2026-09-27): "add a spot slope annotation tool similar to how Revit has it for
Roofs, sidewalks, ramps, etc." Floors were always level, so sidewalks and ramps had no slope to
read. The owner approved a Spot Slope element and a slope on floors (both change the file; older
files open unchanged).

- **Sloped floors** (Revit's slope arrow, as two properties):
  - *Slope*: typed as 2%, 1:12, 1/4"/12", 6/12 or 5°.
  - *Slopes Down Toward*: one of eight directions.
  - The floor's top is at its level plus offset along its highest edge and falls from there.
  - Regeneration gives the slab a `Tilt`:
    - 3D, and the section box caps made from it, use the sheared prism (planar faces);
    - sections cut it as a parallelogram with its layer lines;
    - elevations draw its faces;
    - spot elevations read its height where clicked.
  - IFC writes a sloped floor as a tessellation, as roofs are written.
- **Spot Slope** (Annotate > Symbol; plans, elevations, sections): one click on the surface.
  It reads the model and follows it:
  - *In plans*, it reads the highest surface there: roofs on that level, floors at or below
    the cut plane, else the ground (the site topography's gradient). The arrow points downhill.
  - *In elevations and sections*, it reads a sloped edge within 3 mm (on paper) of the click:
    - a roof face or sloped floor cut by the section, or seen nearly edge-on (within about 15°);
    - the ground where a section cuts it.
    The arrow sits just above the edge, or it can be a slope triangle (Representation), with
    legs labelled 12 and the rise, 100 and the percent, or the run and 1.
  - Clicking where nothing slopes is refused with a message. If the model later changes under
    it, it reads NO SLOPE so it can be found.
- **Formats** (Properties > Slope Format): Auto, Rise / 12" (6" / 12", 1/4" / 12"), Percent
  (2.00%), Ratio (1:12) and Degrees.
  - Auto writes roofs as rise over 12".
  - It writes floors at 1:20 or steeper (ramps) as a ratio, gentler floors (sidewalks) and
    the ground as a percent.
- **Not yet:** slope arrows drawn in sketch mode, floors that warp (more than one slope),
  sloped ceilings, stair and ramp families, and a default key (Revit's SS here turns snap
  overrides off).

## ADR-050 Edit Model with Claude — Accepted (2026-09-27)
Owner request (2026-09-27): "create an edit model button that is tied into Claude and the
model. So you can add a prompt like change all doors to be 3'-0" or update building to be
50'-0" Wide … and the building model will respond", following the handoff (EDIT MODEL pill
and modal, README and prototype).

- **Claude proposes; Rust decides.** Claude gets:
  - the prompt;
  - a summary of the model (`model_edit::describe`): levels, the building's size, and for
    each category its count, types and every writable property with its current values;
  - the active view and the selection.
  It answers with one structured edit through a forced tool call (`edit_model`), using
  Opus 5.5 and the key in the OS credential store. studio-core checks the edit against the
  model: the category exists, the property is writable, the value parses and a choice
  exists. It then previews the edit on a copy of the document, so nothing changes until
  Apply.
- **Edits**
  - `set_parameter`: any property the Properties panel can write, for one category.
    - Scope: the model, a level, the selection, or what the view shows. An optional type-name
      filter narrows it further.
    - A type property (a door's Width) moves each instance to a type with that value, as you
      would in Revit. It reuses a type that already matches, or makes one named for its size
      ("Single Six-Panel 36" x 80"").
    - The history's summary and the preview's count are computed, not taken from Claude.
      Elements that already have the value aren't counted.
  - `resize_building`: overall width (east–west) or depth (north–south), outside face to
    outside face. It is Revit's Stretch across the building's middle (`modify::stretch`):
    - everything past the middle moves: walls' ends, grids, slab and roof outlines, rooms,
      columns, beams, railings, stairs, section lines, plan annotations;
    - walls that cross the middle get longer;
    - doors and windows stay where the stretch puts them.
    The anchor side (west or south by default, or east, north, center) stays put.
  - `none`: Claude says why, and suggests what would work; the dialog shows it.
- **One undo step.** Apply runs the edit and folds its transactions into one undo step
  (`Document::merge_undo`) named "Edit model: summary". A failed edit rolls back.
- **UI** (per the handoff):
  - The EDIT MODEL pill sits at the lower right of plan views (floor, ceiling, site),
    elevations, sections and 3D views (elevations and sections added at the owner's request,
    2026-09-27). The 3D chips and terrain bar move up above it.
  - Claude is told the active view's kind. In a building elevation, "view" scope ("the
    windows on this elevation") is the facade seen face-on in front and its openings, not
    the far facades drawn behind it or the side walls seen edge-on. In sections it is what
    the section shows.
  - ⌘K / Ctrl+K opens and closes it there.
  - The dialog closes and drops its preview when the view changes to one it isn't offered
    in (sheets, schedules, an activated viewport, the Standards tab, sketch mode).
  - It has history, a preview card with the changed elements highlighted like a selection,
    the error line, suggestion chips, and PREVIEW CHANGE, or CANCEL / APPLY when a preview
    is showing.
  - History UNDO/REDO acts on the app's undo stack. Only the step at the top of that stack
    can go from the history (the undo stack is linear); Ctrl+Z works as always.
- **Not yet:**
  - several changes in one prompt;
  - geometry beyond overall size (moving a wall, adding rooms; Generate still builds whole
    buildings);
  - filters by other parameters ("the doors narrower than 3'").

## ADR-051 Edit Model plans: any change to the model — Accepted (2026-09-27)
Owner request (2026-09-27): "make the prompt change anything in the model … I should be able to
type anything and it should do it, including creating new elements, editing existing ones,
adding new items, annotating, etc."

- **A plan, not one edit.** Claude now answers with a plan (`studio_regen::model_ops::ModelPlan`):
  a list of operations run in order. Each operation calls the same function the app's tools
  do, so the model stays consistent (joins, hosting, bound floors, tags, dimensions anchored
  to walls).
  - **Create:** levels, grids, walls (type, level, top level or height), doors and windows
    (in a wall by offset or point), floors (outline, or fill the level's walls), ceilings
    (outline, or fill a room), roofs (over a level's walls, or an outline that sits on the
    walls' tops), rooms, room separators, columns, beams, railings, stairs, sections,
    elevation markers, sheets, views placed on sheets, types (copies of a type), materials.
  - **Change:** `set_parameter` (every element of a category, ADR-050), `set_property`
    (particular elements; type properties retype them), `resize_building`.
  - **Move and remove:** move, copy (count), rotate, mirror, delete.
  - **Annotate:** text, aligned dimensions, Tag All, spot elevations and slopes, north
    arrows, graphic scales, key plans. In elevations and sections, a point is the spot's plan
    x, y plus a height z.
  - **Paint:** elements, or a category on a level, with a project material, a library
    material, or a new colour.
  - Every create operation also takes properties to set by the names Properties shows.
- **References.** Claude is given an inventory of the model (`model_edit::inventory`):
  levels, views, sheets, types, walls, openings, slabs, roofs, rooms and grids, with ids and
  plan coordinates in feet, and the active view's annotations and the selection. An
  operation refers to existing elements by id or unique name. It refers to what an earlier
  step made by `as` / `$name`.
- **Preview and apply as before.**
  - The whole plan runs on a copy of the document.
  - The preview card shows the summary, what it creates, changes and deletes by category,
    and the steps. The existing elements it changes or deletes are highlighted.
  - A single bulk change still shows the parameter card.
  - Apply runs the plan as one undo step. A failing step stops the plan with "step N: why",
    and nothing changes.
- Plans can be long, so Claude (Opus 5.5) may take up to 16k tokens. The live check
  (`live_model_edit`) covers an addition, a note, a level and bulk edits.
- **Not yet:**
  - operations the app has no tool for (curtain walls, furniture, sketch arcs);
  - editing sketches point by point;
  - a follow-up conversation (each prompt is one plan).

## ADR-052 Level ends in elevations and sections — Accepted (2026-09-27)
Owner request (2026-09-27): "in elevations and sections … have a handle to drag the levels left
and right, similar to how Revit does them". The owner chose per-view ends (Revit's 2D extents)
and approved saving them (a new optional field on views; older files open unchanged).

- **Grips.** Select a level in an elevation, section or interior elevation to show a grip at
  each end of its line. Dragging one moves that end along the view; the level's head (target,
  name and elevation) goes with the right end. A line is never shorter than 1'-0". Each drag
  is one undo.
- **Per view.** The ends are kept on the view (`View.level_ends`). Other elevations and
  sections, and levels you haven't dragged, span the view as before. Once one end of a level
  is dragged, both of its ends are fixed in that view.
- **How it's drawn.** The view is generated as before; then each dragged level's line
  (its horizontal center line) takes the stored ends, and the rest of what's drawn for it
  shifts with its right end.
- **Not yet:** Revit's 3D extents (dragging every parallel view at once), a bubble at the
  left end, a "reset to 3D extents" command, and the same grips for grids in elevations.

## ADR-053 Revit's default elevation and section marks — Accepted (2026-09-27)
Owner request (2026-09-27): "make the default elevation and section markers look like Revit's
default ones".

- **Filled Arrow** (`filled_arrow`). Revit's default pointer, now used for both marks, is
  two lines tangent to the round body meeting at a right angle on the side it looks, filled
  between them and the body. Before, it was a small triangle that didn't meet the circle.
  Four of them (an interior mark with four views) square the body into Revit's diamond.
- **Elevation marks** (the default Circle / Filled Arrow type). The body is split by a line,
  with the detail number over the sheet number, as before, with the new pointer.
- **Section marks** (Revit's Section Head - Filled and Section Tail - Filled).
  - The head is a 1/2" bubble split by a line, with the detail number over the sheet number
    ("—" until the section is placed on a sheet) and a Filled Arrow toward the view.
  - The tail is a filled 3/32" x 3/8" bar on the view's side at the far end.
  - The line is unchanged: dash-dot, with heavy ends.
- The other mark types (Circle Half, Diamond; ADR-022) are unchanged.

Amended (2026-09-27, the owner's screenshot): the elevation mark pointer is now a right-angled
triangle behind the body (`mark_arrow`). Its point is 1.95 radii out and its base 0.45 radii
out, 3 radii long, so the circle drawn over it leaves two black wings and the point, as in
the owner's reference. Marks with several views (interior elevations) put the sheet number in
the circle and each view number outside, just beyond its point, as Revit does. Section heads
keep the tangent Filled Arrow.

## ADR-054 Detail lines and model lines — Accepted (2026-09-27)
Owner request (2026-09-27): "create detail lines and model lines like Revit has". The owner
approved the two new element kinds (older files open unchanged).

- **Detail Line** (Annotate > Detail, DL): 2D lines owned by one view (`DetailLine`). They
  can go in a plan, ceiling plan, elevation, section or sheet, and are drawn only there.
  Copy, Rotate and Mirror act on them in plans, as with text notes; Move works anywhere.
- **Model Line** (Architecture > Model, LI): lines on a level's work plane (`ModelLine`),
  drawn from a plan of that level. They show in that level's plans, in elevations and
  sections (at the level's height; a section sees those within its depth), and in 3D as
  lines without faces.
- **Drawing.** Both use the sketch tools' draw modes (`sketch::draw`):
  - line (with Chain), rectangle, inscribed and circumscribed polygon (sides), circle,
    start-end-radius arc and center-ends arc;
  - the options bar has these, and the Line Style;
  - a rubber band previews the result (`lines_preview`);
  - each drawing is one undo.
- **Line styles.** Revit's defaults, each drawn with a pen weight and a pattern
  (`studio_views::line_style`):
  - Thin Lines, Medium Lines, Wide Lines;
  - \<Hidden\> (dashed), \<Centerline\> (dash-dot), \<Overhead\> (dashed, thin),
    \<Demolished\> (dashed), \<Beyond\> (thin).
  A line's style can be changed in Properties.
- **Editing.** A straight line has a grip at each end. Its ends snap, and other tools snap
  to them. Delete works as for any element. Edit Model can draw both (`create_detail_line`,
  `create_model_line`).
- **Not yet:**
  - custom line styles (Manage > Line Styles);
  - splines and ellipses;
  - Pick Lines;
  - model lines on vertical work planes (in elevations);
  - Revit's halftone for \<Beyond\>.

Amended again (2026-09-27): the large point is for a mark with one view (the owner's building
elevation reference). A mark with several views (interior elevations, or a building mark with
more views checked) draws the small tangent Filled Arrows again, so four of them square the
circle into Revit's diamond, with the sheet number in the circle and each view number outside
by its point. The divider line in a one-view mark is as heavy as the circle.

Amended (2026-09-27, measured from the owner's two references): the one-view pointer is the
right-angled triangle whose sides are tangent to the body. Its base is the body's diameter
across the look, reaching √2 radii each side, and its point is √2 radii out. The body drawn
over it leaves the black point and a wing at each end of the diameter: Revit's standard
exterior elevation mark.

## ADR-055 Contextual Modify tab — Accepted (2026-09-27)
Owner request (2026-09-27): "when you select an object can you have a prompt to modify that
object like Revit does in the ribbon … do this for as many elements as it makes sense".

- **The tab.** Selecting elements turns the Modify tab into Revit's green contextual tab,
  named for the selection: "Modify | Walls", "Modify | Floors", "Modify | Lines",
  "Modify | Multi-Select" and so on (`contextLabel`). The ribbon switches to it on each
  selection. Clearing the selection goes back to the tab the user was on. Types, materials,
  sheets and project settings picked in the browser keep the plain ribbon.
- **Panels.** In order:
  - Properties (Properties, Type Properties);
  - the usual Modify and Move tools;
  - View (Hide in View, Hide Element, Isolate Element, Isolate Category);
  - Create (Create Similar, Select All Instances, Edit with Claude);
  - Geometry (Paint, for model elements);
  - one panel for each selected category that has its own tools:
    - Walls: Modify Wall (Attach Top, Detach Top, Flip);
    - Floors and ceilings: Mode (Edit Boundary);
    - Doors and windows: Flip, and Browse Types (the type picker) when only one of the two
      is selected;
    - Doors, windows, rooms, columns and beams: Tag (Tag Room for rooms), in floor plans;
    - Detail and model lines: Line Style;
    - Sections, elevations, callouts and viewports: Go to View, and Activate View for a
      viewport on the active sheet.
- **Moved.** Flip, Edit Boundary, Attach Top and Detach Top leave the plain Modify tab.
  They were disabled there without a selection, and Revit shows them only in context.
- **Backend.** Two thin commands:
  - `selection_categories` returns the selection's categories;
  - `tag_elements` (studio-core `visibility::tag_elements`) tags the untagged taggable
    elements of the selection as one undo step.
- **Not yet:**
  - Filter (by category) for a multi-selection;
  - Revit's per-category geometry tools: Edit Profile, Edit Footprint, Pick New Host, Wall
    Opening, Join/Cut Geometry, Match Type;
  - setting a Line Style on several lines is one undo step per line.

## ADR-056 Tab selection — Accepted (2026-09-27)
Owner request (2026-09-27): "when you're inside any view allow to tab through selections
until you get to the object you want to modify … copy what Revit does".

- **Hover.** In the Modify (select) tool, the element under the cursor is pre-highlighted.
  The status bar names it as Revit does ("Wall : Generic - 8\"", "Door : 36\" x 84\"").
  It adds "(Tab for the next)" when there's more than one thing there.
- **Tab** steps to the next candidate under the cursor and **Shift+Tab** steps back. It
  wraps around at the end, and the status bar says "2 of 3". A wall is followed by the
  chain of walls joined to it end to end ("Chain of walls (4)"), as in Revit.
- **Click** selects the lit candidate, a whole chain included. Shift+click adds it to the
  selection, or takes it off if it's all selected already. Moving the cursor off the
  point (beyond the pick tolerance) ends the cycle, and the next hover starts over.
- **Where.** Plans, ceiling plans, elevations, sections, drafting and sheets use the
  display list (`studio_views::pick_all`): every element within the pick tolerance,
  nearest first, then topmost. `pick` is now the first of `pick_all`, so a plain click is
  unchanged. 3D uses everything the ray hits, nearest first, with the candidate lit a
  lighter blue than the selection.
- **Backend.** `pick_candidates` turns the hits into Tab's steps with their labels. The app
  commands are `pick_cycle` (2D) and `pick_candidates` (3D).
- **Not yet:**
  - chains of lines (detail and model lines, sketch lines);
  - Tab inside other tools' picks (Align, Trim, Tag…), except the dimension tools, which
    already step through references (ADR-040);
  - the hover name in 3D before Tab is pressed;
  - Revit's "Select elements by face" and "Select links" options.

Amended (2026-09-27, owner request "make the section marker look like the exterior
elevation marker"): the section head is now the building elevation mark: its body, the
large right-angled pointer toward the view, and the detail over sheet number. It's drawn at
the default exterior mark type's style and size (`detail::mark_symbol(doc, None, false)`),
at the start of the section line. The line, its heavy end segments and Section Tail - Filled
are unchanged. An unplaced section shows "—" like an unplaced elevation, not digits from
its name.

## ADR-057 Openings, Lighting and Sheets tabs; lighting fixtures and sun settings — Accepted (2026-09-27)
Owner request (2026-09-27): move the View tab's sheets to a Sheets tab. Move doors and windows
to a new tab after Architecture, which will also hold wall and roof openings. Add a Lighting
tab before Materials for exterior and interior lights and sun settings, mimicking Revit, with
lights added like doors and windows from a pop-up of typical fixtures for all building types.
Owner decisions (2026-09-27):
- The tab is **Openings**: Revit's name for its panel of wall, shaft, dormer and vertical
  cuts, so it stays right when those arrive.
- Two additions to the project file, and older files open unchanged:
  - a Lighting Fixture element and its type;
  - Sun Settings saved in ProjectInfo.
- Fixtures show in ceiling plans and 3D. Floor-standing, wall and site fixtures show in floor
  plans too.

**Ribbon.** The tab order is:
- Standards, Site, Architecture, **Openings**, **Lighting**, Materials, Rendering;
- Structure, Modify, Annotate, View, **Sheets**, Manage, Rufplan.

The new tabs hold:
- Openings: Door (Door, Load Doors) and Window (Window, Load Windows).
- Sheets: Sheet Composition (New Sheet, Place View, Key Plan), Sets (Sheet Sets, Issue Set)
  and Export (PDF, IFC).
- Lighting:
  - Lighting Fixture (the tool, and Load Fixtures);
  - Sun (Sun Settings);
  - Artificial Lights;
  - Render.

**Lighting fixtures.**
- **Type.** A type carries Revit's photometrics:
  - initial intensity (lumens) and wattage;
  - initial color (color temperature, drawn with a black-body fit);
  - the light source's emit shape (point, line, rectangle, circle) and distribution
    (spherical, hemispherical, spot, with a beam angle);
  - its body size, suspension and mounting.
- **Library.** It holds 38 typical fixtures:
  - recessed and ceiling (downlights, gimbals, wall washers, 2x4 and 2x2 troffers, flat
    panels, flush mounts, fans, track);
  - pendants and chandeliers;
  - linear (pendants, slots, strips, under-cabinet, cove);
  - wall (sconces, vanity bars);
  - floor and table lamps;
  - high bays;
  - emergency (exit signs, battery units);
  - site (wall packs, lanterns, bollards, 20' area poles, post tops, floods, in-grade,
    step and landscape lights).
  Each is tagged with the building types it is typical of: residential, office, retail,
  hospitality, healthcare, education, industrial, site. A new project starts with a 6"
  downlight, a 2x4 troffer and a wall sconce.
- **Picker.** It works like the door picker: rendered thumbnails with the lens lit, "In This
  Project" and "Lighting Library" tabs, groups, a building-type filter, a grid/list toggle,
  and photometrics in the details. Load & Place, Load, and Change Selected work as for doors.
- **Placement.**
  - In a plan or ceiling plan, ceiling and pendant fixtures go at the ceiling over the
    point: that ceiling's height, else 9'-0".
  - Wall fixtures snap to the nearest wall's face within 5', facing out, at their mounting
    height.
  - Lamps and site fixtures sit on the floor or ground.
  - In 3D, a fixture goes on the face clicked. A floor or ground click uses the type's
    height instead.
- **Instance.** Its properties are Level, Elevation from Level, Rotation, Light On and
  Dimming. Revit's Artificial Lights dialog lists the fixtures by type, with each one's on/off
  and dimming, All On and All Off (`set_lights`, one undo step). The contextual tab has Light
  On, Light Off and Browse Types.
- **Drawing.** Ceiling-plan symbols include the downlight circle, the troffer with its
  diagonals, linear rectangles, pendant canopies, the chandelier and fan, sconce half-rounds,
  the exit sign, poles and uplights. In 3D the body is shaded, and a lit lens glows in its
  light's color (`Mesh.glow`).
- **IFC.** Fixtures export as IfcLightFixture (point source, direction source or security
  lighting) with Pset_LightFixtureTypeCommon.TotalWattage.

**Sun Settings.**
- Revit's Solar Study is saved with the project. Still is the site's sun on a date and time;
  Lighting is an azimuth and altitude. The dialog has presets (solstices, equinoxes and
  afternoons; sunlight from the southwest, southeast or overhead).
- Renders start from them.

**Renders.**
- **Lighting Scheme.** Revit's six schemes:
  - Exterior or Interior;
  - Sun only, Sun and Artificial, or Artificial only.
- **Artificial lights.** They add each lit fixture's light (`fixtureLight`):
  - rectangle and line sources become area lights of their size (luminance from the lumens);
  - spherical sources become point lights;
  - spots and hemispheres become spot lights (a uniform cone, or a cosine hemisphere).
  Candela is converted with the sky map's scale of about 6,000 lux per unit, in millimetre
  scene units. Lenses glow.
- **Exposure.** Each scheme starts from an exposure, as Revit's exposure control does:
  interiors ×2.5, night exteriors ×12, night interiors ×25.

**Not yet:**
- Openings' wall, shaft, dormer and roof openings;
- photometric web (IES) files;
- light groups other than by type;
- Revit's Single Day and Multi-Day solar studies and the sun path;
- a lighting fixture schedule;
- fixtures in sections and elevations;
- Edit Model operations for fixtures;
- lights in the 3D view's Realistic style (only renders light the scene).

## ADR-058 Wall Opening of any shape — Accepted (2026-09-27)
Owner request (2026-09-27): "make an opening tool that allows you to create a hole of any shape
within a wall, and place this tool under Openings, similar to how Revit does. Also allow
sketching the opening in 3D, aligned to the surface you're cutting beforehand."
Owner decisions (2026-09-27):
- A new Wall Opening element (older files open unchanged).
- In 3D, after the face is picked, the view turns to look square at it.

**Tool.** It is Openings > Opening > **Wall Opening**, as Revit's.
- In an elevation or section, click the wall; the face toward the view is the work plane.
  The wall must be square to the view.
- In 3D, click the wall's face; that face is the work plane (Revit's Set Work Plane > Pick a
  plane), and the view swings to face it. **Orient to Plane** on the sketch tab brings it
  back after orbiting.
- Then it's Revit's sketch mode on that plane, starting with Rectangle:
  - the tools are Line (with chain), Rectangle, the polygons, Circle, both arcs, Fillet Arc,
    Trim/Extend, Delete and Undo;
  - Pick Walls, Pick Lines and Flip are left out;
  - Finish checks Revit's loop rules and adds that loops can't nest and the sketch must cut
    the wall.
- Each loop is its own hole, so one opening can have several.
- To change one, double-click it (in an elevation, its face; in 3D, its reveal) or use the
  contextual tab's Edit Sketch.

**Element.** `WallOpening { host, sketch }`: the loops in the wall's frame, u along the
location line from its start and z up from its base. So the opening moves and stretches with
its wall, and is deleted with it. Its properties are read-only: Host, Width, Height, Base
Offset (bottom), Area and Holes. The app's `WallPlane` maps the sketch's coordinates (the
view's, or the wall's own in 3D) onto the wall, mirroring arcs for a view seen from behind.

**Drawing.** `WallSolid.holes` carries each hole as (opening, ring in (u, absolute z)).
- **3D.** A wall with holes keeps its pieces' tops, ends and door and window reveals. Its two
  long faces are rebuilt as regions minus every hole and door or window, then triangulated
  (`edges::wall_with_holes`). Each hole's reveals are the opening's own mesh, shaded like
  the wall, so a click selects the opening. Its outline is drawn on both faces.
- **Plans.** Where a hole crosses the cut plane, its span is cut out of the wall's poché and
  layer lines (`hole_cuts_at`).
- **Elevations.** The hole is its own face over the wall, as door openings are.
- **Sections.** Through a hole, the cut wall stops below it and starts again above it.
- **IFC.** Each hole is an IfcOpeningElement voiding the wall: its profile on a plane just
  outside the face, extruded through it.

**Not yet:**
- openings in walls seen at an angle in elevations (use 3D);
- holes that reach a wall's top or end still leave that edge closed;
- material takeoffs don't subtract the holes;
- elevations don't show what's seen through a hole;
- Revit's other openings: shaft, vertical, dormer, by face (floors and roofs).

## ADR-059 Doors, windows, fixtures and dimensions in elevations and sections — Accepted (2026-09-27)
Owner request (2026-09-27): "make sure you can place light fixtures, doors and windows in
elevations and sections, as well as dimension in elevations and sections".

**Doors and windows.** Door and Window work in elevations and sections, as in Revit:
- hover a wall's face (one not seen edge-on) and the opening goes where the view's line of
  sight meets that wall, facing the viewer, at its type's sill;
- the preview is its outline with the spacing to the wall ends; like plans, it snaps to the
  wall's center, keeps whole inches, and refuses overlaps (`opening_preview_in_view`,
  `view_refs::model_point`).

**Lighting fixtures.** Lighting Fixture works there too:
- click a wall's face (or, in a section, anywhere on the cut plane);
- its level is the one at or below the click;
- wall fixtures go on that face at the height clicked; ceiling, pendant, floor and site
  fixtures at their usual height over that point.

**Dimensions.** Elevations and sections now have Revit's references and snaps. They come
from the model edges the view draws (`view_refs`):
- **References:** levels, grids, wall and roof outlines, doors' and windows' heads, sills
  and jambs, floor and ceiling lines, and fixtures. Levels and grids win ties with the edges
  on them, and Tab reaches the others.
- **Snaps:** those edges' ends, midpoints and crossings, then the nearest point on one.
  Without a snap, the height rounds to the inch as before.
- **Kinds:** aligned, linear and angular dimensions all work there.
- **Not associative:** these dimensions measure where the references were when placed.
  Plans' wall and grid references still follow the model.

**Not yet:**
- dimensions that follow levels and openings in elevations (that needs new reference
  anchors in the file format);
- placing doors and windows in walls seen at an angle.

## ADR-060 Grid ends that snap to grids, frames in plan, draggable tags — Accepted (2026-09-27)
Owner request (2026-09-27): "when moving the grid bubbles, allow them to snap to other
gridlines as well; show the door frame and window frame in plan view; drag the room name tag
when it's selected, in plan, elevation and section views".

**Grid bubbles.** A grid end's grip stays on its grid's line, as in Revit
(`handles::grip_snap`). While dragging, it snaps:
- level with the ends of the grids parallel to it, so bubbles line up;
- to where other gridlines cross it.
The status bar names what it snapped to ("Aligned with Grid 3", "Grid B"). Before, the end
could wander off its line.

**Frames in plan.** Door and window symbols now show their frames, sized as in 3D:
- **Doors:** the jambs through the wall (a 4 1/2" frame for glass and garage doors), and the
  casings on both faces (not on barn doors).
- **Windows:** the frame's jambs, as deep as the frame.

**Tags drag.** A selected tag (a room's name, number and area, or a door, window, column or
beam tag) drags by its text in any view it's drawn in. The drag area sets its offset from its
element (`drag_handle` key `tag`), one undo step. Move still works too.

**Room tags in sections and elevations.** Tag works in sections and elevations:
- click inside a room, at its level's height;
- the room is the first one the line of sight enters on the level at or below the click
  (from the cut, in sections and interior elevations);
- its tag goes where it was clicked, stored as an offset from a spot over the room's point,
  4'-0" above its level (`view_refs::room_tag_base`), so it follows the room;
- one tag per room per view.

## ADR-061 High-resolution wood sidings and modern roofing — Accepted (2026-09-27)
Owner request (2026-09-27): "come up with a bunch of different exterior wood siding materials
at extremely high resolution; do the same for roof materials, specifically for a modern
house, including a better asphalt shingle and standing seam".

**Generated textures** (`studio_views::texgen`). The textures are made in Rust, not
downloaded: 26 kinds, each a seamless colour, OpenGL normal and roughness set.
- **Resolution.** The app makes them at 4096 px. On an 8' tile that is 0.6 mm a pixel; a
  45" shingle tile is 0.28 mm.
- **Seamless.** Noise lattices, courses, boards, panels and joints all divide each tile, so
  it repeats exactly. They are generated on every core, about 1.5 s a set.
- **Wood.** Each board has its own grain: growth rings bending into flat-sawn cathedrals,
  fine fibres and pores, a tone that drifts along its length, and knots where the species
  has them. Weathered boards go silver with water streaks. Charred cedar crazes into
  alligator scales.
- **Board geometry** is in the relief and the shading:
  - bevel and Dutch lap tapers, with the shadow under each butt;
  - shiplap and channel gaps, V-grooves;
  - battens standing 3/4" proud;
  - open rainscreen joints over a black membrane;
  - staggered butt joints.
- **Asphalt shingles.** 5-5/8" courses of laminated random-width tabs showing the darker
  layer below (the dragon teeth), with the shadow line and multicoloured ~1 mm granules.
- **Standing seam.** 1-1/2" rounded seams, striated pans, slight oil-canning, and bare
  Galvalume's spangle.

**Presets.** A new **Siding** category holds 16 presets:
- natural cedar bevel, and weathered silver bevel;
- shiplap in white and in black;
- Dutch lap in sage;
- board and batten in black, white and natural cedar;
- vertical tongue and groove cedar;
- horizontal thermally modified ash;
- shou sugi ban;
- cedar shingles, natural and weathered;
- channel rustic in a walnut stain;
- ipe open-joint rainscreen;
- Accoya slat rainscreen;
- reclaimed barn wood.

**Roofing** gains:
- architectural shingles in black, weathered wood and pewter;
- standing seam in matte black 16", charcoal 18", dark bronze 16", bright white 18", bare
  Galvalume 16" and pre-weathered zinc 18";
- black EPDM with taped laps, river rock ballast, a sedum green roof, and flat charcoal
  concrete tile.

The existing "Architectural Asphalt Shingles, Charcoal" and "Standing Seam Metal, Charcoal"
keep their ids and now use the new sets. Painted ones (shiplap, Dutch lap, painted board and
batten, painted standing seam, concrete tile) take the relief and gloss from the set and the
colour from the material, so their colour can be changed. New surface patterns for
elevations:
- 5-5/8" shingle courses and 13" tile courses;
- 2" and 6" vertical boards, 12" board and batten, and 16" and 18" standing seam.
The vertical ones are grid columns with rows too tall to meet, so the file format doesn't
change.

**Loading.**
- `material_texture` serves `gen:` sets by making all three maps once, one set at a time,
  and caching them as PNG under the app's data folder (`textures/generated`). The page tells
  PNG from JPEG by its first byte.
- The path tracer packs all textures at one size. It now picks the largest that fits about
  512 MB (4096, 2048 or 1024 px, `textureSizeFor`), so a render with a few generated
  materials uses them at full resolution.

**Roof mapping.** Real-world box mapping projected sloped faces in plan, so courses always
ran east-west and seams north-south. Sloped faces now map along their contour and up their
slope at true length, whichever way they face: shingle courses run level, seams run down
the roof, and the exposure is true.

**Previews.** The 32 new or changed presets' cubes were path-traced like the others, from 1K
versions of the sets, by a throwaway page in headless Edge.

## ADR-062 A V-Ray-quality Realistic view; archviz-grade wood sidings — Accepted (2026-09-28)
Owner request (2026-09-28): "make the realistic view in 3D view look extremely realistic, like
V-Ray quality, and update all the wood siding materials to look like they were rendered by
MIR, Boundary, Squint Opera or another high-end rendering company".

**Realistic view, V-Ray's interactive render.**
- **Still frames.** When the camera rests (0.65 s) with nothing selected and the Modify tool
  active, the view is path-traced in place, refining sample by sample. It uses the Render
  pipeline (`realScene`):
  - the project's materials and generated textures;
  - the site's physical sky and sun from Sun Settings;
  - at night, the lit fixtures.
  A chip reads "Refining… n%", and at 512 samples the frame is denoised.
- **Moving the camera.** Orbiting, a selection, a tool or a model change hands back to the
  live view at once.
- **One tracer** (`pathtrace.Refiner`) keeps its scene. Its BVH is rebuilt only when the
  model or the sun changes; otherwise only its camera moves.
- **Not refined:** sketch mode and a section box (the tracer has no clipping). Nor is the
  ViewCube's corner, which stays live.
- **The live (raster) Realistic view** also gets closer:
  - the physical sky from Sun Settings lights it and shows behind it (turned into the z-up
    world), without its sun disk;
  - the sun is a shadow-casting light at the sky model's sun-to-sky ratio, in the sun's
    colour, with 4096 px soft shadows;
  - GTAO ambient occlusion through a multisampled composer;
  - ACES tone mapping.
- **Exposure.** Daylight renders at 0.85, night at 12.

**Wood sidings.** The generator (`texgen`) was refined against high-end archviz stills:
- **Cedar colour.** Each board draws from a real western red cedar palette (honey, salmon,
  amber, chocolate, occasional sapwood), pulled toward the lot's mean so a wall reads as one
  lot of wood rather than a patchwork. The tones are deeper and more saturated.
- **Grain.** Asymmetric growth rings (earlywood fading into latewood that stops sharply),
  more fibre and streak contrast, and latewood a touch glossier than earlywood.
- **Board shape.** A slight cup across each board.
- **Shadow lines.** Deeper and wider under every course.
- **Joints and nails.** End grain beside butt joints. Stainless nail heads every 16",
  painted over on painted boards, with rust tears on weathered ones.
- **Weathering.** Drip staining along the lower edges of weathered boards.
- **Boards.** 8' long between joints on bevel siding.
- **Exact tiling.** Every board, course, panel, cell and saw-mark index is wrapped to the
  tile, so every set now repeats exactly. A test checks samples a tile apart in x, y and
  both.
- **Previews.** The 17 siding previews were re-rendered.

## ADR-063 An Enscape-style live view; Walk and Fly; V-Ray and Corona finishing on Render — Accepted (2026-09-28)
Owner request (2026-09-28): "I don't like the rendering after the camera stops, I would rather
it renders only when I press Render. This is to zoom around the 3D model while seeing realistic
materials — like Enscape, D5, Twinmotion or Lumion for the materials and sun settings. Then for
rendering make it V-Ray, Corona or Chaos Vantage quality … add a sun setting in the 3D view's
top-left corner", and "create Enscape's walk / fly mode buttons, only in the 3D view".

**Auto-refine removed.** ADR-062's still-frame path tracing (`pathtrace.Refiner`,
`render/realScene.ts`) is gone. The Realistic view is always the live raster view, and path
tracing happens only through Render (RR).

**The live Realistic view, Enscape and D5's real-time look.**
- **Light.** The physical sky from the sun lights the view and shows behind it. The sun is a
  shadow-casting light at the sky model's sun-to-sky ratio and in the sun's colour.
- **Post-processing.** GTAO ambient occlusion and a light bloom on true highlights only
  (threshold above white, so sky and pale surfaces don't haze), through a multisampled
  half-float composer.
- **Auto exposure.** The sun's and sky's light on level ground maps to a steady brightness
  at any hour. The sun panel's Exposure (0.3–2.5) scales it, and ACES tone mapping finishes it.
- **Night.** When the sun is down, the sky dims and the view's lit fixtures (up to 48) become
  point and spot lights.
- **Shadows.** three r186 dropped PCFSoftShadowMap, so the sun uses PCF with a blur radius
  and 4096 px maps.
- **Textures** use 16× anisotropic filtering (clamped to the GPU's limit), so siding and
  seams stay sharp at grazing angles.

**Sun panel (top left of the 3D view).** A chip shows the time and date or the azimuth and
altitude. It opens to the time of day (4 am–10 pm in quarter hours), the month and day, a
Lighting study's azimuth and altitude, Exposure, and the Sun Settings presets. Dragging
previews at once on a coarser sky. After 700 ms without a change, it saves to Sun Settings
as one undoable change. A new `sun_for` command gives the sun's position for unsaved settings.

**Walk and Fly (Enscape's navigation).** Orbit, Walk and Fly buttons sit under the sun chip.
- **Moving.** W A S D moves (and the arrow keys); in Fly, Q and E go down and up. Dragging
  looks around, Shift triples the speed, and the wheel scales it.
- **Switching.** Space swaps Walk and Fly, and Esc returns to Orbit.
- **Walk** keeps the eyes 1650 mm above the floor under it. It climbs steps up to 450 mm
  at once and eases down to lower floors. It slides along walls, keeping 350 mm clear;
  doors let it through.
- **Where the rules live.** The navigation is `render/navigate.ts`, which is pure and
  tested. The view supplies the ground and the walls by raycasting.

**Render, V-Ray and Corona's finish.** Filmic tone mapping is the default, and two new options
are on by default:
- **Lens glare:** the brightest highlights (sun glints, fixtures, sky through glass)
  bloom tight and wide;
- **Vignette:** a soft darkening toward the corners.

## ADR-064 Vegetation tab: Enscape's Asset Library, 3D grass and site ground — Accepted (2026-09-28)
Owner request (2026-09-28): "create a vegetation tab that allows you to place common vegetation
around the site, as well as realistic 3D-looking grass, pine cones, asphalt or any other typical
base … in 3D view in realistic mode as well as in renderings. Make the library as extensive as
Enscape's but mainly focus on trees and shrubs", and "copy or model after Enscape as closely as
possible".

**Model (studio-core `planting`).** New element kinds:
- `PlantingType { name, spec }` holds a species: its group, crown form, foliage, bark, height,
  spread, clear trunk, caliper, stems, leaf, flower and autumn colours, season and density.
- `Planting { type_id, level, at, offset, rotation, scale }` is one placed plant. It stands on
  the topography where its level is within 5' of grade, and otherwise on its level.
- `GroundRegion { level, material, boundary, sketch }` is Revit's subregion. It is sketched
  with a new `SketchKind::GroundRegion` and draped over the topography.
- `ProjectInfo.ground` is the base ground's material. All new fields default when missing, so
  older files still open.

Plantings move, copy, rotate, mirror and array like any element and have Revit-style properties
(size, offset, rotation; on the type: height, spread, trunk, form, season, density and colours).

**The Asset Library.** The catalog has 183 species:
- 119 trees: 43 deciduous, 18 flowering, 17 broadleaf evergreen, 28 conifer and 13 palm;
- 38 bushes: shrubs and clipped hedges;
- 20 grasses and flowers;
- 6 succulents and cacti.

Each has real sizes, a botanical name, a description and the climates it suits (cold,
temperate, Mediterranean, tropical, arid). Like Enscape's, deciduous and flowering trees come
in season variants (spring bloom, summer, autumn colour, bare winter).

The Asset Library window (components/AssetLibrary.tsx) follows Enscape's:
- **Layout:** a dark window with Vegetation categories and groups down the left, and season
  chips, a climate filter and search across the top.
- **Tiles:** thumbnails rendered from each plant's own model and textures, with a height badge.
- **Details:** size, botanical name and climates.
- **Placing:** Place (or a double-click) loads the asset and starts placing it. Each click in a
  plan or the 3D view places one more, with Enscape's random rotation and random size (±%) on
  the options bar.
- **Vegetation tab:** Asset Library, Place Plant, Trees, Conifers, Palms, Shrubs and Grasses
  (each opens the library on its category), a Season control that sets every seasonal type at
  once, Base Ground and Ground Region.

**Plants in views (studio-views `plants`, `foliage`).**
- **Models are grown, not stored.**
  - Broadleaf trees, pines and shrubs grow by space colonization: branches grow toward points
    scattered through the crown's envelope, thickened by the pipe model, as bark tubes with
    root flare.
  - Firs, spruces and cypresses are a leader with whorled branches that droop with age.
  - Palms have curved trunks with feather fronds or fan leaves (and a dead-frond skirt on
    fan palms).
  - Also: rosettes (agave, yucca, hosta), grass clumps with plumes, clipped hedges and ribbed
    cacti.
  - Three variants per species; an instance's variant comes from its id.
- **Foliage is alpha-cut cards** textured with a per-species atlas drawn in Rust:
  - leaves with shaped outlines (pointed maple lobes, oak lobes, hearts), serrations, midribs,
    veins, a folded-light gradient, pale undersides and soft shadows between leaves;
  - needle and scale sprays, palm fronds and fans, grass plumes and blossoms.

  Card normals point out of the crown, and the crown's occlusion is baked into their colour, so
  a crown reads as one soft volume.
- **Bark** is a seamless texture per bark kind (smooth, furrowed, plated, papery, mottled,
  fibrous, palm rings, green).
- **Working views** show a low-poly proxy, as Enscape's assets show in Revit.
- **Plans** show Revit-style symbols: a scalloped canopy with branches, a conifer star, palm
  fronds, a shrub cloud, a grass tuft and a scalloped hedge. Site plans show every planting
  at grade.
- **Elevations and sections** show silhouettes that hide and are hidden like other faces.
  Ground regions show as outlines in plan.

**Realistic view and renders.**
- **Plants:** full models, instanced in the live view with a gentle wind sway, and merged
  into the path tracer's scene.
  - Foliage mipmaps preserve coverage (Castaño), so thin needles and narrow leaves don't
    vanish in the distance.
  - Leaves use a low specular, avoiding the white rim full Fresnel gives cards seen edge-on.
  - Foliage is on layer 1, which ambient occlusion doesn't see, because its normal pass can't
    cut leaves out of their cards.
  - The proxy stays pickable and shows only while selected.
  - Renders leave the proxies out. They keep half the baked crown occlusion, since the
    path tracer shades the crown itself.
  - Columnar conifers (cypress, arborvitae) grow by space colonization with foliage along
    their branches, not by whorls.
  - Plant and grass vertex colours are RGBA. The path tracer merges every mesh into one and
    gives meshes without colours RGBA white, so RGB colours among them misaligned every
    later mesh's colours, texture coordinates and materials. Depending on the random merge
    order, trees rendered leafless with flat bark.
  - Foliage renders with a leaf's real albedo (the live view's colour times 1.45) and only a
    fifth of the baked crown shading, since the path tracer shades crowns itself.
- **Ground:** the base ground's material covers the topography (unless it shows satellite
  imagery), or the ground around the model. Ground regions sit 30 mm above it.
- **Enscape's Grass material type** (`Appearance.grass`: height, height variation; the Type
  control in the Material Browser) grows real 3D blades.
  - Clumps are scattered over every grass surface once. The nearest (about 45,000 in the
    live view, 70,000 in a render) are drawn at full density and shrink to nothing at the
    edge, following the camera. The lawn's texture carries on beyond.
  - No grass grows through paving, slabs or decks: a 250 mm cover grid of the other surfaces
    near grade masks them out.
  - Pine straw gets fallen 3D pine cones.
- **Ground materials:** 23 generated Site & Landscape materials:
  - lawns, meadow, pine straw, leaf litter, three mulches;
  - pea gravel, crushed stone, river rock, decomposed granite;
  - two asphalts, broom concrete, running-bond and herringbone pavers, flagstone;
  - sand, soil, moss and snow.

  Their picker swatches are bundled in app/public/ground (dev aid: `cargo test --release -p
  rufplan-studio write_ground_previews -- --ignored`).

**Not yet:** temporary hide and the section box don't apply to the full models in Realistic,
though Visibility/Graphics hiding does. There's no scatter-by-area tool; place plants one click
at a time or array them.

## ADR-065 D5 Render's look: grass, sky, colour and the Grass Brush — Accepted (2026-09-28)
Owner request (2026-09-28): "base all the rendering assets off of D5 Render … update the default
sky in Realistic mode or render to be like D5, including how it treats all its assets including
the grass. Grass is probably most important to match, and add a paint feature for more grass".

**Grass, D5's kinds.** Seven kinds, each with its own clump (render/grass.ts `clumpGeometry`):
- **Lawn and Lush Lawn:** dense, fine blades.
- **Meadow with Flowers:** mixed wildflowers (white, yellow, purple, pink).
- **Wild Grass and Tall Grass:** seed heads.
- **Dry Grass:** mostly straw.
- **Clover:** trefoil leaves and white heads.

Every blade gets its own hue (yellow-green to blue-green) and brightness. It is dark at the
root and lighter and yellower at the tip, and it bends and twists. The base colour is baked
into the clump's vertex colours, so flowers keep their own colours.

The lawn has gentle patches, tinted per clump from smooth noise at about 9 m and 2.6 m, as
D5's lawns do.

Grass-type materials (Enscape's, ADR-064) grow a lawn, or a meadow when taller than 200 mm.
Densities are per kind.

**The Grass Brush, D5's scatter brush.** It is a 3D-view tool (Vegetation > Grass Brush, GB):
- **Cursor:** a ring on the ground (or a floor or roof) under the cursor.
- **Painting:** dragging lays round dabs, each recording the surface's height; releasing
  paints one Grass Patch (one undo step).
- **Options bar:** the grass kind, brush size, density and Erase. Erase removes the dabs a
  stroke covers and deletes emptied patches.
- **The element:** `GrassPatch { level, dabs [x, y, z, r], spec }` (studio-core `grass`)
  moves and copies like any element. Its properties are kind (which brings its own height and
  colour), height, height variation, density and colour.
- **In the views:**
  - Plans show its merged outline, dashed; site plans show patches at grade.
  - 3D shows its area laid over the topography (or at the painted height) in its colour; in
    Realistic only its blades show, and the area stays pickable.
  - Renders grow the blades and leave the flat area out.

**Sky.** `physicalSky` gains `clouds` (the share of sky covered).
- **The clouds:** a fair-weather cumulus deck seen in perspective, with bright sunlit tops,
  silver edges toward the sun and greyer, thicker undersides, fading into the horizon haze.
  Between them the sky is a clearer blue (turbidity 2.4).
- **Where it's used:** Realistic's sky uses 42% cover. It is also the renders' new default
  background, "D5 Sky (clouds, matches the sun)", lit by the site's sun. The photo skies
  remain as choices.

**Colour.** D5's slightly richer, punchier images come from 1.14 saturation and 1.06 contrast
after tone mapping. The live view applies them as a final shader pass, and renders apply them
as a "D5 colour" option (on by default).

**Zooming into a render.** The Render dialog's preview zooms: the wheel zooms toward the
cursor, dragging pans, and double-click fits the image back. Fit, 100% (actual pixels), + and −
sit at its corner with the zoom level. Past 200% the pixels show crisp rather than blurred.

## ADR-066 Box selection and the Filter dialog, after Revit — Accepted (2026-09-28)
Owner request (2026-09-28): "in plans, elevations and most other views allow dragging the mouse
to select a bunch of objects at once, then a filter pop-up to filter the objects you want
selected, similar to how Revit has it".

- **Box selection.** In any 2D view or sheet, with the Modify tool, dragging from empty space
  draws Revit's box (studio-views `pick_in_rect`, over the view's display list):
  - **Window** (left to right, solid line): the elements drawn wholly inside.
  - **Crossing** (right to left, dashed): also every element the box touches.
  - **Modifiers:** Ctrl adds to the selection and Shift takes away, as in Revit.
  - The view's own crop region is never selected.
- **Filter.** Revit's Filter dialog lists the selection's categories with their counts, with
  Check All, Check None and the total, and keeps only the categories left checked. It opens
  from Modify | Multi-Select > Selection > Filter, or from the status bar's funnel, which
  shows how many elements are selected. A new command, `element_categories`, gives each
  element's category.

## ADR-067 Detail Level (Coarse, Medium, Fine), after Revit — Accepted (2026-09-28)
Owner request (2026-09-28): "a similar toggle button like the one in 3D view for wireframe to
realistic, but for plans, sections, elevations and most views, showing different detail levels
of walls, doors, windows, equipment etc.: coarse, medium and fine", per the design handoff
"Detail Level Toggle (icon set 2C, wall layers)".

- **The model.** `DetailLevel` (Coarse, Medium, Fine) is a view property, as in Revit.
  - It is stored as `View.detail_level` (optional, serde default). Older files open unchanged,
    so this is not a file format change.
  - Until one is chosen, a view follows its scale: Fine at 1/4" = 1'-0" and larger, Coarse at
    smaller scales. Views at 1/4" and larger therefore draw exactly as they did before.
  - It appears in the view's Properties as Graphics > Detail Level.
- **What each level draws:**
  - **Coarse:**
    - Walls: cut walls are solid poché.
    - Doors in plan: the leaves and swings only.
    - Windows in plan: the wall's faces across the opening and one line of glass.
    - Sections: cut floors and roofs are solid, without layer lines.
    - Door and window elevations: keep the outlines of sashes, panels and glass, and drop
      muntins, rails and swing marks.
  - **Medium:**
    - Walls: a lighter fill with only the core's boundaries, the faces of the structure
      layers inside the wall (`compound::core_boundaries`, `WallSolid.core`).
    - Door and window frames appear; window mullions and casement swings do not.
    - Elevations leave off only the dashed swing marks.
  - **Fine:** everything, as before. Every layer is drawn, finishes wrap at free ends and
    openings, cut patterns (batt insulation) show, and door casings, mullions and swing
    marks appear.
- **The toggle.** A pill in each drawn view's lower-left corner, like the 3D view's Visual
  Style pill (ADR-038).
  - It shows the current level. Hovering or focusing opens Coarse, Medium and Fine; arrow keys
    step between them.
  - The icons are set 2C: the same wall slab drawn as poché, as core lines, and as every layer
    with insulation.
  - Choosing a level sets the view's property, which is undoable like any change.
- **Shortcuts.** The handoff's single keys C, M and F would clash with the two-letter Revit
  shortcuts (CO, MV, …), so the levels get two-letter shortcuts of our own, listed in
  Keyboard Shortcuts: **DC** Coarse, **DD** Medium, **DF** Fine.
- **Not yet:** equipment, furniture and other families have no level-specific geometry yet.
  3D views keep their Visual Style and have no Detail Level.

## ADR-068 Model In-Place, after Revit — Accepted (2026-09-28)
Owner request (2026-09-28): "create an option like Revit has and model objects in place like
custom objects, also define what type of object it is, window, door, wall, ceiling, floor,
etc... similar to how Revit does it".

- **The element.** `ElementData::InPlace` has a name, a category, a level, an optional
  material and a list of forms (studio-core `inplace`).
  - Its category is the one chosen, so it truly is a wall, a door or casework to everything
    that goes by category. That covers Visibility/Graphics and Hide Category, the Filter
    dialog, the contextual Modify tab, the pick label ("Casework : Model In-Place :
    Kitchen Counter") and IFC.
  - The categories are those of Revit's Family Category and Parameters dialog: Casework,
    Ceilings, Columns, Doors, Floors, Furniture, Generic Models, Lighting Fixtures,
    Planting, Plumbing Fixtures, Railings, Roofs, Specialty Equipment, Stairs, Structural
    Framing, Walls and Windows.
  - Five categories are new (Generic Models, Furniture, Casework, Specialty Equipment,
    Plumbing Fixtures); only in-place elements use them yet.
  - The category can be changed later in Properties.
- **Forms.** Heights are measured from the element's level.
  - **Extrusion:** closed loops from a start height to an end height.
  - **Blend:** from a base loop to a top loop.
    - The loops are matched corner to corner when they have as many corners, so a tapered
      box keeps its corners. Otherwise both are resampled evenly.
  - **Sweep:** a rectangle or round profile along a sketched path, which may be open or
    closed. Corners are mitred.
  - **Void Extrusion:** cuts the element's solid extrusions.
    - The solid is split where voids start and stop, and each layer loses the voids that
      pass through it.
- **Solids** are built in studio-regen `inplace`, as closed triangle meshes for each piece
  of a form. Tests check volumes: the box with its notch, the frustum, and a mitred L sweep.
  Sections cut each piece on its own and join the cuts, so forms that overlap stay solid.
- **Views** (studio-views `inplace`):
  - **Plans** cut elements of cuttable categories where the cut plane passes through them.
    - Walls, columns, framing, floors, roofs and stairs get solid poché; other categories a
      lighter fill.
    - Anything below the cut shows as its outline.
    - Furniture, fixtures, equipment and planting are never cut, as in Revit.
  - **Elevations** show each element's silhouette among the model's faces; windows are
    drawn as glass.
  - **Sections** also cut it where the section plane passes through it.
  - **3D** shows a mesh in its category and material.
  - **IFC:** the element is exported tessellated, as its category's class:
    - IfcWall, IfcDoor, IfcWindow, IfcSlab, IfcCovering, IfcRoof, IfcColumn, IfcBeam,
      IfcStair and IfcRailing;
    - IfcFurniture for furniture and casework;
    - IfcLightFixture and IfcSanitaryTerminal for lighting and plumbing fixtures;
    - IfcBuildingElementProxy for everything else.
    - Its ObjectType is 'Model In-Place'.
- **The workflow** follows Revit:
  - Architecture > Build > **Model In-Place** opens the Family Category and Parameters
    dialog, with a filterable list and a Name that follows the category ("Casework 1").
  - The **In-Place Editor** tab then replaces the ribbon:
    - Forms: Extrusion, Blend, Sweep. Void Forms: Void Extrusion.
    - The model's forms, each with Edit Sketch and Delete.
    - Finish Model and Cancel Model.
  - A form is sketched with the usual sketch tools, in plan or in 3D on the level's work
    plane. From an elevation or section, the level's plan opens.
    - The sketch tab shows the form's settings: extrusion start and end, blend base and top,
      and a sweep's profile, size and elevation.
    - A blend's Finish goes from its base on to its top.
  - Each form is saved when it's finished.
    - **Finish Model** folds everything done in the editor into one undo step ("Model
      In-Place" or "Edit In-Place").
    - **Cancel Model** undoes back to where the editor opened.
  - **Edit In-Place** reopens the editor: on the contextual Modify tab, or by double-clicking
    the element.
- **Editing.** Move, Copy, Rotate, Mirror and Array transform every form's sketch. Each
  form's heights and profile are also in Properties.
- **The sample house** has a kitchen counter modelled in place as Casework, 3' high with a
  sink cut by a void. CI's IfcOpenShell check now expects one IfcFurniture.
- **File format:** a new element variant and five new categories. Files without in-place
  elements are unchanged and older files open as before. A file with in-place elements
  won't open in an older build of the app.
- **Not yet:**
  - Revolves and swept blends.
  - Voids cutting blends and sweeps.
  - Sketching on vertical work planes (for example a profile in an elevation).
  - Sweeps with sketched profiles.
  - Schedules listing in-place elements.
  - In-place walls bounding rooms or hosting doors and windows (Revit doesn't host in them
    either).

## ADR-069 Drafting views, filled regions and the Detail Library — Accepted (2026-09-28)
Owner request (2026-09-28): "create a new tab for details and have it act like the door or
window library grid of details, but for typical details, and have each detail drawn at a
typical scale for that type of detail. If you need to create a detail view like Revit has
first, please do. I think we should have detail views anyway."

- **Drafting views.** `ViewKind::Drafting` is Revit's drafting view: a 2D-only view at a
  scale, holding its own detail lines, filled regions and text, with no model in it.
  - The project browser lists them under Drafting Views, and they go on sheets like any
    view.
  - Their extent is whatever is drawn in them.
  - Tools there are the 2D ones: Detail Line, Text, Filled Region, Move, Copy, Rotate,
    Mirror, Array and Offset.
  - Revit's model detail views are our callouts (ADR-020); the Details tab offers Callout
    too.
- **Filled regions.** `ElementData::FilledRegion` has a view, its loops (the first is the
  outline, the rest are holes), a pattern and an optional outline line style.
  - They are sketched in sketch mode (`SketchKind::FilledRegion`) in any 2D view, with the
    pattern picked on the sketch tab. Double-click one to edit its boundary; change its
    pattern and outline in Properties.
  - **Patterns** are Revit's drafting patterns, sized in paper mm so they read the same at
    any scale: Solid, Gray, Diagonal, Crosshatch, Concrete (aggregate stipple), Earth,
    Gravel, Sand/Gypsum, Masonry, Rigid Insulation, Wood and Steel (studio-views
    `drafting`).
  - The pattern is clipped to the region's even-odd area, holes included.
  - Regions draw over the model and under the view's lines and text.
- **The Detail Library.** 22 typical details for US light-frame construction (studio-core
  `details`), each drawn at the scale that kind of detail is usually drawn at:
  - **Foundations:** thickened slab edge, stem wall at crawlspace, basement wall with
    footing drain (3/4" = 1'-0"); interior spread footing (1" = 1'-0").
  - **Walls:** exterior wall assembly in plan (3" = 1'-0"); brick veneer at foundation and
    CMU wall at slab (1 1/2" = 1'-0").
  - **Openings:** window head, sill and jamb, and interior door jamb (3" = 1'-0"); exterior
    door threshold (6" = 1'-0", a new scale).
  - **Roofs:** eave with gutter, rake, parapet with coping and ridge vent
    (1 1/2" = 1'-0").
  - **Floors & Stairs:** rim joist and deck guardrail (1 1/2" = 1'-0"); stair tread and
    riser, and handrail at wall (3" = 1'-0").
  - **Interiors:** base cabinet (1 1/2" = 1'-0"); acoustical ceiling at wall and partition
    head (3" = 1'-0").
  - **How they're drawn:** a small builder draws in inches, with parts for cut lumber (the
    X), boards, sheathing, gypsum, batts (Revit's zigzag), break lines, rebar, flashing,
    earth and lap siding.
    - Cut material has a wide outline over its pattern.
    - Notes are set out in columns on either side, spaced so they never overlap, each with a
      leader and a solid arrowhead.
    - A test checks every detail's scale, lines, regions and notes.
- **Inserting a detail** makes a drafting view at the detail's scale in one undo step.
  - Every line, region and note becomes its own element to edit.
  - A second copy is named "(2)".
- **The Details tab** comes after Annotate:
  - Create: Drafting View (name and scale), Detail Library and Callout.
  - Detail: Detail Line, Filled Region and Text.
  - **The Detail Library window** is laid out like the door and window libraries:
    - Categories with counts, and a search box.
    - A grid of drawn thumbnails (the view's display list drawn as SVG) showing each
      detail's scale.
    - The chosen detail larger with its notes, and Insert Detail (or double-click a card).
- **Not yet:**
  - Saving your own details to the library.
  - Detail components (Revit's repeating 2D families, such as brick or CMU coursing along a
    line).
  - Dimensions in drafting views (they measure model elements).
  - Revit's Insulation tool, which draws a batt along a line with a width.

## ADR-070 Revit's Text tool, leaders and in-place editing — Accepted (2026-09-28)
Owner request (2026-09-28): "creating a text note with a leader and try and copy Revit's text
tool and leader as closely as possible. Right now there is a pop up that's not styled when
the text command is entered; this should be changed to match how Revit does it."

- **The note.** `TextNote` gains `leaders`, `align` (Left, Center or Right) and `width`
  (a wrap width in paper mm). All are serde defaults, so older files open unchanged.
  - `at` is where the first line sits, at the line's vertical middle.
  - Lines split at typed line breaks and wrap at word breaks.
  - The layout is studio-core `text` (`layout`, `wrap`, `attach`, `leader_points`,
    `arrowhead`).
- **Leaders** are Revit's: one segment, two segments (through an elbow) or curved.
  - They leave the side of the text nearer their arrowhead, at the first line.
  - Each ends in Revit's "Arrow Filled 30 Degree", 2.4 paper mm long.
  - The note draws an invisible box so the whole note can be picked.
- **The Text tool (TX)** replaces the ribbon with Revit's "Modify | Place Text" tab:
  - The text type: 3/32", 1/8", 3/16", 1/4" or 1/2" Arial.
  - The leader (No Leader, One Segment, Two Segments, Curved) and the alignment.
  - Placing: leader clicks come first (the arrowhead, then the elbow), then a click for the
    text, or a drag that also sets its width.
  - The text is typed in an on-canvas editor in the note's own font and size at the view's
    zoom. This replaces the browser's unstyled `window.prompt`.
  - Enter starts a new line; clicking outside, Esc or Ctrl+Enter finishes. An empty note is
    dropped, as in Revit.
- **Editing.** Double-click a note to edit its text in place.
  - Grips drag each leader's arrowhead and elbow; dragging a straight leader's middle bends
    it.
  - Another grip sets the wrap width, and dragging the text moves it while its arrowheads
    stay.
  - Modify | Text Notes has Add Left Leader, Add Right Leader, Remove Last Leader and Edit
    Text.
  - Move, Rotate and Mirror carry the leaders with the note.
- **The Detail Library's notes** are now leadered text notes. Earlier they were text plus
  separate lines and arrowhead regions.

## ADR-071 Detail components, Repeating Detail and Insulation — Accepted (2026-09-28)
Owner request (same day): "make components that are typical in Revit like break lines,
plywood, gyp board, header block, etc.… all typical components you see in Revit", plus
"Revit's Insulation tool" and "detail components, Revit's repeating 2D pieces such as brick
coursing along a line".

- **The element.** `ElementData::DetailComponent` has a view, a `type_key`, a start, an end
  and a flip flag.
  - Line-based components run from start to end. Point-based ones sit at the start, turned
    toward the end.
  - Geometry is built in the component's own frame, in inches (studio-core
    `details::components`).
- **Families** (17, with 52 types in all), named as in Revit's Detail Items library:
  - Break Line, which masks the band beside it.
  - Plywood/OSB sections (with ply lines) and Gypsum Wallboard.
  - Nominal Cut Lumber (with its X) and Nominal Lumber side views.
  - Wood Header: (2) with a 1/2" spacer, or (3).
  - Batt Insulation (Revit's loops) and Rigid Insulation.
  - Brick and CMU coursing, as repeating units with joints; CMU units have hollow cells.
  - Metal Flashing with a hem, and Anchor Bolts with hook, nut and washer.
  - Sealant with Backer Rod, and Rebar.
  - W wide flanges, metal studs and angles.
- **A Masking fill pattern** (paper white) lets break lines and lumber hide what's behind
  them. Filled regions can use it too.
- **Placing.**
  - **Detail Component (CM)**, Revit's Component shortcut, works in any 2D view.
    - The options bar has a type selector grouped by family, rotation and Flip; Space turns
      a point-based component 90°.
    - A preview follows the cursor.
  - **Repeating Detail** starts with brick coursing, and **Insulation** with a 5 1/2" batt.
- **Editing.** Grips drag a line-based component's ends; a point-based one drags as a whole.
  Properties change the type (within its family) and Flip. Mirror flips it.
- **Draw order.** Components draw in the view's order, so a break line masks what was drawn
  before it.

## ADR-072 Dimensions in drafting views — Accepted (2026-09-28)
- **The tools.** Aligned and Linear dimensions work in drafting views, on detail lines and
  detail components. Drafting views snap to their lines' ends, midpoints and crossings,
  without the elevations' height labels.
- **New anchors:**
  - `Anchor::DetailLine`: a fraction along the line.
  - `Anchor::Component`: a point in the component's own frame.
- **Behaviour:**
  - Dimensions stretch when what they measure moves, as in Revit.
  - Copies re-link their anchors.
  - Align can move detail lines and components.

## ADR-073 Saving your own details to the library — Accepted (2026-09-28)
- **Save to Library** (Details tab, with a drafting view active) saves the view's detail
  lines, filled regions, components, text notes and dimensions, with a name, a category
  ("My Details" by default, or any library category) and a description.
  - The file is `my-details.json` in the app's local data folder, one per computer (studio-core
    `details::user`, app `detail_cmds`).
  - Saving again with the same name and category replaces it.
- **Inserting** makes a drafting view with copies of the elements. Dimensions' anchors are
  pointed at the copies.
- **In the library**, your details show alongside the built-in ones, drawn in the grid.
  They can be deleted with a two-click Delete, not a pop-up.
- **Tests:** studio-core gains `serde_json` as a dev-dependency only, for the file
  round-trip test.
