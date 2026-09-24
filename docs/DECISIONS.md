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
