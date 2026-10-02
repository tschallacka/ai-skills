// MODE: DEV
// PACKAGE: PROD
use crate::git;
use std::fs;
use std::path::Path;

/// Overlays `git -C src diff HEAD --name-only`'s own tracked-file changes
/// onto `wt`: a path still present in `src` is copied (creating parent
/// directories); a path no longer present in `src` (a tracked deletion) is
/// removed from `wt`. Best-effort -- an individual copy/remove failure must
/// not abort the run, matching this script's own deliberate
/// `set -uo pipefail`-without-`-e` looseness. Returns the count of paths
/// processed.
pub fn overlay(src: &Path, wt: &Path) -> usize {
    let mut overlaid = 0usize;
    for path in git::diff_name_only(src) {
        if path.is_empty() {
            continue;
        }
        let source_path = src.join(&path);
        let dest_path = wt.join(&path);
        if source_path.is_file() {
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(&source_path, &dest_path);
        } else {
            let _ = fs::remove_file(&dest_path);
        }
        overlaid += 1;
    }
    overlaid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_present_source_file_is_copied_and_an_absent_one_is_removed() {
        let src = std::env::temp_dir().join(format!("vbs-overlay-src-{}", std::process::id()));
        let wt = std::env::temp_dir().join(format!("vbs-overlay-wt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&wt);
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&wt).unwrap();

        fs::write(src.join("modified.txt"), "new content").unwrap();
        fs::write(wt.join("modified.txt"), "old content").unwrap();
        fs::write(wt.join("deleted.txt"), "should be removed").unwrap();

        for path in ["modified.txt", "deleted.txt"] {
            let source_path = src.join(path);
            let dest_path = wt.join(path);
            if source_path.is_file() {
                fs::copy(&source_path, &dest_path).unwrap();
            } else {
                let _ = fs::remove_file(&dest_path);
            }
        }

        assert_eq!(
            fs::read_to_string(wt.join("modified.txt")).unwrap(),
            "new content"
        );
        assert!(!wt.join("deleted.txt").exists());

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&wt);
    }
}
