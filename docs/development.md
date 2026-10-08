# Development and packaging

## Windows 10/11 x64

Install Node.js 22 or newer, pnpm 11.24.0, a current stable Rust MSVC toolchain,
Microsoft C++ Build Tools (Desktop development with C++, MSVC and Windows SDK),
and Microsoft Edge WebView2 Runtime. See [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

From the repository root:

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

The Vite server alone cannot manage files; launch through Tauri to exercise native commands.
The app initializes its SQLite database in the OS app-data directory for `io.modelshelf.desktop`.
Development uses the same identifier: back up your database before testing migrations against valuable data.

## Checks

```powershell
pnpm typecheck
pnpm lint
pnpm test
pnpm build:ui
pnpm format:check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Network recovery tests use loopback HTTP servers and small deterministic files; they do not download model weights.
Tests needing Windows symbolic-link privileges document their constraints.
The live provider smoke test, when present, is explicitly ignored by default and can be run separately.

## Production package

```powershell
pnpm build
```

Tauri compiles the frontend and native Rust application, then creates a per-user NSIS installer under
`target/release/bundle/nsis`. The standalone executable is `target/release/modelshelf.exe`.
The installer can provision WebView2 if missing. Users do not need development toolchains.
The initial package is unsigned; no updater is enabled. A maintainer must configure code signing
and complete the manual release checklist before publishing a stable release.

## Dependency management

Commit both `pnpm-lock.yaml` and `Cargo.lock`. Only esbuild's install script is allowed by pnpm.
Review dependency changes and run checks before updating lockfiles. CI uses Windows as the reference platform.
Linux and macOS source portability is planned; installers and credential backends there are not release-validated.

## Native smoke test

1. Search for `openai-community/gpt2`; open its details and select only `config.json`.
2. Choose an empty writable directory and download. Confirm library presence after restart.
3. Import a test GGUF or model folder, remove the entry, and confirm the original files still exist.
4. Start a larger transfer, pause, restart the app and resume. Inspect reused-byte reporting.
5. Test invalid token, gated access denial, missing drive, and insufficient disk space.
6. Switch theme and language, restart and confirm settings persist.

An optional WebView2 automation script is provided in `scripts/native-smoke.mjs`. Launch a test instance with
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223`, then run `node scripts/native-smoke.mjs`.
It uses real IPC and downloads the tiny public `config.json` into the configured default directory.
After closing/reopening that instance, run `node scripts/native-smoke.mjs --restart-check`.
The script is intentionally opt-in: it writes real application state. Only enable remote debugging for a local test session.

See [implementation-status.md](implementation-status.md) for checks actually executed in this environment.
