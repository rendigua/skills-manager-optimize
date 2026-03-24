use anyhow::{Context, Result};
use std::path::Path;

use crate::core::policy_engine::{classify_policy_block, PolicyDecision};

#[derive(Debug, Clone, Copy)]
pub enum SyncMode {
    Symlink,
    Copy,
}

impl SyncMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncMode::Symlink => "symlink",
            SyncMode::Copy => "copy",
        }
    }

}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub mode: SyncMode,
    pub status: String,
    pub reason: Option<String>,
}

pub fn sync_mode_for_tool(tool_key: &str, configured_mode: Option<&str>) -> SyncMode {
    match configured_mode {
        Some("copy") => SyncMode::Copy,
        Some("symlink") => SyncMode::Symlink,
        _ => match tool_key {
            "cursor" => SyncMode::Copy,
            _ => SyncMode::Symlink,
        },
    }
}

pub fn sync_skill(source: &Path, target: &Path, mode: SyncMode) -> Result<SyncMode> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir {:?}", parent))?;
    }

    // Remove existing target
    remove_target(target).ok();

    match mode {
        SyncMode::Symlink => {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(source, target)
                    .with_context(|| format!("Failed to create symlink {:?} -> {:?}", target, source))?;
                Ok(SyncMode::Symlink)
            }
            #[cfg(not(unix))]
            {
                copy_dir_recursive(source, target)?;
                Ok(SyncMode::Copy)
            }
        }
        SyncMode::Copy => {
            copy_dir_recursive(source, target)?;
            Ok(SyncMode::Copy)
        }
    }
}

pub fn sync_skill_with_policy(
    source: &Path,
    target: &Path,
    mode: SyncMode,
    tool_key: &str,
    skill_name: &str,
    dry_run: bool,
) -> Result<SyncResult> {
    match classify_policy_block(tool_key, skill_name, target) {
        PolicyDecision::Allowed => {}
        PolicyDecision::Blocked { reason, .. } => {
            return Ok(SyncResult {
                mode,
                status: "policy-blocked".to_string(),
                reason: Some(reason),
            });
        }
    }

    if dry_run {
        return Ok(SyncResult {
            mode,
            status: "dry-run".to_string(),
            reason: Some("Dry-run enabled; no filesystem writes were made.".to_string()),
        });
    }

    let actual_mode = sync_skill(source, target, mode)?;
    Ok(SyncResult {
        mode: actual_mode,
        status: "ok".to_string(),
        reason: None,
    })
}

pub fn remove_target(target: &Path) -> Result<()> {
    if target.is_symlink() {
        std::fs::remove_file(target)?;
    } else if target.is_dir() {
        std::fs::remove_dir_all(target)?;
    } else if target.exists() {
        std::fs::remove_file(target)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ft.is_dir() {
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
