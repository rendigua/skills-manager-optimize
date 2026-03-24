use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const ORIGIN_FILENAME: &str = "origin.json";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    Git,
    Marketplace,
    Local,
    Manual,
    Archive,
    LegacyUntracked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    VerifiedUpstream,
    VerifiedDistribution,
    InferredUpstream,
    CustomNoSource,
    ReplacedEquivalent,
    SystemReserved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginMetadata {
    pub schema_version: u32,
    pub source_type: SourceType,
    pub source_ref: Option<String>,
    pub source_ref_resolved: Option<String>,
    pub source_subpath: Option<String>,
    pub source_branch: Option<String>,
    pub source_revision: Option<String>,
    pub installed_via: Option<String>,
    pub upstream_project: Option<String>,
    pub imported_at: Option<String>,
    pub created_by: Option<String>,
    pub creation_mode: Option<String>,
    pub derivation_summary: Option<String>,
    pub source_kind: Option<SourceKind>,
    pub distribution_ref: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub confidence: Option<ConfidenceLevel>,
    pub resolution_method: Option<String>,
    pub replacement_ref: Option<String>,
    pub replacement_reason: Option<String>,
    pub notes: Option<String>,
}

impl OriginMetadata {
    pub fn legacy_untracked() -> Self {
        Self {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            source_type: SourceType::LegacyUntracked,
            source_ref: None,
            source_ref_resolved: None,
            source_subpath: None,
            source_branch: None,
            source_revision: None,
            installed_via: None,
            upstream_project: None,
            imported_at: None,
            created_by: None,
            creation_mode: None,
            derivation_summary: None,
            source_kind: None,
            distribution_ref: None,
            evidence_refs: Vec::new(),
            confidence: None,
            resolution_method: None,
            replacement_ref: None,
            replacement_reason: None,
            notes: None,
        }
    }
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::VerifiedUpstream => "verified-upstream",
            SourceKind::VerifiedDistribution => "verified-distribution",
            SourceKind::InferredUpstream => "inferred-upstream",
            SourceKind::CustomNoSource => "custom-no-source",
            SourceKind::ReplacedEquivalent => "replaced-equivalent",
            SourceKind::SystemReserved => "system-reserved",
        }
    }
}

pub fn load_origin_metadata(skill_dir: &Path) -> Result<OriginMetadata> {
    let origin_path = origin_path(skill_dir);
    if !origin_path.exists() {
        return Ok(OriginMetadata::legacy_untracked());
    }

    let raw = fs::read_to_string(&origin_path)
        .with_context(|| format!("Failed to read {}", origin_path.display()))?;
    let metadata: OriginMetadata = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse {}", origin_path.display()))?;

    if metadata.schema_version != SUPPORTED_SCHEMA_VERSION {
        bail!(
            "Unsupported origin schema version {} in {}",
            metadata.schema_version,
            origin_path.display()
        );
    }

    Ok(metadata)
}

pub fn save_origin_metadata(skill_dir: &Path, metadata: &OriginMetadata) -> Result<()> {
    let origin_path = origin_path(skill_dir);
    let json = serde_json::to_string_pretty(metadata)
        .with_context(|| format!("Failed to serialize {}", origin_path.display()))?;
    fs::write(&origin_path, format!("{json}\n"))
        .with_context(|| format!("Failed to write {}", origin_path.display()))?;
    Ok(())
}

pub fn origin_path(skill_dir: &Path) -> PathBuf {
    skill_dir.join(ORIGIN_FILENAME)
}
