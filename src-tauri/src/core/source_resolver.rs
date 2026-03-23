use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::skills_find_cli::{expand_search_queries, search_skills_find};
use crate::core::skillssh_api::SkillsShSkill;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCandidate {
    pub source_kind: String,
    pub source_ref: String,
    pub source_ref_resolved: String,
    pub source_subpath: Option<String>,
    pub source_branch: Option<String>,
    pub distribution_ref: Option<String>,
    pub title: String,
    pub skill_id: String,
    pub installs: u64,
    pub confidence: String,
    pub evidence_refs: Vec<String>,
    pub install_command: Option<String>,
}

pub fn normalize_skillssh_candidate(skill: &SkillsShSkill) -> SourceCandidate {
    let repo_url = format!("https://github.com/{}.git", skill.source);
    let distribution_ref = format!("https://skills.sh/{}/{}", skill.source, skill.skill_id);
    SourceCandidate {
        source_kind: "verified-distribution".to_string(),
        source_ref: skill.id.clone(),
        source_ref_resolved: repo_url,
        source_subpath: Some(skill.skill_id.clone()),
        source_branch: None,
        distribution_ref: Some(distribution_ref.clone()),
        title: skill.name.clone(),
        skill_id: skill.skill_id.clone(),
        installs: skill.installs,
        confidence: "high".to_string(),
        evidence_refs: provider_evidence_refs("skills.sh-api", vec![distribution_ref.clone()]),
        install_command: Some(format!(
            "npx skills add https://github.com/{} --skill {}",
            skill.source, skill.skill_id
        )),
    }
}

pub fn normalize_skillsmp_candidate(entry: &Value) -> Option<SourceCandidate> {
    let repository = first_string(entry, &["repository", "repo", "github_repository", "githubRepo", "source_repo"])?;
    let skill_id = first_string(entry, &["skill_id", "skillId", "slug", "id"])?;
    let title = first_string(entry, &["name", "title"]).unwrap_or_else(|| skill_id.clone());
    let installs = first_u64(entry, &["installs", "stars", "downloads"]).unwrap_or(0);
    let distribution_ref = first_string(entry, &["url", "permalink", "href", "link"]);
    let source_ref = format!("{}/{}", repository.trim_matches('/'), skill_id.trim_matches('/'));
    let source_ref_resolved = format!("https://github.com/{}.git", repository.trim_matches('/'));
    let evidence_refs = distribution_ref
        .as_ref()
        .map(|url| provider_evidence_refs("skillsmp-api", vec![url.clone()]))
        .unwrap_or_else(|| provider_evidence_refs("skillsmp-api", vec![source_ref_resolved.clone()]));

    Some(SourceCandidate {
        source_kind: "verified-distribution".to_string(),
        source_ref,
        source_ref_resolved,
        source_subpath: Some(skill_id.clone()),
        source_branch: None,
        distribution_ref,
        title,
        skill_id: skill_id.clone(),
        installs,
        confidence: "high".to_string(),
        evidence_refs,
        install_command: Some(format!(
            "npx skills add https://github.com/{} --skill {}",
            repository.trim_matches('/'),
            skill_id
        )),
    })
}

pub fn resolve_source_candidates(
    query: &str,
    limit: usize,
    skillsmp_api_key: Option<&str>,
) -> Result<Vec<SourceCandidate>> {
    let mut candidates = Vec::new();

    let api_key = skillsmp_api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var("SKILLSMP_API_KEY").ok().map(|value| value.trim().to_string()))
        .filter(|value| !value.is_empty());
    let query_variants = expand_search_queries(query);
    let mut strong_match = false;

    for candidate_query in &query_variants {
        match crate::core::skillssh_api::search_skills(candidate_query, limit) {
            Ok(skillssh) => {
                candidates.extend(skillssh.iter().map(normalize_skillssh_candidate));
            }
            Err(err) => {
                log::warn!("skills.sh search failed for {candidate_query}: {err:#}");
            }
        }

        if let Some(api_key) = api_key.as_deref() {
            match search_skillsmp(candidate_query, limit, api_key) {
                Ok(skillsmp) => {
                    candidates.extend(
                        skillsmp
                            .into_iter()
                            .filter_map(|entry| normalize_skillsmp_candidate(&entry)),
                    );
                }
                Err(err) => {
                    log::warn!("SkillsMP search failed for {candidate_query}: {err:#}");
                }
            }
        }

        if has_strong_query_match(query, &candidates) {
            strong_match = true;
            break;
        }
    }

    if !strong_match {
        match search_skills_find(query) {
            Ok(skills_find) => {
                candidates.extend(skills_find.into_iter().map(normalize_skills_find_candidate));
            }
            Err(err) => {
                log::warn!("npx skills find failed for {query}: {err:#}");
            }
        }
    }

    Ok(merge_source_candidates(candidates))
}

pub fn merge_source_candidates(mut candidates: Vec<SourceCandidate>) -> Vec<SourceCandidate> {
    candidates.sort_by(|a, b| candidate_rank(b).cmp(&candidate_rank(a)));
    let mut merged: Vec<SourceCandidate> = Vec::new();

    for candidate in candidates {
        let key = dedupe_key(&candidate);
        if let Some(existing) = merged.iter_mut().find(|current| dedupe_key(current) == key) {
            if should_replace(existing, &candidate) {
                *existing = candidate;
            } else {
                existing.evidence_refs.extend(candidate.evidence_refs);
                existing.evidence_refs.sort();
                existing.evidence_refs.dedup();
            }
        } else {
            merged.push(candidate);
        }
    }

    merged.sort_by(|a, b| {
        candidate_rank(b)
            .cmp(&candidate_rank(a))
            .then_with(|| b.installs.cmp(&a.installs))
            .then_with(|| a.title.cmp(&b.title))
    });
    merged
}

fn should_replace(existing: &SourceCandidate, candidate: &SourceCandidate) -> bool {
    let existing_rank = candidate_rank(existing);
    let candidate_rank = candidate_rank(candidate);
    candidate_rank > existing_rank || (candidate_rank == existing_rank && candidate.installs > existing.installs)
}

fn candidate_rank(candidate: &SourceCandidate) -> u8 {
    match candidate.source_kind.as_str() {
        "verified-upstream" => 4,
        "verified-distribution" => 3,
        "inferred-upstream" => 2,
        "custom-no-source" => 1,
        "system-reserved" => 0,
        _ => 0,
    }
}

fn dedupe_key(candidate: &SourceCandidate) -> String {
    if !candidate.source_ref_resolved.is_empty() {
        candidate.source_ref_resolved.clone()
    } else {
        candidate.source_ref.clone()
    }
}

fn search_skillsmp(query: &str, limit: usize, api_key: &str) -> Result<Vec<Value>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let url = format!(
        "https://skillsmp.com/api/v1/skills/search?q={}&limit={}",
        urlencoding::encode(query),
        limit
    );

    let resp: Value = client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("User-Agent", "skills-manager/1.0.0")
        .send()
        .context("Failed to search SkillsMP")?
        .json()
        .context("Failed to parse SkillsMP response")?;

    Ok(extract_entries(&resp))
}

fn extract_entries(value: &Value) -> Vec<Value> {
    if let Some(arr) = value.as_array() {
        return arr.clone();
    }

    let mut entries = Vec::new();
    for key in ["skills", "items", "data", "results"] {
        if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
            entries.extend(arr.iter().cloned());
        }
    }

    entries
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(raw) = value.get(key).and_then(|v| v.as_str()) {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn first_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(raw) = value.get(key).and_then(|v| v.as_u64()) {
            return Some(raw);
        }
    }
    None
}

fn has_strong_query_match(query: &str, candidates: &[SourceCandidate]) -> bool {
    let normalized_query = normalize_token(query);
    if normalized_query.is_empty() {
        return false;
    }

    candidates.iter().any(|candidate| {
        let normalized_title = normalize_token(&candidate.title);
        let normalized_skill_id = normalize_token(&candidate.skill_id);
        let normalized_source_ref = normalize_token(&candidate.source_ref);
        normalized_query == normalized_title
            || normalized_query == normalized_skill_id
            || normalized_source_ref.ends_with(&normalized_query)
    })
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

pub fn normalize_skills_find_candidate(candidate: crate::core::skills_find_cli::SkillsFindCandidate) -> SourceCandidate {
    let owner = candidate.owner;
    let repo = candidate.repo;
    let skill_id = candidate.skill_id;
    let installs = candidate.installs;
    let distribution_ref = candidate.distribution_ref;

    SourceCandidate {
        source_kind: "verified-distribution".to_string(),
        source_ref: format!("{}/{}/{}", owner, repo, skill_id),
        source_ref_resolved: format!("https://github.com/{}/{}.git", owner, repo),
        source_subpath: Some(skill_id.clone()),
        source_branch: None,
        distribution_ref: Some(distribution_ref.clone()),
        title: skill_id.clone(),
        skill_id: skill_id.clone(),
        installs,
        confidence: "high".to_string(),
        evidence_refs: provider_evidence_refs("npx-skills-find", vec![distribution_ref]),
        install_command: Some(format!(
            "npx skills add https://github.com/{}/{} --skill {}",
            owner, repo, skill_id
        )),
    }
}

fn provider_evidence_refs(provider: &str, refs: Vec<String>) -> Vec<String> {
    let mut evidence_refs = vec![format!("provider:{provider}")];
    evidence_refs.extend(refs.into_iter().filter(|value| !value.trim().is_empty()));
    evidence_refs
}

#[cfg(test)]
mod tests {
    use super::{merge_source_candidates, normalize_skillssh_candidate, normalize_skillsmp_candidate};
    use crate::core::skillssh_api::SkillsShSkill;
    use serde_json::json;

    #[test]
    fn normalizes_skillssh_result_into_verified_distribution_candidate() {
        let skill = SkillsShSkill {
            id: "openai/skills/playwright".to_string(),
            skill_id: "playwright".to_string(),
            name: "Playwright".to_string(),
            source: "openai/skills".to_string(),
            installs: 42,
        };

        let normalized = normalize_skillssh_candidate(&skill);

        assert_eq!(normalized.source_kind, "verified-distribution");
        assert_eq!(normalized.source_ref, "openai/skills/playwright");
        assert_eq!(
            normalized.source_ref_resolved,
            "https://github.com/openai/skills.git"
        );
        assert_eq!(
            normalized.install_command.as_deref(),
            Some("npx skills add https://github.com/openai/skills --skill playwright")
        );
    }

    #[test]
    fn normalizes_skillsmp_entry_with_repository_field() {
        let entry = json!({
            "repository": "openai/skills",
            "slug": "playwright",
            "name": "Playwright",
            "installs": 99,
            "url": "https://skillsmp.com/skills/openai/playwright"
        });

        let normalized = normalize_skillsmp_candidate(&entry).expect("candidate should parse");

        assert_eq!(normalized.source_kind, "verified-distribution");
        assert_eq!(normalized.source_ref, "openai/skills/playwright");
        assert_eq!(
            normalized.source_ref_resolved,
            "https://github.com/openai/skills.git"
        );
        assert_eq!(
            normalized.distribution_ref.as_deref(),
            Some("https://skillsmp.com/skills/openai/playwright")
        );
    }

    #[test]
    fn merge_candidates_deduplicates_by_upstream_and_prefers_higher_signal() {
        let skillssh = normalize_skillssh_candidate(&SkillsShSkill {
            id: "openai/skills/playwright".to_string(),
            skill_id: "playwright".to_string(),
            name: "Playwright".to_string(),
            source: "openai/skills".to_string(),
            installs: 40,
        });
        let mut skillsmp = normalize_skillsmp_candidate(&json!({
            "repository": "openai/skills",
            "slug": "playwright",
            "name": "Playwright",
            "installs": 120,
            "url": "https://skillsmp.com/skills/openai/playwright"
        }))
        .expect("candidate should parse");
        skillsmp.source_ref_resolved = "https://github.com/openai/skills.git".to_string();

        let other = normalize_skillssh_candidate(&SkillsShSkill {
            id: "vercel-labs/agent-browser/agent-browser".to_string(),
            skill_id: "agent-browser".to_string(),
            name: "Agent Browser".to_string(),
            source: "vercel-labs/agent-browser".to_string(),
            installs: 80,
        });

        let merged = merge_source_candidates(vec![skillssh, skillsmp, other]);

        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged[0].source_ref_resolved,
            "https://github.com/openai/skills.git"
        );
        assert_eq!(merged[0].installs, 120);
        assert_eq!(
            merged[1].source_ref_resolved,
            "https://github.com/vercel-labs/agent-browser.git"
        );
    }
}
