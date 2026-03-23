use std::path::PathBuf;

use rusqlite::Connection;
use tempfile::tempdir;

use app_lib::core::skill_store::SkillStore;
use app_lib::core::origin_metadata::SourceKind;
use app_lib::core::update_checker::{decide_update_status, UpdateStatus};

#[test]
fn persists_origin_and_update_fields() {
    let dir = tempdir().expect("temp dir");
    let db_path: PathBuf = dir.path().join("skills-manager.db");

    let _store = SkillStore::new(&db_path).expect("init store");
    let conn = Connection::open(db_path).expect("open db");

    let mut stmt = conn
        .prepare("PRAGMA table_info(skills)")
        .expect("table info");
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query map");
    let columns: Vec<String> = rows.filter_map(|r| r.ok()).collect();

    assert!(columns.contains(&"origin_json_path".to_string()));
    assert!(columns.contains(&"source_type".to_string()));
    assert!(columns.contains(&"source_ref".to_string()));
    assert!(columns.contains(&"update_status".to_string()));
    assert!(columns.contains(&"last_checked_at".to_string()));
}

#[test]
fn returns_up_to_date_for_equal_git_revisions() {
    let decision = decide_update_status(
        "git",
        Some(&SourceKind::VerifiedUpstream),
        Some("abc123"),
        Some("abc123"),
        None,
        false,
        None,
    );
    assert_eq!(decision.status, UpdateStatus::UpToDate);
}

#[test]
fn returns_update_available_for_different_git_revisions() {
    let decision = decide_update_status(
        "git",
        Some(&SourceKind::VerifiedUpstream),
        Some("abc123"),
        Some("def456"),
        None,
        false,
        None,
    );
    assert_eq!(decision.status, UpdateStatus::UpdateAvailable);
}

#[test]
fn returns_reimport_available_for_existing_local_source() {
    let decision = decide_update_status(
        "local",
        None,
        None,
        None,
        Some(true),
        false,
        None,
    );
    assert_eq!(decision.status, UpdateStatus::ReimportAvailable);
}

#[test]
fn returns_legacy_untracked_when_source_missing() {
    let decision = decide_update_status(
        "local",
        None,
        None,
        None,
        Some(false),
        false,
        None,
    );
    assert_eq!(decision.status, UpdateStatus::LegacyUntracked);
}

#[test]
fn returns_policy_blocked_when_policy_blocks_skill() {
    let decision = decide_update_status(
        "git",
        Some(&SourceKind::VerifiedUpstream),
        Some("abc"),
        Some("abc"),
        None,
        true,
        None,
    );
    assert_eq!(decision.status, UpdateStatus::PolicyBlocked);
}

#[test]
fn returns_check_failed_on_remote_error() {
    let decision = decide_update_status(
        "git",
        Some(&SourceKind::VerifiedUpstream),
        Some("abc123"),
        None,
        None,
        false,
        Some("network unavailable"),
    );
    assert_eq!(decision.status, UpdateStatus::CheckFailed);
}

#[test]
fn returns_legacy_untracked_for_custom_no_source() {
    let decision = decide_update_status(
        "manual",
        Some(&SourceKind::CustomNoSource),
        None,
        None,
        Some(true),
        false,
        None,
    );
    assert_eq!(decision.status, UpdateStatus::LegacyUntracked);
    assert_eq!(decision.reason.as_deref(), Some("custom skill has no confirmed upstream"));
}
