use crate::{Database, LocalFile, Model};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

/// Logical bytes deduplicated by operating-system file identity, not allocated disk blocks.
pub fn usage(models: &[Model]) -> serde_json::Value {
    use std::collections::{BTreeMap, HashMap};
    let mut identities: HashMap<same_file::Handle, String> = HashMap::new();
    let mut hashes: HashMap<String, String> = HashMap::new();
    let mut formats: BTreeMap<String, u64> = BTreeMap::new();
    let (mut managed, mut external) = (0u64, 0u64);
    let mut duplicates = Vec::new();
    let mut unavailable = 0;
    // Prefer managed ownership when one physical file is indexed in both categories.
    let mut ordered: Vec<_> = models.iter().collect();
    ordered.sort_by_key(|m| m.ownership != "managed");
    for model in ordered {
        for file in &model.files {
            let Ok(handle) = same_file::Handle::from_path(&file.path) else {
                unavailable += 1;
                continue;
            };
            if let Some(previous) = identities.get(&handle) {
                duplicates.push(
                    serde_json::json!({"kind":"same_physical_file","paths":[previous,file.path]}),
                );
                continue;
            }
            identities.insert(handle, file.path.clone());
            let size = match fs::metadata(&file.path) {
                Ok(metadata) => metadata.len(),
                Err(_) => {
                    unavailable += 1;
                    continue;
                }
            };
            if model.ownership == "managed" {
                managed = managed.saturating_add(size);
            } else {
                external = external.saturating_add(size);
            }
            let extension = Path::new(&file.path)
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("other")
                .to_ascii_lowercase();
            *formats.entry(extension).or_default() += size;
            if let Some(hash) = &file.actual_sha256 {
                if let Some(previous) = hashes.insert(hash.to_ascii_lowercase(), file.path.clone())
                {
                    duplicates.push(serde_json::json!({"kind":"same_recorded_sha256","paths":[previous,file.path]}));
                }
            }
        }
    }
    serde_json::json!({"managed_bytes":managed,"external_bytes":external,"unique_logical_bytes":managed.saturating_add(external),"by_format":formats.into_iter().map(|(format,bytes)|serde_json::json!({"format":format,"bytes":bytes})).collect::<Vec<_>>(),"duplicates":duplicates,"unavailable_files":unavailable})
}

/// Recheck indexed files without reading their contents or changing source files.
pub fn recheck_model(db: &Database, id: &str) -> Result<Model> {
    let mut model = db
        .models()?
        .into_iter()
        .find(|m| m.id == id)
        .context("Model not found")?;
    for file in &mut model.files {
        validate_tree(Path::new(&file.path))?;
        match fs::metadata(&file.path) {
            Ok(metadata) if metadata.is_file() => {
                if metadata.len() != file.size {
                    file.verification = "Corrupted".into();
                    file.actual_sha256 = None;
                } else if file.verification == "Missing" {
                    file.verification = "Unverified".into();
                }
            }
            Ok(_) => {
                file.verification = "Missing".into();
                file.actual_sha256 = None;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                file.verification = "Missing".into();
                file.actual_sha256 = None;
            }
            Err(error) => return Err(error.into()),
        }
    }
    db.save_model(&model)?;
    Ok(model)
}

pub fn rescan_model(db: &Database, id: &str) -> Result<Model> {
    let mut model = db
        .models()?
        .into_iter()
        .find(|m| m.id == id)
        .context("Model not found")?;
    // Managed scope is fixed by the download manifest; never claim ownership of newly added files.
    if model.ownership == "managed" {
        return recheck_model(db, id);
    }
    let path = Path::new(&model.path);
    validate_tree(path)?;
    let root = if path.is_dir() {
        path
    } else {
        path.parent().context("File has no parent")?
    };
    let mut files = Vec::new();
    collect(path, root, &mut files)?;
    model.files = files;
    db.save_model(&model)?;
    Ok(model)
}

pub fn validate_relative(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path.starts_with('/')
        || path.chars().any(|c| c.is_control())
    {
        bail!("Unsafe repository path");
    }
    for part in path.split('/') {
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.contains(['<', '>', '"', '|', '?', '*'])
            || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && base.as_bytes()[3].is_ascii_digit())
        {
            bail!("Unsafe or Windows-reserved repository path");
        }
    }
    Ok(())
}
fn reject_link(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        bail!("Symbolic links are not permitted: {}", path.display());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            bail!(
                "Windows reparse points are not permitted: {}",
                path.display()
            );
        }
    }
    Ok(())
}
/// Validate every existing ancestor, including junctions. Nonexistent suffixes are allowed.
pub fn validate_tree(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!("An absolute path without parent traversal is required");
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => reject_link(ancestor)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
pub fn sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
pub fn disk_info(path: &Path) -> Result<(u64, u64)> {
    Ok((fs2::total_space(path)?, fs2::available_space(path)?))
}
fn collect(path: &Path, root: &Path, files: &mut Vec<LocalFile>) -> Result<()> {
    reject_link(path)?;
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            collect(&entry?.path(), root, files)?;
        }
    } else if metadata.is_file() {
        if files.len() >= 100_000 {
            bail!("Folder exceeds 100,000 files; select a smaller folder");
        }
        files.push(LocalFile {
            path: path.to_string_lossy().into(),
            relative_path: path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/"),
            size: metadata.len(),
            expected_sha256: None,
            actual_sha256: None,
            verification: "Unverified".into(),
        });
    }
    Ok(())
}
pub fn import_path(db: &Database, path: &Path) -> Result<Model> {
    validate_tree(path)?;
    let path = fs::canonicalize(path)?;
    if let Some(existing) = db
        .models()?
        .into_iter()
        .find(|m| Path::new(&m.path) == path)
    {
        return Ok(existing);
    }
    let root = if path.is_dir() {
        path.as_path()
    } else {
        path.parent().context("File has no parent")?
    };
    let mut files = Vec::new();
    collect(&path, root, &mut files)?;
    if files.is_empty() {
        bail!("Folder contains no regular files");
    }
    let model = Model {
        id: uuid::Uuid::new_v4().to_string(),
        display_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path: path.to_string_lossy().into(),
        ownership: "external".into(),
        repository: None,
        revision: None,
        created_at: crate::now(),
        favorite: false,
        notes: String::new(),
        tags: vec![],
        files,
    };
    db.save_model(&model)?;
    Ok(model)
}
pub fn verify_model(db: &Database, id: &str) -> Result<Model> {
    let mut model = db
        .models()?
        .into_iter()
        .find(|m| m.id == id)
        .context("Model not found")?;
    for file in &mut model.files {
        let path = Path::new(&file.path);
        validate_tree(path)?;
        if !path.try_exists()? {
            file.verification = "Missing".into();
            file.actual_sha256 = None;
            continue;
        }
        let actual = sha256(path)?;
        let correct_size = fs::metadata(path)?.len() == file.size;
        file.verification = match &file.expected_sha256 {
            Some(expected) if correct_size && actual.eq_ignore_ascii_case(expected) => "Verified",
            Some(_) => "Corrupted",
            None if !correct_size => "Corrupted",
            None => "Unverified",
        }
        .into();
        file.actual_sha256 = Some(actual);
    }
    db.save_model(&model)?;
    Ok(model)
}
/// Deletes only individually recorded files in the exact job-owned directory. Never recursively removes a directory.
pub fn delete_managed(db: &Database, id: &str, confirmed: bool) -> Result<()> {
    if !confirmed {
        bail!("Explicit confirmation is required");
    }
    let model = db
        .models()?
        .into_iter()
        .find(|m| m.id == id)
        .context("Model not found")?;
    if model.ownership != "managed" {
        bail!("External files cannot be deleted");
    }
    let root = Path::new(&model.path);
    validate_tree(root)?;
    if !db.jobs()?.iter().any(|job| {
        Path::new(&job.destination) == root
            && job.repository == model.repository.clone().unwrap_or_default()
    }) {
        bail!("No matching managed download ownership record");
    }
    let canonical_root = fs::canonicalize(root)?;
    for file in &model.files {
        validate_relative(&file.relative_path)?;
        let path = Path::new(&file.path);
        validate_tree(path)?;
        if path.try_exists()? {
            let canonical = fs::canonicalize(path)?;
            if !canonical.starts_with(&canonical_root)
                || canonical == canonical_root
                || canonical != fs::canonicalize(root.join(&file.relative_path))?
            {
                bail!("File escapes its owned directory");
            }
            if !fs::metadata(path)?.is_file() {
                bail!("Recorded model file is not a regular file");
            }
        }
    }
    for file in &model.files {
        let path = Path::new(&file.path);
        validate_tree(path)?;
        if path.try_exists()? {
            fs::remove_file(path)?;
        }
    }
    // Unknown files and nonempty directories are preserved.
    if fs::read_dir(root)?.next().is_none() {
        fs::remove_dir(root)?;
    }
    db.remove_model(id)?;
    Ok(())
}
