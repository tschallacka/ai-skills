// MODE: DEV
// PACKAGE: PROD
//! Which top-level directories under a release tree are skills. No shipped
//! manifest to read yet (install.sh's SKILL_NAMES table lives in
//! installer/src/05-config.sh, hand-maintained, not yet ported) — a
//! directory counts as a skill purely by having its own `SKILL.md`, the same
//! signal install.sh's own skill_files() ultimately answers to. This is
//! looser than the real selection (no kind/description/hidden-skill
//! handling yet), a known gap until the manifest ports.

use std::fs;
use std::path::Path;

pub fn discover_skills(source_root: &Path) -> std::io::Result<Vec<String>> {
    let mut skills: Vec<String> = fs::read_dir(source_root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("SKILL.md").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    skills.sort();
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    #[test]
    fn finds_every_directory_with_a_skill_md() {
        let root = tempfile::tempdir().unwrap();
        touch(&root.path().join("todo/SKILL.md"));
        touch(&root.path().join("bug-report/SKILL.md"));
        touch(&root.path().join("not-a-skill/README.md"));
        fs::write(root.path().join("installer"), "binary").unwrap();

        let found = discover_skills(root.path()).unwrap();

        assert_eq!(found, vec!["bug-report".to_string(), "todo".to_string()]);
    }

    #[test]
    fn empty_root_finds_nothing() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(discover_skills(root.path()).unwrap(), Vec::<String>::new());
    }
}
