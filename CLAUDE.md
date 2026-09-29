# CLAUDE.md — Rufplan Studio

> Claude Code reads this file automatically at the start of every session. Keep it current.
> "Rufplan Studio" is a working name.

## What we are building
A desktop BIM authoring application in the spirit of Revit. The user models a building from
parametric elements (levels, walls, doors, windows, floors, rooms). The app **generates**
construction documents from that model: plans, sections, elevations, schedules and sheets,
exported as vector PDF at true scale. Projects link to **Rufplan** (the owner's AEC
marketplace, React + Supabase) so a published drawing set and IFC model can flow into a
Rufplan project for bidding and hiring.

Product owner: Rufus Kerr, architect (Revit power user) and solo founder of Rufplan.
Treat him as the domain expert on architecture; explain software trade-offs plainly.

Read these before writing code:
- `docs/PRODUCT_BRIEF.md` — goals, users, scope, non-goals, glossary
- `docs/ARCHITECTURE.md` — layers, crates, data flow, regeneration, views
- `docs/DATA_MODEL.md` — elements, types, parameters, units, IDs
- `docs/ROADMAP.md` — milestones with task checklists and acceptance criteria
- `docs/SYNC_AND_RUFPLAN.md` — auth, cloud tables, publishing
- `docs/CONVENTIONS.md` — code style, testing, commits
- `docs/DECISIONS.md` — architecture decision records (ADRs)
- `docs/SETUP.md` — toolchain setup (Windows-first)

## Locked stack (change only via a new ADR + owner approval)
| Layer | Choice |
|---|---|
| App shell | Tauri 2 |
| UI | React 18 + TypeScript + Vite, Zustand for UI state |
| Core engine | Rust (Cargo workspace, crates under `crates/`) |
| Project file | SQLite via `rusqlite` (one file per project, extension `.rfproj`) |
| 2D geometry | `glam` (vectors), `i_overlay` or `geo` (polygon booleans) |
| 3D kernel | Rust-native extrusions for M1–M4 behind a `GeometryKernel` trait; OpenCascade later (ADR-006) |
| Plan/sheet rendering | Canvas 2D behind a `Renderer` interface in TS; WebGL2 when perf requires |
| 3D view | three.js |
| PDF | `krilla` or `printpdf` (vector, true scale) |
| Interop | IFC4 export (hand-written STEP writer), validated with IfcOpenShell in tests |
| Cloud | Supabase (Auth, Postgres, Storage) — same project as Rufplan |

## Golden rules
1. **The model is the source of truth.** Drawings are derived, never edited as raw lines
   (annotations are the only view-owned elements).
2. **All geometry and business logic lives in Rust.** TypeScript renders and handles input
   only. Tauri commands are thin wrappers over core crate functions.
3. **Every model change goes through a transaction.** No direct mutation. Transactions give
   undo/redo and change sets for sync.
4. **Internal units are millimeters (f64).** Convert only at the UI boundary. Default display
   is US architectural feet-inches. Never mix units inside core.
5. **Element IDs are UUID v7** and never reused. They must be stable for sync and IFC GUIDs.
6. **Work one milestone at a time** from `docs/ROADMAP.md`. Tick checkboxes as tasks land.
   Do not start the next milestone until acceptance criteria pass.
7. **Tests with every core change.** Geometry and regeneration code needs unit tests with
   explicit numeric expectations. `cargo test` and `npm test` must pass before a commit.
8. **Ask before** adding a major dependency, changing the file format, touching Supabase
   schema, or deviating from the locked stack. Record the answer as an ADR.
9. Small commits, conventional messages (`feat(core): add wall join resolution`).
10. Never commit secrets. The desktop app only ever holds the Supabase **anon** key.

## Repository layout (target)
```
/
├─ CLAUDE.md
├─ docs/
├─ crates/
│  ├─ studio-core/     # element DB, params, types, transactions, undo
│  ├─ studio-geom/     # math, 2D booleans, GeometryKernel trait + native impl
│  ├─ studio-regen/    # dependency graph + incremental regeneration
│  ├─ studio-views/    # plan/section/elevation generation, annotations, graphics
│  ├─ studio-sheets/   # sheets, title blocks, viewports, schedules, PDF export
│  ├─ studio-io/       # .rfproj persistence (SQLite), IFC export
│  └─ studio-sync/     # Supabase client, publish, (later) worksharing
├─ app/
│  ├─ src-tauri/       # Tauri shell, IPC commands only
│  └─ src/             # React UI
└─ .github/workflows/  # CI: fmt, clippy, test, build
```

## Commands
- `cargo test --workspace` (also regenerates `app/src/bindings/*.ts` via ts-rs; commit them)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all`
- `cd app && npm run tauri dev` (run the app)
- `cd app && npm test` / `npm run lint` / `npm run format:check`
- `cd app && npm run tauri build` (NSIS installer in `target/release/bundle/nsis/`)
- `cd app && npm run install:local` (build + silently install/upgrade the app for this Windows user)

## Current status
- M0 complete. **Prototype slice built ahead of M1–M4** (ADR-011): levels, grids, walls,
  doors and windows (ADR-012), rooms and Move (ADR-013), documents and PDF (ADR-014),
  floors, ceilings; floor plans, ceiling plans, elevations and 3D; Rufplan styling; design
  stages. Editing tools (ADR-017); layered walls, roofs, stairs (ADR-018); columns, beams,
  railings, L/U stairs, stair openings, attached walls, L/T/U roofs, location lines,
  layered floors/roofs and cut patterns (ADR-019); bound floors, room separators,
  section box, materials, schedules, callouts (ADR-020); Revit sketch mode for floor and
  ceiling boundaries, Revit level and elevation symbols, interior elevations (ADR-021);
  elevation mark families, editing in 3D, 3D ground grid (ADR-022); Site tab with Google Maps,
  Regrid parcels and USGS 3DEP topography (ADR-023; keys in the OS credential store only). The "Prototype slice" section of docs/ROADMAP.md lists the shortcuts still open.
- Try it: `cd app && npm run tauri dev -- -- -- --sample` opens the sample house.
- M3 met; M4 met pending the owner's scale-ruler print check.
- M5 built (ADR-016): IFC4 export (validated by IfcOpenShell in CI), Rufplan sign-in,
  project link and Publish into Rufplan's existing deliverables (no schema change).
  Owner actions pending: allow-list the Google redirect URL; allow `application/x-step` in
  the `project-media` bucket so the IFC uploads too.
- Editing and foundation round (ADR-017, ADR-018): Revit modify tools (Copy, Rotate, Mirror,
  Array, Align, Trim/Extend, Offset, Split, Flip), grips, temporary dimensions, typed lengths;
  incremental regeneration with stamp caches (M2's 50 ms target met), category index,
  project parameters; layered wall types, roofs, stairs and crop regions. M1 and M2 are met
  except binary IPC.
- Model integrity and detailing (ADR-020): bound floors/ceilings, room separators, 3D
  section box, column cleanup, materials library, structure/material schedules, column
  marks and structural tags, callouts. M3 is fully met. Next: sections hatching cut layers,
  beam/wall cleanup, section box caps, binary IPC.
- Revit keyboard shortcuts (ADR-024): the two-letter defaults (AL, TR, MV, CO, PN, HH, VV,
  ZF, SE…), a Keyboard Shortcuts dialog (KS) with overrides saved per computer, pinning,
  Temporary Hide/Isolate, Hide in View, Visibility/Graphics, Thin Lines and 3D styles.
- Sketching in 3D (ADR-025): floor/ceiling sketch mode, Edit Boundary, Move and Copy in 3D.
- Satellite overlay (ADR-026): Google Static Maps imagery on the topo in 3D and Site plans
  (never saved), topography over 2x-4x the lot.
- Rendering tab (ADR-027): Camera tool and camera views, path-traced Render (RR) with the
  site's sun (three-gpu-pathtracer, loaded on first use).
- Materials tab (ADR-029): V-Ray-style appearance on materials, a 78-preset library
  (studio-core `library`), Poly Haven 2K textures cached on first use, cube previews in
  app/public/materials.
- Generate with Claude (ADR-030): Architecture > Generate; studio-core `generate` builds a
  model from a room plan; studio-sync `claude` streams the Messages API (key in the OS
  credential store).
- Window families (ADR-031): studio-core `windows` (families, layouts, catalog, load),
  studio-views `windows` (plan, elevation, 3D parts, thumbnails); Architecture > Load
  Windows opens the Window Library.
- Door families (ADR-033): studio-core `doors`, studio-views `doors` and `thumbs`; the
  TypePicker (rendered thumbnails, app/src/render/thumbs.ts) replaces the Window Library.
- Terrain (ADR-045): studio-regen `SiteSolid::skirt`, studio-views `terrain`; the 3D view's
  TerrainBar (earth depth, contours, labels, interval).
- Detail Level (ADR-067): studio-core `DetailLevel` (View `detail_level`, `for_scale`),
  `compound::core_boundaries`; studio-regen `WallSolid.core`; studio-views `Builder.detail`,
  `windows::shown_at`; components/DetailLevelToggle.tsx; shortcuts DC / DD / DF.
- Box selection and Filter (ADR-066): studio-views `pick_in_rect`; app `pick_in_rect`,
  `element_categories`; ViewCanvas `boxSelect`; components/FilterDialog.tsx; status-bar funnel.
- D5 look (ADR-065): studio-core `grass` (GrassKind, GrassSpec, GrassPatch paint/erase); studio-views
  `plants::grass_patch_mesh`; app `planting_cmds` grass_*; render/grass.ts (D5 kinds, patches,
  camera-following field); sky.ts `clouds`; View3D Grass Brush and D5 grade pass.
- Vegetation (ADR-064): studio-core `planting` (PlantSpec, 183-species catalog with season variants,
  Planting/PlantingType/GroundRegion, base ground); studio-views `plants` (grown models, proxies,
  plan symbols, silhouettes, ground regions) and `foliage` (leaf atlases, bark); app
  `planting_cmds`; components/AssetLibrary.tsx, GroundDialog.tsx; render/plants.ts,
  grass.ts (Enscape Grass type), plantThumbs.ts. Ground swatches: `cargo test --release -p
  rufplan-studio write_ground_previews -- --ignored`.
- Live Realistic view (ADR-063): View3D composer (GTAO, bloom), auto exposure, physical sky;
  components/View3DPanels.tsx (SunPanel, NavBar), render/navigate.ts (Walk/Fly), studio-core
  `lighting::sun_for`; Render adds pathtrace `lensEffects` (glare, vignette).
- Generated materials (ADR-061): studio-views `texgen` (KINDS, generate); studio-core library
  Siding category and `gen:` roofing presets; app `material_cmds::generated_map` (4096 px PNG
  cache); pathtrace `textureSizeFor`; boxUv maps sloped faces along the slope.
- Grids, frames and tags (ADR-060): studio-views `handles::grip_snap` (grid ends), tag drag
  areas (key `tag`), `view_refs::room_in_view` / `room_tag_base`; studio-core
  `visibility::tag_room_in_view`; door and window jambs (and casings) in plan symbols.
- Elevations and sections (ADR-059): studio-views `view_refs` (segments, snaps, references,
  model_point); `opening_preview` there places on the wall face; `create_lighting_fixture` works
  out level and height from the click.
- Wall openings (ADR-058): studio-core `wall_opening` (WallFrame, finish, rings); SketchKind
  WallOpening with a wall work plane (app `sketching::WallPlane`); studio-regen `WallSolid.holes`;
  studio-views `edges::wall_with_holes` / `hole_cuts_at`, `view_frame`; tool wallOpening (Openings tab).
- Lighting (ADR-057): studio-core `lighting` (FixtureSpec, catalog, create_fixture, on_wall,
  set_lights, SunSettings, project_sun_now); studio-views `lighting` (plan symbols, meshes with
  `Mesh.glow`, thumb, lights); app `lighting_cmds`; components/LightPicker.tsx and
  LightingDialogs.tsx; pathtrace `fixtureLight`; RenderDialog `SCHEMES`. Ribbon: Openings (doors,
  windows) after Architecture, Lighting before Materials, Sheets after View.
- Tab selection (ADR-056): studio-views `pick_all`, `pick_candidates` (PickCandidate); app
  `pick_cycle` / `pick_candidates`; the `select-tab` event in ViewCanvas and View3D; store `hoverLabel`.
- Contextual Modify tab (ADR-055): components/ModifyContext.tsx (contextLabel, the panels by
  category); app `selection_categories`, `tag_elements` (studio-core `visibility::tag_elements`).
- Detail and model lines (ADR-054): studio-core `lines` (LineStyle, create_lines), studio-views
  `line_style`; app `lines_cmds`, `lines.ts`; tools detailLine (DL) / modelLine (LI).
- Level ends (ADR-052): View `level_ends` (studio-core `LevelEnds`), studio-views `level_line` /
  `apply_level_ends`, level grips in `handles`, `edit::drag_handle` key `level_end:view:side:other`.
- Edit Model plans (ADR-051): studio-regen `model_ops` (ModelPlan, the operations, preview/apply);
  studio-core `model_edit::inventory` (ids and coordinates Claude is given).
- Edit Model with Claude (ADR-050): studio-core `model_edit` (ModelEdit, preview/apply, describe)
  and `modify::stretch`; app `model_edit_cmds`; components/EditModel.tsx (pill + dialog, ⌘K).
  Live check: `cargo test -p rufplan-studio live_model_edit -- --ignored --nocapture` (uses API credit).
- Slopes (ADR-049): studio-core `slope` (FloorSlope, SlopeFormat, parse/format), studio-regen
  `SlabSolid::tilt`, studio-views `slopes` (slope_at, spot_slope).
- Symbols (ADR-048): studio-core `symbols` (elements, styles) and `standards_catalog` (choices
  for all 90 standards); studio-views `symbols` and `standards_preview`; studio-sheets placed key
  plans; components/StandardChoices.tsx; Annotate > Symbol.
- Standards tab (ADR-047): studio-core `standards` (14 categories, 90 items, libraries),
  app `standards_cmds`, components/Standards.tsx; the first ribbon tab.
- Section box caps (ADR-044): studio-geom `mesh_section`, `even_odd_in_rect`; studio-views
  `caps::section_caps`; View3D `buildCaps` / `applyCaps`.
- Dimensions (ADR-040): studio-core `dimension` (references, strings, linear, angular);
  studio-views `dimension_string`, `angular`; tools dimension / dimensionLinear /
  dimensionAngular.
- Activate View (ADR-039): store `activeViewport`, ViewCanvas `onSheet`; stretchable view
  titles are `Viewport.title_length` (studio-sheets `title_line`, `sheet_handles`,
  `drag_title`).
- Visual styles (ADR-038): app/src/render/visualStyle.ts, components/VisualStyleToggle.tsx;
  studio-views `edges` (a wall's seamless triangles and its own lines, `Mesh.edges`).
- ViewCube (ADR-037): app/src/render/viewCube.ts (drawn by the 3D view's renderer in its
  corner) and components/ViewCubeOverlay.tsx (Home, arrows, menu).
- Plans to 3D (ADR-036): studio-core `plans` (tracing to model) and app `plans_cmds`; pdf.js
  (`pdfjs-dist`, blob worker) reads PDF pages. Live check: `PLANS_IN=inputs.json
  PLANS_OUT=reading.json cargo test -p rufplan-studio live_plans -- --ignored` (uses API credit).
- IFC import (ADR-035): studio-io `step` (STEP reader) and `ifc_import`; File > Open IFC.
  Dev aid: `IFC_IN=file.ifc cargo test -p studio-io import_file -- --ignored --nocapture`.
- Paint (ADR-034): studio-core `paint` (per-element material as the `rufplan.paint`
  parameter); Materials > Paint or PT.
- Sheet sets (ADR-032): studio-sheets `sets` (plan, create, deliverables, as_of_stage);
  View > Sheet Sets. Dev aid: `cargo test -p rufplan-studio write_sample_sets -- --ignored`.
- V-Ray-style lighting and photo backgrounds (ADR-028): Preetham sun & sky, Poly Haven CC0
  panoramas in app/public/backgrounds, ground projection, transparent PNG export.
- Sample IFC: `cargo test -p rufplan-studio write_sample_ifc -- --ignored`.
- Review the sample PDF: `cargo test -p rufplan-studio write_sample_pdf -- --ignored`
  writes target/sample-drawing-set.pdf.
- Last updated: 2026-09-28
