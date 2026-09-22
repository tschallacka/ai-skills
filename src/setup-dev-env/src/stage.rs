// MODE: DEV
// PACKAGE: PROD

//! The per-crate build loop: `cargo build --release --target <triple>` for
//! every (crate, binary) in the plan, then stage the resulting binary into
//! bin/<triple>/, the planning/scripts/ sibling copy when warranted, and
//! the three per-skill bin/<triple>/ copies for bug-report/todo/
//! interactive-shell.
//!
//! Every staging copy uses write-to-a-temp-file-in-the-same-directory-then-
//! atomic-rename, not a truncate-in-place copy: once this crate is wired,
//! a run that rebuilds ITS OWN crate is overwriting the exact file it is
//! currently executing from. On Linux, opening an existing destination
//! O_TRUNC while that inode is the running image of a process is refused
//! (ETXTBSY); rename(2) instead atomically swaps the directory entry
//! without touching a still-open file description, so the running process
//! keeps serving its own already-mapped old inode undisturbed. Empirically
//! confirmed on real macOS (arm64/APFS) that Darwin does NOT enforce the
//! same restriction a plain truncate-in-place copy would hit on Linux --
//! write-then-rename is adopted uniformly anyway because rename(2) is
//! atomic on POSIX while a truncate+write in place is not, so a crash
//! mid-write only corrupts a discarded temp file rather than the binary a
//! future invocation would exec.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::plan;

/// Writes `contents` to a temp file in the same directory as `dest`, then
/// renames it over `dest`. Fails cleanly (leaving `dest` untouched) when
/// the temp file's parent directory does not exist -- the standard,
/// privilege-independent way this helper's own failure path is exercised.
pub fn write_then_rename(dest: &Path, contents: &[u8], executable: bool) -> std::io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination has no parent",
        )
    })?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        dest.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&temp, contents)?;
    if executable {
        set_executable(&temp)?;
    }
    match std::fs::rename(&temp, dest) {
        Ok(()) => Ok(()),
        Err(error) => replace_running_binary(&temp, dest, error),
    }
}

/// Windows refuses to replace an executable while a process is running from
/// it, which is exactly the case when this crate rebuilds itself. It does
/// allow such a file to be RENAMED, so the running image is moved aside and
/// the new file takes its name; the old one is removed on a later run, when
/// nothing holds it. Anywhere else the original error stands.
fn replace_running_binary(temp: &Path, dest: &Path, error: std::io::Error) -> std::io::Result<()> {
    if !cfg!(windows) || !dest.is_file() {
        let _ = std::fs::remove_file(temp);
        return Err(error);
    }
    let name = dest.file_name().unwrap_or_default().to_string_lossy();
    let aside = dest.with_file_name(format!(".{name}.old-{}", std::process::id()));
    let moved = std::fs::rename(dest, &aside).and_then(|()| std::fs::rename(temp, dest));
    if moved.is_ok() {
        let _ = std::fs::remove_file(&aside);
        return Ok(());
    }
    let _ = std::fs::remove_file(temp);
    if !dest.is_file() && aside.is_file() {
        let _ = std::fs::rename(&aside, dest);
    }
    moved
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn stage_from(source: &Path, dest: &Path) -> std::io::Result<()> {
    let contents = std::fs::read(source)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_then_rename(dest, &contents, true)
}

pub struct BuildOutcome {
    pub built: u32,
    pub failed: Vec<String>,
}

/// Runs the full build loop against `repo_root` for `triple`, printing one
/// progress line per crate.
pub fn run(repo_root: &Path, triple: &str, exe_suffix: &str) -> BuildOutcome {
    let mut built = 0u32;
    let mut failed = Vec::new();
    for (crate_name, binary) in plan::plan() {
        let src = repo_root.join("src").join(crate_name);
        if !src.join("Cargo.toml").is_file() {
            println!("  {crate_name:<16} no crate at src/{crate_name}; skipped");
            continue;
        }
        print!("  {crate_name:<16} ");
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(src.join("Cargo.toml"))
            .arg("--target")
            .arg(triple)
            .current_dir(repo_root)
            // built_artifact() below assumes cargo's output lands under
            // repo_root/target/ -- true by cargo's own default, but an
            // inherited CARGO_TARGET_DIR (an absolute path, set process-wide
            // rather than per invocation; this repo's own CI workflow sets
            // one for the outer build) overrides that regardless of
            // current_dir, silently redirecting this nested build's output
            // elsewhere and making the staging read below fail with "No
            // such file or directory". Pin it explicitly so this build's
            // output location cannot depend on the calling environment.
            .env("CARGO_TARGET_DIR", repo_root.join("target"))
            .output();
        match status {
            Ok(output) if output.status.success() => {
                if let Err(error) = stage_primary(repo_root, triple, exe_suffix, binary) {
                    println!("FAILED");
                    eprintln!("      | staging failed: {error}");
                    failed.push(crate_name.to_string());
                    continue;
                }
                println!("ok -> bin/{triple}/{binary}{exe_suffix}");
                // A failure staging the sibling/skill-dir copies (rare: it
                // would need permissions or disk-space trouble right after
                // the primary copy just succeeded) is reported and this one
                // crate is marked failed, but the run continues rather than
                // aborting the entire remaining build over one crate's
                // extra-copy failure -- the same per-crate failure isolation
                // the primary copy and the build step above already have.
                if let Err(error) = stage_extras(repo_root, triple, exe_suffix, crate_name, binary)
                {
                    eprintln!("      | staging failed: {error}");
                    failed.push(crate_name.to_string());
                    continue;
                }
                built += 1;
            }
            Ok(output) => {
                println!("FAILED");
                let combined = [output.stdout, output.stderr].concat();
                for line in String::from_utf8_lossy(&combined).lines() {
                    eprintln!("      | {line}");
                }
                failed.push(crate_name.to_string());
            }
            Err(error) => {
                println!("FAILED");
                eprintln!("      | {error}");
                failed.push(crate_name.to_string());
            }
        }
    }
    BuildOutcome { built, failed }
}

fn built_artifact(repo_root: &Path, triple: &str, exe_suffix: &str, binary: &str) -> PathBuf {
    repo_root
        .join("target")
        .join(triple)
        .join("release")
        .join(format!("{binary}{exe_suffix}"))
}

/// Just the bin/<triple>/ copy -- the ONE staging step completed before
/// printing "ok -> bin/...". Splitting this out from the sibling/skill-dir
/// copies below is what makes that print ordering reproducible: the
/// "ok -> ..." print runs BEFORE the planning/scripts and skill-dir cp
/// calls, not after them.
fn stage_primary(
    repo_root: &Path,
    triple: &str,
    exe_suffix: &str,
    binary: &str,
) -> std::io::Result<()> {
    let built_path = built_artifact(repo_root, triple, exe_suffix, binary);
    let dest = repo_root
        .join("bin")
        .join(triple)
        .join(format!("{binary}{exe_suffix}"));
    stage_from(&built_path, &dest)
}

/// The planning/scripts sibling copy and the bug-report/todo/interactive-shell
/// skill-dir copy, each printing its own "   -> ..." line -- called AFTER
/// stage_primary and its own "ok -> bin/..." line have already printed.
fn stage_extras(
    repo_root: &Path,
    triple: &str,
    exe_suffix: &str,
    crate_name: &str,
    binary: &str,
) -> std::io::Result<()> {
    let built_path = built_artifact(repo_root, triple, exe_suffix, binary);

    if plan::stages_into_planning_scripts(repo_root, binary) {
        let sibling = repo_root
            .join("planning/scripts")
            .join(format!("{binary}{exe_suffix}"));
        stage_from(&built_path, &sibling)?;
        println!("   -> planning/scripts/{binary}{exe_suffix}");
    }

    // bug-report and todo resolve their tool at <skill>/bin/<triple>/, which
    // is what skill_files() promises and what CI's own staging steps do;
    // interactive-shell's binaries.tsv resolves the same way (B291).
    // interactive-shell-mcp is its own crate but not its own skill: its
    // binary ships in the interactive-shell skill's mcp-mode install
    // (integration.tsv), so it lands in THAT skill's bin/<triple>/.
    let skill_dir = match crate_name {
        "bug-report" | "todo" | "interactive-shell" => Some(crate_name),
        "interactive-shell-mcp" => Some("interactive-shell"),
        _ => None,
    };
    if let Some(skill_dir) = skill_dir {
        let skill_dest = repo_root
            .join(skill_dir)
            .join("bin")
            .join(triple)
            .join(format!("{binary}{exe_suffix}"));
        stage_from(&built_path, &skill_dest)?;
        println!("   -> {skill_dir}/bin/{triple}/{binary}{exe_suffix}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("setup-dev-env-stage-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_then_rename_replaces_existing_content() {
        let dir = scratch("replace");
        let dest = dir.join("binary");
        fs::write(&dest, b"old").unwrap();
        write_then_rename(&dest, b"new", false).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new");
    }

    #[test]
    fn write_then_rename_leaves_no_temp_file_on_success() {
        let dir = scratch("no-leftover");
        let dest = dir.join("binary");
        write_then_rename(&dest, b"content", false).unwrap();
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn write_then_rename_leaves_destination_untouched_on_a_missing_parent() {
        // A nonexistent parent directory fails privilege-independently,
        // unlike a permission-bit trigger (which a root-run test process
        // commonly ignores, making the assertion pass vacuously).
        let dir = scratch("missing-parent");
        let dest = dir.join("does-not-exist-dir").join("binary");
        let result = write_then_rename(&dest, b"new", false);
        assert!(result.is_err());
        assert!(!dest.exists());
    }
}
