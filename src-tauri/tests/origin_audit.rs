use std::fs;

use tempfile::tempdir;

use app_lib::core::origin_audit::{apply_origin_resolution, plan_origin_resolution, ResolutionAction};
use app_lib::core::origin_metadata::{load_origin_metadata, SourceKind, SourceType};

fn write_skill(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let skill_dir = dir.join(name);
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# skill").expect("skill md");
    skill_dir
}

#[test]
fn plans_custom_no_source_for_legacy_untracked_skill() {
    let root = tempdir().expect("temp dir");
    let legacy = write_skill(root.path(), "legacy-skill");

    let plan = plan_origin_resolution(root.path()).expect("plan");
    let entry = plan
        .entries
        .into_iter()
        .find(|entry| entry.path == legacy.to_string_lossy())
        .expect("legacy entry");

    assert_eq!(entry.action, ResolutionAction::WriteCustomNoSource);
    assert_eq!(entry.recommended_source_kind, Some(SourceKind::CustomNoSource));
    assert!(!entry.needs_network_review);
}

#[test]
fn plans_network_review_for_git_skill_missing_source_kind() {
    let root = tempdir().expect("temp dir");
    let git_skill = write_skill(root.path(), "git-skill");
    fs::write(
        git_skill.join("origin.json"),
        r#"{
  "schema_version": 1,
  "source_type": "git",
  "source_ref": "https://github.com/example/repo"
}"#,
    )
    .expect("origin");

    let plan = plan_origin_resolution(root.path()).expect("plan");
    let entry = plan
        .entries
        .into_iter()
        .find(|entry| entry.path == git_skill.to_string_lossy())
        .expect("git entry");

    assert_eq!(entry.action, ResolutionAction::NeedsNetworkReview);
    assert_eq!(entry.recommended_source_kind, Some(SourceKind::InferredUpstream));
    assert!(entry.needs_network_review);
}

#[test]
fn applies_custom_no_source_origin_metadata() {
    let root = tempdir().expect("temp dir");
    let legacy = write_skill(root.path(), "legacy-skill");

    let result = apply_origin_resolution(root.path(), &[legacy.to_string_lossy().to_string()]).expect("apply");
    assert_eq!(result.applied, 1);
    assert_eq!(result.skipped, 0);

    let metadata = load_origin_metadata(&legacy).expect("load metadata");
    assert_eq!(metadata.source_type, SourceType::Manual);
    assert_eq!(metadata.source_kind, Some(SourceKind::CustomNoSource));
    assert_eq!(metadata.installed_via.as_deref(), Some("origin-backfill"));
}
