use chrono::Utc;

use crate::core::origin_metadata::{ConfidenceLevel, OriginMetadata, SourceKind, SourceType};

pub fn build_install_origin_metadata(
    source_type: &str,
    source_ref: Option<String>,
    source_ref_resolved: Option<String>,
    source_subpath: Option<String>,
    source_branch: Option<String>,
    source_revision: Option<String>,
) -> OriginMetadata {
    match source_type {
        "git" => {
            let upstream_project =
                infer_upstream_project(source_ref.as_deref(), source_ref_resolved.as_deref());
            let evidence_refs =
                collect_evidence_refs(source_ref.as_deref(), source_ref_resolved.as_deref(), None);
            OriginMetadata {
            schema_version: 1,
            source_type: SourceType::Git,
            source_ref: source_ref.clone(),
            source_ref_resolved,
            source_subpath,
            source_branch,
            source_revision,
            installed_via: Some("git-install".to_string()),
            upstream_project,
            imported_at: Some(Utc::now().to_rfc3339()),
            created_by: None,
            creation_mode: None,
            derivation_summary: None,
            source_kind: Some(SourceKind::VerifiedUpstream),
            distribution_ref: None,
            evidence_refs,
            confidence: Some(ConfidenceLevel::High),
            resolution_method: Some("install-command-git".to_string()),
            replacement_ref: None,
            replacement_reason: None,
            notes: Some("Captured automatically during Git install".to_string()),
        }
        }
        "skillssh" => {
            let distribution_ref = source_ref
                .as_deref()
                .map(|value| format!("https://skills.sh/{value}"));
            let evidence_refs = collect_evidence_refs(
                source_ref.as_deref(),
                source_ref_resolved.as_deref(),
                distribution_ref.as_deref(),
            );
            OriginMetadata {
                schema_version: 1,
                source_type: SourceType::Marketplace,
                source_ref: source_ref.clone(),
                source_ref_resolved,
                source_subpath,
                source_branch,
                source_revision,
                installed_via: Some("skillssh-install".to_string()),
                upstream_project: infer_upstream_project(source_ref.as_deref(), None),
                imported_at: Some(Utc::now().to_rfc3339()),
                created_by: None,
                creation_mode: None,
                derivation_summary: None,
                source_kind: Some(SourceKind::VerifiedDistribution),
                distribution_ref: distribution_ref.clone(),
                evidence_refs,
                confidence: Some(ConfidenceLevel::High),
                resolution_method: Some("install-command-skillssh".to_string()),
                replacement_ref: None,
                replacement_reason: None,
                notes: Some("Captured automatically during marketplace install".to_string()),
            }
        }
        "local" | "import" => OriginMetadata {
            schema_version: 1,
            source_type: SourceType::Local,
            source_ref,
            source_ref_resolved,
            source_subpath,
            source_branch,
            source_revision,
            installed_via: Some("local-import".to_string()),
            upstream_project: None,
            imported_at: Some(Utc::now().to_rfc3339()),
            created_by: None,
            creation_mode: None,
            derivation_summary: None,
            source_kind: Some(SourceKind::CustomNoSource),
            distribution_ref: None,
            evidence_refs: Vec::new(),
            confidence: Some(ConfidenceLevel::Low),
            resolution_method: Some("local-import-default".to_string()),
            replacement_ref: None,
            replacement_reason: None,
            notes: Some("Imported from local path without a confirmed upstream".to_string()),
        },
        _ => OriginMetadata::legacy_untracked(),
    }
}

pub fn build_generated_origin_metadata(
    created_by: String,
    creation_mode: Option<String>,
    derivation_summary: Option<String>,
) -> OriginMetadata {
    OriginMetadata {
        schema_version: 1,
        source_type: SourceType::Manual,
        source_ref: None,
        source_ref_resolved: None,
        source_subpath: None,
        source_branch: None,
        source_revision: None,
        installed_via: Some("agent-generated".to_string()),
        upstream_project: None,
        imported_at: Some(Utc::now().to_rfc3339()),
        created_by: Some(created_by),
        creation_mode,
        derivation_summary,
        source_kind: Some(SourceKind::CustomNoSource),
        distribution_ref: None,
        evidence_refs: Vec::new(),
        confidence: Some(ConfidenceLevel::Low),
        resolution_method: Some("agent-generated-default".to_string()),
        replacement_ref: None,
        replacement_reason: None,
        notes: Some("Generated locally by an agent; no upstream is assumed".to_string()),
    }
}

fn infer_upstream_project(source_ref: Option<&str>, source_ref_resolved: Option<&str>) -> Option<String> {
    source_ref
        .and_then(parse_owner_repo)
        .or_else(|| source_ref_resolved.and_then(parse_owner_repo))
}

fn parse_owner_repo(value: &str) -> Option<String> {
    if value.contains("github.com/") {
        let marker = value.split("github.com/").nth(1)?;
        let trimmed = marker.trim_end_matches(".git");
        let parts: Vec<&str> = trimmed.split('/').collect();
        if parts.len() >= 2 {
            return Some(format!("{}/{}", parts[0], parts[1]));
        }
    }

    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() >= 2 {
        return Some(format!("{}/{}", parts[0], parts[1]));
    }

    None
}

fn collect_evidence_refs(
    source_ref: Option<&str>,
    source_ref_resolved: Option<&str>,
    distribution_ref: Option<&str>,
) -> Vec<String> {
    let mut refs = Vec::new();
    for value in [source_ref, source_ref_resolved, distribution_ref].into_iter().flatten() {
        if !refs.iter().any(|existing| existing == value) {
            refs.push(value.to_string());
        }
    }
    refs
}
