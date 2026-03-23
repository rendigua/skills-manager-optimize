use std::fs;

use app_lib::core::sync_engine::{sync_skill_with_policy, SyncMode};
use tempfile::tempdir;

#[test]
fn dry_run_does_not_write_target() {
    let dir = tempdir().expect("temp dir");
    let source = dir.path().join("source-skill");
    let target = dir.path().join("target-skill");
    fs::create_dir_all(&source).expect("create source");
    fs::write(source.join("SKILL.md"), "# Skill").expect("write source");

    let result = sync_skill_with_policy(
        &source,
        &target,
        SyncMode::Copy,
        "cursor",
        "source-skill",
        true,
    )
    .expect("sync result");

    assert_eq!(result.status, "dry-run");
    assert!(!target.exists());
}

#[test]
fn policy_blocked_when_target_hits_codex_system_layer() {
    let dir = tempdir().expect("temp dir");
    let source = dir.path().join("source-skill");
    fs::create_dir_all(&source).expect("create source");
    fs::write(source.join("SKILL.md"), "# Skill").expect("write source");

    let target = dir
        .path()
        .join(".codex")
        .join("skills")
        .join(".system")
        .join("skill-creator");
    let result = sync_skill_with_policy(
        &source,
        &target,
        SyncMode::Copy,
        "codex",
        "skill-creator",
        false,
    )
    .expect("sync result");

    assert_eq!(result.status, "policy-blocked");
    assert!(result.reason.is_some());
}

#[test]
fn policy_blocked_for_reserved_skill_name() {
    let dir = tempdir().expect("temp dir");
    let source = dir.path().join("source-skill");
    let target = dir.path().join("_archives");
    fs::create_dir_all(&source).expect("create source");
    fs::write(source.join("SKILL.md"), "# Skill").expect("write source");

    let result =
        sync_skill_with_policy(&source, &target, SyncMode::Copy, "cursor", "_archives", false)
            .expect("sync result");

    assert_eq!(result.status, "policy-blocked");
}

