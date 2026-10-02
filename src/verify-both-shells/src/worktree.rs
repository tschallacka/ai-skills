// MODE: DEV
// PACKAGE: PROD
use std::collections::hash_map::RandomState;
use std::fs::{self, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::{Path, PathBuf};

/// AR-78: the real sed pattern `^worktree \(.*/verify-wt\..*\)$` is a plain,
/// UNANCHORED substring match for the literal text `/verify-wt.` occurring
/// anywhere in the porcelain path (with arbitrary text before and after) --
/// not a "last two path components equal verify-wt.<suffix>/tree" check.
/// Backslashes are read as slashes first: a path some Windows tool wrote
/// names the same directory.
pub fn is_stale_candidate(path: &str) -> bool {
    path.replace('\\', "/").contains("/verify-wt.")
}

const NAME_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Six characters from the same alphabet mktemp's `XXXXXX` draws on, taken
/// from the standard library's OS-seeded hasher keys so no randomness crate
/// is needed for a scratch name.
fn random_suffix() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    let mut bits = hasher.finish();
    let mut out = String::with_capacity(6);
    for _ in 0..6 {
        out.push(NAME_CHARS[(bits % NAME_CHARS.len() as u64) as usize] as char);
        bits /= NAME_CHARS.len() as u64;
    }
    out
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

/// `mktemp -d "<base>/verify-wt.XXXXXX"`, done here rather than by shelling
/// out to it: naming is identical (a leftover created by one implementation
/// is sweepable by the other) and so is atomic creation -- an existing name
/// is retried, never reused -- but the path comes back in the platform's own
/// form. Git for Windows' mktemp answers `/tmp/verify-wt.abc`, a path no
/// Windows program, git included, can open.
pub fn mktemp_scratch_dir(base: &Path) -> Option<PathBuf> {
    for _ in 0..100 {
        let candidate = base.join(format!("verify-wt.{}", random_suffix()));
        match create_private_dir(&candidate) {
            Ok(()) => return Some(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    None
}

/// Real bash creates `log5`/`log3` as INDEPENDENT `mktemp` files directly
/// under `${TMPDIR:-/tmp}` (`verify-log5.XXXXXX`/`verify-log3.XXXXXX`) --
/// siblings of, not nested inside, the worktree's own parent directory.
/// `cleanup()`'s own real order removes the worktree's parent
/// unconditionally and decides the logs' own keep-or-delete fate entirely
/// separately, so the two must never share a directory.
pub fn mktemp_file(base: &Path, prefix: &str) -> Option<PathBuf> {
    for _ in 0..100 {
        let candidate = base.join(format!("{prefix}.{}", random_suffix()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(_) => return Some(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vbs-worktree-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_path_containing_the_literal_substring_is_a_stale_candidate() {
        assert!(is_stale_candidate("/tmp/verify-wt.abc123/tree"));
        assert!(is_stale_candidate(
            "/tmp/some-prefix/verify-wt.abc123-suffix/tree/nested"
        ));
    }

    #[test]
    fn a_windows_spelling_of_the_path_is_a_stale_candidate_too() {
        assert!(is_stale_candidate(
            "C:\\Users\\x\\Temp\\verify-wt.abc123\\tree"
        ));
    }

    #[test]
    fn a_path_without_the_substring_is_not_a_stale_candidate() {
        assert!(!is_stale_candidate("/tmp/other-wt.abc123/tree"));
        assert!(!is_stale_candidate("/tmp/unrelated/path"));
    }

    #[test]
    fn the_scratch_dir_is_created_with_the_mktemp_shaped_name_and_never_reused() {
        let base = scratch("dir");
        let first = mktemp_scratch_dir(&base).expect("first dir");
        let second = mktemp_scratch_dir(&base).expect("second dir");
        assert_ne!(first, second);
        let name = first.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("verify-wt."), "{name}");
        assert_eq!(name.len(), "verify-wt.".len() + 6, "{name}");
        assert!(first.is_dir() && second.is_dir());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_scratch_dir_fails_when_its_base_is_missing() {
        assert!(mktemp_scratch_dir(&scratch("gone").join("missing")).is_none());
    }

    #[test]
    fn a_temp_file_is_created_empty_under_the_base() {
        let base = scratch("file");
        let path = mktemp_file(&base, "verify-log5").expect("file");
        assert!(path.is_file());
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("verify-log5."));
        let _ = fs::remove_dir_all(&base);
    }
}
