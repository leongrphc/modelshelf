//! Opt-in real-provider smoke test. Downloads only gpt2's tiny config.json.
use modelshelf_core::{Database, DownloadState};
use modelshelf_download::DownloadManager;
use modelshelf_hub::Hub;
use std::{sync::Arc, time::Duration};

#[tokio::test]
#[ignore = "requires internet; downloads a small public configuration file"]
async fn public_search_to_download_survives_database_reopen() {
    let hub = Hub::new(None).unwrap();
    let found = hub
        .search("openai-community/gpt2", "downloads", "")
        .await
        .unwrap();
    assert!(found.iter().any(|r| r.id == "openai-community/gpt2"));
    let repository = hub.details("openai-community/gpt2", "main").await.unwrap();
    assert!(repository
        .files
        .iter()
        .any(|f| f.path == "config.json" && f.size > 0));
    let dir = tempfile::tempdir().unwrap();
    let database_path = dir.path().join("smoke.db");
    let db = Arc::new(Database::open(&database_path).unwrap());
    let manager = DownloadManager::new(db.clone());
    let job = manager
        .create_job(
            repository,
            vec!["config.json".into()],
            dir.path().to_owned(),
        )
        .await
        .unwrap();
    manager.start();
    tokio::time::timeout(Duration::from_secs(120), async {
        loop {
            let current = db.jobs().unwrap().remove(0);
            assert_ne!(current.status, DownloadState::Failed, "{:?}", current.error);
            if current.status == DownloadState::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let reopened = Database::open(&database_path).unwrap();
    let models = reopened.models().unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, job.id);
    let contents = std::fs::read(&models[0].files[0].path).unwrap();
    let config: serde_json::Value = serde_json::from_slice(&contents).unwrap();
    assert_eq!(config["model_type"], "gpt2");
    assert_eq!(models[0].files[0].verification, "Unverified");
}
