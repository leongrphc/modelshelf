# Architecture

ModelShelf is a Tauri 2 desktop application. React renders state; Rust owns every filesystem, credential, network and database operation. The packaged app requires no Node, Python or Rust installation.

## Boundaries

- `modelshelf-core`: typed domain records, SQLite migrations, path validation, safe local library operations.
- `modelshelf-hub`: Hugging Face provider implementation. Search and metadata are independent of filesystem operations.
- `modelshelf-download`: persistent streaming transfer service and scheduler, independent of Tauri.
- `apps/desktop/src-tauri`: native dialogs, OS credentials, hardware information, validated commands and throttled change events.
- `apps/desktop/src`: React feature pages with TanStack Query. Backend snapshots are authoritative; no synthetic metrics.

SQLite lives in Tauri's OS-specific application data directory. WAL and foreign keys are enabled. Credentials live in the OS credential manager, never SQLite. Model directories are selected by the user and downloads create separate job directories to avoid overwriting unrelated files. External library entries confer no ownership.

Each job pins the provider commit and enumerates individual files. Partial files remain separate from final files. Transfer completion requires size validation and SHA-256 when supplied by trusted metadata. A locally computed hash without a trusted comparison is Unverified. Recovery reconciles disk state with persisted records.

The frontend receives a coalesced state event once per second. Slow operations run in async commands or blocking workers, not the webview/main event loop. A single-instance plugin prevents two schedulers writing the same database.

## Design

The interface uses a permanent narrow sidebar and a workspace with precise technical tables. Violet indicates actions; ownership and integrity get explicit text labels. Dark backgrounds, subtle borders, and readable monospace paths keep large repositories legible. Light and system themes share the same hierarchy.

## References researched

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Hugging Face Hub endpoints](https://huggingface.co/docs/hub/api)
- [Hugging Face downloads](https://huggingface.co/docs/huggingface_hub/guides/download)
- [Xet storage](https://huggingface.co/docs/hub/xet/index)

HTTP-compatible Xet downloads do not imply native Xet chunk acceleration. The service reports reuse based on actual validated partial bytes, not renewed signed URLs.
