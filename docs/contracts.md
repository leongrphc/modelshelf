# Internal contracts

All JSON uses snake_case. Rust domain contracts live in modelshelf-core.

RemoteFile { path: String, size: u64, sha256: Option<String> }
Repository { id: String, sha: String, task: Option<String>, downloads: u64, likes: u64, tags: Vec<String>, license: Option<String>, updated_at: Option<String>, gated: bool, files: Vec<RemoteFile>, readme: String, revisions: Vec<String> }
Model { id, display_name, path, ownership (managed/external), repository: Option<String>, revision: Option<String>, created_at: String, favorite: bool, notes: String, tags: Vec<String>, files: Vec<LocalFile> }
LocalFile { path: String (absolute), relative_path: String, size: u64, expected_sha256: Option<String>, actual_sha256: Option<String>, verification: String (Verified/Unverified/Corrupted/Missing) }
DownloadFile { path: String (remote relative), size: u64, downloaded: u64, sha256: Option<String>, etag: Option<String>, status: DownloadState, verification: String, reused_bytes: u64 }
DownloadJob { id, repository, revision, destination: String (dedicated newly created job directory), status: DownloadState, files: Vec<DownloadFile>, error: Option<String>, created_at: String, speed: u64 }
DownloadState strings: Queued Preparing Downloading Pausing Paused Resuming Verifying Completed Failed Cancelled.
Settings { theme: String (dark/light/system), language: String (en/tr), concurrency: usize (1..3), retries: u32 (0..5), notifications: bool, default_directory: String }
StorageLocation { path: String, kind: String }

Core public APIs (anyhow::Result): Database::open(path: &Path) -> Self; models() -> Vec<Model>; save_model(&Model); remove_model(id: &str); jobs() -> Vec<DownloadJob>; save_job(&DownloadJob); settings() -> Settings; save_settings(&Settings); locations() -> Vec<StorageLocation>; add_location(&StorageLocation). Database internally synchronized, share Arc<Database>.
Storage public functions in core::storage: import_path(db: &Database, path: &Path) -> Model; verify_model(db: &Database,id: &str) -> Model; delete_managed(db: &Database,id: &str,confirmed: bool) -> (); validate_relative(path: &str) -> (); validate_tree(path: &Path) -> (); sha256(path: &Path) -> String; disk_info(path: &Path) -> (u64,u64) capacity/free. No follow of reparse points on writes/deletes. External imports index in place. remove_model only removes index.
core::parse_repository(input: &str) -> Result<String>; core::quantization(name: &str) -> Option<String>.

Hub API: Hub::new(token: Option<String>) -> Result<Self>; async search(query: &str,sort: &str,task: &str) -> Result<Vec<Repository>>; async details(repo: &str,revision: &str) -> Result<Repository>; async username() -> Result<String>. Remote URLs fixed HF origin. Hub handles safe redirects/token stripping.
Download API: DownloadManager::new(db: Arc<Database>) -> Arc<Self>; start(self: &Arc<Self>) launches scheduler; async create_job(self: &Arc<Self>,repo: Repository,selected: Vec<String>,destination: PathBuf) -> Result<DownloadJob>; action(&self,id: &str,action: &str) -> Result<()> (pause/resume/cancel/retry/up); set_token(&self,Option<String>). Scheduler consults DB settings. Persist recovery states on start. Poll snapshots via DB from Tauri; root can emit throttled changes.

Tauri commands frontend invokes:

- snapshot() -> {models: Model[],jobs: DownloadJob[],settings: Settings,locations: StorageLocation[],hardware: {os:string,cpu:string,ram:number,gpu:string},disks: {path:string,total:number,free:number,error?:string}[]}
- search_models({query,sort,task}) -> Repository[]
- repository_details({repo,revision}) -> Repository
- choose_directory() -> string|null (native Rust dialog; records authorization)
- import_model({directory:boolean}) -> Model|null (native dialog + confirmation for folder)
- create_download({repo,revision,selected:string[],destination:string}) -> DownloadJob (backend refetch pinned metadata)
- download_action({id,action}) -> void
- remove_model({id}) -> void (index only)
- delete_model({id}) -> void (native confirmation)
- verify_model({id}) -> Model
- update_model({id,favorite,notes,tags}) -> void
- open_model_folder({id}) -> void
- save_settings({settings}) -> void
- add_location() -> string|null
- connect_account({token}) -> string
- account_status() -> string|null
- disconnect_account() -> void
- open_repository({repo}) -> void (validated HF URL)
- export_diagnostics() -> string|null (native save dialog, excludes credentials/paths)
  Event: state-changed (invalidate snapshot; emitted every 1s when jobs change).
  Frontend must show desktop-required error if opened in browser; no mock production state.
