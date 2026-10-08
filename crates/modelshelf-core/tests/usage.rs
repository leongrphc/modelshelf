use modelshelf_core::{storage, Database};

#[test]
fn hardlinks_and_overlapping_imports_count_once() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("models");
    std::fs::create_dir(&folder).unwrap();
    let a = folder.join("a.gguf");
    let b = folder.join("b.gguf");
    std::fs::write(&a, b"model").unwrap();
    std::fs::hard_link(&a, &b).unwrap();
    let db = Database::open(&dir.path().join("library.db")).unwrap();
    storage::import_path(&db, &a).unwrap();
    storage::import_path(&db, &folder).unwrap();
    let usage = storage::usage(&db.models().unwrap());
    assert_eq!(usage["unique_logical_bytes"], 5);
    assert_eq!(usage["duplicates"].as_array().unwrap().len(), 2);
    assert_eq!(usage["by_format"][0]["format"], "gguf");
}

#[test]
fn rescan_is_read_only_and_recheck_marks_missing() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("models");
    std::fs::create_dir(&folder).unwrap();
    let a = folder.join("a.gguf");
    std::fs::write(&a, b"first").unwrap();
    let db = Database::open(&dir.path().join("library.db")).unwrap();
    let model = storage::import_path(&db, &folder).unwrap();
    std::fs::write(folder.join("config.json"), b"{}").unwrap();
    assert_eq!(
        storage::rescan_model(&db, &model.id).unwrap().files.len(),
        2
    );
    assert_eq!(std::fs::read(&a).unwrap(), b"first");
    std::fs::remove_file(&a).unwrap();
    assert!(storage::recheck_model(&db, &model.id)
        .unwrap()
        .files
        .iter()
        .any(|f| f.verification == "Missing"));
}
