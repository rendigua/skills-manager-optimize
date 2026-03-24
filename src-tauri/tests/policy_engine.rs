use app_lib::core::policy_engine::{
    classify_policy_block, is_system_reserved, should_skip_directory, PolicyDecision,
};
use tempfile::tempdir;

#[test]
fn blocks_codex_system_shadowing() {
    let dir = tempdir().expect("temp dir");
    let system_skill_path = dir
        .path()
        .join(".codex")
        .join("skills")
        .join(".system")
        .join("skill-creator");

    let decision = classify_policy_block("codex", "skill-creator", &system_skill_path);
    assert!(matches!(
        decision,
        PolicyDecision::Blocked { ref code, .. } if code == "system-reserved"
    ));
    assert!(is_system_reserved(
        "codex",
        "skill-creator",
        &system_skill_path
    ));
}

#[test]
fn ignores_archive_and_backup_directories() {
    assert!(should_skip_directory("_archives"));
    assert!(should_skip_directory("_backups"));
    assert!(should_skip_directory("_disabled_by_policy"));
    assert!(should_skip_directory(".system"));
    assert!(should_skip_directory("legacy_backup_2026-03-18"));
}
