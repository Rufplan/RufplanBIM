# Setup (Windows first)

## Toolchain
1. **Rust** via rustup (stable, MSVC target `x86_64-pc-windows-msvc`).
2. **Visual Studio 2022 Build Tools** with "Desktop development with C++" (MSVC + Windows SDK).
3. **Node.js** LTS (20+).
4. **WebView2 Runtime** (preinstalled on Windows 11).
5. Tauri CLI: `npm i -D @tauri-apps/cli` inside `app/`.
6. For M5 IFC validation tests only: Python 3.11+ with `pip install ifcopenshell`.

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

## Secrets
Create `app/.env.local` (git-ignored) with:
```
VITE_SUPABASE_URL=...
VITE_SUPABASE_ANON_KEY=...
```
Never put the service-role key anywhere in this repository.
