use crate::{DownloadJob, Model, Settings, StorageLocation};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

pub struct Database {
    connection: Mutex<Connection>,
}
impl Database {
    /// Move within queued tasks atomically; row ordering is kept separate from public timestamps.
    pub fn move_job_up(&self, id: &str) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let mut jobs: Vec<(i64, String, DownloadJob)> = Vec::new();
        {
            let mut statement = tx.prepare(
                "SELECT rowid,created_at,payload FROM download_jobs ORDER BY created_at,rowid",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (rowid, sort, payload) = row?;
                let job: DownloadJob = serde_json::from_str(&payload)?;
                if job.status == crate::DownloadState::Queued {
                    jobs.push((rowid, sort, job));
                }
            }
        }
        let index = jobs
            .iter()
            .position(|(_, _, job)| job.id == id)
            .ok_or_else(|| anyhow::anyhow!("Queued job not found"))?;
        if index > 0 {
            let (left_row, left_sort, left) = &jobs[index - 1];
            let (right_row, right_sort, right) = &jobs[index];
            tx.execute("UPDATE download_jobs SET rowid=-1 WHERE id=?1", [&left.id])?;
            tx.execute(
                "UPDATE download_jobs SET rowid=?1,created_at=?2 WHERE id=?3",
                params![left_row, left_sort, right.id],
            )?;
            tx.execute(
                "UPDATE download_jobs SET rowid=?1,created_at=?2 WHERE id=?3",
                params![right_row, right_sort, left.id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn open(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            bail!("Database belongs to a newer ModelShelf version");
        }
        if version == 0 {
            let tx = connection.transaction()?;
            tx.execute_batch(include_str!("../migrations/001_initial.sql"))?;
            tx.commit()?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock poisoned"))
    }
    fn list<T: DeserializeOwned>(&self, sql: &str) -> Result<Vec<T>> {
        let conn = self.lock()?;
        let mut statement = conn.prepare(sql)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
    pub fn models(&self) -> Result<Vec<Model>> {
        self.list("SELECT payload FROM models ORDER BY rowid DESC")
    }
    pub fn jobs(&self) -> Result<Vec<DownloadJob>> {
        self.list("SELECT payload FROM download_jobs ORDER BY created_at, rowid")
    }
    pub fn save_model(&self, model: &Model) -> Result<()> {
        self.lock()?.execute("INSERT INTO models(id,path,payload) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET path=excluded.path,payload=excluded.payload",params![model.id,model.path,serde_json::to_string(model)?])?;
        Ok(())
    }
    pub fn remove_model(&self, id: &str) -> Result<()> {
        self.lock()?
            .execute("DELETE FROM models WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn save_job(&self, job: &DownloadJob) -> Result<()> {
        self.lock()?.execute("INSERT INTO download_jobs(id,created_at,payload) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![job.id,job.created_at,serde_json::to_string(job)?])?;
        Ok(())
    }
    pub fn complete_job(&self, job: &DownloadJob, model: &Model) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        tx.execute("INSERT INTO models(id,path,payload) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET path=excluded.path,payload=excluded.payload",params![model.id,model.path,serde_json::to_string(model)?])?;
        tx.execute("INSERT INTO download_jobs(id,created_at,payload) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![job.id,job.created_at,serde_json::to_string(job)?])?;
        tx.commit()?;
        Ok(())
    }
    pub fn settings(&self) -> Result<Settings> {
        let value: Option<String> = self
            .lock()?
            .query_row(
                "SELECT value FROM settings WHERE key='application'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let result = match value {
            Some(v) => serde_json::from_str(&v)?,
            None => Settings::default(),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        settings.validate()?;
        self.lock()?.execute("INSERT INTO settings(key,value) VALUES('application',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn locations(&self) -> Result<Vec<StorageLocation>> {
        let conn = self.lock()?;
        let mut statement =
            conn.prepare("SELECT path,kind FROM storage_locations ORDER BY path")?;
        let rows = statement.query_map([], |row| {
            Ok(StorageLocation {
                path: row.get(0)?,
                kind: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn add_location(&self, location: &StorageLocation) -> Result<()> {
        if !matches!(location.kind.as_str(), "managed" | "external") {
            bail!("Unknown location type");
        }
        self.lock()?.execute("INSERT INTO storage_locations(path,kind) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET kind=excluded.kind",params![location.path,location.kind])?;
        Ok(())
    }
}
