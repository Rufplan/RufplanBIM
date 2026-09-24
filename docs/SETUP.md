# Setup (Windows first)

Verified on Windows 11 Pro starting from a machine with no Rust or MSVC installed (2026-09-23).

## Toolchain
1. **Rust** via rustup (stable, MSVC target `x86_64-pc-windows-msvc`).
   `winget install --id Rustlang.Rustup -e --source winget`
   The repo's `rust-toolchain.toml` pins `stable` with `rustfmt` and `clippy`; rustup
   installs them on first use. Open a new terminal afterwards so `cargo` is on `PATH`.
2. **Visual Studio 2022 Build Tools** with "Desktop development with C++" (MSVC + Windows SDK).
   `winget install --id Microsoft.VisualStudio.2022.BuildTools -e --source winget --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
   (about 6 GB, needs an admin prompt). Required by Tauri and by rusqlite's bundled SQLite.
3. **Node.js** LTS (20+; CI uses 22, development verified on 24).
4. **WebView2 Runtime** (preinstalled on Windows 11).
5. Tauri CLI: installed as a dev dependency by `npm install` in `app/` (run it with `npm run tauri`).
6. For M5 IFC validation tests only: Python 3.11+ with `pip install ifcopenshell`.

If `winget install` reports that several sources match, add `--source winget`.

## Editor
- **VS Code** with the Claude Code extension, plus `rust-analyzer`, `Tauri`, `ESLint`, `Prettier`.
- If using full Visual Studio instead, run Claude Code from the integrated terminal (`claude`)
  at the repo root. Rust development is smoother in VS Code with rust-analyzer.

## Run
```
cargo test --workspace
cd app
npm install
npm run tauri dev
```
The first `tauri dev` compiles about 390 crates (a few minutes); later runs are incremental.

## Checks (same as CI)
From the repo root:
```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
From `app/`:
```
npm run lint
npm run format:check
npm test
npm run tauri build     # installer: target/release/bundle/nsis/Rufplan Studio_<ver>_x64-setup.exe
```

## Install on this machine
```
cd app
npm run install:local
```
Builds the release installer and installs it per-user (no admin prompt) to
`%LOCALAPPDATA%\Rufplan Studio`, with a Start menu shortcut. Re-run it to upgrade in place;
it closes a running copy first. Uninstall from Windows Settings > Apps. Use `npm run tauri dev`
for day-to-day development and `install:local` when you want the installed app updated.

## TypeScript bindings
IPC payload types are Rust structs deriving `ts_rs::TS`. `cargo test` regenerates them into
`app/src/bindings/` (configured in `.cargo/config.toml`). Commit the regenerated files; CI
fails if they are out of date.

## Repository location
The repo can live in a cloud-synced folder, but the sync client should ignore `target/`
(several GB, rewritten on every build) and `app/node_modules/`. If the client offers
folder exclusions, add both. Close any `.rfproj` in the app before the sync client
uploads it.

## Secrets
Not needed until M5. Then create `app/.env.local` (git-ignored) with:
```
VITE_SUPABASE_URL=...
VITE_SUPABASE_ANON_KEY=...
```
Never put the service-role key anywhere in this repository.
