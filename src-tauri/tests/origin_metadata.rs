use std::fs;

use tempfile::tempdir;

use app_lib::core::origin_metadata::{
    load_origin_metadata, ConfidenceLevel, SourceKind, SourceType,
};

#[test]
fn parses_git_origin_metadata() {
    let dir = tempdir().expect("temp dir");
    let origin_path = dir.path().join("origin.json");
    fs::write(
        &origin_path,
        r#"{
  "schema_version": 1,
  "source_type": "git",
  "source_ref": "https://github.com/vercel-labs/agent-browser",
  "source_subpath": "skills/agent-browser",
  "source_branch": "main",
  "source_revision": "a865dd56e0053a894a83e0569191985232000f26",
  "installed_via": "git-subpath-import",
  "upstream_project": "vercel-labs/agent-browser",
  "imported_at": "2026-03-18T00:00:00+08:00",
  "source_kind": "verified-upstream",
  "distribution_ref": "https://skills.sh/vercel-labs/agent-browser",
  "evidence_refs": [
    "https://github.com/vercel-labs/agent-browser",
    "https://skills.sh/vercel-labs/agent-browser"
  ],
  "confidence": "high",
  "resolution_method": "github-repo-confirmed"
}"#,
    )
    .expect("write origin");

    let metadata = load_origin_metadata(dir.path()).expect("load metadata");
    assert_eq!(metadata.source_type, SourceType::Git);
    assert_eq!(
        metadata.source_kind,
        Some(SourceKind::VerifiedUpstream)
    );
    assert_eq!(metadata.confidence, Some(ConfidenceLevel::High));
    assert_eq!(
        metadata.distribution_ref.as_deref(),
        Some("https://skills.sh/vercel-labs/agent-browser")
    );
    assert_eq!(metadata.evidence_refs.len(), 2);
    assert_eq!(
        metadata.source_subpath.as_deref(),
        Some("skills/agent-browser")
    );
}

#[test]
fn falls_back_to_legacy_untracked_when_origin_is_missing() {
    let dir = tempdir().expect("temp dir");
    let metadata = load_origin_metadata(dir.path()).expect("load metadata");
    assert_eq!(metadata.source_type, SourceType::LegacyUntracked);
    assert!(metadata.source_ref.is_none());
    assert!(metadata.source_kind.is_none());
    assert!(metadata.evidence_refs.is_empty());
}

#[test]
fn parses_custom_no_source_origin_metadata() {
    let dir = tempdir().expect("temp dir");
    let origin_path = dir.path().join("origin.json");
    fs::write(
        &origin_path,
        r#"{
  "schema_version": 1,
  "source_type": "manual",
  "source_kind": "custom-no-source",
  "installed_via": "manual-backfill",
  "notes": "No reliable upstream found after GitHub and marketplace lookup"
}"#,
    )
    .expect("write origin");

    let metadata = load_origin_metadata(dir.path()).expect("load metadata");
    assert_eq!(metadata.source_type, SourceType::Manual);
    assert_eq!(metadata.source_kind, Some(SourceKind::CustomNoSource));
    assert_eq!(metadata.installed_via.as_deref(), Some("manual-backfill"));
}

#[test]
fn rejects_unsupported_schema_versions() {
    let dir = tempdir().expect("temp dir");
    let origin_path = dir.path().join("origin.json");
    fs::write(
        &origin_path,
        r#"{
  "schema_version": 99,
  "source_type": "git"
}"#,
    )
    .expect("write origin");

    let result = load_origin_metadata(dir.path());
    assert!(result.is_err());
}
