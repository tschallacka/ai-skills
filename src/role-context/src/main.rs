// MODE: DEV
// PACKAGE: PROD
use plan_crypt::sha256::{hex, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const ROLES: &[(&str, &str)] = &[
    ("alex", "Alex"),
    ("benny", "Benny"),
    ("chris", "Chris"),
    ("christian", "Christian"),
    ("christoph", "Christoph"),
    ("dana", "Dana"),
    ("frank", "Frank"),
    ("maintainer", "Willie"),
    ("installer", "Felix"),
    ("oracle", "Pythia"),
    ("eve", "Eve"),
];

fn usage(code: u8) -> ! {
    print!("role-context.sh — role-gated context reader (persona registry + scope docs).\n\nGiven a role id or canonical name, print the .md documents that single role\nneeds, concatenated with provenance headers and the role's voice preamble, so\nan agent gets a scoped payload instead of loading unrelated knowledge.\n\nUsage:\n  role-context.sh <role-id|name> [-p N|--page N] [--page-size BYTES]\n  role-context.sh --list                    # -l; identity-free (safe mode)\n  ROLE_ID=maintainer role-context.sh --paths <role-id|name>  # maintainer-only\n  role-context.sh --help\n\nOutput is BYTE-budgeted and paginated: -p2 (or -p 2) prints the next page when\na \"more: ...\" footer is shown; --page-size sets the per-page byte budget\n(default 12000). Every page is a deterministic slice; no TTY is needed.\n\nGATING: identity-aware and FAILS CLOSED. Any content read requires a ROLE_ID\nresolving to a registered persona; reads are restricted to the caller's own\nrole (reviewer family mutual, maintainer may read all). --list is open.\n");
    std::process::exit(code.into());
}

fn resolve(token: &str) -> Option<&'static str> {
    let token = token.to_ascii_lowercase();
    if token.starts_with("benny") && token[5..].chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Some("benny");
    }
    ROLES
        .iter()
        .find(|(id, name)| token == *id || token == name.to_ascii_lowercase())
        .map(|(id, _)| *id)
}

fn list_roles() -> String {
    ROLES
        .iter()
        .map(|(id, name)| format!("{id:<11} {name}\n"))
        .collect()
}

fn role_docs(id: &str) -> &'static [&'static str] {
    match id {
        "alex" => &[
            "SKILL.md",
            "ROLES.md",
            "roles/planning.md",
            "roles/execution.md",
        ],
        "benny" => &["ROLES.md", "roles/planning.md", "roles/execution.md"],
        "chris" | "christian" | "christoph" => &["ROLES.md", "roles/planning.md", "REVIEWER.md"],
        "dana" => &["ROLES.md", "roles/execution.md"],
        "frank" => &["ROLES.md", "roles/cleanup.md"],
        "maintainer" => &[
            "MAINTAINER-STYLE-CONTRACT.md",
            "ROLES.md",
            "roles/planning.md",
            "roles/execution.md",
            "roles/cleanup.md",
        ],
        "installer" => &["ROLES.md", "MAINTAINER-STYLE-CONTRACT.md"],
        "oracle" | "eve" => &["MAINTAINER-STYLE-CONTRACT.md"],
        _ => &["ROLES.md"],
    }
}

fn can_access(caller: &str, target: &str) -> bool {
    caller == target
        || caller == "maintainer"
        || matches!(caller, "chris" | "christian" | "christoph")
            && matches!(target, "chris" | "christian" | "christoph")
}

/// Mirrors src/plan-mutate/src/main.rs's own skill_root(), and this repo's
/// build-plan-libs/generate-skill-docs/verify-skill-load precedent:
/// PLANNING_SKILL_ROOT first (exported by plan_exec_compiled_binary_if_present
/// before every exec), then an ancestor walk of the running binary's own path
/// and the current working directory, looking for the first ancestor whose
/// planning/scripts subdirectory exists. A fixed current_exe()-relative
/// "two parents up" guess (the previous implementation) is wrong the moment
/// the binary is exec'd from anywhere other than planning/scripts itself --
/// which is always, since plan_exec_compiled_binary_if_present execs it from
/// plan_bin_dir()'s own bin/<triple> location, not planning/scripts/. Every
/// registered persona's scope docs silently came back "missing" through the
/// real role-context.sh wrapper as a result, confirmed by direct comparison
/// against invoking the bare planning/scripts/role-context copy.
fn skill_root_from(
    env_root: Option<&Path>,
    exe_path: Option<&Path>,
    cwd: Option<&Path>,
) -> PathBuf {
    if let Some(root) = env_root {
        if root.join("planning/scripts").is_dir() {
            return root.to_path_buf();
        }
    }
    let mut places = Vec::new();
    if let Some(exe) = exe_path {
        places.extend(exe.ancestors().map(Path::to_path_buf));
    }
    if let Some(cwd) = cwd {
        places.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    places
        .into_iter()
        .find(|root| root.join("planning/scripts").is_dir())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn skill_dir() -> PathBuf {
    let env_root = env::var_os("PLANNING_SKILL_ROOT").map(PathBuf::from);
    let exe = env::current_exe().ok();
    let cwd = env::current_dir().ok();
    skill_root_from(env_root.as_deref(), exe.as_deref(), cwd.as_deref()).join("planning")
}

fn hash(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut digest = Sha256::new();
    digest.update(&bytes);
    Some(hex(&digest.finish()))
}

fn voice(dir: &Path, id: &str) -> Option<String> {
    let text = fs::read_to_string(dir.join("roles/VOICES.md")).ok()?;
    text.lines().find_map(|line| {
        if !line.starts_with('|') {
            return None;
        }
        let cells: Vec<_> = line.split('|').map(str::trim).collect();
        (cells.len() > 3 && cells[1].trim_matches('`') == id).then(|| cells[2].to_string())
    })
}

fn payload(dir: &Path, id: &str) -> String {
    let mut out = format!(
        "# Role context: {id} ({})\n\n",
        ROLES
            .iter()
            .find(|(role, _)| *role == id)
            .map(|(_, name)| *name)
            .unwrap_or(id)
    );
    if let Some(text) = voice(dir, id) {
        out.push_str(&format!("# Voice ({id}): {text}\n\n"));
    }
    for rel in role_docs(id) {
        let file = dir.join(rel);
        if file.is_file() {
            if *rel == "REVIEWER.md" {
                let pin = fs::read_to_string(&file).ok().and_then(|text| {
                    text.lines().find_map(|line| {
                        line.strip_prefix("> Source SHA-256: `")
                            .and_then(|v| v.strip_suffix('`'))
                            .map(str::to_string)
                    })
                });
                if let (Some(pin), Some(actual)) = (pin, hash(&dir.join("skill-source.txt"))) {
                    if pin != actual {
                        out.push_str("\n# (stale: REVIEWER.md pins an older skill-source.txt — regenerate with scripts/generate-reviewer.sh)\n");
                        continue;
                    }
                }
            }
            out.push_str(&format!("\n===== {rel} =====\n"));
            out.push_str(&fs::read_to_string(file).unwrap_or_default());
        } else if *rel == "REVIEWER.md" {
            out.push_str("\n# (missing: REVIEWER.md — generated by scripts/generate-reviewer.sh from skill-source.txt; run it and re-run this command)\n");
        } else {
            out.push_str(&format!("\n# (missing: {rel})\n"));
        }
    }
    out
}

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().skip(1).collect();
    if raw.iter().any(|a| a == "--help" || a == "-h") {
        usage(0);
    }
    if raw.iter().any(|a| a == "--list" || a == "-l") {
        print!("{}", list_roles());
        return ExitCode::SUCCESS;
    }
    let paths = raw.iter().any(|a| a == "--paths");
    let mut role = String::new();
    let mut skip_value = false;
    for argument in &raw {
        if skip_value {
            skip_value = false;
            continue;
        }
        if argument == "-p" || argument == "--page" || argument == "--page-size" {
            skip_value = true;
        } else if !argument.starts_with('-') {
            role = argument.clone();
            break;
        }
    }
    let target = resolve(&role).unwrap_or_else(|| {
        eprintln!("role-context: unknown role or name: {role}");
        eprintln!("  valid: {}", list_roles());
        std::process::exit(64);
    });
    if paths {
        if resolve(&env::var("ROLE_ID").unwrap_or_default()) != Some("maintainer") {
            eprintln!("role-context: --paths is maintainer-only; run with ROLE_ID=maintainer");
            return ExitCode::from(64);
        }
        for rel in role_docs(target) {
            println!("{rel}");
        }
        return ExitCode::SUCCESS;
    }
    let caller = match env::var("ROLE_ID") { Ok(value) => resolve(&value).unwrap_or_else(|| { eprintln!("role-context: FAIL-CLOSED identity: unknown ROLE_ID \"{value}\". This worker has no persona and cannot continue; the coordinator must respawn it with a valid ROLE_ID."); std::process::exit(64) }), Err(_) => { eprintln!("role-context: FAIL-CLOSED identity: no ROLE_ID set. This worker lacks a persona and cannot read scoped context; the coordinator must respawn it with a valid ROLE_ID (or use --list)."); return ExitCode::from(64); } };
    if !can_access(caller, target) {
        eprintln!("role-context: role {caller} may not read context for {target} (own-role gate)");
        eprintln!("  your role: {caller}; valid ids: {}", list_roles());
        return ExitCode::from(64);
    }
    let page = raw
        .windows(2)
        .find_map(|w| {
            (w[0] == "-p" || w[0] == "--page")
                .then(|| w[1].parse::<usize>().ok())
                .flatten()
        })
        .or_else(|| {
            raw.iter()
                .find_map(|a| a.strip_prefix("-p").and_then(|v| v.parse().ok()))
        })
        .unwrap_or(1);
    let budget = raw
        .windows(2)
        .find_map(|w| {
            (w[0] == "--page-size")
                .then(|| w[1].parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(12000);
    let text = payload(&skill_dir(), target);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut starts = vec![0usize];
    let mut ends = Vec::new();
    let mut bytes = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if i > *starts.last().unwrap() && bytes + line.len() > budget {
            ends.push(i);
            starts.push(i);
            bytes = 0;
        }
        bytes += line.len();
    }
    ends.push(lines.len());
    let total = starts.len();
    println!(
        "# role-context {target} ({}) — page {page}/{total}",
        ROLES
            .iter()
            .find(|(id, _)| *id == target)
            .map(|(_, n)| *n)
            .unwrap_or(target)
    );
    if page == 0 || page > total {
        println!("# (page out of range; expected 1..{total})");
        return ExitCode::SUCCESS;
    }
    print!("{}", lines[starts[page - 1]..ends[page - 1]].concat());
    if page < total {
        println!("\n# more: role-context.sh <{target}|name> -p {}", page + 1);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{can_access, hash, payload, resolve, skill_root_from};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn resolves_ids_names_and_benny_instances() {
        assert_eq!(resolve("Willie"), Some("maintainer"));
        assert_eq!(resolve("PYTHIA"), Some("oracle"));
        assert_eq!(resolve("benny-07"), Some("benny"));
        assert_eq!(resolve("unknown"), None);
    }

    #[test]
    fn reviewer_family_is_mutually_accessible_but_other_roles_are_not() {
        assert!(can_access("christian", "christoph"));
        assert!(can_access("maintainer", "eve"));
        assert!(!can_access("chris", "dana"));
    }

    /// Regression: skill_root_from must resolve the real repo root when the
    /// binary is exec'd from a bin/<triple> directory that is NOT two
    /// parents below the repo root -- exactly what
    /// plan_exec_compiled_binary_if_present's own exec does, and what the
    /// previous current_exe()-relative implementation got wrong.
    #[test]
    fn skill_root_prefers_the_env_var_over_a_bin_dir_exe_path() {
        let dir = scratch("skill-root-env");
        fs::create_dir_all(dir.join("planning/scripts")).unwrap();
        let exe_in_unrelated_bin_dir =
            PathBuf::from("/some/other/bin/x86_64-unknown-linux-musl/role-context");
        let root = skill_root_from(Some(&dir), Some(&exe_in_unrelated_bin_dir), None);
        assert_eq!(root, dir);
    }

    #[test]
    fn skill_root_falls_back_to_walking_up_from_the_exe_path_without_the_env_var() {
        let dir = scratch("skill-root-exe-walk");
        fs::create_dir_all(dir.join("planning/scripts")).unwrap();
        let exe = dir.join("bin/x86_64-unknown-linux-musl/role-context");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        let root = skill_root_from(None, Some(&exe), None);
        assert_eq!(root, dir);
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("role-context-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("roles")).unwrap();
        dir
    }

    /// Regression for B361: the REVIEWER.md staleness check must hash the
    /// same file generate-reviewer.sh pins (skill-source.txt), not SKILL.md
    /// -- hashing the wrong file made every fresh REVIEWER.md report stale.
    #[test]
    fn reviewer_stale_check_hashes_skill_source_not_skill_md() {
        let dir = scratch("reviewer-stale");
        fs::write(dir.join("skill-source.txt"), "authored source content").unwrap();
        // SKILL.md deliberately differs from skill-source.txt (it always
        // does -- one is a generated index, the other the full source), so a
        // stale-check comparing against the wrong file would always fail.
        fs::write(dir.join("SKILL.md"), "generated index content").unwrap();
        let source_hash = hash(&dir.join("skill-source.txt")).unwrap();
        fs::write(
            dir.join("REVIEWER.md"),
            format!("> Source SHA-256: `{source_hash}`\n\nreviewer body\n"),
        )
        .unwrap();

        let text = payload(&dir, "chris");
        assert!(
            text.contains("===== REVIEWER.md =====") && text.contains("reviewer body"),
            "expected fresh REVIEWER.md content, got: {text}"
        );
        assert!(
            !text.contains("(stale:"),
            "unexpectedly reported stale: {text}"
        );
    }

    #[test]
    fn reviewer_stale_check_still_fires_on_a_real_mismatch() {
        let dir = scratch("reviewer-mismatch");
        fs::write(dir.join("skill-source.txt"), "authored source content").unwrap();
        fs::write(
            dir.join("REVIEWER.md"),
            "> Source SHA-256: `not-a-real-hash`\n\nreviewer body\n",
        )
        .unwrap();

        let text = payload(&dir, "chris");
        assert!(
            text.contains("(stale:"),
            "expected a stale warning, got: {text}"
        );
        assert!(!text.contains("reviewer body"));
    }
}
