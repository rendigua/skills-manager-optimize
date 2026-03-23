use app_lib::core::skills_find_cli::{expand_search_queries, SkillsFindCandidate};
use app_lib::core::source_resolver::{
    merge_source_candidates, normalize_skillssh_candidate, normalize_skills_find_candidate,
    normalize_skillsmp_candidate,
};
use app_lib::core::skillssh_api::SkillsShSkill;
use serde_json::json;

#[test]
fn expands_generic_flow_query_to_high_signal_variants() {
    let variants = expand_search_queries("flow");

    assert_eq!(variants.first().map(String::as_str), Some("flow"));
    assert!(variants.iter().any(|variant| variant == "react flow"));
}

#[test]
fn normalizes_candidates_with_provider_evidence_tags() {
    let skillssh = normalize_skillssh_candidate(&SkillsShSkill {
        id: "openai/skills/playwright".to_string(),
        skill_id: "playwright".to_string(),
        name: "Playwright".to_string(),
        source: "openai/skills".to_string(),
        installs: 42,
    });
    assert_eq!(
        skillssh.evidence_refs,
        vec![
            "provider:skills.sh-api".to_string(),
            "https://skills.sh/openai/skills/playwright".to_string()
        ]
    );

    let skillsmp = normalize_skillsmp_candidate(&json!({
        "repository": "openai/skills",
        "slug": "playwright",
        "name": "Playwright",
        "installs": 99,
        "url": "https://skillsmp.com/skills/openai/playwright"
    }))
    .expect("candidate should parse");
    assert_eq!(
        skillsmp.evidence_refs,
        vec![
            "provider:skillsmp-api".to_string(),
            "https://skillsmp.com/skills/openai/playwright".to_string()
        ]
    );

    let skills_find = normalize_skills_find_candidate(SkillsFindCandidate {
        owner: "openai".to_string(),
        repo: "skills".to_string(),
        skill_id: "playwright".to_string(),
        installs: 777,
        distribution_ref: "https://skills.sh/openai/skills/playwright".to_string(),
    });
    assert_eq!(
        skills_find.evidence_refs,
        vec![
            "provider:npx-skills-find".to_string(),
            "https://skills.sh/openai/skills/playwright".to_string()
        ]
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
