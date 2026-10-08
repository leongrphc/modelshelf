use modelshelf_core::{storage::*, Database, DownloadJob, DownloadState, Settings};
use std::{fs, path::Path};

#[test]
fn external_index_removal_and_duplicate_import_preserve_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model.gguf");
    fs::write(&path, b"original model").unwrap();
    let db = Database::open(&dir.path().join("library.db")).unwrap();
    let model = import_path(&db, &path).unwrap();
    let duplicate = import_path(&db, &path).unwrap();
    assert_eq!(model.id, duplicate.id);
    assert_eq!(db.models().unwrap().len(), 1);
    assert!(delete_managed(&db, &model.id, true).is_err());
    let verified = verify_model(&db, &model.id).unwrap();
    assert_eq!(verified.files[0].verification, "Unverified");
    assert!(verified.files[0].actual_sha256.is_some());
    db.remove_model(&model.id).unwrap();
    assert!(db.models().unwrap().is_empty());
    assert_eq!(fs::read(&path).unwrap(), b"original model");
}
#[test]
fn database_reopen_and_settings_validation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.db");
    {
        let db = Database::open(&path).unwrap();
        let settings = Settings {
            language: "tr".into(),
            ..Default::default()
        };
        db.save_settings(&settings).unwrap();
        let bad = Settings {
            concurrency: 9,
            ..settings
        };
        assert!(db.save_settings(&bad).is_err());
    }
    assert_eq!(
        Database::open(&path).unwrap().settings().unwrap().language,
        "tr"
    );
}
#[test]
fn malicious_paths_and_hash() {
    for path in [
        "../outside",
        "a/../../b",
        "C:/x",
        "/absolute",
        "a\\..\\b",
        "CON.gguf",
        "folder/NUL",
        "trailing.",
        "a//b",
        "a:b",
        "file?",
    ] {
        assert!(validate_relative(path).is_err(), "{path}");
    }
    validate_relative("nested/model-Q4_K_M.gguf").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hash");
    fs::write(&path, b"abc").unwrap();
    assert_eq!(
        sha256(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
#[test]
fn managed_deletion_requires_confirmation_and_preserves_unknown_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("owned");
    fs::create_dir(&root).unwrap();
    let path = root.join("model.gguf");
    fs::write(&path, b"abc").unwrap();
    let db = Database::open(&dir.path().join("library.db")).unwrap();
    let mut model = import_path(&db, &root).unwrap();
    model.ownership = "managed".into();
    model.repository = Some("owner/model".into());
    db.save_model(&model).unwrap();
    let job = DownloadJob {
        id: "job".into(),
        repository: "owner/model".into(),
        revision: "sha".into(),
        destination: model.path.clone(),
        status: DownloadState::Completed,
        files: vec![],
        error: None,
        created_at: "1".into(),
        speed: 0,
    };
    db.save_job(&job).unwrap();
    fs::write(root.join("keep.txt"), b"unrelated").unwrap();
    assert!(delete_managed(&db, &model.id, false).is_err());
    delete_managed(&db, &model.id, true).unwrap();
    assert!(!path.exists());
    assert_eq!(fs::read(root.join("keep.txt")).unwrap(), b"unrelated");
}
#[test]
fn integrity_detects_corruption_and_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model");
    fs::write(&path, b"abc").unwrap();
    let db = Database::open(&dir.path().join("db")).unwrap();
    let mut model = import_path(&db, &path).unwrap();
    model.files[0].expected_sha256 = Some(sha256(&path).unwrap());
    db.save_model(&model).unwrap();
    assert_eq!(
        verify_model(&db, &model.id).unwrap().files[0].verification,
        "Verified"
    );
    fs::write(&path, b"abd").unwrap();
    assert_eq!(
        verify_model(&db, &model.id).unwrap().files[0].verification,
        "Corrupted"
    );
    fs::remove_file(path).unwrap();
    assert_eq!(
        verify_model(&db, &model.id).unwrap().files[0].verification,
        "Missing"
    );
}
#[test]
fn linked_directory_cannot_escape_deletion_scope() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    let link = dir.path().join("link");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("preserve"), b"safe").unwrap();
    create_directory_link(&outside, &link);
    assert!(validate_tree(&link.join("preserve")).is_err());
    assert_eq!(fs::read(outside.join("preserve")).unwrap(), b"safe");
}
#[cfg(unix)]
fn create_directory_link(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}
#[cfg(windows)]
fn create_directory_link(target: &Path, link: &Path) {
    // Junction creation does not require developer mode or elevated privileges.
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
