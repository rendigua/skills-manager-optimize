use std::sync::Arc;
use tauri::State;

use crate::core::{
    skill_store::SkillStore,
    skillssh_api::{self, LeaderboardType, SkillsShSkill},
    source_resolver::{self, SourceCandidate},
};

const LEADERBOARD_CACHE_TTL: i64 = 300; // 5 minutes

#[tauri::command]
pub async fn fetch_leaderboard(
    board: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<SkillsShSkill>, String> {
    let cache_key = format!("leaderboard_{}", board);

    // Check cache
    if let Ok(Some(cached)) = store.get_cache(&cache_key, LEADERBOARD_CACHE_TTL) {
        if let Ok(skills) = serde_json::from_str::<Vec<SkillsShSkill>>(&cached) {
            return Ok(skills);
        }
    }

    let board_type = LeaderboardType::from_str(&board);
    let skills = tauri::async_runtime::spawn_blocking(move || {
        skillssh_api::fetch_leaderboard(board_type).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("failed to join leaderboard task: {e}"))??;

    // Update cache
    if let Ok(json) = serde_json::to_string(&skills) {
        store.set_cache(&cache_key, &json).ok();
    }

    Ok(skills)
}

#[tauri::command]
pub async fn search_skillssh(query: String, limit: Option<usize>) -> Result<Vec<SkillsShSkill>, String> {
    let requested = limit.unwrap_or(60);
    let bounded = requested.clamp(1, 300);
    tauri::async_runtime::spawn_blocking(move || {
        skillssh_api::search_skills(&query, bounded).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("failed to join search task: {e}"))?
}

#[tauri::command]
pub async fn resolve_source_candidates(
    query: String,
    limit: Option<usize>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<SourceCandidate>, String> {
    let requested = limit.unwrap_or(60);
    let bounded = requested.clamp(1, 200);
    let store = store.inner().clone();
    let skillsmp_api_key = store
        .get_setting("skillsmp_api_key")
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty());

    tauri::async_runtime::spawn_blocking(move || {
        source_resolver::resolve_source_candidates(&query, bounded, skillsmp_api_key.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("failed to join source resolver task: {e}"))?
}
