use std::collections::HashMap;
use std::fs;

use app_lib::core::migration_scanner::{
    classify_runtime_placement, scan_runtime_snapshot, MigrationClass, PlacementKind,
};
use tempfile::tempdir;

#[test]
fn classify_runtime_placement_distinguishes_system_shared_and_local() {
    assert_eq!(
        classify_runtime_placement(".system", false, true),
        PlacementKind::CodexSystem
    );
    assert_eq!(
        classify_runtime_placement("figma", true, true),
        PlacementKind::SharedJunction
    );
    assert_eq!(
        classify_runtime_placement("broken-skill", true, false),
        PlacementKind::BrokenLink
    );
    assert_eq!(
        classify_runtime_placement("custom-skill", false, true),
        PlacementKind::CustomLocal
    );
}

#[test]
fn scan_runtime_reports_placement_kind_and_source_kind() {
    let root = tempdir().expect("temp dir");

    let tracked = root.path().join("tracked-skill");
    fs::create_dir_all(&tracked).expect("tracked dir");
    fs::write(
        tracked.join("origin.json"),
        r#"{
  "schema_version": 1,
  "source_type": "git",
  "source_kind": "verified-upstream",
  "source_ref": "https://github.com/example/repo"
}"#,
    )
    .expect("tracked origin");

    let local = root.path().join("local-skill");
    fs::create_dir_all(&local).expect("local dir");
    fs::write(local.join("SKILL.md"), "# local").expect("local skill");

    fs::create_dir_all(root.path().join(".system")).expect("system");

    let result = scan_runtime_snapshot(root.path()).expect("scan runtime");
    let by_name: HashMap<String, (MigrationClass, Option<String>, Option<PlacementKind>)> = result
        .entries
        .into_iter()
        .map(|entry| (entry.name, (entry.class, entry.source_kind, entry.placement_kind)))
        .collect();

    assert_eq!(
        by_name.get("tracked-skill"),
        Some(&(
            MigrationClass::Tracked,
            Some("verified-upstream".to_string()),
            Some(PlacementKind::CustomLocal)
        ))
    );
    assert_eq!(
        by_name.get("local-skill"),
        Some(&(
            MigrationClass::LegacyUntracked,
            None,
            Some(PlacementKind::CustomLocal)
        ))
    );
    assert_eq!(
        by_name.get(".system"),
        Some(&(
            MigrationClass::PolicyBlocked,
            Some("system-reserved".to_string()),
            Some(PlacementKind::CodexSystem)
        ))
    );
}
