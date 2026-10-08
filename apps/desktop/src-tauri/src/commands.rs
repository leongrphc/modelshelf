use crate::AppState;
use modelshelf_core::storage;
use modelshelf_core::{Database, DownloadJob, Model, Repository, Settings, StorageLocation};
use modelshelf_hub::Hub;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::State;

type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn credential() -> keyring::Result<keyring::Entry> {
    keyring::Entry::new("io.modelshelf.desktop", "huggingface")
}
fn hub(state: &AppState) -> Result<Hub> {
    Hub::new(state.token.lock().map_err(error)?.clone()).map_err(error)
}
async fn blocking<T: Send + 'static>(
    operation: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(error)?
        .map_err(error)
}

#[tauri::command]
pub async fn snapshot(state: State<'_, AppState>) -> Result<Value> {
    let db = state.db.clone();
    let hardware = state.hardware.clone();
    blocking(move || {
        let locations = db.locations()?;
        let disks: Vec<Value> = locations.iter().map(|location| match storage::disk_info(Path::new(&location.path)) {
            Ok((total, free)) => json!({"path":location.path,"total":total,"free":free}),
            Err(e) => json!({"path":location.path,"total":0,"free":0,"error":e.to_string()})
        }).collect();
        let models = db.models()?;
        let usage = storage::usage(&models);
        let jobs = db.jobs()?;
        let temporary_bytes: u64 = jobs.iter().flat_map(|j| (0..j.files.len()).map(move |i| Path::new(&j.destination).join(format!(".modelshelf-{i}.partial")))).filter_map(|p| std::fs::symlink_metadata(p).ok()).filter(|m| m.is_file() && !m.file_type().is_symlink()).map(|m|m.len()).sum();
        Ok(json!({"models":models,"jobs":jobs,"settings":db.settings()?,"locations":locations,"hardware":hardware,"disks":disks,"usage":usage,"temporary_bytes":temporary_bytes}))
    }).await
}

#[tauri::command]
pub async fn search_models(
    state: State<'_, AppState>,
    query: String,
    sort: String,
    task: String,
) -> Result<Vec<Repository>> {
    hub(&state)?
        .search(&query, &sort, &task)
        .await
        .map_err(error)
}
#[tauri::command]
pub async fn repository_details(
    state: State<'_, AppState>,
    repo: String,
    revision: String,
) -> Result<Repository> {
    hub(&state)?.details(&repo, &revision).await.map_err(error)
}

async fn directory(db: Arc<Database>) -> Result<Option<String>> {
    let Some(folder) = rfd::AsyncFileDialog::new()
        .set_title("Choose model storage directory")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    let path = folder.path().to_owned();
    blocking(move || {
        storage::validate_tree(&path)?;
        let path = path.canonicalize()?.to_string_lossy().into_owned();
        db.add_location(&StorageLocation {
            path: path.clone(),
            kind: "managed".into(),
        })?;
        Ok(Some(path))
    })
    .await
}
#[tauri::command]
pub async fn choose_directory(state: State<'_, AppState>) -> Result<Option<String>> {
    directory(state.db.clone()).await
}
#[tauri::command]
pub async fn add_location(state: State<'_, AppState>) -> Result<Option<String>> {
    directory(state.db.clone()).await
}

#[tauri::command]
pub async fn import_model(state: State<'_, AppState>, directory: bool) -> Result<Option<Model>> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Index existing model — original files stay in place");
    let selected = if directory {
        dialog.pick_folder().await
    } else {
        dialog.pick_file().await
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    if directory && !confirm("Index this folder?", "ModelShelf will read file names and sizes in this folder. Large folders may take time. Original files will remain untouched.").await { return Ok(None) }
    let db = state.db.clone();
    let path = selected.path().to_owned();
    blocking(move || storage::import_path(&db, &path).map(Some)).await
}

#[tauri::command]
pub async fn create_download(
    state: State<'_, AppState>,
    repo: String,
    revision: String,
    selected: Vec<String>,
    destination: String,
) -> Result<DownloadJob> {
    if selected.is_empty() {
        return Err("Select at least one file".into());
    }
    let path = PathBuf::from(destination).canonicalize().map_err(error)?;
    let allowed = state.db.locations().map_err(error)?.iter().any(|l| {
        l.kind == "managed" && Path::new(&l.path).canonicalize().ok().as_ref() == Some(&path)
    });
    if !allowed {
        return Err("Choose a destination using the native folder picker first".into());
    }
    storage::validate_tree(&path).map_err(error)?;
    let repository = hub(&state)?
        .details(&repo, &revision)
        .await
        .map_err(error)?;
    state
        .downloads
        .create_job(repository, selected, path)
        .await
        .map_err(error)
}

#[tauri::command]
pub async fn download_action(state: State<'_, AppState>, id: String, action: String) -> Result<()> {
    state.downloads.action(&id, &action).map_err(error)
}
#[tauri::command]
pub async fn remove_model(state: State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.remove_model(&id)).await
}
async fn confirm(title: &str, message: &str) -> bool {
    rfd::AsyncMessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        .await
        == rfd::MessageDialogResult::Yes
}
#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, id: String) -> Result<()> {
    let model = state
        .db
        .models()
        .map_err(error)?
        .into_iter()
        .find(|m| m.id == id)
        .ok_or("Model not found")?;
    if model.ownership != "managed" {
        return Err("External files cannot be deleted by ModelShelf".into());
    }
    if !confirm("Permanently delete owned model files?", &format!("Delete {} at {}? This cannot be undone. Only files recorded as owned by ModelShelf may be deleted.", model.display_name, model.path)).await { return Ok(()) }
    let db = state.db.clone();
    blocking(move || storage::delete_managed(&db, &id, true)).await
}
#[tauri::command]
pub async fn verify_model(state: State<'_, AppState>, id: String) -> Result<Model> {
    let db = state.db.clone();
    blocking(move || storage::verify_model(&db, &id)).await
}
#[tauri::command]
pub async fn recheck_model(state: State<'_, AppState>, id: String) -> Result<Model> {
    let db = state.db.clone();
    blocking(move || storage::recheck_model(&db, &id)).await
}
#[tauri::command]
pub async fn rescan_model(state: State<'_, AppState>, id: String) -> Result<Model> {
    if !confirm("Rescan indexed folder?", "Read file names and sizes again? This may take time for large folders. Original files remain untouched.").await { return Err("Rescan cancelled".into()) }
    let db = state.db.clone();
    blocking(move || storage::rescan_model(&db, &id)).await
}
#[tauri::command]
pub async fn update_model(
    state: State<'_, AppState>,
    id: String,
    favorite: bool,
    notes: String,
    tags: Vec<String>,
) -> Result<()> {
    if notes.len() > 100_000 || tags.len() > 100 || tags.iter().any(|t| t.len() > 100) {
        return Err("Notes or tags exceed the allowed length".into());
    }
    let db = state.db.clone();
    blocking(move || {
        let mut model = db
            .models()?
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow::anyhow!("Model not found"))?;
        model.favorite = favorite;
        model.notes = notes;
        model.tags = tags;
        db.save_model(&model)
    })
    .await
}
#[tauri::command]
pub async fn open_model_folder(state: State<'_, AppState>, id: String) -> Result<()> {
    let model = state
        .db
        .models()
        .map_err(error)?
        .into_iter()
        .find(|m| m.id == id)
        .ok_or("Model not found")?;
    blocking(move || {
        let path = Path::new(&model.path);
        let folder = if path.is_dir() {
            path
        } else {
            path.parent()
                .ok_or_else(|| anyhow::anyhow!("Parent directory unavailable"))?
        };
        anyhow::ensure!(
            folder.is_dir(),
            "Directory unavailable; reconnect the drive and try again"
        );
        open::that(folder)?;
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<()> {
    let db = state.db.clone();
    blocking(move || {
        anyhow::ensure!(
            db.locations()?
                .iter()
                .any(|l| l.kind == "managed" && l.path == settings.default_directory),
            "Choose an authorized storage directory"
        );
        db.save_settings(&settings)
    })
    .await
}
#[tauri::command]
pub async fn connect_account(state: State<'_, AppState>, token: String) -> Result<String> {
    if token.trim() != token || !token.starts_with("hf_") || token.len() > 1024 {
        return Err("Enter a valid Hugging Face access token".into());
    }
    let username = Hub::new(Some(token.clone()))
        .map_err(error)?
        .username()
        .await
        .map_err(error)?;
    let stored = token.clone();
    blocking(move || {
        credential()?.set_password(&stored)?;
        Ok(())
    })
    .await?;
    *state.token.lock().map_err(error)? = Some(token.clone());
    state.downloads.set_token(Some(token));
    Ok(username)
}
#[tauri::command]
pub async fn account_status(state: State<'_, AppState>) -> Result<Option<String>> {
    let token = state.token.lock().map_err(error)?.clone();
    if token.is_none() {
        return Ok(None);
    }
    Hub::new(token)
        .map_err(error)?
        .username()
        .await
        .map(Some)
        .map_err(error)
}
#[tauri::command]
pub async fn disconnect_account(state: State<'_, AppState>) -> Result<()> {
    blocking(|| match credential()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    })
    .await?;
    *state.token.lock().map_err(error)? = None;
    state.downloads.set_token(None);
    Ok(())
}
#[tauri::command]
pub async fn open_repository(repo: String) -> Result<()> {
    let id = modelshelf_core::parse_repository(&repo).map_err(error)?;
    blocking(move || {
        open::that(format!("https://huggingface.co/{id}"))?;
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn export_diagnostics(state: State<'_, AppState>) -> Result<Option<String>> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_file_name("modelshelf-diagnostics.json")
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let db = state.db.clone();
    let path = file.path().to_owned();
    blocking(move || {
        let jobs = db.jobs()?;
        let diagnostic = json!({"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"models":db.models()?.len(),"jobs":jobs.iter().map(|j| json!({"status":j.status,"file_count":j.files.len(),"bytes":j.files.iter().map(|f| f.downloaded).sum::<u64>()})).collect::<Vec<_>>()});
        // Native save dialog provides explicit destination/overwrite consent.
        std::fs::write(&path, serde_json::to_vec_pretty(&diagnostic)?)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }).await
}
