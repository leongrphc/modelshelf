# Windows acceptance record

Date: 2026-10-08. Host: **Windows 11 Pro x64, build 26200**. Installed WebView2:
**154.0.4258.62**. This is one development machine, not a clean installation matrix.
All fixtures are temporary; existing user libraries and credentials are excluded.

## Automated filesystem checks

Command: `cargo test -p modelshelf-core --test windows_acceptance`.

| Scenario                                              | Expected                                                                            | Observed                                                               |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Spaces and Turkish characters in model/database paths | Import, reopen and hash verification succeed; removing index preserves originals    | Passed on this host                                                    |
| Model paths longer than 260 UTF-16 code units         | Import, hashing, usage and managed deletion work; unrelated files survive           | Passed on this host; native picker long-path behavior remains untested |
| Indexed storage directory renamed away and restored   | Missing status without lost records; integrity verification works after restoration | Passed; this is a directory rename, not physical disk removal          |
| Managed file held with exclusive Windows sharing mode | Deletion reports sharing violation and retains index; retry succeeds after release  | Passed with OS error 32; this is not an ACL denial test                |

These checks run in the ordinary workspace Rust suite on Windows.

## Native desktop checks

Command: `pnpm test:native` (requires an interactive Windows desktop and internet access).
The runner builds the bundled debug app and creates a fresh temporary profile. It checks
the backend profile before interactions and scopes dialog automation to the child process.
Native dialog automation never uses global keyboard input. Only fixture paths inside the
temporary profile may be selected. Only files from that profile may be deleted.

Each run retains `native-test.json`, `windows-acceptance.json`, downloaded fixtures and
screenshots in its printed temporary directory. Reports record actual OS/build and failure
details. The runner closes its own native window normally; forced termination is failure cleanup.

The search/download/library/integrity flow and restart/settings/theme checks passed on
this host, with normal native window closure between stages. Eight native acceptance scenarios passed on this host:

| Scenario                                    | Expected                                                | Observed |
| ------------------------------------------- | ------------------------------------------------------- | -------- |
| File picker import with Turkish/space path  | Index as external; preserve original bytes              | Passed   |
| External permanent deletion request         | Reject before confirmation; preserve bytes              | Passed   |
| Remove external library entry               | Remove index only                                       | Passed   |
| Folder picker and native index confirmation | Index only after Yes; preserve all source bytes         | Passed   |
| Storage folder picker                       | Register exactly the chosen Turkish/space path          | Passed   |
| File picker cancellation                    | Return no selection; library unchanged                  | Passed   |
| Native permanent-delete No                  | Managed bytes and library entry unchanged               | Passed   |
| Native permanent-delete Yes                 | Recorded managed file deleted; unrelated file preserved | Passed   |

Completed runner evidence: `modelshelf-test-4RjvRo` in the host OS temporary directory.
No private absolute user paths are copied into the repository. Native dialog automation
uses UIAutomation and, where Windows exposes legacy controls without patterns, validated
child window handles. It never presses keys or clicks outside the test process.

## Still required before stable release

| Environment or scenario                                | Status                                                           |
| ------------------------------------------------------ | ---------------------------------------------------------------- |
| Clean Windows 10 x64 install, launch, close, reinstall | Not run; no Windows 10 VM available in this session              |
| Clean Windows 11 x64 install and reinstall             | Not run; current host has development tooling and existing state |
| Installer with WebView2 present and absent             | Not run; installed runtime was not removed from the host         |
| Actual external disk unplug and changed drive letter   | Not run; directory rename coverage is not equivalent             |
| ACL write denial and low disk capacity                 | Not run; exclusive sharing violation coverage is not equivalent  |
| Disk full during active download                       | Not run; requires a constrained disposable volume                |

Use disposable VMs and volumes for these cases. Record installer hash, Windows build,
runtime version, steps, expected/observed behavior and retained logs for each run.
Do not check off the whole P0/2 milestone based only on the automated cases above.

## Follow-up: cancellation and error feedback

Managed deletion now returns `false` on native No and `true` only after completion.
Rescan cancellation returns `null`; save-dialog cancellation also remains `null`.
The frontend treats these as cancellation, clears stale notices when starting an action,
and shows success only for completed operations. Detail dialogs remain open after
cancellation or failure, and failed account input remains editable.

Additional regression cases exercise rescan No/Yes, invalid account input, cancelled
diagnostic save, and unavailable-directory deletion followed by successful retry.
The managed-delete cases now click the actual UI button and verify notification and
modal behavior as well as filesystem contents.

Final follow-up run: `modelshelf-test-FPXqZe`; all **13** native scenarios and normal
window closure passed. Additional expected/observed cases:

| Scenario                                     | Expected                                           | Observed |
| -------------------------------------------- | -------------------------------------------------- | -------- |
| Rescan No                                    | Keep detail/data; no success or error toast        | Passed   |
| Rescan Yes                                   | Refresh files and show success                     | Passed   |
| Invalid account input                        | Show error; retain editable input                  | Passed   |
| Diagnostic-save cancellation                 | Clear stale notice; no success/error               | Passed   |
| Delete unavailable directory, restore, retry | Show error and retain detail/index; retry succeeds | Passed   |

The existing delete No/Yes cases additionally verify modal and toast behavior through UI buttons.
