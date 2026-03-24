use std::path::Path;

const RESERVED_NAMES: [&str; 3] = [".system", "_archives", "_backups"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allowed,
    Blocked { code: String, reason: String },
}

pub fn is_reserved_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }

    name.starts_with('.')
        || name.starts_with('_')
        || name.contains("_backup_")
        || RESERVED_NAMES.contains(&name)
}

pub fn should_skip_directory(name: &str) -> bool {
    is_reserved_name(name)
}

pub fn is_system_reserved(tool_key: &str, skill_name: &str, path: &Path) -> bool {
    if tool_key != "codex" || skill_name.is_empty() {
        return false;
    }

    let path_string = path.to_string_lossy();
    let path_norm = path_string.replace('\\', "/").to_lowercase();
    let system_anchor = "/.codex/skills/.system/";
    let ends_with_skill = path_norm.ends_with(&format!("/{}", skill_name.to_lowercase()));
    path_norm.contains(system_anchor) && ends_with_skill
}

pub fn classify_policy_block(tool_key: &str, skill_name: &str, path: &Path) -> PolicyDecision {
    if should_skip_directory(skill_name) {
        return PolicyDecision::Blocked {
            code: "reserved-name".to_string(),
            reason: format!("'{}' is a reserved directory name", skill_name),
        };
    }

    if is_system_reserved(tool_key, skill_name, path) {
        return PolicyDecision::Blocked {
            code: "system-reserved".to_string(),
            reason: format!(
                "'{}' is reserved by {} system skill layer",
                skill_name, tool_key
            ),
        };
    }

    PolicyDecision::Allowed
}

