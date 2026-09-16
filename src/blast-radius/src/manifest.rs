// MODE: DEV
// PACKAGE: PROD
use crate::finding::{fail, warn, Line};
use crate::git;
use std::path::Path;

/// For each changed path under `planning/`, skip if already tracked;
/// otherwise count LITERAL SUBSTRING occurrences of the path anywhere in
/// `planning/PACKAGE-MANIFEST.tsv` -- reproducing `grep -Fc` exactly, a
/// substring count, not an exact-line match. A path that happens to be a
/// substring of an unrelated manifest row's own field silently counts as
/// "has a row" under this real bash semantic, and this is preserved, not
/// "fixed".
pub fn missing_rows(repo_root: &Path, manifest: &Path, changed: &[String]) -> Vec<Line> {
    let manifest_text = std::fs::read_to_string(manifest).unwrap_or_default();
    let mut lines = Vec::new();

    for path in changed {
        if path.is_empty() {
            continue;
        }
        if !path.starts_with("planning/") {
            continue;
        }
        if git::is_tracked(repo_root, path) {
            continue;
        }
        let rows = manifest_text
            .lines()
            .filter(|row| row.contains(path.as_str()))
            .count();
        if rows > 0 {
            continue;
        }
        if path.ends_with(".json") {
            lines.push(fail(format!(
                "{path}: a new registry under planning/ with no PACKAGE-MANIFEST row will not ship, and a gate reading it through skill_root will die looking for it"
            )));
        } else {
            lines.push(warn(format!(
                "{path}: new under planning/ with no PACKAGE-MANIFEST row — ship it, or record why it is dev-only"
            )));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    fn write_manifest(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
        let path = dir.join("PACKAGE-MANIFEST.tsv");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn substring_count_counts_a_path_embedded_in_an_unrelated_row() {
        // AR-69's sibling subtlety, called out explicitly for grep -Fc: a
        // path that is merely a SUBSTRING of another row's own field still
        // counts as "has a row", matching bash's real (loose) semantics.
        let dir =
            std::env::temp_dir().join(format!("blast-radius-manifest-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = write_manifest(&dir, "planning/some/other-file.txt.old\tskill\tnotes\n");
        let text = std::fs::read_to_string(&manifest).unwrap();
        let rows = text
            .lines()
            .filter(|row| row.contains("planning/some/other-file.txt"))
            .count();
        assert_eq!(rows, 1, "a substring match must still count as a row");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
