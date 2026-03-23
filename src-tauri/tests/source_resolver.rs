use app_lib::core::source_resolver::{
    merge_source_candidates, normalize_skillssh_candidate, normalize_skillsmp_candidate,
    SourceCandidate,
};
use app_lib::core::skillssh_api::SkillsShSkill;
use serde_json::json;

fn candidate(source_kind: &str, source_ref_resolved: &str, installs: u64) -> SourceCandidate {
    SourceCandidate {
        source_kind: source_kind.to_string(),
        source_ref: "openai/skills/playwright".to_string(),
        source_ref_resolved: source_ref_resolved.to_string(),
        source_subpath: Some("playwright".to_string()),
        source_branch: None,
        distribution_ref: Some("https://example.invalid/playwright".to_string()),
        title: "Playwright".to_string(),
        skill_id: "playwright".to_string(),
        installs,
        confidence: "high".to_string(),
        evidence_refs: vec!["evidence".to_string()],
        install_command: Some("npx skills add openai/skills --skill playwright".to_string()),
    }
}

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
    let skillssh = candidate(
        "verified-distribution",
        "https://github.com/openai/skills.git",
        40,
    );
    let skillsmp = candidate(
        "verified-distribution",
        "https://github.com/openai/skills.git",
        120,
    );
    let other = candidate(
        "verified-distribution",
        "https://github.com/vercel-labs/agent-browser.git",
        80,
    );

    let merged = merge_source_candidates(vec![skillssh, skillsmp, other]);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].source_ref_resolved, "https://github.com/openai/skills.git");
    assert_eq!(merged[0].installs, 120);
    assert_eq!(merged[1].source_ref_resolved, "https://github.com/vercel-labs/agent-browser.git");
}
