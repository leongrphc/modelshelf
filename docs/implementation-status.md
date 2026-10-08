# Implementation status

Experimental 0.1.0 Windows MVP. No stable release claim. Validation performed on Windows x64, 2026-10-08.

- [x] Inspect empty workspace and current official Tauri/Hugging Face docs
- [x] Define contracts and ownership boundaries
- [x] SQLite/domain/storage safety foundation
- [x] Hugging Face discovery and repository files
- [x] Persistent downloads, identity-checked recovery and integrity
- [x] Desktop commands and native dialogs
- [x] Six-page UI, English/Turkish, themes
- [x] Unit/integration/recovery tests
- [x] Windows NSIS package generated from final validated sources
- [x] Community documentation and honest limitations

## Checks executed

| Check                                | Result                                                                                                                                    |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| pnpm dependency installation         | Passed                                                                                                                                    |
| TypeScript validation                | Passed                                                                                                                                    |
| ESLint                               | Passed                                                                                                                                    |
| Vitest                               | 6 passed                                                                                                                                  |
| Frontend production build            | Passed; upstream Zod comment annotation warnings only                                                                                     |
| Rust formatting                      | Passed                                                                                                                                    |
| Clippy, all targets, warnings denied | Passed                                                                                                                                    |
| Rust workspace tests                 | 19 passed; 1 opt-in live test ignored in ordinary suite                                                                                   |
| Live Hugging Face test               | Passed separately: public search, details, config.json download, database reopen                                                          |
| pnpm production dependency audit     | No known vulnerabilities found                                                                                                            |
| Tauri production build               | Passed; Windows x64 NSIS installer generated                                                                                              |
| Native WebView2 automation           | Search, file selection, download, library, integrity check, restart persistence, all six pages, Turkish and themes passed; no page errors |

## Implemented scope

Search/details and a paginated recursive file browser; commit-pinned selected-file downloads; queue reorder,
1–3 concurrent transfers; bounded retries; pause/resume/cancel; identity-checked prefix reuse;
streaming integrity checks; persistent crash recovery; ownership-safe imports/removal/deletion;
favorites, notes and tags; multiple storage locations; physical-file-aware logical usage;
recorded SHA/physical duplicate analysis; existence recheck/rescan; Windows DXGI hardware information;
secure optional tokens; local diagnostics; primary UI translations; community and CI configuration.

The database uses versioned migrations and indexed JSON domain payloads rather than one SQL table per file.
This keeps per-job updates transactional for the MVP. Large-catalog indexing/normalized file queries remain a scaling task.

## Remaining work and genuine limitations

- Stable-release acceptance on Windows 10/11 across disconnected disks, constrained storage, gated accounts,
  very large catalogs, and long multi-gigabyte transfers. Automated fixtures use small deterministic files.
- Native folder import dialogs and permanent-delete confirmation require broader interactive acceptance testing;
  underlying external preservation and junction rejection are covered by Rust tests.
- Native Xet chunk acceleration, shared cache symlink scanning, copy-on-import, launch-at-login,
  relocation across drives, automatic partial cleanup, and runtime dependency inference are deferred.
- Manual association of an external model with a provider repository and richer model-header inspection are deferred.
- Search displays up to 48 results per query. Repository file retrieval follows provider pagination.
- Hardlink-capable storage is required for no-overwrite publication. Unsupported filesystems fail without replacing output.
- Invalid range responses fail safely; a server returning a full body restarts safely. Cooperative pause can wait for the read timeout.
- External symlinks/junctions are rejected conservatively; logical size is not allocated disk size.
- Error messages/native dialogs remain English. Main page controls and state labels have English/Turkish translations.
- Public repository/support URLs, code signing and a private security contact must be configured by the publishing maintainer.
- Linux/macOS packaging, signed updates and future providers remain roadmap items.

No claim is made that every item in the broader product vision is complete. The delivered scope is a functioning MVP
with tested safety/recovery mechanisms and explicitly recorded release limitations.

## Follow-up: public repository and profile isolation (2026-10-08)

- Public source: https://github.com/leongrphc/modelshelf. Initial MVP snapshot: `a3c79ea`.
- Repository description/topics and private vulnerability reporting configured. README includes CI, MIT, issue and contribution links.
- Production data/credentials retain their existing identity; debug builds use a separate development identity. No production data migration occurs.
- Test mode requires an explicit temporary directory, isolates WebView storage and never opens the OS credential store. Four profile tests cover defaults, invalid configurations, memory-only credentials and unchanged production/development database fixtures.
- Native automation launches its own disposable profile, verifies runtime identity before interactions, keeps screenshots/downloads there, and restarts only its own process. Search, download, library, integrity, restart, Turkish and themes passed with no page errors while the existing production app remained open.
- Initial CI failures exposed Windows checkout line endings and missing audit check permission; both corrected in focused commits. Remote validation of `22a4c46` passed: Windows checks, NSIS packaging and dependency audit ([run](https://github.com/leongrphc/modelshelf/actions/runs/37829673331)). The two audit warnings remain visible.
- Rust audit reported `proc-macro-error 1.0.4` unmaintained (`RUSTSEC-2024-0370`) and `glib 0.18.5` unsound iterator implementations (`RUSTSEC-2024-0429`). Warnings remain visible; dependency remediation has not been performed.
- This follow-up does not claim a new release installer or the broader Windows acceptance matrix.

Follow-up local checks: 23 Rust tests and 6 Vitest tests passed; TypeScript, ESLint, workspace clippy with warnings denied, formatting and bundled debug build passed. `cargo tree --target x86_64-pc-windows-msvc -i` found neither warned crate in the Windows dependency tree; other-platform remediation remains pending.

## Follow-up: Windows acceptance automation (2026-10-08)

Host: Windows 11 Pro x64 build 26200, WebView2 154.0.4258.62. See [acceptance record](windows-acceptance.md).

- Four new Windows filesystem tests passed: Turkish/space paths with database reopening, >260 UTF-16-unit paths, missing/restored directory recovery, and exclusive sharing violation with safe retry.
- Eight native dialog scenarios passed: file/folder indexing, external deletion refusal, non-destructive library removal, storage selection, picker cancellation, native No preservation and Yes deletion preserving unrelated files.
- The automation scopes native controls to its own process and fixture directory, reports per-scenario JSON evidence, and now closes the real native window normally between stages. Download, library and settings persist across restart.
- Windows 10/clean installation/reinstallation, missing WebView2, physical disk removal, drive-letter changes, ACL denial and low/full disk tests remain unexecuted.
- Code inspection identified cancellation reported as successful deletion in the frontend; preservation is validated, but result messaging needs a separate fix.

Final local acceptance checks: 27 workspace Rust tests passed (one separate opt-in live test ignored); targeted core clippy and workspace formatting passed. The final native runner passed all three phases with normal shutdown and all eight dialog cases. Frontend source was unchanged in this follow-up.

## Follow-up: accurate cancellation and failure feedback (2026-10-08)

- Managed-delete No returns false; confirmed completion returns true. Rescan No returns null instead of an error.
- The action runner distinguishes cancellation from completed void commands and clears stale notices when a new action starts. Detail dialogs stay open on cancellation/error; unsuccessful account connection retains editable input.
- Native acceptance now has 13 scenarios. Added rescan No/Yes, invalid account input retention, diagnostic-save cancellation and failed managed deletion followed by retry. Delete cases use the real UI and assert success/error toast and modal behavior as well as unchanged/deleted bytes.
- Final runner `modelshelf-test-FPXqZe` passed all 13 cases and all three phases with normal native closure. The cancellation screenshot confirms the detail remains open without a success notice.
- 27 workspace Rust tests, 6 Vitest tests, TypeScript, ESLint, workspace clippy, bundled debug build and formatting passed. The Impeccable mechanical detector reported no findings in changed frontend files.
- Clean OS/installer and physical disk matrix remains pending as documented; no new release installer is claimed.
