# Roadmap

Work top to bottom. Tick boxes as tasks land. A milestone is done only when every
acceptance criterion passes and is demonstrable to the owner.

## Prototype slice (ADR-011, 2026-09-23)
Built ahead of the milestone order at the owner's request. **Works now:**
- Levels, grids, walls (4 types), doors (3 types), windows (3 types), floors, and ceilings
  (new element kind), rooms with tags and live areas, all undoable. Doors and windows:
  ADR-012; rooms and Move: ADR-013.
- Views: floor plans with cut poché, reflected ceiling plans with ACT grid, four
  elevations with level heads and grid bubbles, 3D with orbit and click-select.
- Wall joins: mitered L corners; T and cross junctions merged into clean poché.
- Tools: chained walls with snapping (endpoint, midpoint, intersection, perpendicular,
  nearest, 15° increments); grids; levels (placed in elevations); floors by sketch or
  pick-walls; ceilings by sketch or auto-room; doors and windows with live placement
  preview; rooms (click an enclosed area); Move with joined walls stretching. Revit-style
  shortcuts (WA, DR, WN, RM, MV, GR, LL, SB …).
- Editing: selection, delete (with dependents), properties panel with ft-in input, type
  switching and type editing, design stage picker, project information.
- Documents (ADR-014): door/window/room tags, dimensions, text, sections, schedules, sheets with
  the Rufplan title block, and vector PDF export at true scale.
- Files: schema 2 `.rfproj` storing all elements; sample project with a 4-sheet set;
  unsaved-changes prompts.

- Structure and circulation (ADR-019): columns (placed or at grid intersections), beams,
  railings, L- and U-shaped stairs with landings and handrails, stair openings in floors.
- Envelope and assemblies (ADR-019): walls attached to roofs, roofs over L/T/U plans with
  valleys, wall location lines, layered floor and roof types, finish wraps at ends and
  openings, and cut patterns.

- Model integrity and detailing (ADR-020): floors and ceilings that follow walls, room
  separation lines, the 3D section box, column/wall cleanup, a materials library (cut and
  surface patterns, colors), column/framing schedules and a material takeoff, column
  location marks and structural tags, and callouts (detail views).

- Sketch mode and symbols (ADR-021): Revit's boundary sketch for floors and ceilings (every
  Boundary Line tool, Pick Walls with Tab, locked wall lines, Finish checks), Revit level
  heads and elevation marks, and interior elevation markers with room-cropped views.

- Families and 3D (ADR-022): elevation mark types chosen in the type selector, placing walls,
  doors, windows, floors, ceilings, roofs, columns and rooms in the 3D view, and a
  toggleable cyan ground grid.

- Site (ADR-023): find a lot on Google Maps, Regrid parcel boundaries, preliminary USGS 3DEP
  topography, site plans with contours, bearings and north, and ground in sections and 3D.

- Shortcuts (ADR-024): Revit's two-letter keyboard shortcuts for modify, draw, view and snap
  commands, editable in Keyboard Shortcuts (KS); Pin/Unpin, Match Type, Create Similar,
  Select All Instances, Repeat; Temporary Hide/Isolate, Hide in View, Visibility/Graphics;
  Thin Lines, 3D visual styles, zoom commands and one-pick snap overrides.

- Sketching in 3D (ADR-025): Floor and Sketch Ceiling sketch mode on the level's work plane in
  the 3D view with every draw, pick and modify tool; Edit Boundary and double-click on floors
  and ceilings in 3D; Move and Copy in 3D.

- Satellite overlay (ADR-026): Google imagery draped on the 3D topography and under the Site
  plan (toggle), and topography over 2x to 4x the lot.

- Rendering (ADR-027): a Rendering tab with Revit's Camera tool (eye, then target, in a plan)
  making perspective camera views, cameras shown and gripped in plans, and Render (RR): a
  path-traced image with the sun at the site's date and time, saved as PNG.

- V-Ray-style rendering (ADR-028): physical sun & sky or photo dome light, photo backgrounds
  (Sky, Mountains, Grass Plain, City) with ground projection, filmic tone, denoise, and
  saving with the background or as a transparent PNG; the Camera tool places cameras.

- Material library (ADR-029): a Materials tab and Material Browser with 78 V-Ray-style
  presets for US residential, hospitality and multifamily work (typical to high-end), 2K
  photo textures on first use, path-traced cube previews, and Apply to Selection.

- Generate with Claude (ADR-030): describe a building (type, stories, count, style, roof,
  lot and setbacks, references and images, prompt); Claude plans it as rooms per story and
  it is built as a model (levels, walls, doors, windows, stairs, floors, roof, rooms,
  materials) in one undoable step.

- Window families (ADR-031): 12 common US window families (double- and single-hung,
  casement, awning, hopper, sliders, fixed and transom, picture combinations, bay,
  storefront) with mulled units, grilles and frame finishes, drawn by family in plan,
  elevation and 3D; a Window Library of 77 standard sizes (or a custom size) to load.

- Sheet sets (ADR-032): each design phase's deliverables (Program, 100% SD, 100% DD,
  100% CD, Permit, Bid, IFC) and their sheets per building type, numbered to the US
  National CAD Standard, with views placed and scaled to fit, consultant placeholders, and
  one PDF per deliverable.

**Shortcuts still open** (see ADR-011): JSON display lists; roofs on non-right-angled,
non-convex footprints need sketching per face (no straight skeleton); section box has no
caps; beams aren't cleaned up against walls; sections don't hatch cut layers yet. (The regeneration
graph, parameter map, endpoint dragging and typed lengths landed in ADR-017; layered walls,
roofs, stairs and crop regions in ADR-018.) Door leaves are thin: select a door by clicking its leaf or swing arc.

## M0 — Scaffold
- [x] Cargo workspace with empty crates listed in CLAUDE.md; each compiles with a smoke test
- [x] Tauri 2 + React + TypeScript + Vite app in `app/`, launches a window
- [x] One IPC round-trip: UI button calls `core_version` command, shows result
- [x] `studio-io`: create/open/save an empty `.rfproj` with `meta` table and schema_version
- [x] File > New / Open / Save wired in the UI (native dialogs)
- [x] CI workflow (Windows runner): `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `npm test`, `tauri build`
- [x] `docs/SETUP.md` verified on a clean Windows machine

**Acceptance:** app launches, creates a project file, reopens it, CI is green.

## M1 — Model core
- [x] `ElementId` (UUID v7), `ParamValue`, `ParamDef`, units module with ft-in formatting/parsing (`12'-6 1/2"` ↔ mm) + tests — *project parameters live in a per-element map; built-ins stay typed fields (ADR-017)*
- [x] Element store with category/level indexes — *category index (ADR-017); level queries filter a category*
- [x] Transactions with before-images; undo/redo stack; typed errors
- [x] Levels, Grids, WallTypes (3 defaults: 6" int stud, 8" ext stud, 12" CMU), Walls (straight)
- [x] Wall solids via native kernel: footprint polygon × base/top constraint heights
- [x] Regen graph: Level → Wall heights; Type → Wall thickness; change propagates — *steps memoized on their inputs, cached by content stamp; equivalence property test (ADR-017)*
- [x] Design stages (ADR-010): ProjectInfo + ProjectStage elements with the six defaults, set current stage (undoable, logged in stage history), edit stage names/dates; Project Info panel in the UI
- [x] Persistence round-trip of all M1 elements (property test: save → load → equal)

**Acceptance:** tests prove that changing the current stage records history and undo restores it; changing a level's elevation updates wall heights, changing a
wall type's thickness updates footprints, and undo restores both exactly.

## M2 — Plan view and editing
- [x] View element (FloorPlan) with view range; plan generation of cut walls (poché) + projection
- [ ] Display list format + binary IPC transfer — *format done; JSON, cached per model state; binary deferred (ADR-017)*
- [x] Canvas 2D renderer: pan, zoom, line weights by scale, selection highlight
- [x] Wall tool: chained drawing, typed lengths (ft-in), snapping (endpoint, midpoint, perpendicular, 15° increments, grid intersections)
- [x] Wall joins: L (miter), T (butt), cross; clean poché at joins
- [x] Selection, drag endpoints, delete, properties panel editing type/instance params — *plus temporary dimensions and Copy / Rotate / Mirror / Array / Align / Trim / Offset / Split / Flip (ADR-017)*
- [x] Project browser: levels, views, sheets tree; open multiple plan views as tabs
- [x] Undo/redo in UI (Ctrl+Z / Ctrl+Y)

**Acceptance:** draw a 40' × 30' rectangle of exterior walls with two interior walls forming
T-joins; joins are clean; edit a wall type and see all plans update in < 50 ms for 500 walls.
*Status 2026-09-24: met — 535 connected walls redraw in ~6.5 ms after a type edit (release build, `bench_500_walls`); binary IPC deferred.*

## M3 — Hosted elements, floors, rooms, 3D
- [x] Door and window families (built-in) with types; place on wall with hover preview
- [x] Openings cut in wall solids and plan poché; swing arcs and symbols in plan
- [x] Doors/windows move with host wall; delete with host
- [x] Floors by sketched boundary or "pick walls"
- [x] Rooms: place in bounded region; boundary + area computed; name/number params
- [x] 3D view in three.js from wall/floor/opening solids; orbit, section box (basic) — *section box with face handles (ADR-020)*

**Acceptance:** a small two-room building with 3 doors, 4 windows, a floor, and 2 rooms;
moving a wall updates room areas, door positions and the 3D view.
*Status 2026-09-24: met, and covered by tests (regen room-area test, modify.rs opening tests); the 3D section box landed in ADR-020.*

## M4 — Documents
- [x] Annotations: door tag, window tag, room tag (name, number, area), aligned dimensions to wall faces/centerlines, text — *tags are elements created on placement (movable, deletable, Tag All); dimension ends attach to walls/grids and follow them (ADR-015)*
- [x] Section and elevation views (painter's-order hidden lines, cut poché)
- [x] Schedules: door schedule (mark, type, width, height, level), room schedule — *plus window schedule and sheet index*
- [x] Sheets: title block (built-in, 24×36 ARCH D and 11×17), viewports at standard scales, sheet list
- [x] PDF export of selected sheets: vector, true scale, line weights, fonts embedded — *exports all sheets; per-sheet selection comes with issuances*
- [x] ProjectInfo feeds title block fields, including the current design stage
- [x] Sheets assigned to stage deliverable sets; project browser filters by stage; issuances record their stage — *Issue Set records the issuance and exports the PDF; title blocks list issues (ADR-015)*

**Acceptance:** export a 4-sheet PDF (A0.0 cover/sheet index, A1.0 plan, A2.0 elevations,
A3.0 sections + door schedule) that prints at true scale when measured with a scale ruler.
*Status 2026-09-24: the sample project exports exactly this set; true scale is covered by tests.
Awaiting the owner's scale-ruler check on a print.*

## M5 — Interop and Rufplan link
- [x] IFC4 export: IfcProject/Site/Building/BuildingStorey, IfcWall, IfcSlab, IfcDoor, IfcWindow, IfcSpace, materials, property sets (ADR-016)
- [x] CI test validates exported IFC with IfcOpenShell (Python, dev-only)
- [x] Supabase auth: email/password, and Google via system browser + loopback redirect (ADR-016; owner must allow-list the redirect URL)
- [x] Link a Studio project to a Rufplan project
- [x] Publish: uploads the stage set PDF (+ IFC) as a Rufplan deliverable in the matching phase tab and records the issuance. IFC upload waits on the owner allowing `application/x-step` in the `project-media` bucket (ADR-016)

**Acceptance:** sign in, link, publish; the published set appears on the Rufplan project and the
IFC opens correctly in an IFC viewer (e.g., That Open Engine / web-ifc on the Rufplan side).

## Later (not scheduled)
- OpenCascade kernel (ADR-006), sloped/curved walls, roofs, stairs, railings
- Family editor with constraint solver (PlaneGCS)
- Element check-out worksharing, then real-time co-editing (Automerge/Yjs)
- IFC import, DWG export (ODA), Revit bridge via the owner's C# Revit add-in
- Revisions/clouds, keynotes, detail components, drafting views
