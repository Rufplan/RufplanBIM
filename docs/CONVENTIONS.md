# Conventions

## Rust
- Edition 2021, stable toolchain. `rustfmt` default, `clippy -D warnings`.
- Errors: `thiserror` in library crates, `anyhow` only in the Tauri app layer.
- No `unwrap()`/`expect()` outside tests except with a comment proving it can't fail.
- Public functions in core crates get doc comments with units stated (`/// length in mm`).
- Geometry tests assert with explicit tolerances from `studio_geom::tol`.
- Serialization: `serde` + `rmp-serde` for storage; `serde_json` for IPC payloads, binary
  `Vec<u8>` / typed arrays for large display lists.

## TypeScript / React
- Strict mode. Function components and hooks. No geometry math beyond screen transforms.
- IPC calls go through one typed module `app/src/ipc.ts`; generate types from Rust with
  `ts-rs` or `specta` so the boundary stays in sync.
- Tests with Vitest; UI tests with Testing Library where useful.
- Styling: keep it simple (CSS modules or Tailwind). Visual design will align with Rufplan
  later; don't over-invest in styling before M4.

## Testing
- Every milestone adds tests at the core level first. Golden-file tests for display lists
  and PDF output (compare normalized primitive lists, not bytes).
- Performance benchmarks with `criterion` for regeneration and plan generation from M2.

## Git
- Conventional commits: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, with crate scope.
- One logical change per commit. Keep `main` green.

## Documentation
- Update `docs/ROADMAP.md` checkboxes and `CLAUDE.md` "Current status" at the end of each session.
- Any decision that changes stack, file format, or architecture → new ADR in `DECISIONS.md`.
