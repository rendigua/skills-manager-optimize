use crate::core::origin_metadata::SourceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable,
    ReimportAvailable,
    LegacyUntracked,
    PolicyBlocked,
    CheckFailed,
}

impl UpdateStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateStatus::UpToDate => "up-to-date",
            UpdateStatus::UpdateAvailable => "update-available",
            UpdateStatus::ReimportAvailable => "reimport-available",
            UpdateStatus::LegacyUntracked => "legacy-untracked",
            UpdateStatus::PolicyBlocked => "policy-blocked",
            UpdateStatus::CheckFailed => "check-failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDecision {
    pub status: UpdateStatus,
    pub reason: Option<String>,
}

pub fn decide_update_status(
    source_type: &str,
    source_kind: Option<&SourceKind>,
    source_revision: Option<&str>,
    remote_revision: Option<&str>,
    source_exists: Option<bool>,
    policy_blocked: bool,
    check_error: Option<&str>,
) -> UpdateDecision {
    if matches!(source_kind, Some(SourceKind::SystemReserved)) {
        return UpdateDecision {
            status: UpdateStatus::PolicyBlocked,
            reason: Some("reserved by system skill layer".to_string()),
        };
    }

    if matches!(source_kind, Some(SourceKind::CustomNoSource)) {
        return UpdateDecision {
            status: UpdateStatus::LegacyUntracked,
            reason: Some("custom skill has no confirmed upstream".to_string()),
        };
    }

    if policy_blocked {
        return UpdateDecision {
            status: UpdateStatus::PolicyBlocked,
            reason: Some("blocked by policy layer".to_string()),
        };
    }

    if let Some(err) = check_error {
        return UpdateDecision {
            status: UpdateStatus::CheckFailed,
            reason: Some(err.to_string()),
        };
    }

    match source_type {
        "git" | "skillssh" => match (source_revision, remote_revision) {
            (Some(local), Some(remote)) if local == remote => UpdateDecision {
                status: UpdateStatus::UpToDate,
                reason: None,
            },
            (Some(_), Some(_)) => UpdateDecision {
                status: UpdateStatus::UpdateAvailable,
                reason: None,
            },
            (_, Some(_)) => UpdateDecision {
                status: UpdateStatus::ReimportAvailable,
                reason: Some("remote revision found but local revision missing".to_string()),
            },
            _ => UpdateDecision {
                status: UpdateStatus::CheckFailed,
                reason: Some("missing remote revision".to_string()),
            },
        },
        "local" | "import" | "manual" => match source_exists {
            Some(true) => UpdateDecision {
                status: UpdateStatus::ReimportAvailable,
                reason: None,
            },
            Some(false) => UpdateDecision {
                status: UpdateStatus::LegacyUntracked,
                reason: Some("source path missing".to_string()),
            },
            None => UpdateDecision {
                status: UpdateStatus::LegacyUntracked,
                reason: Some("source path unknown".to_string()),
            },
        },
        _ => UpdateDecision {
            status: UpdateStatus::LegacyUntracked,
            reason: Some("unsupported source type".to_string()),
        },
    }
}
