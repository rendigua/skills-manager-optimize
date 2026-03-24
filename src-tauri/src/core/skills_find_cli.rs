use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillsFindCandidate {
    pub owner: String,
    pub repo: String,
    pub skill_id: String,
    pub installs: u64,
    pub distribution_ref: String,
}

static SKILLS_FIND_CACHE: OnceLock<Mutex<HashMap<String, Vec<SkillsFindCandidate>>>> = OnceLock::new();

pub fn expand_search_queries(query: &str) -> Vec<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let normalized = normalize_token(trimmed);
    let mut variants = vec![trimmed.to_string()];

    if should_expand_with_context(&normalized) {
        let contextual = format!("react {}", trimmed);
        if !variants.iter().any(|variant| normalize_token(variant) == normalize_token(&contextual)) {
            variants.push(contextual);
        }
    }

    variants
}

pub fn search_skills_find(query: &str) -> Result<Vec<SkillsFindCandidate>> {
    let mut merged = Vec::new();

    for variant in expand_search_queries(query) {
        if let Some(cached) = cached_skills_find_result(&variant) {
            merged.extend(cached);
            continue;
        }

        let output = Command::new("npx")
            .args(["skills", "find", &variant])
            .output()
            .context("Failed to run npx skills find")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "npx skills find exited with status {}",
                output.status
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_skills_find_output(&stdout);
        cache_skills_find_result(&variant, parsed.clone());
        merged.extend(parsed);
    }

    Ok(deduplicate_skills_find_candidates(merged))
}

pub fn parse_skills_find_output(output: &str) -> Vec<SkillsFindCandidate> {
    let ansi = Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").expect("ANSI regex");
    let cleaned = ansi.replace_all(output, "");
    let mut candidates = Vec::new();
    let mut current: Option<(String, String, String, u64)> = None;

    let skill_line = Regex::new(
        r"^(?P<owner>[^/\s@]+)/(?P<repo>[^@\s]+)@(?P<skill>[^\s]+)\s+(?P<installs>[\d.,]+[KMB]?)\s+installs$",
    )
    .expect("skill line regex");
    let url_line = Regex::new(r"^└\s+(?P<url>https://skills\.sh/[^\s]+)$").expect("url regex");

    for raw_line in cleaned.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("████") || line.starts_with("Install with") {
            continue;
        }

        if let Some(caps) = skill_line.captures(line) {
            let installs = parse_installs(caps.name("installs").map(|value| value.as_str()).unwrap_or("0"));
            current = Some((
                caps.name("owner").map(|value| value.as_str().to_string()).unwrap_or_default(),
                caps.name("repo").map(|value| value.as_str().to_string()).unwrap_or_default(),
                caps.name("skill").map(|value| value.as_str().to_string()).unwrap_or_default(),
                installs,
            ));
            continue;
        }

        if let Some(caps) = url_line.captures(line) {
            if let Some((owner, repo, skill_id, installs)) = current.take() {
                candidates.push(SkillsFindCandidate {
                    owner,
                    repo,
                    skill_id,
                    installs,
                    distribution_ref: caps
                        .name("url")
                        .map(|value| value.as_str().to_string())
                        .unwrap_or_default(),
                });
            }
        }
    }

    candidates
}

fn parse_installs(raw: &str) -> u64 {
    let trimmed = raw.trim().to_uppercase();
    let (value, multiplier) = if let Some(value) = trimmed.strip_suffix('K') {
        (value, 1_000.0)
    } else if let Some(value) = trimmed.strip_suffix('M') {
        (value, 1_000_000.0)
    } else if let Some(value) = trimmed.strip_suffix('B') {
        (value, 1_000_000_000.0)
    } else {
        (trimmed.as_str(), 1.0)
    };

    value
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .map(|v| (v * multiplier).round() as u64)
        .unwrap_or(0)
}

fn should_expand_with_context(normalized_query: &str) -> bool {
    matches!(normalized_query, "flow" | "fix") || normalized_query.len() <= 5
}

fn normalize_token(value: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;

    for ch in value.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_dash = false;
        } else if !last_dash {
            output.push('-');
            last_dash = true;
        }
    }

    output.trim_matches('-').to_string()
}

fn cached_skills_find_result(query: &str) -> Option<Vec<SkillsFindCandidate>> {
    let cache = SKILLS_FIND_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    cache.lock().ok().and_then(|guard| guard.get(query).cloned())
}

fn cache_skills_find_result(query: &str, result: Vec<SkillsFindCandidate>) {
    let cache = SKILLS_FIND_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(query.to_string(), result);
    }
}

fn deduplicate_skills_find_candidates(candidates: Vec<SkillsFindCandidate>) -> Vec<SkillsFindCandidate> {
    let mut seen = HashMap::new();
    let mut deduped = Vec::new();

    for candidate in candidates {
        let key = candidate.distribution_ref.clone();
        if seen.insert(key.clone(), ()).is_none() {
            deduped.push(candidate);
        }
    }

    deduped
}

#[cfg(test)]
mod tests {
    use super::{expand_search_queries, parse_skills_find_output};

    #[test]
    fn parses_skills_find_output() {
        let output = r#"
███████╗██╗  ██╗██╗██╗     ██╗     ███████╗
Install with npx skills add <owner/repo@skill>
openai/skills@imagegen 484 installs
└ https://skills.sh/openai/skills/imagegen
facebook/react@fix 1.2K installs
└ https://skills.sh/facebook/react/fix
"#;

        let parsed = parse_skills_find_output(output);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].owner, "openai");
        assert_eq!(parsed[0].repo, "skills");
        assert_eq!(parsed[0].skill_id, "imagegen");
        assert_eq!(parsed[0].installs, 484);
        assert_eq!(
            parsed[1].distribution_ref,
            "https://skills.sh/facebook/react/fix"
        );
        assert_eq!(parsed[1].installs, 1200);
    }

    #[test]
    fn expands_short_generic_queries() {
        let variants = expand_search_queries("flow");
        assert_eq!(variants[0], "flow");
        assert!(variants.iter().any(|variant| variant == "react flow"));
    }
}
