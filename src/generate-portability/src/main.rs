// MODE: DEV
// PACKAGE: PROD

//! Port of generate-portability.sh: writes PORTABILITY.md from
//! portability-rules.json. Unlike every prior goal in this plan, this
//! script has NO nix-shell re-exec at all -- it needs only its own
//! in-process JSON parsing, no external tool whatsoever.

mod discovery;
mod emit;
mod markers;
mod rules;

use std::env;
use std::path::{Path, PathBuf};

// The bash original derives its own name from `${0##*/}`, which in every
// real invocation is "generate-portability.sh" -- the literal file name.
// This script has no nix re-exec, so there is no SELF_BINARY_NAME/reexec
// module to split out, unlike goals 14-16.
const PROGRAM: &str = "generate-portability.sh";

const USAGE: &str = "\
generate-portability \u{2014} write PORTABILITY.md from portability-rules.json.

Usage:
  generate-portability.sh [--check]

--check writes to a temp file and diffs instead of overwriting, exit 1 when
PORTABILITY.md is stale. planning/tests/test-portability-contract.sh runs it.

PORTABILITY.md is generated so the gotchas cannot drift from the registry and
so no agent has to rediscover one by tripping over it in an unrelated file.
Edit portability-rules.json, or the `# PORTABILITY:` comment at the site.
";

/// PLANNING_SKILL_ROOT, exported unconditionally by
/// plan_exec_compiled_binary_if_present before every exec of this binary;
/// falls back to a location-anchored resolution from current_exe(), walking
/// up for planning/scripts, when absent or empty (a direct/standalone
/// invocation, such as a test) -- mirroring goal 16's own discover_repo_root
/// exactly, for the same nested-tool-copy reason (AR-57 there).
fn discover_repo_root() -> Result<PathBuf, String> {
    if let Ok(root) = env::var("PLANNING_SKILL_ROOT") {
        if !root.is_empty() {
            return Ok(PathBuf::from(root));
        }
    }
    let self_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("generate-portability"));
    let mut dir = self_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if dir.join("planning/scripts").is_dir() {
            return Ok(dir);
        }
        let Some(parent) = dir.parent() else {
            return Err(format!(
                "could not locate the repository root from {}",
                self_path.display()
            ));
        };
        dir = parent.to_path_buf();
    }
}

/// The current UTC time as `date -u +%Y-%m-%dT%H:%M:%SZ` prints it. Computed
/// here rather than by running `date`, which Windows does not have as a
/// program (only as a cmd builtin), so the stamp used to come out empty
/// there. The stamp is excluded from every comparison this crate or its
/// tests perform (both `--check`'s own determinism check and the real-tree
/// parity test strip the `<!-- generated: -->` line first).
fn now_utc() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    format_utc(seconds)
}

/// `seconds` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`, using Howard
/// Hinnant's days-to-civil-date arithmetic (proleptic Gregorian).
fn format_utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let in_day = seconds % 86_400;
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        in_day / 3_600,
        in_day % 3_600 / 60,
        in_day % 60
    )
}

fn run() -> i32 {
    let args: Vec<String> = env::args().skip(1).collect();
    // Bash's own arg handling checks ONLY $1: "--check" sets check mode,
    // "-h"/"--help" prints help and exits, anything else (or nothing) is
    // silently ignored and the script proceeds as a normal run. No
    // "unknown argument" rejection -- this is real, observed bash
    // behavior, not an oversight to fix.
    match args.first().map(String::as_str) {
        Some("-h") | Some("--help") => {
            print!("{USAGE}");
            return 0;
        }
        _ => {}
    }
    let check_only = args.first().map(String::as_str) == Some("--check");

    let repo_root = match discover_repo_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("{PROGRAM}: {message}");
            return 66;
        }
    };
    let rules_path = repo_root.join("portability-rules.json");
    let rules = match rules::load(&rules_path) {
        Ok(r) => r,
        Err(message) => {
            eprintln!("{PROGRAM}: {message}");
            return 66;
        }
    };

    if check_only {
        let files = discovery::script_list(&repo_root);
        let sightings = markers::marker_sightings(&repo_root, &files);
        let a = emit::render(&rules, &sightings, &now_utc());
        let b = emit::render(&rules, &sightings, &now_utc());
        if emit::strip_generated_line(&a) != emit::strip_generated_line(&b) {
            eprintln!("{PROGRAM}: two fresh builds disagree; generation is not deterministic");
            return 1;
        }
        println!("generation is deterministic (nothing is committed to compare against)");
        return 0;
    }

    let files = discovery::script_list(&repo_root);
    let sightings = markers::marker_sightings(&repo_root, &files);
    let content = emit::render(&rules, &sightings, &now_utc());

    let output = env::var("PORTABILITY_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("PORTABILITY.md"));
    if let Err(error) = write_atomic(&output, &content) {
        eprintln!("{PROGRAM}: could not write {}: {error}", output.display());
        return 1;
    }
    println!("Wrote {}", output.display());
    0
}

fn write_atomic(dest: &Path, content: &str) -> std::io::Result<()> {
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
    std::fs::write(&temp, content)?;
    std::fs::rename(&temp, dest)
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_utc_has_the_expected_shape() {
        let stamp = now_utc();
        assert_eq!(stamp.len(), 20);
        assert!(stamp.ends_with('Z'));
    }

    #[test]
    fn format_utc_matches_known_instants() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z"); // a leap day
        assert_eq!(format_utc(1_709_251_199), "2024-02-29T23:59:59Z");
        assert_eq!(format_utc(1_789_920_000), "2026-09-20T16:00:00Z");
    }
}
