use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::path::Path;

use crate::core::origin_metadata::{
    load_origin_metadata, save_origin_metadata, ConfidenceLevel, OriginMetadata, SourceKind,
    SourceType,
};
use crate::core::policy_engine::should_skip_directory;
use crate::core::source_resolver::SourceCandidate;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BackfillAction {
    Keep,
    ApplyCandidate,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginBackfillEntry {
    pub name: String,
    pub path: String,
    pub query: String,
    pub current_source_type: String,
    pub current_source_kind: Option<String>,
    pub candidate_count: usize,
    pub top_candidate: Option<SourceCandidate>,
    pub action: BackfillAction,
    pub reason: String,
    pub needs_network_review: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginBackfillPlan {
    pub root: String,
    pub entries: Vec<OriginBackfillEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginBackfillApplyResult {
    pub applied: usize,
    pub skipped: usize,
    pub review_needed: usize,
}

pub fn plan_origin_backfill<F>(root: &Path, limit: usize, resolver: F) -> Result<OriginBackfillPlan>
where
    F: Fn(&str, usize, Option<&str>) -> Result<Vec<SourceCandidate>>,
{
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(OriginBackfillPlan {
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
        let entry = classify_skill(&name, &path, metadata, limit, &resolver)?;
        entries.push(entry);
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(OriginBackfillPlan {
        root: root.to_string_lossy().to_string(),
        entries,
    })
}

pub fn apply_origin_backfill<F>(
    root: &Path,
    selected_paths: &[String],
    limit: usize,
    resolver: F,
) -> Result<OriginBackfillApplyResult>
where
    F: Fn(&str, usize, Option<&str>) -> Result<Vec<SourceCandidate>>,
{
    let mut applied = 0;
    let mut skipped = 0;
    let mut review_needed = 0;

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
        let candidates = resolver(&name, limit, None)?;
        let entry = classify_with_candidates(&name, path, metadata.clone(), candidates)?;
        match entry.action {
            BackfillAction::Keep => {
                skipped += 1;
            }
            BackfillAction::NeedsReview => {
                review_needed += 1;
            }
            BackfillAction::ApplyCandidate => {
                if let Some(candidate) = entry.top_candidate {
                    let next = build_resolved_origin_metadata(metadata, &candidate);
                    save_origin_metadata(path, &next)?;
                    applied += 1;
                } else {
                    skipped += 1;
                }
            }
        }
    }

    let _ = root;
    Ok(OriginBackfillApplyResult {
        applied,
        skipped,
        review_needed,
    })
}

fn classify_skill<F>(
    name: &str,
    path: &Path,
    metadata: OriginMetadata,
    limit: usize,
    resolver: &F,
) -> Result<OriginBackfillEntry>
where
    F: Fn(&str, usize, Option<&str>) -> Result<Vec<SourceCandidate>>,
{
    let query = name.to_string();
    let candidates = resolver(&query, limit, None)?;
    classify_with_candidates(name, path, metadata, candidates)
}

fn classify_with_candidates(
    name: &str,
    path: &Path,
    metadata: OriginMetadata,
    candidates: Vec<SourceCandidate>,
) -> Result<OriginBackfillEntry> {
    let current_source_kind = metadata
        .source_kind
        .as_ref()
        .map(|kind| kind.as_str().to_string());

    if metadata.source_kind.is_some() && !matches!(metadata.source_kind, Some(SourceKind::CustomNoSource)) {
        return Ok(OriginBackfillEntry {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            query: name.to_string(),
            current_source_type: source_type_as_str(&metadata.source_type).to_string(),
            current_source_kind,
            candidate_count: candidates.len(),
            top_candidate: candidates.first().cloned(),
            action: BackfillAction::Keep,
            reason: "already classified".to_string(),
            needs_network_review: false,
        });
    }

    let top_candidate = candidates.first().cloned();
    if let Some(candidate) = top_candidate.clone() {
        if is_unambiguous_match(name, &candidate, &candidates) {
            return Ok(OriginBackfillEntry {
                name: name.to_string(),
                path: path.to_string_lossy().to_string(),
                query: name.to_string(),
                current_source_type: source_type_as_str(&metadata.source_type).to_string(),
                current_source_kind,
                candidate_count: candidates.len(),
                top_candidate,
                action: BackfillAction::ApplyCandidate,
                reason: format!(
                    "resolver matched {} to {} with strong signal",
                    name, candidate.source_ref_resolved
                ),
                needs_network_review: false,
            });
        }

        return Ok(OriginBackfillEntry {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            query: name.to_string(),
            current_source_type: source_type_as_str(&metadata.source_type).to_string(),
            current_source_kind,
            candidate_count: candidates.len(),
            top_candidate,
            action: BackfillAction::NeedsReview,
            reason: "resolver found candidates but signal is ambiguous".to_string(),
            needs_network_review: true,
        });
    }

    Ok(OriginBackfillEntry {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        query: name.to_string(),
        current_source_type: source_type_as_str(&metadata.source_type).to_string(),
        current_source_kind,
        candidate_count: 0,
        top_candidate: None,
        action: BackfillAction::Keep,
        reason: "no resolver candidates found".to_string(),
        needs_network_review: false,
    })
}

fn build_resolved_origin_metadata(existing: OriginMetadata, candidate: &SourceCandidate) -> OriginMetadata {
    let mut metadata = existing;
    metadata.source_type = SourceType::Marketplace;
    metadata.source_kind = Some(SourceKind::VerifiedDistribution);
    metadata.source_ref = Some(candidate.source_ref.clone());
    metadata.source_ref_resolved = Some(candidate.source_ref_resolved.clone());
    metadata.source_subpath = candidate.source_subpath.clone();
    metadata.source_branch = candidate.source_branch.clone();
    metadata.source_revision = None;
    metadata.installed_via = Some("origin-resolver".to_string());
    metadata.upstream_project = candidate
        .source_ref_resolved
        .strip_suffix(".git")
        .map(|value| value.to_string())
        .or_else(|| Some(candidate.source_ref_resolved.clone()));
    metadata.imported_at = Some(Utc::now().to_rfc3339());
    metadata.distribution_ref = candidate.distribution_ref.clone();
    metadata.confidence = Some(ConfidenceLevel::High);
    metadata.resolution_method = Some("resolver-auto-match".to_string());
    metadata.notes = Some(format!(
        "Auto-resolved from candidate {}",
        candidate.source_ref_resolved
    ));
    metadata
}

fn is_unambiguous_match(name: &str, candidate: &SourceCandidate, candidates: &[SourceCandidate]) -> bool {
    if candidates.is_empty() {
        return false;
    }

    let normalized_name = normalize_token(name);
    let normalized_title = normalize_token(&candidate.title);
    let normalized_skill_id = normalize_token(&candidate.skill_id);
    if normalized_name != normalized_title && normalized_name != normalized_skill_id {
        return false;
    }

    let top = candidate.installs;
    let second = candidates.get(1).map(|value| value.installs).unwrap_or(0);
    top > 0 && (second == 0 || top >= second.saturating_mul(2))
}

fn normalize_token(value: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;

    for ch in value.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_dash = false;
        } else if !last_dash {
            output.push('-');
            last_dash = true;
        }
    }

    output.trim_matches('-').to_string()
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

#[cfg(test)]
mod tests {
    use super::{plan_origin_backfill, BackfillAction};
    use crate::core::source_resolver::SourceCandidate;
    use std::fs;
    use tempfile::tempdir;

    fn candidate(title: &str, installs: u64, source_ref_resolved: &str) -> SourceCandidate {
        SourceCandidate {
            source_kind: "verified-distribution".to_string(),
            source_ref: "openai/skills/playwright".to_string(),
            source_ref_resolved: source_ref_resolved.to_string(),
            source_subpath: Some("playwright".to_string()),
            source_branch: None,
            distribution_ref: Some("https://skills.sh/openai/skills/playwright".to_string()),
            title: title.to_string(),
            skill_id: "playwright".to_string(),
            installs,
            confidence: "high".to_string(),
            evidence_refs: vec!["https://skills.sh/openai/skills/playwright".to_string()],
            install_command: Some("npx skills add https://github.com/openai/skills --skill playwright".to_string()),
        }
    }

    #[test]
    fn plans_safe_backfill_for_unambiguous_custom_skill() {
        let root = tempdir().expect("temp dir");
        let skill_dir = root.path().join("playwright");
        fs::create_dir_all(&skill_dir).expect("skill dir");
        fs::write(
            skill_dir.join("origin.json"),
            r#"{
  "schema_version": 1,
  "source_type": "manual",
  "source_kind": "custom-no-source",
  "source_ref": null
}"#,
        )
        .expect("origin json");
        fs::write(skill_dir.join("SKILL.md"), "# Playwright").expect("skill md");

        let plan = plan_origin_backfill(root.path(), 10, |query, _, _| {
            assert_eq!(query, "playwright");
            Ok(vec![candidate(
                "Playwright",
                120,
                "https://github.com/openai/skills.git",
            )])
        })
        .expect("plan");

        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].action, BackfillAction::ApplyCandidate);
        assert_eq!(
            plan.entries[0].top_candidate.as_ref().map(|value| value.source_ref_resolved.as_str()),
            Some("https://github.com/openai/skills.git")
        );
    }

    #[test]
    fn keeps_ambiguous_candidates_for_review() {
        let root = tempdir().expect("temp dir");
        let skill_dir = root.path().join("agent-browser");
        fs::create_dir_all(&skill_dir).expect("skill dir");
        fs::write(
            skill_dir.join("origin.json"),
            r#"{
  "schema_version": 1,
  "source_type": "legacy-untracked"
}"#,
        )
        .expect("origin json");
        fs::write(skill_dir.join("SKILL.md"), "# Agent Browser").expect("skill md");

        let plan = plan_origin_backfill(root.path(), 10, |_query, _, _| {
            Ok(vec![
                candidate("Agent Browser", 70, "https://github.com/a/repo.git"),
                candidate("Agent Browser", 69, "https://github.com/b/repo.git"),
            ])
        })
        .expect("plan");

        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].action, BackfillAction::NeedsReview);
    }
}
