CREATE TABLE models (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);
CREATE TABLE download_jobs (id TEXT PRIMARY KEY, created_at TEXT NOT NULL, payload TEXT NOT NULL);
CREATE INDEX download_order ON download_jobs(created_at);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE storage_locations (path TEXT PRIMARY KEY, kind TEXT NOT NULL);
PRAGMA user_version = 1;
