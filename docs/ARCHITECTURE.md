# Architecture

## Layers
```
┌──────────────────────────── React UI (TypeScript) ────────────────────────────┐
│ Project browser │ Properties panel │ Ribbon/tools │ View canvas │ 3D │ Sheets │
└────────────────────────────── Tauri IPC (thin) ───────────────────────────────┘
┌─────────────────────────────── Rust core ─────────────────────────────────────┐
│ studio-core   Element store · types/params · transactions · undo/redo         │
│ studio-regen  Dependency graph · dirty tracking · incremental recompute       │
│ studio-geom   Math · 2D booleans · GeometryKernel trait (native → OCCT later) │
│ studio-views  Plan/section/elevation generation · graphics · annotations      │
│ studio-sheets Sheets · viewports · schedules · PDF export                     │
│ studio-io     .rfproj (SQLite) persistence · IFC4 export                      │
│ studio-sync   Supabase auth · publish · (later) worksharing                   │
└───────────────────────────────────────────────────────────────────────────────┘
```

## Data flow for an edit
1. User drags a wall endpoint in the plan canvas (TS). TS sends an **intent**:
   `move_wall_endpoint { wall_id, end: "start", point_mm }`.
2. The Tauri command opens a transaction in `studio-core`, applies the change, and commits.
3. Commit produces a **ChangeSet** (created/modified/deleted element IDs + before/after).
4. `studio-regen` marks dependents dirty (joined walls, hosted doors, rooms bounded by
   the wall, tags, dimensions, every view showing them) and recomputes in topological order.
5. `studio-views` regenerates affected **view graphics** (display lists) for open views only;
   closed views are marked stale and regenerated lazily.
6. The command returns a compact **ViewDelta** (per view: replaced display-list chunks keyed
   by element ID). Large buffers use `tauri::ipc::Response` with binary payloads.
7. TS applies the delta and redraws. The UI never computes geometry.

## Element store (`studio-core`)
- In-memory `HashMap<ElementId, Element>` plus secondary indexes (by category, by level,
  by host). Persisted to SQLite on save and autosave (see DATA_MODEL.md).
- `Element` = common header (id, category, type_id, level_id, params) + category payload enum.
- **Transactions**: `doc.transact("Move wall", |tx| { ... })`. A transaction records
  before-images of every touched element. Commit pushes onto the undo stack; undo re-applies
  before-images and reruns regen. Nested transactions are not supported; use sub-steps.
- Validation runs at commit (e.g., hosted door must lie within its host wall's length).
  Failed validation rolls back and returns a typed error the UI can show.

## Regeneration (`studio-regen`)
- Explicit dependency edges stored alongside elements, e.g.:
  - Level → elements constrained to it (base/top constraints)
  - Type → instances
  - Wall → hosted doors/windows; Wall ↔ Wall (joins)
  - Walls → Room (boundary); Element → Tag; Element refs → Dimension
  - Element → Views that display it (derived, not persisted)
- Use `petgraph` for the graph. Recompute derived data (**solids, plan footprints, room
  boundaries, tag text**) only for dirty nodes, topologically ordered. Detect cycles and
  reject the transaction that would create one.
- Derived data is cached and never persisted; it's rebuilt on load.

## Geometry (`studio-geom`)
- `GeometryKernel` trait with operations the rest of the system needs:
  extrude profile, boolean subtract/union, section by plane → 2D loops, project to plane,
  bounding box. M1–M4 ship a **native Rust implementation** limited to prismatic solids
  (extruded polygons with vertical sides) — enough for walls, slabs, openings, and
  simple families. OpenCascade replaces/augments this later (ADR-006).
- 2D polygon booleans via `i_overlay` (or `geo` BooleanOps). All tolerances in one module
  (`tol::LINEAR = 0.01 mm`, `tol::ANGULAR = 1e-9 rad`).
- Wall joins: compute on 2D footprints first (miter for L, butt/extend for T, per join
  settings), then extrude.

## Views (`studio-views`)
- A **View** is an element with a definition (type, level, view range, crop box, scale,
  visibility/graphics overrides) and a derived **display list**.
- Display list = ordered primitives: polylines, filled polygons (poché/hatch), arcs, text,
  symbols — each tagged with source element ID, line weight index, line pattern, and
  cut/projection/annotation layer. The same display list feeds the canvas and the PDF writer.
- **Plan generation (prismatic model)**: for each element intersecting the view range,
  slice at the cut plane → cut polygons (heavy + poché); project elements below the cut
  but above view-range bottom → projection outlines (light). Door/window symbols (swing arcs,
  panel lines) come from the family's plan-symbol definition, not from slicing.
- **Section/elevation**: slice/project the prismatic solids against a vertical plane.
  Hidden-line removal for elevations is a known hard problem: M4 ships a painter's-order
  approach on planar faces; exact HLR comes with OCCT.
- Line weights: a table of 16 pen weights mapped per scale, like Revit's object styles.

## Sheets and output (`studio-sheets`)
- Sheet = title block family instance + viewports (view_id, center on sheet, scale).
- PDF writer walks each viewport's display list, transforms model mm → paper mm by scale,
  clips to the crop region, and writes true vector geometry with real line weights.
- Schedules are views whose display list is a table built from an element query.

## UI (`app/src`)
- Zustand stores for UI state only (selection, active tool, open views, panels). Model
  state is fetched from Rust; never duplicated as a source of truth in TS.
- `Renderer` interface with a Canvas 2D implementation first. Pan/zoom in screen space;
  hit-testing uses a spatial index returned from Rust with the display list, or a Rust
  `pick(view_id, point, tolerance)` command.
- Tools are state machines (idle → first point → second point → commit) sending intents.
- Snapping (endpoints, midpoints, perpendicular, grid intersections, angle increments)
  is computed in Rust via a `snap` command for consistency.

## Threading
- Core runs off the UI thread. Long operations (PDF export, IFC export, publish) run as
  async tasks that report progress events to the UI and can be cancelled.
