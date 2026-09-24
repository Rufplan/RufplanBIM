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
  layered floors/roofs and cut patterns (ADR-019). The "Prototype slice" section of docs/ROADMAP.md lists the shortcuts still open.
- Try it: `cd app && npm run tauri dev -- -- -- --sample` opens the sample house.
- M3 met (except the 3D section box); M4 met pending the owner's scale-ruler print check.
- M5 built (ADR-016): IFC4 export (validated by IfcOpenShell in CI), Rufplan sign-in,
  project link and Publish into Rufplan's existing deliverables (no schema change).
  Owner actions pending: allow-list the Google redirect URL; allow `application/x-step` in
  the `project-media` bucket so the IFC uploads too.
- Editing and foundation round (ADR-017, ADR-018): Revit modify tools (Copy, Rotate, Mirror,
  Array, Align, Trim/Extend, Offset, Split, Flip), grips, temporary dimensions, typed lengths;
  incremental regeneration with stamp caches (M2's 50 ms target met), category index,
  project parameters; layered wall types, roofs, stairs and crop regions. M1 and M2 are met
  except binary IPC. Next: room separation lines, floors following walls, non-convex roofs,
  stair openings, the 3D section box.
- Sample IFC: `cargo test -p rufplan-studio write_sample_ifc -- --ignored`.
- Review the sample PDF: `cargo test -p rufplan-studio write_sample_pdf -- --ignored`
  writes target/sample-drawing-set.pdf.
- Last updated: 2026-09-24
