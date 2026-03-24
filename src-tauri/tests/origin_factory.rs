use app_lib::core::origin_factory::{build_generated_origin_metadata, build_install_origin_metadata};
use app_lib::core::origin_metadata::{ConfidenceLevel, SourceKind, SourceType};

#[test]
fn builds_verified_upstream_origin_for_git_install() {
    let metadata = build_install_origin_metadata(
        "git",
        Some("https://github.com/example/repo/tree/main/skills/sample".to_string()),
        Some("https://github.com/example/repo.git".to_string()),
        Some("skills/sample".to_string()),
        Some("main".to_string()),
        Some("abc123".to_string()),
    );

    assert_eq!(metadata.source_type, SourceType::Git);
    assert_eq!(metadata.source_kind, Some(SourceKind::VerifiedUpstream));
    assert_eq!(metadata.confidence, Some(ConfidenceLevel::High));
    assert_eq!(metadata.source_ref_resolved.as_deref(), Some("https://github.com/example/repo.git"));
    assert_eq!(metadata.source_subpath.as_deref(), Some("skills/sample"));
}

#[test]
fn builds_verified_distribution_origin_for_skillssh_install() {
    let metadata = build_install_origin_metadata(
        "skillssh",
        Some("owner/repo/sample-skill".to_string()),
        Some("https://github.com/owner/repo.git".to_string()),
        Some("skills/sample-skill".to_string()),
        None,
        Some("def456".to_string()),
    );

    assert_eq!(metadata.source_type, SourceType::Marketplace);
    assert_eq!(metadata.source_kind, Some(SourceKind::VerifiedDistribution));
    assert_eq!(metadata.confidence, Some(ConfidenceLevel::High));
    assert_eq!(
        metadata.distribution_ref.as_deref(),
        Some("https://skills.sh/owner/repo/sample-skill")
    );
    assert_eq!(metadata.source_ref_resolved.as_deref(), Some("https://github.com/owner/repo.git"));
}

#[test]
fn builds_custom_no_source_origin_for_local_install() {
    let metadata = build_install_origin_metadata(
        "local",
        Some("C:\\skills\\local-skill".to_string()),
        None,
        None,
        None,
        None,
    );

    assert_eq!(metadata.source_type, SourceType::Local);
    assert_eq!(metadata.source_kind, Some(SourceKind::CustomNoSource));
    assert_eq!(metadata.confidence, Some(ConfidenceLevel::Low));
    assert_eq!(metadata.resolution_method.as_deref(), Some("local-import-default"));
}

#[test]
fn builds_generated_origin_for_agent_created_skill() {
    let metadata = build_generated_origin_metadata(
        "codex".to_string(),
        Some("session-derived".to_string()),
        Some("Built from recent work summary".to_string()),
    );

    assert_eq!(metadata.source_type, SourceType::Manual);
    assert_eq!(metadata.source_kind, Some(SourceKind::CustomNoSource));
    assert_eq!(metadata.installed_via.as_deref(), Some("agent-generated"));
    assert_eq!(metadata.created_by.as_deref(), Some("codex"));
    assert_eq!(metadata.creation_mode.as_deref(), Some("session-derived"));
}
