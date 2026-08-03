# Build and development

This repository uses the standard Tauri project layout:

- `src/` — React/Vite frontend
- `src-tauri/` — Rust application, `Cargo.toml`, Tauri configuration, capabilities and icons
- `src-tauri/assets/` — optional bundled fonts, audio, FFmpeg and Unicode data
- `src-tauri/assets/data/` — `UnicodeData.txt` and local `UnicodeBlocks.txt`

The repository root does not contain a `Cargo.toml`. Run Tauri commands from the repository root so the CLI can discover `src-tauri/tauri.conf.json` and execute the frontend hooks. Run plain Cargo commands either from `src-tauri` or with `--manifest-path src-tauri/Cargo.toml`.

## Prerequisites

- Node.js 22 or 24
- pnpm 11.18.0, pinned by `package.json`
- Rust stable with `rustfmt` and `clippy`
- Tauri CLI 2.9 or newer
- FFmpeg and the platform dependencies listed in the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

For a new environment, enable a current Corepack installation:

```powershell
npm install --global corepack@latest
corepack enable pnpm
```

## Development

From the repository root:

```powershell
pnpm install --frozen-lockfile
rustup component add rustfmt clippy
cargo install tauri-cli --version "^2.9.0" --locked
cargo tauri dev
```

Run only the browser frontend with:

```powershell
pnpm dev
```

The browser preview cannot invoke Tauri dialogs, task events or Rust rendering commands.

## Build unsigned installers

From the repository root:

```powershell
pnpm install --frozen-lockfile
cargo tauri build --ci --no-sign -- --locked
```

Tauri runs `pnpm build` through `build.beforeBuildCommand`. Bundles are written below:

```text
src-tauri/target/release/bundle/
```

For a signed production build, configure platform signing credentials and omit `--no-sign`.

## CLI-only build or execution

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- <command>
```

On macOS and Linux, a GUI application launched outside a shell may not inherit the shell's complete `PATH`. Use an explicit FFmpeg path or bundle a redistributable FFmpeg build when needed.
