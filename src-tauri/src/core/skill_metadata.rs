use std::path::Path;

pub struct SkillMeta {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub fn parse_skill_md(dir: &Path) -> SkillMeta {
    let candidates = ["SKILL.md", "skill.md", "CLAUDE.md"];
    for candidate in &candidates {
        let path = dir.join(candidate);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return parse_frontmatter(&content);
            }
        }
    }
    SkillMeta {
        name: None,
        description: None,
    }
}

fn parse_frontmatter(content: &str) -> SkillMeta {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return SkillMeta {
            name: None,
            description: None,
        };
    }

    let rest = &trimmed[3..];
    if let Some(end) = rest.find("---") {
        let yaml_str = &rest[..end];
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            let name = yaml
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let description = yaml
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            return SkillMeta { name, description };
        }
    }

    SkillMeta {
        name: None,
        description: None,
    }
}

const SKILL_DIR_MARKERS: &[&str] = &[
    "SKILL.md",
    "skill.md",
    "CLAUDE.md",
    "README.md",
    "readme.md",
];

pub fn is_valid_skill_dir(dir: &Path) -> bool {
    dir.is_dir() && SKILL_DIR_MARKERS.iter().any(|name| dir.join(name).exists())
}

const WINDOWS_RESERVED: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const WINDOWS_RESERVED_BASENAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn sanitize_skill_name(name: &str) -> Option<String> {
    let last = std::path::Path::new(name)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())?;

    if last == ".." || last == "." {
        return None;
    }

    let clean: String = last
        .chars()
        .map(|c| {
            if c.is_control() || WINDOWS_RESERVED.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();

    let trimmed = clean.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        None
    } else {
        let reserved = trimmed
            .split('.')
            .next()
            .map(|base| base.to_ascii_uppercase())
            .map(|upper| WINDOWS_RESERVED_BASENAMES.contains(&upper.as_str()))
            .unwrap_or(false);

        if reserved {
            Some(format!("_{}", trimmed))
        } else {
            Some(trimmed.to_string())
        }
    }
}

pub fn infer_skill_name(dir: &Path) -> String {
    let meta = parse_skill_md(dir);
    if let Some(name) = meta.name {
        if let Some(sanitized) = sanitize_skill_name(&name) {
            return sanitized;
        }
    }
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown-skill".to_string())
}
