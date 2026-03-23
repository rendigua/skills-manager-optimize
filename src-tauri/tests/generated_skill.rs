use std::fs;

use app_lib::core::generated_skill::{
    build_generated_skill_markdown, write_generated_skill_source,
};

#[test]
fn wraps_plain_content_with_frontmatter() {
    let markdown = build_generated_skill_markdown("agent-note", "Do the thing");
    assert!(markdown.contains("name: agent-note"));
    assert!(markdown.contains("Do the thing"));
}

#[test]
fn preserves_existing_frontmatter() {
    let input = "---\nname: existing\n---\n\nBody";
    let markdown = build_generated_skill_markdown("agent-note", input);
    assert_eq!(markdown, input);
}

#[test]
fn writes_generated_skill_source_to_temp_dir() {
    let temp = write_generated_skill_source("agent-note", "Body").expect("temp dir");
    let content = fs::read_to_string(temp.path().join("SKILL.md")).expect("skill md");
    assert!(content.contains("name: agent-note"));
    assert!(content.contains("Body"));
}
