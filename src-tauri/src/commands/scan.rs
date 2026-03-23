use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::core::{
    central_repo,
    installer,
    migration_scanner,
    origin_backfill,
    origin_audit,
    scanner,
    skill_store::SkillStore,
};

#[derive(Debug, Serialize)]
pub struct ScanResultDto {
    pub tools_scanned: usize,
    pub skills_found: usize,
    pub groups: Vec<scanner::DiscoveredGroup>,
}

#[derive(Debug, Serialize)]
pub struct MigrationScanResultDto {
    pub root: String,
    pub entries: Vec<migration_scanner::MigrationEntry>,
}

#[derive(Debug, Serialize)]
pub struct OriginResolutionPlanDto {
    pub root: String,
    pub entries: Vec<origin_audit::OriginResolutionEntry>,
}

#[derive(Debug, Serialize)]
pub struct OriginResolutionApplyResultDto {
    pub applied: usize,
    pub skipped: usize,
    pub network_review_needed: usize,
}

#[tauri::command]
pub async fn scan_local_skills(store: State<'_, Arc<SkillStore>>) -> Result<ScanResultDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let all_targets = store.get_all_targets().map_err(|e| e.to_string())?;
        let managed_paths: Vec<String> = all_targets.iter().map(|t| t.target_path.clone()).collect();
        let managed_skills = store.get_all_skills().map_err(|e| e.to_string())?;

        let mut plan = scanner::scan_local_skills(&managed_paths).map_err(|e| e.to_string())?;

        for rec in &mut plan.discovered {
            if let Some(name) = rec.name_guess.as_deref() {
                if let Some(existing) = managed_skills.iter().find(|skill| skill.name == name) {
                    rec.imported_skill_id = Some(existing.id.clone());
                }
            }
        }

        // Clear and repopulate discovered
        store.clear_discovered().map_err(|e| e.to_string())?;
        for rec in &plan.discovered {
            store.insert_discovered(rec).map_err(|e| e.to_string())?;
        }

        let all_discovered = store.get_all_discovered().map_err(|e| e.to_string())?;
        let groups = scanner::group_discovered(&all_discovered);

        Ok(ScanResultDto {
            tools_scanned: plan.tools_scanned,
            skills_found: plan.skills_found,
            groups,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_existing_skill(
    source_path: String,
    name: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(&source_path);
        let resolved_name =
            installer::resolve_local_skill_name(&path, name.as_deref()).map_err(|e| e.to_string())?;
        let central_path = central_repo::skills_dir().join(&resolved_name);

        if let Some(existing) = store
            .get_skill_by_central_path(&central_path.to_string_lossy())
            .map_err(|e| e.to_string())?
        {
            if let Ok(Some(scenario_id)) = store.get_active_scenario_id() {
                store
                    .add_skill_to_scenario(&scenario_id, &existing.id)
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        let result =
            installer::install_from_local_to_destination(&path, Some(&resolved_name), &central_path)
                .map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().timestamp_millis();
        let id = uuid::Uuid::new_v4().to_string();

        let record = crate::core::skill_store::SkillRecord {
            id: id.clone(),
            name: result.name,
            description: result.description,
            source_type: "import".to_string(),
            source_ref: Some(source_path),
            source_ref_resolved: None,
            source_subpath: None,
            source_branch: None,
            source_revision: None,
            remote_revision: None,
            central_path: result.central_path.to_string_lossy().to_string(),
            content_hash: Some(result.content_hash),
            enabled: true,
            created_at: now,
            updated_at: now,
            status: "ok".to_string(),
            update_status: "local_only".to_string(),
            last_checked_at: Some(now),
            last_check_error: None,
        };

        store.insert_skill(&record).map_err(|e| e.to_string())?;

        // Auto-add to active scenario
        if let Ok(Some(scenario_id)) = store.get_active_scenario_id() {
            store
                .add_skill_to_scenario(&scenario_id, &id)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_all_discovered(store: State<'_, Arc<SkillStore>>) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let discovered = store.get_all_discovered().map_err(|e| e.to_string())?;
        let groups = scanner::group_discovered(&discovered);

        let active_scenario = store.get_active_scenario_id().ok().flatten();

        for group in groups {
            if group.imported {
                continue;
            }
            if let Some(first) = group.locations.first() {
                let path = PathBuf::from(&first.found_path);
                let central_path = central_repo::skills_dir().join(&group.name);

                if let Ok(Some(existing)) =
                    store.get_skill_by_central_path(&central_path.to_string_lossy())
                {
                    if let Some(ref scenario_id) = active_scenario {
                        store.add_skill_to_scenario(scenario_id, &existing.id).ok();
                    }
                    continue;
                }

                if let Ok(result) =
                    installer::install_from_local_to_destination(&path, Some(&group.name), &central_path)
                {
                    let now = chrono::Utc::now().timestamp_millis();
                    let id = uuid::Uuid::new_v4().to_string();
                    let record = crate::core::skill_store::SkillRecord {
                        id: id.clone(),
                        name: result.name,
                        description: result.description,
                        source_type: "import".to_string(),
                        source_ref: Some(first.found_path.clone()),
                        source_ref_resolved: None,
                        source_subpath: None,
                        source_branch: None,
                        source_revision: None,
                        remote_revision: None,
                        central_path: result.central_path.to_string_lossy().to_string(),
                        content_hash: Some(result.content_hash),
                        enabled: true,
                        created_at: now,
                        updated_at: now,
                        status: "ok".to_string(),
                        update_status: "local_only".to_string(),
                        last_checked_at: Some(now),
                        last_check_error: None,
                    };
                    store.insert_skill(&record).ok();

                    if let Some(ref scenario_id) = active_scenario {
                        store.add_skill_to_scenario(scenario_id, &id).ok();
                    }
                }
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn scan_runtime_migration(root_path: String) -> Result<MigrationScanResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = PathBuf::from(root_path);
        let result = migration_scanner::scan_runtime_snapshot(&root).map_err(|e| e.to_string())?;
        Ok(MigrationScanResultDto {
            root: result.root,
            entries: result.entries,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn scan_origin_resolution(root_path: Option<String>) -> Result<OriginResolutionPlanDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root_path
            .map(PathBuf::from)
            .unwrap_or_else(|| central_repo::skills_dir());
        let plan = origin_audit::plan_origin_resolution(&root).map_err(|e| e.to_string())?;
        Ok(OriginResolutionPlanDto {
            root: plan.root,
            entries: plan.entries,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_origin_resolution(
    root_path: Option<String>,
    selected_paths: Vec<String>,
) -> Result<OriginResolutionApplyResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root_path
            .map(PathBuf::from)
            .unwrap_or_else(|| central_repo::skills_dir());
        let result = origin_audit::apply_origin_resolution(&root, &selected_paths)
            .map_err(|e| e.to_string())?;
        Ok(OriginResolutionApplyResultDto {
            applied: result.applied,
            skipped: result.skipped,
            network_review_needed: result.network_review_needed,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn scan_origin_backfill(
    root_path: Option<String>,
    limit: Option<usize>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<origin_backfill::OriginBackfillPlan, String> {
    let store = store.inner().clone();
    let requested = limit.unwrap_or(60);
    let bounded = requested.clamp(1, 200);
    let skillsmp_api_key = store
        .get_setting("skillsmp_api_key")
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty());

    tauri::async_runtime::spawn_blocking(move || {
        let root = root_path
            .map(PathBuf::from)
            .unwrap_or_else(|| central_repo::skills_dir());
        let resolver_key_for_plan = skillsmp_api_key.clone();
        let plan = origin_backfill::plan_origin_backfill(&root, bounded, move |query, limit, api_key_override| {
            match crate::core::source_resolver::resolve_source_candidates(
                query,
                limit,
                api_key_override.or(resolver_key_for_plan.as_deref()),
            ) {
                Ok(candidates) => Ok(candidates),
                Err(error) => {
                    log::warn!("source resolution failed for {query}: {error}");
                    Ok(Vec::new())
                }
            }
        })
        .map_err(|e| e.to_string())?;
        Ok(plan)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_origin_backfill(
    root_path: Option<String>,
    limit: Option<usize>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<origin_backfill::OriginBackfillApplyResult, String> {
    let store = store.inner().clone();
    let requested = limit.unwrap_or(60);
    let bounded = requested.clamp(1, 200);
    let skillsmp_api_key = store
        .get_setting("skillsmp_api_key")
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty());

    tauri::async_runtime::spawn_blocking(move || {
        let root = root_path
            .map(PathBuf::from)
            .unwrap_or_else(|| central_repo::skills_dir());
        let resolver_key_for_apply_plan = skillsmp_api_key.clone();
        let plan = origin_backfill::plan_origin_backfill(&root, bounded, move |query, limit, api_key_override| {
            match crate::core::source_resolver::resolve_source_candidates(
                query,
                limit,
                api_key_override.or(resolver_key_for_apply_plan.as_deref()),
            ) {
                Ok(candidates) => Ok(candidates),
                Err(error) => {
                    log::warn!("source resolution failed for {query}: {error}");
                    Ok(Vec::new())
                }
            }
        })
        .map_err(|e| e.to_string())?;

        let selected_paths = plan
            .entries
            .iter()
            .filter(|entry| entry.action == origin_backfill::BackfillAction::ApplyCandidate)
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();

        let resolver_key_for_apply_exec = skillsmp_api_key;
        let mut result = origin_backfill::apply_origin_backfill(&root, &selected_paths, bounded, move |query, limit, api_key_override| {
            match crate::core::source_resolver::resolve_source_candidates(
                query,
                limit,
                api_key_override.or(resolver_key_for_apply_exec.as_deref()),
            ) {
                Ok(candidates) => Ok(candidates),
                Err(error) => {
                    log::warn!("source resolution failed for {query}: {error}");
                    Ok(Vec::new())
                }
            }
        })
        .map_err(|e| e.to_string())?;
        result.review_needed = plan
            .entries
            .iter()
            .filter(|entry| entry.action == origin_backfill::BackfillAction::NeedsReview)
            .count();
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
