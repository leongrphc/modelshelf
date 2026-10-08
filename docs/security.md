# Security and privacy design

- No telemetry, cloud account, downloaded-code execution or inference runtime.
- Public discovery works anonymously. Optional Hugging Face tokens are stored by the OS credential store.
- Provider requests use TLS. Bearer credentials are sent only to `huggingface.co`; signed CDN redirects do not receive them.
- The provider accepts only supported Hugging Face model identifiers and URLs. HTTP is used exclusively by local test fixtures.
- README content is displayed as inert text. Remote HTML, scripts and image trackers are not loaded.
- Frontend capabilities expose no general filesystem, HTTP, SQL or shell APIs. Native commands validate their inputs.
- Native directory selection authorizes a managed location. Downloads create unique child directories and refuse to overwrite existing output files.
- Repository paths reject traversal, absolute paths, alternate streams and Windows reserved names.
- Existing symlinks and Windows reparse points are rejected for managed writes and deletion. External files are index-only.
- Managed deletion requires native confirmation and matching download ownership metadata. Only recorded files are removed; unrelated files are retained.
- Local metadata includes absolute model paths. Diagnostic export deliberately excludes paths, repository names, errors, and credentials.

## Threat model and limitations

Remote repositories are untrusted. A malicious process running as the same OS user is outside the strong isolation boundary:
it can alter application data or race filesystem checks. Path validation is defense in depth, not a filesystem sandbox
against a hostile local administrator. Protect OS credentials and use normal user permissions.

SHA-256 verification establishes agreement with provider metadata, not that a model is safe, licensed for your use,
or compatible with a runtime. Files with only a size check stay Unverified. Do not execute untrusted model code.
ModelShelf never accepts gated repository terms on the user's behalf.

No automatic updater is included. Future updates must use Tauri's signed-update mechanism.
