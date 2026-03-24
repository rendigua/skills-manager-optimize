use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn build_generated_skill_markdown(name: &str, content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with("---") {
        return trimmed.to_string();
    }

    format!("---\nname: {name}\n---\n\n{}\n", trimmed)
}

pub fn write_generated_skill_source(name: &str, content: &str) -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let skill_md = temp_dir.path().join("SKILL.md");
    let markdown = build_generated_skill_markdown(name, content);
    fs::write(&skill_md, markdown)?;
    Ok(temp_dir)
}

pub fn write_generated_skill_to_path(path: &Path, name: &str, content: &str) -> Result<()> {
    fs::create_dir_all(path)?;
    let skill_md = path.join("SKILL.md");
    let markdown = build_generated_skill_markdown(name, content);
    fs::write(skill_md, markdown)?;
    Ok(())
}
