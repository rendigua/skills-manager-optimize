use crate::core::{central_repo, git_backup};
use std::sync::Arc;
use tauri::State;

use crate::core::skill_store::SkillStore;

#[tauri::command]
pub async fn git_backup_status(
    store: State<'_, Arc<SkillStore>>,
) -> Result<git_backup::GitBackupStatus, String> {
    let _ = store; // ensure DB is available
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::get_status(&skills_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_backup_init(
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), String> {
    let _ = store;
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::init_repo(&skills_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_backup_set_remote(
    store: State<'_, Arc<SkillStore>>,
    url: String,
) -> Result<(), String> {
    let _ = store;
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::set_remote(&skills_dir, &url).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_backup_commit(
    store: State<'_, Arc<SkillStore>>,
    message: String,
) -> Result<(), String> {
    let _ = store;
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::commit_all(&skills_dir, &message).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_backup_push(
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), String> {
    let _ = store;
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::push(&skills_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_backup_pull(
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), String> {
    let _ = store;
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::pull(&skills_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_backup_clone(
    store: State<'_, Arc<SkillStore>>,
    url: String,
) -> Result<(), String> {
    let _ = store;
    let skills_dir = central_repo::skills_dir();
    tokio::task::spawn_blocking(move || {
        git_backup::clone_into(&skills_dir, &url).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
