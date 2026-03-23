use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::core::{
    skill_store::{SkillStore, SkillTargetRecord},
    sync_engine,
    tool_adapters,
};

#[tauri::command]
pub async fn sync_skill_to_tool(
    skill_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = tool_adapters::find_adapter(&tool)
            .ok_or_else(|| format!("Unknown tool: {}", tool))?;

        if !adapter.is_installed() {
            return Err(format!("{} is not installed", adapter.display_name));
        }

        let skill = store
            .get_skill_by_id(&skill_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Skill not found".to_string())?;

        let source = PathBuf::from(&skill.central_path);
        let target = adapter.skills_dir().join(&skill.name);
        let configured_mode = store
            .get_setting("sync_mode")
            .map_err(|e| e.to_string())?;
        let mode = sync_engine::sync_mode_for_tool(&tool, configured_mode.as_deref());
        let dry_run = store
            .get_setting("sync_dry_run")
            .map_err(|e| e.to_string())?
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let sync_result = sync_engine::sync_skill_with_policy(
            &source,
            &target,
            mode,
            &tool,
            &skill.name,
            dry_run,
        )
        .map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().timestamp_millis();
        let target_record = SkillTargetRecord {
            id: uuid::Uuid::new_v4().to_string(),
            skill_id: skill_id.clone(),
            tool: tool.clone(),
            target_path: target.to_string_lossy().to_string(),
            mode: sync_result.mode.as_str().to_string(),
            status: sync_result.status.clone(),
            synced_at: Some(now),
            last_error: sync_result.reason.clone(),
        };

        store
            .insert_target(&target_record)
            .map_err(|e| e.to_string())?;

        if sync_result.status == "policy-blocked" {
            return Err(
                sync_result
                    .reason
                    .unwrap_or_else(|| "Sync blocked by policy".to_string()),
            );
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn unsync_skill_from_tool(
    skill_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let targets = store
            .get_targets_for_skill(&skill_id)
            .map_err(|e| e.to_string())?;

        if let Some(target) = targets.iter().find(|t| t.tool == tool) {
            let target_path = PathBuf::from(&target.target_path);
            sync_engine::remove_target(&target_path).ok();
        }

        store
            .delete_target(&skill_id, &tool)
            .map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
