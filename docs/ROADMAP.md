# Roadmap

Work top to bottom. Tick boxes as tasks land. A milestone is done only when every
acceptance criterion passes and is demonstrable to the owner.

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
- [ ] `ElementId` (UUID v7), `ParamValue`, `ParamDef`, units module with ft-in formatting/parsing (`12'-6 1/2"` ↔ mm) + tests
- [ ] Element store with category/level indexes
- [ ] Transactions with before-images; undo/redo stack; typed errors
- [ ] Levels, Grids, WallTypes (3 defaults: 6" int stud, 8" ext stud, 12" CMU), Walls (straight)
- [ ] Wall solids via native kernel: footprint polygon × base/top constraint heights
- [ ] Regen graph: Level → Wall heights; Type → Wall thickness; change propagates
- [ ] Persistence round-trip of all M1 elements (property test: save → load → equal)

**Acceptance:** tests prove that changing a level's elevation updates wall heights, changing a
wall type's thickness updates footprints, and undo restores both exactly.

## M2 — Plan view and editing
- [ ] View element (FloorPlan) with view range; plan generation of cut walls (poché) + projection
- [ ] Display list format + binary IPC transfer
- [ ] Canvas 2D renderer: pan, zoom, line weights by scale, selection highlight
- [ ] Wall tool: chained drawing, typed lengths (ft-in), snapping (endpoint, midpoint, perpendicular, 15° increments, grid intersections)
- [ ] Wall joins: L (miter), T (butt), cross; clean poché at joins
- [ ] Selection, drag endpoints, delete, properties panel editing type/instance params
- [ ] Project browser: levels, views, sheets tree; open multiple plan views as tabs
- [ ] Undo/redo in UI (Ctrl+Z / Ctrl+Y)

**Acceptance:** draw a 40' × 30' rectangle of exterior walls with two interior walls forming
T-joins; joins are clean; edit a wall type and see all plans update in < 50 ms for 500 walls.

## M3 — Hosted elements, floors, rooms, 3D
- [ ] Door and window families (built-in) with types; place on wall with hover preview
- [ ] Openings cut in wall solids and plan poché; swing arcs and symbols in plan
- [ ] Doors/windows move with host wall; delete with host
- [ ] Floors by sketched boundary or "pick walls"
- [ ] Rooms: place in bounded region; boundary + area computed; name/number params
- [ ] 3D view in three.js from wall/floor/opening solids; orbit, section box (basic)

**Acceptance:** a small two-room building with 3 doors, 4 windows, a floor, and 2 rooms;
moving a wall updates room areas, door positions and the 3D view.

## M4 — Documents
- [ ] Annotations: door tag, window tag, room tag (name, number, area), aligned dimensions to wall faces/centerlines, text
- [ ] Section and elevation views (painter's-order hidden lines, cut poché)
- [ ] Schedules: door schedule (mark, type, width, height, level), room schedule
- [ ] Sheets: title block (built-in, 24×36 ARCH D and 11×17), viewports at standard scales, sheet list
- [ ] PDF export of selected sheets: vector, true scale, line weights, fonts embedded
- [ ] ProjectInfo feeds title block fields

**Acceptance:** export a 4-sheet PDF (A0.0 cover/sheet index, A1.0 plan, A2.0 elevations,
A3.0 sections + door schedule) that prints at true scale when measured with a scale ruler.

## M5 — Interop and Rufplan link
- [ ] IFC4 export: IfcProject/Site/Building/BuildingStorey, IfcWall, IfcSlab, IfcDoor, IfcWindow, IfcSpace, materials, property sets
- [ ] CI test validates exported IFC with IfcOpenShell (Python, dev-only)
- [ ] Supabase auth via system browser + deep link (see SYNC_AND_RUFPLAN.md)
- [ ] Link a Studio project to a Rufplan project
- [ ] Publish: upload PDF set + IFC + manifest; creates an issuance record visible on Rufplan

**Acceptance:** sign in, link, publish; the published set appears on the Rufplan project and the
IFC opens correctly in an IFC viewer (e.g., That Open Engine / web-ifc on the Rufplan side).

## Later (not scheduled)
- OpenCascade kernel (ADR-006), sloped/curved walls, roofs, stairs, railings
- Family editor with constraint solver (PlaneGCS)
- Element check-out worksharing, then real-time co-editing (Automerge/Yjs)
- IFC import, DWG export (ODA), Revit bridge via the owner's C# Revit add-in
- Revisions/clouds, keynotes, detail components, drafting views
