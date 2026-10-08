use anyhow::{bail, Context, Result};
use modelshelf_core::{
    storage, Database, DownloadFile, DownloadJob, DownloadState as State, LocalFile, Model,
    Repository,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct DownloadManager {
    db: Arc<Database>,
    token: Mutex<Option<String>>,
    active: Mutex<HashSet<String>>,
    gate: Mutex<()>,
    started: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    test_url: Mutex<Option<reqwest::Url>>,
}

impl DownloadManager {
    pub fn new(db: Arc<Database>) -> Arc<Self> {
        Arc::new(Self {
            db,
            token: Mutex::new(None),
            active: Mutex::new(HashSet::new()),
            gate: Mutex::new(()),
            started: std::sync::atomic::AtomicBool::new(false),
            #[cfg(test)]
            test_url: Mutex::new(None),
        })
    }
    pub fn set_token(&self, token: Option<String>) {
        *self.token.lock().unwrap() = token;
    }
    pub fn start(self: &Arc<Self>) {
        if self.started.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let manager = self.clone();
        tokio::spawn(async move {
            if let Ok(jobs) = manager.db.jobs() {
                for mut job in jobs {
                    if matches!(
                        job.status,
                        State::Preparing
                            | State::Downloading
                            | State::Resuming
                            | State::Pausing
                            | State::Verifying
                    ) {
                        job.status = State::Paused;
                        for file in &mut job.files {
                            if file.status != State::Completed {
                                file.status = State::Paused;
                            }
                        }
                        job.speed = 0;
                        job.error=Some("Interrupted session. Resume validates the partial file and remote identity before reusing bytes.".into());
                        if let Err(error) = manager.db.save_job(&job) {
                            tracing::error!(%error,"Cannot persist interrupted download recovery");
                        }
                    }
                }
            }
            loop {
                if let (Ok(jobs), Ok(settings)) = (manager.db.jobs(), manager.db.settings()) {
                    for job in jobs
                        .into_iter()
                        .filter(|j| matches!(j.status, State::Queued | State::Resuming))
                    {
                        let mut active = manager.active.lock().unwrap();
                        if active.len() >= settings.concurrency.clamp(1, 3) {
                            break;
                        }
                        if !active.insert(job.id.clone()) {
                            continue;
                        }
                        let worker = manager.clone();
                        tokio::spawn(async move {
                            let id = job.id.clone();
                            if let Err(error) = worker.run(job).await {
                                if let Err(persistence_error) = worker.update(&id, |j| {
                                    if !matches!(
                                        j.status,
                                        State::Cancelled | State::Paused | State::Pausing
                                    ) {
                                        j.status.transition(State::Failed)?;
                                        j.error = Some(error.to_string());
                                    } else if j.status == State::Pausing {
                                        j.status.transition(State::Paused)?;
                                    }
                                    for file in &mut j.files {
                                        if file.status != State::Completed {
                                            file.status = j.status;
                                        }
                                    }
                                    j.speed = 0;
                                    Ok(())
                                }) {
                                    tracing::error!(%persistence_error,"Cannot persist transfer failure");
                                }
                            }
                            worker.active.lock().unwrap().remove(&id);
                        });
                    }
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        });
    }
    fn update(
        &self,
        id: &str,
        f: impl FnOnce(&mut DownloadJob) -> Result<()>,
    ) -> Result<DownloadJob> {
        let _guard = self.gate.lock().unwrap();
        let mut job = self
            .db
            .jobs()?
            .into_iter()
            .find(|j| j.id == id)
            .context("Download not found")?;
        f(&mut job)?;
        self.db.save_job(&job)?;
        Ok(job)
    }
    pub fn action(&self, id: &str, action: &str) -> Result<()> {
        if matches!(action, "resume" | "retry") && self.active.lock().unwrap().contains(id) {
            bail!("The previous transfer is still stopping; retry in a moment");
        }
        if action == "up" {
            let _guard = self.gate.lock().unwrap();
            return self.db.move_job_up(id);
        }
        self.update(id, |job| {
            let next = match action {
                "pause" if job.status == State::Queued => State::Paused,
                "pause" => State::Pausing,
                "resume" => State::Resuming,
                "cancel" => State::Cancelled,
                "retry" => State::Queued,
                _ => bail!("Unknown download action"),
            };
            job.status.transition(next)?;
            for file in &mut job.files {
                if file.status != State::Completed {
                    file.status = match next {
                        State::Pausing if file.status == State::Queued => State::Paused,
                        _ => next,
                    };
                }
            }
            job.speed = 0;
            job.error = None;
            Ok(())
        })?;
        Ok(())
    }
    pub async fn create_job(
        self: &Arc<Self>,
        repo: Repository,
        selected: Vec<String>,
        destination: PathBuf,
    ) -> Result<DownloadJob> {
        modelshelf_core::parse_repository(&repo.id)?;
        if repo.sha.len() != 40 || !repo.sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("Downloads require a pinned commit");
        }
        storage::validate_tree(&destination)?;
        if !destination.is_dir() {
            bail!("Destination must be an existing directory");
        }
        let mut seen = HashSet::new();
        let mut files = vec![];
        for path in selected {
            storage::validate_relative(&path)?;
            if path.to_lowercase().starts_with(".modelshelf-") {
                bail!("Reserved internal filename");
            }
            if !seen.insert(path.to_lowercase()) {
                bail!("Duplicate or case-colliding file selection");
            }
            let file = repo
                .files
                .iter()
                .find(|f| f.path == path)
                .context("Selected file absent from repository")?;
            files.push(DownloadFile {
                path,
                size: file.size,
                downloaded: 0,
                sha256: file.sha256.clone(),
                etag: None,
                status: State::Queued,
                verification: "Unverified".into(),
                reused_bytes: 0,
            });
        }
        if files.is_empty() {
            bail!("Select at least one file");
        }
        let total = files
            .iter()
            .try_fold(0u64, |sum, f| sum.checked_add(f.size))
            .context("Selection size overflow")?;
        if storage::disk_info(&destination)?.1 < total.saturating_add(16 * 1024 * 1024) {
            bail!("Insufficient disk space for selected files and metadata overhead");
        }
        let id = uuid::Uuid::new_v4().to_string();
        let directory = destination.join(format!("{}-{}", repo.id.replace('/', "--"), id));
        let mut job = DownloadJob {
            id,
            repository: repo.id,
            revision: repo.sha,
            destination: directory.to_string_lossy().into_owned(),
            status: State::Queued,
            files,
            error: None,
            created_at: modelshelf_core::now(),
            speed: 0,
        };
        let _guard = self.gate.lock().unwrap();
        self.db.save_job(&job)?;
        if let Err(error) = std::fs::create_dir(&directory) {
            job.status = State::Failed;
            job.error = Some(format!("Could not create destination: {error}"));
            self.db.save_job(&job)?;
            return Err(error.into());
        }
        Ok(job)
    }
    fn check(&self, id: &str) -> Result<()> {
        let job = self
            .db
            .jobs()?
            .into_iter()
            .find(|j| j.id == id)
            .context("Job disappeared")?;
        if matches!(
            job.status,
            State::Pausing | State::Paused | State::Cancelled
        ) {
            bail!("Transfer stopped");
        }
        Ok(())
    }
    async fn run(&self, mut job: DownloadJob) -> Result<()> {
        job = self.update(&job.id, |j| {
            j.status.transition(State::Preparing)?;
            j.error = None;
            Ok(())
        })?;
        storage::validate_tree(Path::new(&job.destination))?;
        if !Path::new(&job.destination).exists() {
            // Recover a crash after the queued record was committed but before directory creation.
            std::fs::create_dir(&job.destination)?;
        }
        self.update(&job.id, |j| j.status.transition(State::Downloading))?;
        let client = modelshelf_hub::client()?;
        for index in 0..job.files.len() {
            if job.files[index].status == State::Completed {
                continue;
            }
            let retries = self.db.settings()?.retries;
            for attempt in 0..=retries {
                self.check(&job.id)?;
                let token = self.token.lock().unwrap().clone();
                let url = modelshelf_hub::file_url(
                    &job.repository,
                    &job.revision,
                    &job.files[index].path,
                )?;
                #[cfg(test)]
                let url = self.test_url.lock().unwrap().clone().unwrap_or(url);
                let root = PathBuf::from(&job.destination);
                let file = job.files[index].clone();
                let mut last_bytes: Option<u64> = None;
                let mut last_time = Instant::now();
                let result = transfer(
                    &client,
                    url,
                    token.as_deref(),
                    &root,
                    index,
                    &file,
                    |progress| {
                        self.check(&job.id)?;
                        self.update(&job.id, |j| {
                            let elapsed = last_time.elapsed().as_secs_f64();
                            j.speed = last_bytes.filter(|_| elapsed > 0.05).map_or(0, |previous| {
                                (progress.downloaded.saturating_sub(previous) as f64 / elapsed)
                                    as u64
                            });
                            last_bytes = Some(progress.downloaded);
                            last_time = Instant::now();
                            j.files[index] = progress.clone();
                            Ok(())
                        })?;
                        Ok(())
                    },
                )
                .await;
                match result {
                    Ok(done) => {
                        job.files[index] = done;
                        break;
                    }
                    Err(error) => {
                        let message = error.to_string();
                        if attempt == retries
                            || message.contains("401")
                            || message.contains("403")
                            || message.contains("404")
                            || message.contains("SHA-256")
                            || message.contains("stopped")
                        {
                            return Err(error);
                        }
                        self.update(&job.id, |j| {
                            j.error = Some(format!("{message}. Retry {}/{}", attempt + 1, retries));
                            Ok(())
                        })?;
                        tokio::time::sleep(Duration::from_secs(1 << attempt.min(5))).await;
                    }
                }
            }
        }
        job = self.update(&job.id, |j| {
            j.status.transition(State::Verifying)?;
            Ok(())
        })?;
        let mut files = vec![];
        for file in &job.files {
            let path = Path::new(&job.destination).join(&file.path);
            storage::validate_tree(&path)?;
            let actual = format!("{:x}", hash_prefix(&path).await?.finalize());
            if tokio::fs::metadata(&path).await?.len() != file.size
                || file
                    .sha256
                    .as_ref()
                    .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual))
            {
                bail!("Completed file changed or is missing: integrity check failed");
            }
            files.push(LocalFile {
                path: path.to_string_lossy().into_owned(),
                relative_path: file.path.clone(),
                size: file.size,
                expected_sha256: file.sha256.clone(),
                actual_sha256: Some(actual),
                verification: file.verification.clone(),
            });
        }
        let model = Model {
            id: job.id.clone(),
            display_name: job.repository.clone(),
            path: job.destination.clone(),
            ownership: "managed".into(),
            repository: Some(job.repository.clone()),
            revision: Some(job.revision.clone()),
            created_at: modelshelf_core::now(),
            favorite: false,
            notes: String::new(),
            tags: vec![],
            files,
        };
        let _guard = self.gate.lock().unwrap();
        let current = self
            .db
            .jobs()?
            .into_iter()
            .find(|j| j.id == job.id)
            .context("Missing job")?;
        if current.status != State::Verifying {
            bail!("Transfer stopped");
        }
        job.status.transition(State::Completed)?;
        job.error = None;
        job.speed = 0;
        self.db.complete_job(&job, &model)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Checkpoint {
    offset: u64,
    hash: String,
    etag: Option<String>,
}

async fn hash_prefix(path: &Path) -> Result<Sha256> {
    let mut input = tokio::fs::File::open(path).await?;
    let mut hash = Sha256::new();
    let mut bytes = vec![0; 128 * 1024];
    loop {
        let read = input.read(&mut bytes).await?;
        if read == 0 {
            break;
        }
        hash.update(&bytes[..read]);
    }
    Ok(hash)
}

async fn transfer(
    client: &reqwest::Client,
    url: reqwest::Url,
    token: Option<&str>,
    root: &Path,
    index: usize,
    file: &DownloadFile,
    mut report: impl FnMut(&DownloadFile) -> Result<()>,
) -> Result<DownloadFile> {
    storage::validate_tree(root)?;
    storage::validate_relative(&file.path)?;
    let final_path = root.join(&file.path);
    let temporary = root.join(format!(".modelshelf-{index}.partial"));
    let checkpoint = root.join(format!(".modelshelf-{index}.json"));
    for path in [&temporary, &checkpoint, &final_path] {
        storage::validate_tree(path)?;
    }
    if final_path.exists() {
        // A crash may occur between no-replace publication and the database commit.
        // Only a durable checkpoint from this job may authorize reconciliation.
        let saved: Checkpoint = serde_json::from_slice(
            &std::fs::read(&checkpoint)
                .context("Destination exists without publication checkpoint; refusing overwrite")?,
        )?;
        let actual = format!("{:x}", hash_prefix(&final_path).await?.finalize());
        if saved.offset != file.size
            || std::fs::metadata(&final_path)?.len() != file.size
            || saved.hash != actual
            || file
                .sha256
                .as_ref()
                .is_some_and(|s| !s.eq_ignore_ascii_case(&actual))
        {
            bail!("Existing destination does not match verified publication checkpoint");
        }
        let mut done = file.clone();
        done.downloaded = file.size;
        done.status = State::Completed;
        done.verification = if file.sha256.is_some() {
            "Verified"
        } else {
            "Unverified"
        }
        .into();
        report(&done)?;
        if temporary.exists() {
            std::fs::remove_file(&temporary)?;
        }
        std::fs::remove_file(&checkpoint)?;
        return Ok(done);
    }
    let mut progress = file.clone();
    progress.status = State::Downloading;
    progress.reused_bytes = 0;
    let mut offset = 0;
    let mut hash = Sha256::new();
    let mut etag = None;
    if temporary.exists() && checkpoint.exists() {
        if let Ok(saved) =
            serde_json::from_slice::<Checkpoint>(&tokio::fs::read(&checkpoint).await?)
        {
            let length = tokio::fs::metadata(&temporary).await?.len();
            if saved.offset <= length
                && saved.offset <= file.size
                && saved
                    .etag
                    .as_ref()
                    .is_some_and(|s| s.starts_with('"') && s.ends_with('"'))
            {
                tokio::fs::OpenOptions::new()
                    .write(true)
                    .open(&temporary)
                    .await?
                    .set_len(saved.offset)
                    .await?;
                let prefix = hash_prefix(&temporary).await?;
                if format!("{:x}", prefix.clone().finalize()) == saved.hash {
                    offset = saved.offset;
                    hash = prefix;
                    etag = saved.etag;
                }
            }
        }
    }
    let range = if offset > 0 && offset < file.size {
        Some((offset, etag.as_deref().unwrap()))
    } else {
        None
    };
    #[cfg(not(test))]
    let mut response = modelshelf_hub::get(client, url, token, range).await?;
    #[cfg(test)]
    let mut response = if url.scheme() == "http" {
        let mut request = client.get(url.clone());
        if let Some((n, e)) = range {
            request = request
                .header("Range", format!("bytes={n}-"))
                .header("If-Range", e);
        }
        request.send().await?
    } else {
        modelshelf_hub::get(client, url, token, range).await?
    };
    modelshelf_hub::response_error(&response)?;
    let response_etag = response
        .headers()
        .get("etag")
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    let expected_range = format!(
        "bytes {}-{}/{}",
        offset,
        file.size.saturating_sub(1),
        file.size
    );
    let resumed = range.is_some()
        && response.status().as_u16() == 206
        && response
            .headers()
            .get("content-range")
            .and_then(|h| h.to_str().ok())
            == Some(expected_range.as_str())
        && response_etag == etag;
    if response.status().as_u16() == 206 && !resumed {
        bail!("Server returned an invalid range or changed remote identity; partial bytes were not appended");
    }
    if !resumed {
        offset = 0;
        hash = Sha256::new();
    } else {
        progress.reused_bytes = offset;
    }
    progress.etag = response_etag.clone();
    progress.downloaded = offset;
    let reclaimable = if resumed {
        0
    } else {
        tokio::fs::metadata(&temporary)
            .await
            .map(|m| m.len())
            .unwrap_or(0)
    };
    if storage::disk_info(root)?.1.saturating_add(reclaimable)
        < file.size.saturating_sub(offset).saturating_add(1024 * 1024)
    {
        bail!("Insufficient disk space; free space or choose another drive before retrying");
    }
    report(&progress)?;
    let mut output = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!resumed)
        .append(resumed)
        .open(&temporary)
        .await?;
    let mut last = Instant::now();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("Network transfer interrupted or timed out"))?
    {
        if offset.saturating_add(chunk.len() as u64) > file.size {
            bail!("Server sent more bytes than pinned file size");
        }
        output.write_all(&chunk).await?;
        hash.update(&chunk);
        offset += chunk.len() as u64;
        progress.downloaded = offset;
        if last.elapsed() >= Duration::from_millis(500) {
            output.sync_data().await?;
            tokio::fs::write(
                &checkpoint,
                serde_json::to_vec(&Checkpoint {
                    offset,
                    hash: format!("{:x}", hash.clone().finalize()),
                    etag: response_etag.clone(),
                })?,
            )
            .await?;
            report(&progress)?;
            last = Instant::now();
        }
    }
    output.sync_all().await?;
    drop(output);
    if offset != file.size {
        bail!(
            "Incomplete response: expected {} bytes, received {offset}",
            file.size
        );
    }
    let actual = format!("{:x}", hash.finalize());
    if file
        .sha256
        .as_ref()
        .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual))
    {
        progress.verification = "Corrupted".into();
        report(&progress)?;
        bail!("SHA-256 mismatch; file remains quarantined as a partial download");
    }
    progress.verification = if file.sha256.is_some() {
        "Verified"
    } else {
        "Unverified"
    }
    .into();
    let mut manifest = tokio::fs::File::create(&checkpoint).await?;
    manifest
        .write_all(&serde_json::to_vec(&Checkpoint {
            offset,
            hash: actual,
            etag: response_etag,
        })?)
        .await?;
    manifest.sync_all().await?;
    drop(manifest);
    storage::validate_tree(root)?;
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
        storage::validate_tree(parent)?;
    }
    // Hard-link publication is atomic, same-volume, and fails rather than replacing an existing file.
    std::fs::hard_link(&temporary, &final_path)?;
    progress.status = State::Completed;
    report(&progress)?;
    std::fs::remove_file(&temporary)?;
    if checkpoint.exists() {
        std::fs::remove_file(&checkpoint)?;
    }
    Ok(progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn crash_after_publication_reconciles_only_with_matching_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"verified published contents";
        let hash = format!("{:x}", Sha256::digest(bytes));
        let partial = dir.path().join(".modelshelf-0.partial");
        let final_path = dir.path().join("model.bin");
        std::fs::write(&partial, bytes).unwrap();
        std::fs::hard_link(&partial, &final_path).unwrap();
        let file = DownloadFile {
            path: "model.bin".into(),
            size: bytes.len() as u64,
            downloaded: 0,
            sha256: Some(hash.clone()),
            etag: None,
            status: State::Downloading,
            verification: "Unverified".into(),
            reused_bytes: 0,
        };
        let client = modelshelf_hub::client().unwrap();
        let url = reqwest::Url::parse("http://127.0.0.1:1/unreachable").unwrap();
        assert!(
            transfer(&client, url.clone(), None, dir.path(), 0, &file, |_| Ok(()))
                .await
                .is_err()
        );
        std::fs::write(
            dir.path().join(".modelshelf-0.json"),
            serde_json::to_vec(&Checkpoint {
                offset: bytes.len() as u64,
                hash,
                etag: Some("\"stable\"".into()),
            })
            .unwrap(),
        )
        .unwrap();
        let done = transfer(&client, url, None, dir.path(), 0, &file, |_| Ok(()))
            .await
            .unwrap();
        assert_eq!(done.status, State::Completed);
        assert_eq!(done.verification, "Verified");
        assert!(!partial.exists());
        assert_eq!(std::fs::read(final_path).unwrap(), bytes);
    }

    #[tokio::test]
    async fn changed_etag_range_is_rejected_without_appending() {
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join(".modelshelf-0.partial");
        std::fs::write(&partial, b"abc").unwrap();
        std::fs::write(
            dir.path().join(".modelshelf-0.json"),
            serde_json::to_vec(&Checkpoint {
                offset: 3,
                hash: format!("{:x}", Sha256::digest(b"abc")),
                etag: Some("\"old\"".into()),
            })
            .unwrap(),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = reqwest::Url::parse(&format!("http://{}/file", listener.local_addr().unwrap()))
            .unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 4096];
            let mut request = Vec::new();
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let read = socket.read(&mut buffer).await.unwrap();
                assert!(read > 0, "Request ended before headers");
                request.extend_from_slice(&buffer[..read]);
            }
            assert!(String::from_utf8_lossy(&request)
                .to_lowercase()
                .contains("if-range: \"old\""));
            socket.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nETag: \"changed\"\r\nConnection: close\r\n\r\ndef").await.unwrap();
        });
        let file = DownloadFile {
            path: "model.bin".into(),
            size: 6,
            downloaded: 3,
            sha256: None,
            etag: Some("\"old\"".into()),
            status: State::Paused,
            verification: "Unverified".into(),
            reused_bytes: 0,
        };
        assert!(transfer(
            &modelshelf_hub::client().unwrap(),
            url,
            None,
            dir.path(),
            0,
            &file,
            |_| Ok(())
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("invalid range"));
        assert_eq!(std::fs::read(partial).unwrap(), b"abc");
        assert!(!dir.path().join("model.bin").exists());
        server.await.unwrap();
    }

    async fn server(
        data: Vec<u8>,
        interrupt: bool,
        ignore_range: bool,
    ) -> (reqwest::Url, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = reqwest::Url::parse(&format!("http://{}/file", listener.local_addr().unwrap()))
            .unwrap();
        let task = tokio::spawn(async move {
            let mut requests = vec![];
            for attempt in 0..if interrupt { 2 } else { 1 } {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = vec![];
                loop {
                    let mut bytes = [0; 1024];
                    let n = socket.read(&mut bytes).await.unwrap();
                    request.extend_from_slice(&bytes[..n]);
                    if request.windows(4).any(|x| x == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).unwrap().to_lowercase();
                let offset = request
                    .lines()
                    .find_map(|line| line.strip_prefix("range: bytes="))
                    .and_then(|s| s.trim_end_matches('-').parse::<usize>().ok())
                    .unwrap_or(0);
                requests.push(request);
                let offset = if ignore_range { 0 } else { offset };
                let status = if offset > 0 {
                    "206 Partial Content"
                } else {
                    "200 OK"
                };
                let range = if offset > 0 {
                    format!(
                        "Content-Range: bytes {offset}-{}/{}\r\n",
                        data.len() - 1,
                        data.len()
                    )
                } else {
                    String::new()
                };
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nETag: \"stable\"\r\n{range}Connection: close\r\n\r\n",data.len()-offset).as_bytes()).await.unwrap();
                if interrupt && attempt == 0 {
                    socket.write_all(&data[..65536]).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(650)).await;
                    socket.write_all(&data[65536..131072]).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(200)).await;
                } else {
                    socket.write_all(&data[offset..]).await.unwrap();
                }
            }
            requests
        });
        (url, task)
    }

    async fn recovery(ignore_range: bool) {
        let dir = tempfile::tempdir().unwrap();
        let data: Vec<u8> = (0..2_000_000).map(|i| (i % 251) as u8).collect();
        let expected = format!("{:x}", Sha256::digest(&data));
        let (url, server) = server(data.clone(), true, ignore_range).await;
        let database_path = dir.path().join("state.sqlite");
        let destination = dir.path().join("owned");
        std::fs::create_dir(&destination).unwrap();
        let file = DownloadFile {
            path: "nested/model.gguf".into(),
            size: data.len() as u64,
            downloaded: 0,
            sha256: Some(expected.clone()),
            etag: None,
            status: State::Downloading,
            verification: "Unverified".into(),
            reused_bytes: 0,
        };
        let db = Database::open(&database_path).unwrap();
        let mut job = DownloadJob {
            id: "job".into(),
            repository: "owner/model".into(),
            revision: "a".repeat(40),
            destination: destination.to_string_lossy().into(),
            status: State::Downloading,
            files: vec![file.clone()],
            error: None,
            created_at: "1".into(),
            speed: 0,
        };
        db.save_job(&job).unwrap();
        let client = modelshelf_hub::client().unwrap();
        assert!(transfer(
            &client,
            url.clone(),
            None,
            &destination,
            0,
            &file,
            |progress| {
                job.files[0] = progress.clone();
                db.save_job(&job)
            }
        )
        .await
        .is_err());
        drop(db);
        let db = Database::open(&database_path).unwrap();
        let mut recovered = db.jobs().unwrap().remove(0);
        assert!(recovered.files[0].downloaded > 0);
        let done = transfer(
            &client,
            url,
            None,
            &destination,
            0,
            &recovered.files[0].clone(),
            |progress| {
                recovered.files[0] = progress.clone();
                db.save_job(&recovered)
            },
        )
        .await
        .unwrap();
        assert_eq!(done.verification, "Verified");
        assert_eq!(done.reused_bytes > 0, !ignore_range);
        assert_eq!(
            std::fs::read(destination.join("nested/model.gguf")).unwrap(),
            data
        );
        assert_eq!(
            storage::sha256(&destination.join("nested/model.gguf")).unwrap(),
            expected
        );
        assert_eq!(db.jobs().unwrap()[0].files[0].status, State::Completed);
        assert!(!destination.join(".modelshelf-0.partial").exists());
        let requests = server.await.unwrap();
        assert!(requests[1].contains("range: bytes="));
        assert!(requests[1].contains("if-range: \"stable\""));
    }
    #[tokio::test]
    async fn interrupted_restart_reuses_verified_prefix() {
        recovery(false).await;
    }
    #[tokio::test]
    async fn ignored_range_restarts_without_claiming_reuse() {
        recovery(true).await;
    }
    #[tokio::test]
    async fn hash_mismatch_never_publishes() {
        let dir = tempfile::tempdir().unwrap();
        let (url, server) = server(vec![1; 1024], false, false).await;
        let file = DownloadFile {
            path: "model.bin".into(),
            size: 1024,
            downloaded: 0,
            sha256: Some("0".repeat(64)),
            etag: None,
            status: State::Queued,
            verification: "Unverified".into(),
            reused_bytes: 0,
        };
        assert!(transfer(
            &modelshelf_hub::client().unwrap(),
            url,
            None,
            dir.path(),
            0,
            &file,
            |_| Ok(())
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("SHA-256"));
        assert!(!dir.path().join("model.bin").exists());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn manager_restart_completes_job_and_library_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let database_path = dir.path().join("db.sqlite");
        let data = vec![17u8; 2_000_000];
        let hash = format!("{:x}", Sha256::digest(&data));
        let (url, server) = server(data, true, false).await;
        let db = Arc::new(Database::open(&database_path).unwrap());
        let mut settings = db.settings().unwrap();
        settings.retries = 0;
        db.save_settings(&settings).unwrap();
        let manager = DownloadManager::new(db.clone());
        *manager.test_url.lock().unwrap() = Some(url.clone());
        let repository = Repository {
            id: "owner/model".into(),
            sha: "a".repeat(40),
            task: None,
            downloads: 0,
            likes: 0,
            tags: vec![],
            license: None,
            updated_at: None,
            gated: false,
            files: vec![modelshelf_core::RemoteFile {
                path: "model.bin".into(),
                size: 2_000_000,
                sha256: Some(hash.clone()),
            }],
            readme: String::new(),
            revisions: vec![],
        };
        let job = manager
            .create_job(repository, vec!["model.bin".into()], dir.path().to_owned())
            .await
            .unwrap();
        assert!(manager.run(job.clone()).await.is_err());
        assert_eq!(db.jobs().unwrap()[0].status, State::Downloading);
        drop(manager);
        drop(db);
        let db = Arc::new(Database::open(&database_path).unwrap());
        let manager = DownloadManager::new(db.clone());
        *manager.test_url.lock().unwrap() = Some(url);
        manager.start();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if db.jobs().unwrap()[0].status == State::Paused {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        manager.action(&job.id, "resume").unwrap();
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let current = db.jobs().unwrap().remove(0);
                assert_ne!(current.status, State::Failed, "{:?}", current.error);
                if current.status == State::Completed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        let completed = db.jobs().unwrap().remove(0);
        assert!(completed.files[0].reused_bytes > 0);
        let model = db.models().unwrap().remove(0);
        assert_eq!(model.files[0].actual_sha256.as_deref(), Some(hash.as_str()));
        assert_eq!(model.ownership, "managed");
        assert!(!Path::new(&job.destination)
            .join(".modelshelf-0.partial")
            .exists());
        server.await.unwrap();
    }
}
