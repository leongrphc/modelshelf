export interface RemoteFile {
  path: string;
  size: number;
  sha256: string | null;
}
export interface Repository {
  id: string;
  sha: string;
  task: string | null;
  downloads: number;
  likes: number;
  tags: string[];
  license: string | null;
  updated_at: string | null;
  gated: boolean;
  files: RemoteFile[];
  readme: string;
  revisions: string[];
}
export interface LocalFile {
  path: string;
  relative_path: string;
  size: number;
  expected_sha256: string | null;
  actual_sha256: string | null;
  verification: string;
}
export interface Model {
  id: string;
  display_name: string;
  path: string;
  ownership: "managed" | "external";
  repository: string | null;
  revision: string | null;
  created_at: string;
  favorite: boolean;
  notes: string;
  tags: string[];
  files: LocalFile[];
}
export type DownloadState =
  | "Queued"
  | "Preparing"
  | "Downloading"
  | "Pausing"
  | "Paused"
  | "Resuming"
  | "Verifying"
  | "Completed"
  | "Failed"
  | "Cancelled";
export interface DownloadJob {
  id: string;
  repository: string;
  revision: string;
  destination: string;
  status: DownloadState;
  files: {
    path: string;
    size: number;
    downloaded: number;
    sha256: string | null;
    etag: string | null;
    status: DownloadState;
    verification: string;
    reused_bytes: number;
  }[];
  error: string | null;
  created_at: string;
  speed: number;
}
export interface Settings {
  theme: "dark" | "light" | "system";
  language: "en" | "tr";
  concurrency: number;
  retries: number;
  notifications: boolean;
  default_directory: string;
}
export interface Snapshot {
  usage: {
    managed_bytes: number;
    external_bytes: number;
    unique_logical_bytes: number;
    by_format: { format: string; bytes: number }[];
    duplicates: { kind: string; paths: string[] }[];
    unavailable_files: number;
  };
  temporary_bytes: number;
  models: Model[];
  jobs: DownloadJob[];
  settings: Settings;
  locations: { path: string; kind: string }[];
  hardware: { os: string; cpu: string; ram: number; gpu: string };
  disks: { path: string; total: number; free: number; error?: string }[];
}
