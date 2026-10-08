use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFile {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub sha: String,
    pub task: Option<String>,
    pub downloads: u64,
    pub likes: u64,
    pub tags: Vec<String>,
    pub license: Option<String>,
    pub updated_at: Option<String>,
    pub gated: bool,
    pub files: Vec<RemoteFile>,
    pub readme: String,
    pub revisions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub ownership: String,
    pub repository: Option<String>,
    pub revision: Option<String>,
    pub created_at: String,
    pub favorite: bool,
    pub notes: String,
    pub tags: Vec<String>,
    pub files: Vec<LocalFile>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFile {
    pub path: String,
    pub relative_path: String,
    pub size: u64,
    pub expected_sha256: Option<String>,
    pub actual_sha256: Option<String>,
    pub verification: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFile {
    pub path: String,
    pub size: u64,
    pub downloaded: u64,
    pub sha256: Option<String>,
    pub etag: Option<String>,
    pub status: DownloadState,
    pub verification: String,
    pub reused_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: String,
    pub repository: String,
    pub revision: String,
    pub destination: String,
    pub status: DownloadState,
    pub files: Vec<DownloadFile>,
    pub error: Option<String>,
    pub created_at: String,
    pub speed: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadState {
    Queued,
    Preparing,
    Downloading,
    Pausing,
    Paused,
    Resuming,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}
impl DownloadState {
    pub fn can_transition(self, next: Self) -> bool {
        use DownloadState::*;
        self == next
            || matches!(
                (self, next),
                (Queued, Preparing | Cancelled | Paused)
                    | (Preparing, Downloading | Failed | Cancelled | Pausing)
                    | (Downloading, Pausing | Verifying | Failed | Cancelled)
                    | (Pausing, Paused | Failed | Cancelled)
                    | (Paused, Resuming | Queued | Cancelled)
                    | (
                        Resuming,
                        Preparing | Downloading | Failed | Cancelled | Pausing
                    )
                    | (Verifying, Completed | Failed | Cancelled)
                    | (Failed, Queued | Resuming | Cancelled)
                    | (Cancelled, Queued)
            )
    }
    pub fn transition(&mut self, next: Self) -> Result<()> {
        if !self.can_transition(next) {
            bail!("Invalid download transition: {:?} -> {:?}", self, next);
        }
        *self = next;
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub concurrency: usize,
    pub retries: u32,
    pub notifications: bool,
    pub default_directory: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "en".into(),
            concurrency: 2,
            retries: 3,
            notifications: true,
            default_directory: String::new(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if !matches!(self.theme.as_str(), "dark" | "light" | "system")
            || !matches!(self.language.as_str(), "en" | "tr")
            || !(1..=3).contains(&self.concurrency)
            || self.retries > 5
        {
            bail!("Invalid settings");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLocation {
    pub path: String,
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transitions() {
        let mut state = DownloadState::Queued;
        assert!(state.transition(DownloadState::Completed).is_err());
        state.transition(DownloadState::Preparing).unwrap();
        state.transition(DownloadState::Downloading).unwrap();
        state.transition(DownloadState::Pausing).unwrap();
        state.transition(DownloadState::Paused).unwrap();
    }
    #[test]
    fn settings_validation() {
        let mut s = Settings::default();
        s.validate().unwrap();
        s.concurrency = 0;
        assert!(s.validate().is_err());
    }
}
