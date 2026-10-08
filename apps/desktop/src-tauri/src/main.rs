#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod system;

use modelshelf_core::Database;
use modelshelf_download::DownloadManager;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub struct AppState {
    db: Arc<Database>,
    downloads: Arc<DownloadManager>,
    token: Mutex<Option<String>>,
    hardware: serde_json::Value,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("modelshelf=info")
        .init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let db = Arc::new(Database::open(&data.join("modelshelf.db"))?);
            let mut settings = db.settings()?;
            if settings.default_directory.is_empty() {
                let models = data.join("models");
                std::fs::create_dir_all(&models)?;
                settings.default_directory = models.canonicalize()?.to_string_lossy().into_owned();
                db.save_settings(&settings)?;
                db.add_location(&modelshelf_core::StorageLocation {
                    path: settings.default_directory.clone(),
                    kind: "managed".into(),
                })?;
            }
            let token = match commands::credential()?.get_password() {
                Ok(token) => Some(token),
                Err(keyring::Error::NoEntry) => None,
                Err(_) => {
                    tracing::warn!(
                        "OS credential store could not be read; connect account again in Settings"
                    );
                    None
                }
            };
            let downloads = DownloadManager::new(db.clone());
            downloads.set_token(token.clone());
            app.manage(AppState {
                db: db.clone(),
                downloads: downloads.clone(),
                token: Mutex::new(token),
                hardware: system::hardware(),
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                downloads.start();
                let mut previous = String::new();
                let mut completed: std::collections::HashSet<String> = db
                    .jobs()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|j| matches!(j.status, modelshelf_core::DownloadState::Completed))
                    .map(|j| j.id)
                    .collect();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    match db.jobs() {
                        Ok(jobs) => {
                            let current = serde_json::to_string(&jobs).unwrap_or_default();
                            if current != previous {
                                let _ = handle.emit("state-changed", ());
                                previous = current;
                            }
                            for job in jobs.iter().filter(|j| {
                                matches!(j.status, modelshelf_core::DownloadState::Completed)
                            }) {
                                if completed.insert(job.id.clone())
                                    && db.settings().map(|s| s.notifications).unwrap_or(false)
                                {
                                    if let Err(error) = handle
                                        .notification()
                                        .builder()
                                        .title("ModelShelf")
                                        .body(format!("Download completed: {}", job.repository))
                                        .show()
                                    {
                                        tracing::warn!(%error, "Notification unavailable");
                                    }
                                }
                            }
                        }
                        Err(error) => tracing::error!(%error, "Cannot read download status"),
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::search_models,
            commands::repository_details,
            commands::choose_directory,
            commands::import_model,
            commands::create_download,
            commands::download_action,
            commands::remove_model,
            commands::delete_model,
            commands::verify_model,
            commands::update_model,
            commands::open_model_folder,
            commands::recheck_model,
            commands::rescan_model,
            commands::save_settings,
            commands::add_location,
            commands::connect_account,
            commands::account_status,
            commands::disconnect_account,
            commands::open_repository,
            commands::export_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("ModelShelf could not start");
}
