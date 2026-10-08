#![cfg(windows)]

use modelshelf_core::{storage::*, Database, DownloadJob, DownloadState, Model};
use std::{fs, os::windows::ffi::OsStrExt, path::Path};

fn record_managed(db: &Database, root: &Path) -> Model {
    let mut model = import_path(db, root).unwrap();
    model.ownership = "managed".into();
    model.repository = Some("acceptance/model".into());
    db.save_model(&model).unwrap();
    db.save_job(&DownloadJob {
        id: model.id.clone(),
        repository: "acceptance/model".into(),
        revision: "acceptance-revision".into(),
        destination: model.path.clone(),
        status: DownloadState::Completed,
        files: vec![],
        error: None,
        created_at: modelshelf_core::now(),
        speed: 0,
    })
    .unwrap();
    model
}

#[test]
fn turkish_and_space_paths_survive_database_reopen_and_index_removal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Türkçe model arşivi İstanbul ıĞş");
    fs::create_dir(&root).unwrap();
    let source = root.join("küçük model Q4_K_M.gguf");
    fs::write(&source, b"original model bytes").unwrap();
    let database = temp.path().join("kütüphane verisi.sqlite");
    let id = {
        let db = Database::open(&database).unwrap();
        let model = import_path(&db, &root).unwrap();
        assert_eq!(model.files[0].relative_path, "küçük model Q4_K_M.gguf");
        assert_eq!(import_path(&db, &root).unwrap().id, model.id);
        model.id
    };
    let db = Database::open(&database).unwrap();
    let model = verify_model(&db, &id).unwrap();
    assert_eq!(
        model.files[0].actual_sha256.as_deref(),
        Some(sha256(&source).unwrap().as_str())
    );
    assert_eq!(usage(&db.models().unwrap())["external_bytes"], 20);
    assert!(delete_managed(&db, &id, true).is_err());
    db.remove_model(&id).unwrap();
    assert!(db.models().unwrap().is_empty());
    assert_eq!(fs::read(&source).unwrap(), b"original model bytes");
}

#[test]
fn paths_over_260_utf16_units_support_import_verify_and_managed_delete() {
    let temp = tempfile::tempdir().unwrap();
    let mut root = temp.path().to_path_buf();
    for index in 0..6 {
        root.push(format!(
            "uzun model klasörü {index} abcdefghijklmnopqrstuvwxyz"
        ));
    }
    fs::create_dir_all(&root).unwrap();
    let source = root.join("ağırlıklar Q4_K_M.gguf");
    assert!(source.as_os_str().encode_wide().count() > 260);
    fs::write(&source, b"long path model").unwrap();
    let db = Database::open(&temp.path().join("library.sqlite")).unwrap();
    let model = record_managed(&db, &root);
    let verified = verify_model(&db, &model.id).unwrap();
    assert_eq!(
        verified.files[0].actual_sha256,
        Some(sha256(&source).unwrap())
    );
    assert_eq!(usage(&db.models().unwrap())["managed_bytes"], 15);
    let unrelated = root.join("kişisel notlar.txt");
    fs::write(&unrelated, b"keep me").unwrap();
    delete_managed(&db, &model.id, true).unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(unrelated).unwrap(), b"keep me");
    assert!(db.models().unwrap().is_empty());
}

#[test]
fn renamed_storage_marks_missing_then_recovers_without_losing_index() {
    // A directory rename exercises unavailable paths, not physical device removal.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("model storage");
    let renamed = temp.path().join("storage temporarily unavailable");
    fs::create_dir(&root).unwrap();
    let source = root.join("model.gguf");
    fs::write(&source, b"abc").unwrap();
    let db = Database::open(&temp.path().join("library.sqlite")).unwrap();
    let mut model = import_path(&db, &root).unwrap();
    model.files[0].expected_sha256 = Some(sha256(&source).unwrap());
    db.save_model(&model).unwrap();
    assert_eq!(
        verify_model(&db, &model.id).unwrap().files[0].verification,
        "Verified"
    );
    fs::rename(&root, &renamed).unwrap();
    let missing = recheck_model(&db, &model.id).unwrap();
    assert_eq!(missing.files[0].verification, "Missing");
    assert!(missing.files[0].actual_sha256.is_none());
    assert_eq!(usage(&db.models().unwrap())["unavailable_files"], 1);
    assert!(rescan_model(&db, &model.id).is_err());
    assert_eq!(db.models().unwrap()[0].files.len(), 1);
    fs::rename(&renamed, &root).unwrap();
    assert_eq!(
        recheck_model(&db, &model.id).unwrap().files[0].verification,
        "Unverified"
    );
    assert_eq!(
        verify_model(&db, &model.id).unwrap().files[0].verification,
        "Verified"
    );
    assert_eq!(fs::read(source).unwrap(), b"abc");
}

#[test]
fn locked_managed_file_reports_denial_keeps_index_and_can_retry() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    fs::create_dir(&root).unwrap();
    let source = root.join("model.gguf");
    fs::write(&source, b"abc").unwrap();
    let db = Database::open(&temp.path().join("library.sqlite")).unwrap();
    let model = record_managed(&db, &root);
    use std::os::windows::fs::OpenOptionsExt;
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&source)
        .unwrap();
    let result = delete_managed(&db, &model.id, true);
    drop(lock);
    let error = result.unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(32) // ERROR_SHARING_VIOLATION
    );
    assert_eq!(db.models().unwrap().len(), 1);
    assert_eq!(fs::read(&source).unwrap(), b"abc");
    delete_managed(&db, &model.id, true).unwrap();
    assert!(!source.exists());
    assert!(db.models().unwrap().is_empty());
}
