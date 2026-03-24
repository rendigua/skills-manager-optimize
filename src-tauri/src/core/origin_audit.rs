use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use chrono::Utc;

use crate::core::origin_metadata::{
    load_origin_metadata, save_origin_metadata, ConfidenceLevel, OriginMetadata, SourceKind,
    SourceType,
};
use crate::core::policy_engine::should_skip_directory;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionAction {
    Keep,
    WriteCustomNoSource,
    NeedsNetworkReview,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginResolutionEntry {
    pub name: String,
    pub path: String,
    pub current_source_type: String,
    pub current_source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub source_ref_resolved: Option<String>,
    pub recommended_source_kind: Option<SourceKind>,
    pub action: ResolutionAction,
    pub reason: String,
    pub needs_network_review: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginResolutionPlan {
    pub root: String,
    pub entries: Vec<OriginResolutionEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginResolutionApplyResult {
    pub applied: usize,
    pub skipped: usize,
    pub network_review_needed: usize,
}

pub fn plan_origin_resolution(root: &Path) -> Result<OriginResolutionPlan> {
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(OriginResolutionPlan {
            root: root.to_string_lossy().to_string(),
            entries,
        });
    }

    for dir in std::fs::read_dir(root)? {
        let dir = match dir {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = dir.path();
        if !path.is_dir() && !path.is_symlink() {
            continue;
        }

        let name = dir.file_name().to_string_lossy().to_string();
        if name == "_archives" || name == "_backups" || name == ".system" || should_skip_directory(&name) {
            continue;
        }

        let skill_md = path.join("SKILL.md");
        let origin_path = path.join("origin.json");
        if !skill_md.exists() && !origin_path.exists() {
            continue;
        }

        let metadata = load_origin_metadata(&path)?;
        let entry = classify_skill(&name, &path, metadata);
        entries.push(entry);
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(OriginResolutionPlan {
        root: root.to_string_lossy().to_string(),
        entries,
    })
}

pub fn apply_origin_resolution(root: &Path, selected_paths: &[String]) -> Result<OriginResolutionApplyResult> {
    let mut applied = 0;
    let mut skipped = 0;
    let mut network_review_needed = 0;

    for selected in selected_paths {
        let path = Path::new(selected);
        if !path.exists() || !path.is_dir() {
            skipped += 1;
            continue;
        }

        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        if name == "_archives" || name == "_backups" || name == ".system" || should_skip_directory(&name) {
            skipped += 1;
            continue;
        }

        let metadata = load_origin_metadata(path)?;
        match classify_skill(&name, path, metadata.clone()).action {
            ResolutionAction::Keep => {
                skipped += 1;
            }
            ResolutionAction::NeedsNetworkReview => {
                network_review_needed += 1;
            }
            ResolutionAction::WriteCustomNoSource => {
                let next = build_custom_no_source_metadata(metadata);
                save_origin_metadata(path, &next)?;
                applied += 1;
            }
        }
    }

    let _ = root;
    Ok(OriginResolutionApplyResult {
        applied,
        skipped,
        network_review_needed,
    })
}

fn classify_skill(name: &str, path: &Path, metadata: OriginMetadata) -> OriginResolutionEntry {
    let current_source_kind = metadata
        .source_kind
        .as_ref()
        .map(|kind| kind.as_str().to_string());

    if metadata.source_kind.is_some() {
        return OriginResolutionEntry {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            current_source_type: source_type_as_str(&metadata.source_type).to_string(),
            current_source_kind,
            source_ref: metadata.source_ref.clone(),
            source_ref_resolved: None,
            recommended_source_kind: metadata.source_kind,
            action: ResolutionAction::Keep,
            reason: "already classified".to_string(),
            needs_network_review: false,
        };
    }

    let needs_network_review = matches!(
        metadata.source_type,
        SourceType::Git | SourceType::Marketplace
    );
    let (recommended_source_kind, action, reason) = match metadata.source_type {
        SourceType::Git => (
            Some(SourceKind::InferredUpstream),
            ResolutionAction::NeedsNetworkReview,
            "git source needs upstream confirmation before backfill".to_string(),
        ),
        SourceType::Marketplace => (
            Some(SourceKind::VerifiedDistribution),
            ResolutionAction::NeedsNetworkReview,
            "distribution entry needs upstream confirmation before backfill".to_string(),
        ),
        SourceType::Local | SourceType::Manual | SourceType::Archive | SourceType::LegacyUntracked => (
            Some(SourceKind::CustomNoSource),
            ResolutionAction::WriteCustomNoSource,
            "no confirmed upstream; safe to backfill as custom-no-source".to_string(),
        ),
    };

    OriginResolutionEntry {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        current_source_type: source_type_as_str(&metadata.source_type).to_string(),
        current_source_kind,
        source_ref: metadata.source_ref.clone(),
        source_ref_resolved: None,
        recommended_source_kind,
        action,
        reason,
        needs_network_review,
    }
}

fn build_custom_no_source_metadata(existing: OriginMetadata) -> OriginMetadata {
    let mut metadata = existing;
    if matches!(metadata.source_type, SourceType::LegacyUntracked) {
        metadata.source_type = SourceType::Manual;
    }
    metadata.source_kind = Some(SourceKind::CustomNoSource);
    metadata.confidence = Some(ConfidenceLevel::Low);
    metadata.resolution_method = Some("manual-no-source-default".to_string());
    metadata.installed_via = Some("origin-backfill".to_string());
    if metadata.imported_at.is_none() {
        metadata.imported_at = Some(Utc::now().to_rfc3339());
    }
    if metadata.notes.is_none() {
        metadata.notes = Some("No confirmed upstream; classified as custom-no-source".to_string());
    }
    metadata
}

fn source_type_as_str(source_type: &SourceType) -> &'static str {
    match source_type {
        SourceType::Git => "git",
        SourceType::Marketplace => "marketplace",
        SourceType::Local => "local",
        SourceType::Manual => "manual",
        SourceType::Archive => "archive",
        SourceType::LegacyUntracked => "legacy-untracked",
    }
}
