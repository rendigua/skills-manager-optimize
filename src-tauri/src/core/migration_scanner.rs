use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::core::origin_metadata::load_origin_metadata;
use crate::core::policy_engine::should_skip_directory;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationClass {
    Tracked,
    PartiallyTracked,
    LegacyUntracked,
    PolicyBlocked,
    Archived,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementKind {
    CodexSystem,
    SharedJunction,
    CustomLocal,
    BrokenLink,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationEntry {
    pub name: String,
    pub path: String,
    pub class: MigrationClass,
    pub source_kind: Option<String>,
    pub placement_kind: Option<PlacementKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationScanResult {
    pub root: String,
    pub entries: Vec<MigrationEntry>,
}

pub fn scan_runtime_snapshot(root: &Path) -> Result<MigrationScanResult> {
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(MigrationScanResult {
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
        let (class, source_kind, placement_kind) = classify_runtime_skill(&name, &path)?;
        entries.push(MigrationEntry {
            name,
            path: path.to_string_lossy().to_string(),
            class,
            source_kind,
            placement_kind,
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(MigrationScanResult {
        root: root.to_string_lossy().to_string(),
        entries,
    })
}

pub fn classify_runtime_placement(name: &str, is_symlink: bool, target_exists: bool) -> PlacementKind {
    if name == ".system" {
        PlacementKind::CodexSystem
    } else if is_symlink && !target_exists {
        PlacementKind::BrokenLink
    } else if is_symlink {
        PlacementKind::SharedJunction
    } else {
        PlacementKind::CustomLocal
    }
}

fn classify_runtime_skill(
    name: &str,
    path: &PathBuf,
) -> Result<(MigrationClass, Option<String>, Option<PlacementKind>)> {
    let placement_kind = Some(classify_runtime_placement(name, path.is_symlink(), path.exists()));

    if name == "_archives" || name == "_backups" {
        return Ok((MigrationClass::Archived, None, placement_kind));
    }

    if name == ".system" {
        return Ok((
            MigrationClass::PolicyBlocked,
            Some("system-reserved".to_string()),
            placement_kind,
        ));
    }

    if placement_kind == Some(PlacementKind::BrokenLink) {
        return Ok((
            MigrationClass::PolicyBlocked,
            Some("broken-link".to_string()),
            placement_kind,
        ));
    }

    if should_skip_directory(name) {
        return Ok((MigrationClass::PolicyBlocked, None, placement_kind));
    }

    let origin_path = path.join("origin.json");
    if origin_path.exists() {
        let metadata = load_origin_metadata(path)?;
        return Ok((
            MigrationClass::Tracked,
            metadata.source_kind.map(|kind| kind.as_str().to_string()),
            placement_kind,
        ));
    }

    let skill_md = path.join("SKILL.md");
    if skill_md.exists() {
        return Ok((MigrationClass::LegacyUntracked, None, placement_kind));
    }

    let has_any_files = std::fs::read_dir(path)?.next().transpose()?.is_some();
    if has_any_files {
        return Ok((MigrationClass::PartiallyTracked, None, placement_kind));
    }

    Ok((MigrationClass::LegacyUntracked, None, placement_kind))
}
