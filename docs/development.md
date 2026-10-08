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
Release builds preserve the existing `io.modelshelf.desktop` app-data directory and credential entry.
Debug builds default to `io.modelshelf.desktop.development`, with separate SQLite, downloads,
WebView data and credentials. Existing production data is never migrated or copied.
Debug builds refuse the production profile.

`MODELSHELF_PROFILE` accepts `production`, `development` or `test`. Test mode requires
`MODELSHELF_TEST_DATA_DIR` to name an existing absolute child directory of the OS temporary
directory. Invalid combinations fail before startup. Test credentials stay in memory and
never read or write the OS credential store. The test WebView uses the disposable directory;
its single-instance identity is separate from production and development.

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

Run the isolated native automation with:

```powershell
pnpm test:native
```

This builds a debug desktop executable with bundled UI, creates a fresh temporary profile,
launches only that instance, runs the real search/download/library flow, and restarts it to
check persistence, Turkish and themes. The existing production app can remain open.
The runner refuses an occupied debugging port (9223); the smoke script checks the backend's
profile and exact temporary directory before changing any state. Downloads and screenshots
stay inside the printed temporary directory, retained for inspection. It does not overwrite
repository screenshots. Test credentials are intentionally not persisted across restarts.

The runner enables WebView2 remote debugging only for its child process and terminates its
own child after each check. Windows and internet access are required. Do not run the smoke
script against an ordinary production session.

## Commit and push workflow

Preserve each completed, validated improvement in a focused commit and push it to
`origin` (`https://github.com/leongrphc/modelshelf`). Update the next-steps and implementation
status documents with actual validation results. Raise newly discovered significant work
outside the current scope with the maintainer before expanding the task.

See [implementation-status.md](implementation-status.md) for checks actually executed in this environment.

## Windows acceptance automation

`pnpm test:native` also runs eight native-dialog scenarios via a PID-scoped PowerShell helper.
Use an interactive desktop. The helper selects only paths under the temporary test profile
and exercises actual Windows file/folder pickers and Yes/No confirmations. The runner closes
its own window normally between phases. `native-test.json` and `windows-acceptance.json`
retain OS/build, scenario outcomes and errors alongside temporary screenshots.

Run `cargo test -p modelshelf-core --test windows_acceptance` for Windows filesystem cases.
These tests are also included in `cargo test --workspace` on Windows. See the
[acceptance matrix](windows-acceptance.md) for validated and still-unexecuted scenarios.
