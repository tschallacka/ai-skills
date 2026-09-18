// MODE: DEV
// PACKAGE: PROD
//! Regenerates one `.agents/profiles/<id>.json` persona's own `instructions`
//! field from the already-compiled `role-context` binary's live output,
//! instead of the hand-authored, one-time snapshot goals 1/2 of the
//! persona-profile-migration plan wrote by hand. Mirrors
//! `build-plan-libs.sh`'s own generated-artifact contract: `--check` reports
//! drift without writing, `--write` regenerates in place. `name` and
//! `description` are never touched -- role-context has no analog of a
//! profile's own hand-authored description blurb (AR-134), so only
//! `instructions` is replaced.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const COMMAND: &str = "generate-profile-content";

const USAGE: &str = "Usage: generate-profile-content <persona-id> --check\n\
       generate-profile-content <persona-id> --write\n\
       generate-profile-content --help\n\
\n\
  --check  compare the persona's live role-context payload against the\n\
           instructions field already on disk; exit 1 and print a diff if\n\
           they differ, exit 0 if they match.\n\
  --write  regenerate the instructions field from a fresh role-context read\n\
           and write it back, preserving the existing name/description\n\
           verbatim.\n";

fn usage(code: u8) -> ! {
    print!("{USAGE}");
    std::process::exit(code.into());
}

fn die(message: impl AsRef<str>, code: u8) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code.into());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileSpec {
    name: String,
    description: String,
    instructions: String,
}

/// A page size comfortably larger than any real persona payload (the largest
/// shipped profile today is ~32KB), so a single role-context invocation
/// always returns page 1 of 1 -- no multi-page concatenation needed.
const PAGE_SIZE: &str = "10000000";

/// Repo root: this binary is invoked from the repository root during
/// development (matching every other planning tool's own convention), so
/// walk up from the current directory looking for `.agents/profiles`.
fn repo_root() -> PathBuf {
    let mut dir = env::current_dir().unwrap_or_else(|error| die(error.to_string(), 70));
    loop {
        if dir.join(".agents/profiles").is_dir() {
            return dir;
        }
        if !dir.pop() {
            die(
                "could not find the repository root (.agents/profiles not found in any ancestor of the current directory)",
                66,
            );
        }
    }
}

/// Locates the compiled `role-context` binary. It must be the copy staged at
/// `planning/scripts/role-context` specifically, not a `bin/<triple>/`
/// copy: role-context's own `skill_dir()` finds its scope docs by walking up
/// two directories from `env::current_exe()`, which only lands on
/// `planning/` when invoked from `planning/scripts/<name>` (two levels
/// below it) -- the same install location `planning/scripts/role-context.sh`
/// itself execs via `plan_exec_compiled_binary_if_present`. Invoking a
/// `bin/<triple>/role-context` copy directly resolves the wrong directory
/// (`bin/` instead of `planning/`) and silently reports every scope doc
/// missing (confirmed by direct testing, not a hypothetical).
fn role_context_binary(root: &Path) -> PathBuf {
    let candidate = root.join("planning/scripts/role-context");
    if candidate.is_file() {
        return candidate;
    }
    die(
        format!(
            "role-context binary not found at {} (checked path, not AI_SKILLS_BIN_ROOT -- role-context's own scope-doc resolution requires this exact install location); run ./setup-dev-env.sh to build it",
            candidate.display()
        ),
        69,
    );
}

/// Runs the compiled role-context binary for `persona`, strips its own CLI
/// pagination banner ("# role-context <id> (<name>) — page 1/1") and
/// payload()'s leading "# Role context: <id> (<name>)" header line, and
/// returns exactly what goals 1/2 hand-copied into a profile's own
/// instructions field: the Voice line onward.
fn fetch_instructions(role_context: &Path, persona: &str) -> String {
    let output = Command::new(role_context)
        .arg(persona)
        .arg("--page-size")
        .arg(PAGE_SIZE)
        .env("ROLE_ID", "maintainer")
        .output()
        .unwrap_or_else(|error| die(format!("could not run role-context: {error}"), 70));
    if !output.status.success() {
        die(
            format!(
                "role-context {persona} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            70,
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let banner = lines.next().unwrap_or_default();
    if !banner.starts_with("# role-context ") {
        die(
            format!("unexpected role-context output for {persona}: missing CLI banner line"),
            70,
        );
    }
    let rest: Vec<&str> = lines.collect();
    let body_start = if rest
        .first()
        .is_some_and(|line| line.starts_with("# Role context: "))
    {
        // The header line is followed by exactly one blank line before the
        // Voice line (or the first document section, for a persona with no
        // voice) -- skip both.
        2
    } else {
        0
    };
    let body = rest.get(body_start..).unwrap_or(&[]).join("\n");
    if body.ends_with(|c: char| !c.is_whitespace()) {
        format!("{body}\n")
    } else {
        body
    }
}

fn profile_path(root: &Path, persona: &str) -> PathBuf {
    root.join(".agents/profiles")
        .join(format!("{persona}.json"))
}

fn read_profile(path: &Path) -> ProfileSpec {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| die(format!("{}: {error}", path.display()), 66));
    serde_json::from_str(&text).unwrap_or_else(|error| {
        die(
            format!("{}: invalid profile JSON: {error}", path.display()),
            65,
        )
    })
}

fn write_profile(path: &Path, spec: &ProfileSpec) {
    let mut text = serde_json::to_string_pretty(spec)
        .unwrap_or_else(|error| die(format!("cannot render profile JSON: {error}"), 70));
    text.push('\n');
    fs::write(path, text).unwrap_or_else(|error| die(format!("{}: {error}", path.display()), 73));
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        usage(0);
    }
    let check = args.iter().any(|a| a == "--check");
    let write = args.iter().any(|a| a == "--write");
    if check == write {
        die("exactly one of --check or --write is required", 64);
    }
    let persona = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .unwrap_or_else(|| die("a persona id is required", 64))
        .clone();

    let root = repo_root();
    let role_context = role_context_binary(&root);
    let path = profile_path(&root, &persona);
    let existing = read_profile(&path);
    let fresh_instructions = fetch_instructions(&role_context, &persona);

    if check {
        if existing.instructions == fresh_instructions {
            println!("{persona}: up to date");
            return ExitCode::SUCCESS;
        }
        println!("{persona}: instructions drifted from a fresh role-context read");
        println!("--- on disk ({} bytes) ---", existing.instructions.len());
        println!("--- fresh ({} bytes) ---", fresh_instructions.len());
        for (i, (old_line, new_line)) in existing
            .instructions
            .lines()
            .zip(fresh_instructions.lines())
            .enumerate()
        {
            if old_line != new_line {
                println!("first differing line ({}):", i + 1);
                println!("  on disk: {old_line}");
                println!("  fresh:   {new_line}");
                break;
            }
        }
        return ExitCode::FAILURE;
    }

    // write mode: preserve name/description verbatim (AR-134), replace only
    // instructions.
    let updated = ProfileSpec {
        name: existing.name,
        description: existing.description,
        instructions: fresh_instructions,
    };
    write_profile(&path, &updated);
    println!("{persona}: wrote {}", path.display());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fake_role_context(dir: &Path, payload: &str) -> PathBuf {
        let script = dir.join("role-context");
        let body = format!(
            "#!/bin/sh\ncat <<'EOF'\n# role-context fixture (Fixture) - page 1/1\n{payload}\nEOF\n"
        );
        fs::write(&script, body).unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
        }
        fs::set_permissions(&script, perms).unwrap();
        script
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "generate-profile-content-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".agents/profiles")).unwrap();
        dir
    }

    fn write_profile_fixture(root: &Path, persona: &str, spec: &ProfileSpec) -> PathBuf {
        let path = profile_path(root, persona);
        write_profile(&path, spec);
        path
    }

    #[test]
    fn fetch_instructions_strips_the_cli_banner_and_role_context_header() {
        let dir = scratch("fetch");
        let role_context = write_fake_role_context(
            &dir,
            "# Role context: fixture (Fixture)\n\n# Voice (fixture): Be direct.\n\n===== ROLES.md =====\nfixture content\n",
        );
        let got = fetch_instructions(&role_context, "fixture");
        assert_eq!(
            got,
            "# Voice (fixture): Be direct.\n\n===== ROLES.md =====\nfixture content\n"
        );
    }

    #[test]
    fn check_reports_clean_when_content_matches() {
        let dir = scratch("check-clean");
        let role_context = write_fake_role_context(
            &dir,
            "# Role context: fixture (Fixture)\n\n# Voice (fixture): Be direct.\n\n===== ROLES.md =====\nfixture content\n",
        );
        let fresh = fetch_instructions(&role_context, "fixture");
        let path = write_profile_fixture(
            &dir,
            "fixture",
            &ProfileSpec {
                name: "fixture".into(),
                description: "A fixture persona.".into(),
                instructions: fresh.clone(),
            },
        );
        let existing = read_profile(&path);
        assert_eq!(existing.instructions, fresh);
    }

    #[test]
    fn write_preserves_name_and_description() {
        let dir = scratch("write-preserve");
        let role_context = write_fake_role_context(
            &dir,
            "# Role context: fixture (Fixture)\n\n# Voice (fixture): Be direct.\n\n===== ROLES.md =====\nfresh content\n",
        );
        let path = write_profile_fixture(
            &dir,
            "fixture",
            &ProfileSpec {
                name: "fixture".into(),
                description: "A hand-authored blurb.".into(),
                instructions: "stale content".into(),
            },
        );
        let fresh = fetch_instructions(&role_context, "fixture");
        let existing = read_profile(&path);
        let updated = ProfileSpec {
            name: existing.name,
            description: existing.description,
            instructions: fresh,
        };
        write_profile(&path, &updated);
        let reread = read_profile(&path);
        assert_eq!(reread.name, "fixture");
        assert_eq!(reread.description, "A hand-authored blurb.");
        assert!(reread.instructions.contains("fresh content"));
    }
}
