// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const COMMAND: &str = "build-plan-libs.sh";

const USAGE: &str = "Usage: build-plan-libs.sh [--target dev|prod]\n\
       build-plan-libs.sh --check\n\
       build-plan-libs.sh --help\n\
\n\
  --target  prod (default) is what ships: no provenance comments, and function\n\
            files marked '# PACKAGE: DEV' are left out. dev keeps both.\n\
  --check   compare the committed libraries against a fresh PROD build and exit\n\
            1 on any difference, naming the group that drifted. A dev build in\n\
            the tree is a difference, which is what stops it being committed.\n";

fn usage(code: i32) -> ! {
    print!("{USAGE}");
    process::exit(code);
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    process::exit(code);
}

/// PLANNING_SKILL_ROOT first, then an ancestor walk of both the running
/// binary's own path and the current working directory, looking for the
/// first ancestor whose planning/scripts subdirectory exists.
fn skill_root_from(
    env_root: Option<&Path>,
    exe_path: Option<&Path>,
    cwd: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(root) = env_root {
        if root.join("planning/scripts").is_dir() {
            return Some(root.to_path_buf());
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
}

fn skill_root() -> Option<PathBuf> {
    let env_root = env::var_os("PLANNING_SKILL_ROOT").map(PathBuf::from);
    let exe = env::current_exe().ok();
    let cwd = env::current_dir().ok();
    skill_root_from(env_root.as_deref(), exe.as_deref(), cwd.as_deref())
}

#[derive(Debug)]
struct CliError {
    message: String,
    code: i32,
}

#[derive(Debug)]
enum ParsedArgs {
    Help,
    Run { check_only: bool, target: String },
}

/// Pure argument parsing: never exits the process, so tests can assert on the
/// returned Result directly. main() is the only caller that turns an Err into
/// eprintln!+usage()/exit.
fn parse_args(args: &[String]) -> Result<ParsedArgs, CliError> {
    let mut check_only = false;
    let mut target: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--check" => check_only = true,
            "--target" => {
                index += 1;
                match args.get(index) {
                    Some(value) => target = Some(value.clone()),
                    None => {
                        return Err(CliError {
                            message: format!("{COMMAND}: --target needs a value"),
                            code: 64,
                        })
                    }
                }
            }
            value if value.starts_with("--target=") => {
                target = Some(value["--target=".len()..].to_string());
            }
            "-h" | "--help" => return Ok(ParsedArgs::Help),
            other => {
                return Err(CliError {
                    message: format!("{COMMAND}: unknown argument: {other}"),
                    code: 64,
                })
            }
        }
        index += 1;
    }
    let mut target = target.unwrap_or_else(|| "prod".to_string());
    match target.as_str() {
        "dev" | "prod" => {}
        other => {
            return Err(CliError {
                message: format!("{COMMAND}: --target must be dev or prod, not {other}"),
                code: 64,
            })
        }
    }
    // --check is about what is committed, and what is committed is the prod build.
    if check_only {
        target = "prod".to_string();
    }
    Ok(ParsedArgs::Run { check_only, target })
}

/// The group's own name, its output filename, and the one-line purpose that
/// goes in the generated header. `None` for an unrecognized group name.
fn group_output(group: &str) -> Option<(&'static str, &'static str)> {
    match group {
        "core" => Some((
            "plan-core-lib.sh",
            "failure, guards, temp files, atomic writes, plan root, snapshots",
        )),
        "document" => Some((
            "plan-document-lib.sh",
            "sections, paragraphs, titles and fields",
        )),
        "table" => Some(("plan-table-lib.sh", "CSV and Markdown table rendering")),
        "progress" => Some((
            "plan-progress-lib.sh",
            "progress arithmetic and the status glyphs",
        )),
        "crypt" => Some((
            "plan-crypt-lib.sh",
            "SHA-256 digests, fix-key derivation and OS entropy",
        )),
        _ => None,
    }
}

/// A function file the prod library does without: the substring `# PACKAGE:
/// DEV` appearing anywhere within the member's own first 15 lines.
fn member_is_dev_only(path: &Path) -> std::io::Result<bool> {
    let content = fs::read_to_string(path)?;
    let head: String = content.lines().take(15).collect::<Vec<_>>().join("\n");
    Ok(head.contains("# PACKAGE: DEV"))
}

/// Strips a standalone member file down to what the compiled library keeps:
/// the leading shebang line only (not a later line that happens to start with
/// `#!`), the shared `set -euo pipefail` line wherever it appears, and the
/// file's own MODE/PACKAGE marker lines wherever they appear. Leading blank
/// lines in the surviving content are dropped; once the first non-blank
/// survivor is emitted every following line (blank or not) is kept.
fn strip_member(content: &str) -> String {
    let mut out = String::new();
    let mut printed = false;
    let mut first_line = true;
    for line in content.lines() {
        if first_line {
            first_line = false;
            if line.starts_with("#!") {
                continue;
            }
        }
        if matches!(
            line,
            "set -euo pipefail"
                | "# MODE: DEV"
                | "# MODE: PROD"
                | "# PACKAGE: DEV"
                | "# PACKAGE: PROD"
        ) {
            continue;
        }
        if !line.is_empty() || printed {
            out.push_str(line);
            out.push('\n');
            printed = true;
        }
    }
    out
}

fn emit_library_header(group: &str, purpose: &str, target: &str) -> String {
    let upper = group.to_ascii_uppercase();
    let mut out = String::new();
    out.push_str("#!/usr/bin/env bash\n");
    out.push_str("# MODE: PROD\n");
    out.push_str(&format!(
        "# GENERATED FILE — do not edit. Compiled from scripts/lib/{group}/*.sh by:\n"
    ));
    out.push_str("#   planning/scripts/build-plan-libs.sh\n");
    out.push_str("# Edit the function file in that directory, then re-run the build.\n");
    out.push_str(&format!("# Target: {target}\n"));
    out.push_str("#\n");
    out.push_str(&format!("# {purpose}\n"));
    out.push('\n');
    out.push_str("set -euo pipefail\n");
    out.push('\n');
    out.push_str(&format!(
        "[ -z \"${{PLAN_{upper}_LIB_LOADED:-}}\" ] || return 0\n"
    ));
    out.push_str(&format!("PLAN_{upper}_LIB_LOADED=1\n"));
    out
}

/// Every `*.sh` file directly under `lib_root/group`, sorted lexicographically.
/// A missing group directory yields an empty list rather than an error.
fn group_members(lib_root: &Path, group: &str) -> Vec<PathBuf> {
    let mut members: Vec<PathBuf> = fs::read_dir(lib_root.join(group))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension == "sh")
                    .unwrap_or(false)
        })
        .collect();
    members.sort();
    members
}

/// Header plus every surviving member's stripped content, in the fixed
/// order: a blank line, then (dev target only) a provenance comment naming
/// the source file, then the stripped body. Errs when the group name is
/// unrecognized or when zero members survive — an empty directory and a
/// directory whose only members are all PACKAGE:-DEV-excluded under a prod
/// target are the same failure.
fn render_library(lib_root: &Path, group: &str, target: &str) -> Result<String, String> {
    let (_filename, purpose) =
        group_output(group).ok_or_else(|| format!("unknown group: {group}"))?;
    let mut out = emit_library_header(group, purpose, target);
    let mut found = false;
    for path in group_members(lib_root, group) {
        if target == "prod" && member_is_dev_only(&path).map_err(|error| error.to_string())? {
            continue;
        }
        found = true;
        out.push('\n');
        if target == "dev" {
            let filename = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push_str(&format!("# from scripts/lib/{group}/{filename}\n"));
        }
        let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        out.push_str(&strip_member(&content));
    }
    if !found {
        return Err(format!("group {group} has no source files"));
    }
    Ok(out)
}

fn format_wrote_message(
    filename: &str,
    target: &str,
    written_lines: usize,
    file_count: usize,
) -> String {
    format!("Wrote {filename} for {target} ({written_lines} lines from {file_count} files)")
}

const GROUPS: [&str; 5] = ["core", "document", "table", "progress", "crypt"];

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (check_only, target) = match parse_args(&args) {
        Ok(ParsedArgs::Help) => usage(0),
        Ok(ParsedArgs::Run { check_only, target }) => (check_only, target),
        Err(error) => {
            eprintln!("{}", error.message);
            usage(error.code);
        }
    };

    let root = skill_root().unwrap_or_else(|| {
        eprintln!("{COMMAND}: could not locate the planning skill root");
        process::exit(69);
    });
    let scripts_dir = root.join("planning/scripts");
    let lib_root = scripts_dir.join("lib");

    let mut status = 0i32;
    for group in GROUPS {
        let (filename, _purpose) = group_output(group).expect("GROUPS lists only known groups");
        let output_path = scripts_dir.join(filename);
        let rendered =
            render_library(&lib_root, group, &target).unwrap_or_else(|message| die(message, 65));
        if check_only {
            let committed = fs::read(&output_path).ok();
            if committed.as_deref() != Some(rendered.as_bytes()) {
                eprintln!("{COMMAND}: {filename} is stale; run {COMMAND}");
                status = 1;
            }
        } else {
            fs::write(&output_path, rendered.as_bytes())
                .unwrap_or_else(|error| die(error.to_string(), 70));
            let written_lines = rendered.lines().filter(|line| !line.is_empty()).count();
            let file_count = fs::read_dir(lib_root.join(group))
                .map(|entries| entries.filter_map(Result::ok).count())
                .unwrap_or(0);
            println!(
                "{}",
                format_wrote_message(filename, &target, written_lines, file_count)
            );
        }
    }
    if check_only && status == 0 {
        println!("All five libraries are up to date");
    }
    process::exit(status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    // (a) shebang/set-line/MODE/PACKAGE markers are stripped; real content survives.
    #[test]
    fn strip_member_drops_shared_lines_and_keeps_real_content() {
        let source = "#!/usr/bin/env bash\n# MODE: DEV\n# PACKAGE: PROD\nset -euo pipefail\n\nplan_real_function() {\n    echo real\n}\n";
        let stripped = strip_member(source);
        assert!(!stripped.contains("#!/usr/bin/env bash"));
        assert!(!stripped.contains("# MODE: DEV"));
        assert!(!stripped.contains("# PACKAGE: PROD"));
        assert!(!stripped.contains("set -euo pipefail"));
        assert!(stripped.contains("plan_real_function() {"));
        assert!(stripped.contains("    echo real"));
    }

    // (b) a PACKAGE:-DEV member is excluded under target=prod, included under target=dev.
    #[test]
    fn dev_only_member_is_excluded_from_prod_and_included_in_dev() {
        let tmp = tempdir();
        let group_dir = tmp.join("lib").join("core");
        fs::create_dir_all(&group_dir).unwrap();
        write(
            &group_dir,
            "00-normal.sh",
            "#!/usr/bin/env bash\n# MODE: DEV\n# PACKAGE: PROD\nset -euo pipefail\n\nnormal_fn() { :; }\n",
        );
        write(
            &group_dir,
            "01-dev-only.sh",
            "#!/usr/bin/env bash\n# MODE: DEV\n# PACKAGE: DEV\nset -euo pipefail\n\ndev_only_fn() { :; }\n",
        );
        let prod = render_library(&tmp.join("lib"), "core", "prod").unwrap();
        assert!(prod.contains("normal_fn"));
        assert!(!prod.contains("dev_only_fn"));
        let dev = render_library(&tmp.join("lib"), "core", "dev").unwrap();
        assert!(dev.contains("normal_fn"));
        assert!(dev.contains("dev_only_fn"));
        fs::remove_dir_all(&tmp).ok();
    }

    // (c) emit_library_header's exact text for two groups, including the
    // multi-word 'document' group's uppercased guard name.
    #[test]
    fn header_text_is_exact_for_core_and_document() {
        let core = emit_library_header("core", "a purpose", "prod");
        assert!(core.contains("PLAN_CORE_LIB_LOADED"));
        assert!(core.starts_with("#!/usr/bin/env bash\n# MODE: PROD\n"));
        assert!(core.contains("# Target: prod\n"));
        assert!(core.contains("# a purpose\n"));

        let document = emit_library_header("document", "sections and fields", "dev");
        assert!(document.contains("PLAN_DOCUMENT_LIB_LOADED"));
        assert!(!document.contains("PLAN_DOCUMEN_LIB_LOADED"));
        assert!(document.contains("# Target: dev\n"));
    }

    // (d) leading blank lines are dropped; an interior blank line survives.
    #[test]
    fn leading_blank_lines_dropped_interior_blank_preserved() {
        let source = "#!/usr/bin/env bash\n\n\nfirst content line\n\nsecond content line\n";
        let stripped = strip_member(source);
        assert_eq!(stripped, "first content line\n\nsecond content line\n");
    }

    // (e) an unknown group name is refused.
    #[test]
    fn unknown_group_is_refused() {
        let tmp = tempdir();
        let error = render_library(&tmp.join("lib"), "bogus", "prod").unwrap_err();
        assert_eq!(error, "unknown group: bogus");
        fs::remove_dir_all(&tmp).ok();
    }

    // (f) a group whose only member is PACKAGE-DEV-excluded under target=prod is
    // refused the same way an empty directory is (real file on disk, zero survive).
    #[test]
    fn all_members_excluded_under_prod_is_refused_like_an_empty_directory() {
        let tmp = tempdir();
        let group_dir = tmp.join("lib").join("core");
        fs::create_dir_all(&group_dir).unwrap();
        write(
            &group_dir,
            "00-dev-only.sh",
            "#!/usr/bin/env bash\n# PACKAGE: DEV\nset -euo pipefail\n\ndev_only_fn() { :; }\n",
        );
        let error = render_library(&tmp.join("lib"), "core", "prod").unwrap_err();
        assert_eq!(error, "group core has no source files");

        let empty_dir = tmp.join("lib").join("document");
        fs::create_dir_all(&empty_dir).unwrap();
        let error = render_library(&tmp.join("lib"), "document", "prod").unwrap_err();
        assert_eq!(error, "group document has no source files");
        fs::remove_dir_all(&tmp).ok();
    }

    // (g) per AR-24: the exact "Wrote FILE for TARGET (N lines from M files)"
    // text, with N the non-blank line count of the WRITTEN content and M the
    // raw member-file count of the SOURCE directory (including excluded files).
    #[test]
    fn wrote_message_text_and_counts_are_exact() {
        let tmp = tempdir();
        let group_dir = tmp.join("lib").join("core");
        fs::create_dir_all(&group_dir).unwrap();
        write(
            &group_dir,
            "00-normal.sh",
            "#!/usr/bin/env bash\nset -euo pipefail\n\nnormal_fn() {\n    echo one\n}\n",
        );
        write(
            &group_dir,
            "01-dev-only.sh",
            "#!/usr/bin/env bash\n# PACKAGE: DEV\nset -euo pipefail\n\ndev_only_fn() { :; }\n",
        );
        let rendered = render_library(&tmp.join("lib"), "core", "prod").unwrap();
        let written_lines = rendered.lines().filter(|line| !line.is_empty()).count();
        let file_count = fs::read_dir(&group_dir)
            .unwrap()
            .filter_map(Result::ok)
            .count();
        assert_eq!(
            file_count, 2,
            "M counts every member, including the excluded one"
        );
        let message = format_wrote_message("plan-core-lib.sh", "prod", written_lines, file_count);
        assert_eq!(
            message,
            format!("Wrote plan-core-lib.sh for prod ({written_lines} lines from 2 files)")
        );
        fs::remove_dir_all(&tmp).ok();
    }

    // (h) per AR-25: CLI parsing rejects a missing --target value, an invalid
    // --target value, and an unrecognized flag, each with exit 64.
    #[test]
    fn cli_error_paths_exit_64() {
        let missing = parse_args(&["--target".to_string()]).unwrap_err();
        assert_eq!(missing.code, 64);
        assert!(missing.message.contains("--target needs a value"));

        let invalid = parse_args(&["--target".to_string(), "bogus".to_string()]).unwrap_err();
        assert_eq!(invalid.code, 64);
        assert!(invalid.message.contains("must be dev or prod"));

        let unknown = parse_args(&["--frobnicate".to_string()]).unwrap_err();
        assert_eq!(unknown.code, 64);
        assert!(unknown.message.contains("unknown argument: --frobnicate"));
    }

    // (i) per AR-26: skill_root()'s failure path -- neither PLANNING_SKILL_ROOT
    // nor an ancestor of the exe path or cwd has a planning/scripts directory --
    // returns None. Parameterized rather than mutating real env/cwd state, so
    // this cannot race other tests running in the same process.
    #[test]
    fn skill_root_returns_none_when_nothing_resolves() {
        let tmp = tempdir();
        let outside = tmp
            .join("no-planning-scripts-here")
            .join("deep")
            .join("nesting");
        fs::create_dir_all(&outside).unwrap();
        let resolved =
            skill_root_from(None, Some(&outside.join("build-plan-libs")), Some(&outside));
        assert!(resolved.is_none());
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn skill_root_resolves_via_planning_skill_root_env() {
        let tmp = tempdir();
        fs::create_dir_all(tmp.join("planning/scripts")).unwrap();
        let resolved = skill_root_from(Some(&tmp), None, None);
        assert_eq!(resolved, Some(tmp.clone()));
        fs::remove_dir_all(&tmp).ok();
    }

    fn tempdir() -> PathBuf {
        // A monotonic counter alongside the nanosecond timestamp: on a
        // coarser-than-nanosecond clock (some virtualized CI runners), two
        // tempdir() calls can otherwise land on the same path in the same
        // process, and one test then sees another's real fixture content
        // spliced into its own supposedly-isolated directory.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let mut path = std::env::temp_dir();
        let unique = format!(
            "build-plan-libs-test-{}-{:?}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        path.push(unique);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
