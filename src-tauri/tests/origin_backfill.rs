use std::fs;

use app_lib::core::origin_backfill::{plan_origin_backfill, BackfillAction};
use app_lib::core::source_resolver::SourceCandidate;
use tempfile::tempdir;

fn candidate(title: &str, installs: u64, source_ref_resolved: &str) -> SourceCandidate {
    SourceCandidate {
        source_kind: "verified-distribution".to_string(),
        source_ref: "openai/skills/playwright".to_string(),
        source_ref_resolved: source_ref_resolved.to_string(),
        source_subpath: Some("playwright".to_string()),
        source_branch: None,
        distribution_ref: Some("https://skills.sh/openai/skills/playwright".to_string()),
        title: title.to_string(),
        skill_id: "playwright".to_string(),
        installs,
        confidence: "high".to_string(),
        evidence_refs: vec!["https://skills.sh/openai/skills/playwright".to_string()],
        install_command: Some("npx skills add https://github.com/openai/skills --skill playwright".to_string()),
    }
}

#[test]
fn plans_safe_backfill_for_unambiguous_custom_skill() {
    let root = tempdir().expect("temp dir");
    let skill_dir = root.path().join("playwright");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(
        skill_dir.join("origin.json"),
        r#"{
  "schema_version": 1,
  "source_type": "manual",
  "source_kind": "custom-no-source",
  "source_ref": null
}"#,
    )
    .expect("origin json");
    fs::write(skill_dir.join("SKILL.md"), "# Playwright").expect("skill md");

    let plan = plan_origin_backfill(root.path(), 10, |query, _, _| {
        assert_eq!(query, "playwright");
        Ok(vec![candidate(
            "Playwright",
            120,
            "https://github.com/openai/skills.git",
        )])
    })
    .expect("plan");

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].action, BackfillAction::ApplyCandidate);
    assert_eq!(
        plan.entries[0].top_candidate.as_ref().map(|value| value.source_ref_resolved.as_str()),
        Some("https://github.com/openai/skills.git")
    );
}

#[test]
fn keeps_ambiguous_candidates_for_review() {
    let root = tempdir().expect("temp dir");
    let skill_dir = root.path().join("agent-browser");
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(
        skill_dir.join("origin.json"),
        r#"{
  "schema_version": 1,
  "source_type": "legacy-untracked"
}"#,
    )
    .expect("origin json");
    fs::write(skill_dir.join("SKILL.md"), "# Agent Browser").expect("skill md");

    let plan = plan_origin_backfill(root.path(), 10, |_query, _, _| {
        Ok(vec![
            candidate("Agent Browser", 70, "https://github.com/a/repo.git"),
            candidate("Agent Browser", 69, "https://github.com/b/repo.git"),
        ])
    })
    .expect("plan");

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].action, BackfillAction::NeedsReview);
}
