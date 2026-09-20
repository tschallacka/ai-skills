// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const COMMAND: &str = "verify-skill-load.sh";

const USAGE: &str = "Usage: verify-skill-load.sh --part <name> --token <hex> [<skill-directory>]\n       verify-skill-load.sh --help\n";

fn usage(code: i32) -> ! {
    print!("{USAGE}");
    process::exit(code);
}

/// Mirrors src/plan-mutate/src/main.rs's own skill_root(), and this plan's
/// own build-plan-libs/generate-skill-docs/overview-state precedent:
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

#[derive(Debug, PartialEq, Eq)]
enum ParsedArgs {
    Help,
    /// Bad usage: bash's own `usage` (implicit rc=64) prints only the Usage
    /// text, with no extra message.
    BadUsage,
    Run {
        part: String,
        token: String,
        skill_dir: Option<String>,
    },
}

/// Pure argument parsing: never exits the process, so tests can assert on
/// the parse result directly. Mirrors verify-skill-load.sh's own case-based
/// while loop exactly: --part and --token each consume a following value
/// (missing one is bad usage), any other flag (starts with '-') is bad
/// usage, and at most one positional argument (the skill directory) is
/// accepted -- a second positional is bad usage. Both --part and --token
/// are required.
fn parse_args(args: &[String]) -> ParsedArgs {
    let mut part: Option<String> = None;
    let mut token: Option<String> = None;
    let mut skill_dir: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return ParsedArgs::Help,
            "--part" => {
                if i + 1 >= args.len() {
                    return ParsedArgs::BadUsage;
                }
                part = Some(args[i + 1].clone());
                i += 2;
            }
            "--token" => {
                if i + 1 >= args.len() {
                    return ParsedArgs::BadUsage;
                }
                token = Some(args[i + 1].clone());
                i += 2;
            }
            arg if arg.starts_with('-') => return ParsedArgs::BadUsage,
            arg => {
                if skill_dir.is_some() {
                    return ParsedArgs::BadUsage;
                }
                skill_dir = Some(arg.to_string());
                i += 1;
            }
        }
    }
    match (part, token) {
        (Some(part), Some(token)) => ParsedArgs::Run {
            part,
            token,
            skill_dir,
        },
        _ => ParsedArgs::BadUsage,
    }
}

/// Per AR-36/AR-37: a deliberate divergence from bash's own unescaped
/// `grep -oE "SKILL-LOAD-PROOF part=$part token=[0-9a-f]+"` interpolation,
/// which can crash on a --part value that makes the pattern an invalid ERE
/// (no real caller ever supplies a part name containing a regex
/// metacharacter -- every real part name is the plain string "part-N").
/// This is a LITERAL SUBSTRING search for `SKILL-LOAD-PROOF part=<part>
/// token=`, never compiled as a pattern, followed by taking the run of
/// lowercase hex digits immediately after it -- preserving bash's own
/// `[0-9a-f]+` boundary detection for the token itself. Returns the token
/// from the FIRST matching line, matching bash's `| head -1`.
fn extract_token(content: &str, part: &str) -> Option<String> {
    let needle = format!("SKILL-LOAD-PROOF part={part} token=");
    for line in content.lines() {
        if let Some(pos) = line.find(&needle) {
            let rest = &line[pos + needle.len()..];
            let hex_len = rest
                .find(|c: char| !(c.is_ascii_digit() || ('a'..='f').contains(&c)))
                .unwrap_or(rest.len());
            if hex_len > 0 {
                return Some(rest[..hex_len].to_string());
            }
        }
    }
    None
}

/// The result of checking one --part/--token pair against a skill
/// directory, carrying the exact message text so tests can assert on it
/// without spawning a process. Mirrors bash's own four terminal outcomes
/// (no such part, no load-proof line, refused, verified) plus their exit
/// codes -- pure with respect to the filesystem read already performed by
/// the caller.
enum Outcome {
    NoSuchPart { stderr: String },
    NoLoadProof { stderr: String },
    Refused { stderr: [String; 3] },
    Verified { stdout: String },
}

impl Outcome {
    fn exit_code(&self) -> i32 {
        match self {
            Outcome::NoSuchPart { .. } => 66,
            Outcome::NoLoadProof { .. } => 65,
            Outcome::Refused { .. } => 1,
            Outcome::Verified { .. } => 0,
        }
    }
}

/// Reads part_file (already confirmed to exist) and decides the outcome.
/// Kept separate from main() so both the token-extraction and the
/// message/exit-code decision are directly testable.
fn evaluate(part: &str, token: &str, skill_dir: &Path, part_file: &Path, content: &str) -> Outcome {
    let actual = match extract_token(content, part) {
        Some(actual) => actual,
        None => {
            return Outcome::NoLoadProof {
                stderr: format!(
                    "{} carries no load-proof line; run generate-skill-docs.sh",
                    part_file.display()
                ),
            }
        }
    };

    if token == actual {
        return Outcome::Verified {
            stdout: format!("verified: {part} was read at least as far as its load-sanity line"),
        };
    }

    let relative = part_file
        .strip_prefix(skill_dir)
        .unwrap_or(part_file)
        .display()
        .to_string();
    Outcome::Refused {
        stderr: [
            format!("refused: the token for {part} does not match — you have not finished"),
            "reading it, or you are recalling a token from a version that has since".to_string(),
            format!("regenerated. Re-read {relative} and find the current line."),
        ],
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (part, token, skill_dir_arg) = match parse_args(&args) {
        ParsedArgs::Help => usage(0),
        ParsedArgs::BadUsage => usage(64),
        ParsedArgs::Run {
            part,
            token,
            skill_dir,
        } => (part, token, skill_dir),
    };

    let skill_dir = match skill_dir_arg {
        Some(explicit) => PathBuf::from(explicit),
        None => {
            let root = skill_root().unwrap_or_else(|| {
                eprintln!("{COMMAND}: could not locate the planning skill root");
                process::exit(69);
            });
            root.join("planning")
        }
    };

    let part_file = skill_dir.join("parts").join(format!("{part}.md"));
    let outcome = match fs::read_to_string(&part_file) {
        Ok(content) => evaluate(&part, &token, &skill_dir, &part_file, &content),
        Err(_) => Outcome::NoSuchPart {
            stderr: format!("no such part: {part} (looked for {})", part_file.display()),
        },
    };

    let code = outcome.exit_code();
    match outcome {
        Outcome::NoSuchPart { stderr } | Outcome::NoLoadProof { stderr } => eprintln!("{stderr}"),
        Outcome::Refused { stderr } => {
            for line in stderr {
                eprintln!("{line}");
            }
        }
        Outcome::Verified { stdout } => println!("{stdout}"),
    }
    process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    // (a) token extraction finds the correct hex token from a synthetic part
    // file containing a real SKILL-LOAD-PROOF line for the given part name.
    #[test]
    fn extract_token_finds_the_hex_token_for_the_named_part() {
        let content =
            "line one\n<!-- SKILL-LOAD-PROOF part=part-1 token=64647e1270fa28f0 -->\nline three\n";
        assert_eq!(
            extract_token(content, "part-1"),
            Some("64647e1270fa28f0".to_string())
        );
    }

    // extract_token stops the token at the first non-hex character and
    // ignores a load-proof line for a DIFFERENT part name.
    #[test]
    fn extract_token_stops_at_first_non_hex_character_and_ignores_other_parts() {
        let content = "<!-- SKILL-LOAD-PROOF part=part-2 token=aaaaaaaaaaaaaaaa -->\n<!-- SKILL-LOAD-PROOF part=part-1 token=deadbeefcafefeed extra text -->\n";
        assert_eq!(
            extract_token(content, "part-1"),
            Some("deadbeefcafefeed".to_string())
        );
    }

    // (b) a part file with no SKILL-LOAD-PROOF line at all produces the
    // exact "carries no load-proof line" message and exit 65.
    #[test]
    fn no_load_proof_line_produces_the_exact_message_and_exit_65() {
        let skill_dir = PathBuf::from("/skill");
        let part_file = skill_dir.join("parts/part-1.md");
        let outcome = evaluate(
            "part-1",
            "deadbeef",
            &skill_dir,
            &part_file,
            "no load-proof line anywhere in this file\n",
        );
        assert_eq!(outcome.exit_code(), 65);
        match outcome {
            // The path is printed the way the platform writes it (`\` on
            // Windows), so the expectation is built from the same path
            // rather than spelled with unix slashes.
            Outcome::NoLoadProof { stderr } => assert_eq!(
                stderr,
                format!(
                    "{} carries no load-proof line; run generate-skill-docs.sh",
                    part_file.display()
                )
            ),
            _ => panic!("expected NoLoadProof"),
        }
    }

    // (c) a missing part file produces the exact "no such part" message and
    // exit 66 (exercised at the main()-adjacent level: a nonexistent path
    // fails to read, matching bash's [ -f ] check).
    #[test]
    fn missing_part_file_read_failure_maps_to_no_such_part_exit_66() {
        let skill_dir = PathBuf::from("/nonexistent-skill-dir-for-test");
        let part_file = skill_dir.join("parts/part-9.md");
        assert!(fs::read_to_string(&part_file).is_err());
        let outcome = Outcome::NoSuchPart {
            stderr: format!("no such part: part-9 (looked for {})", part_file.display()),
        };
        assert_eq!(outcome.exit_code(), 66);
    }

    // (d) a matching token produces the exact "verified: ..." message and
    // exit 0.
    #[test]
    fn matching_token_produces_verified_message_and_exit_0() {
        let skill_dir = PathBuf::from("/skill");
        let part_file = skill_dir.join("parts/part-1.md");
        let content = "<!-- SKILL-LOAD-PROOF part=part-1 token=abc123 -->\n";
        let outcome = evaluate("part-1", "abc123", &skill_dir, &part_file, content);
        assert_eq!(outcome.exit_code(), 0);
        match outcome {
            Outcome::Verified { stdout } => assert_eq!(
                stdout,
                "verified: part-1 was read at least as far as its load-sanity line"
            ),
            _ => panic!("expected Verified"),
        }
    }

    // (e) a mismatched token produces the exact three-line refused message
    // (including the em-dash) and exit 1.
    #[test]
    fn mismatched_token_produces_the_exact_three_line_refused_message_and_exit_1() {
        let skill_dir = PathBuf::from("/skill");
        let part_file = skill_dir.join("parts/part-1.md");
        let content = "<!-- SKILL-LOAD-PROOF part=part-1 token=abc123 -->\n";
        let outcome = evaluate("part-1", "wrongtoken", &skill_dir, &part_file, content);
        assert_eq!(outcome.exit_code(), 1);
        match outcome {
            Outcome::Refused { stderr } => {
                assert_eq!(
                    stderr[0],
                    "refused: the token for part-1 does not match — you have not finished"
                );
                assert_eq!(
                    stderr[1],
                    "reading it, or you are recalling a token from a version that has since"
                );
                assert_eq!(
                    stderr[2],
                    "regenerated. Re-read parts/part-1.md and find the current line."
                );
            }
            _ => panic!("expected Refused"),
        }
    }

    // (f) CLI parsing rejects a missing --part value, a missing --token
    // value, an unrecognized flag, and more than one positional argument,
    // all with exit 64 (BadUsage).
    #[test]
    fn cli_parsing_rejects_bad_usage_shapes() {
        assert_eq!(parse_args(&args(&["--part"])), ParsedArgs::BadUsage);
        assert_eq!(
            parse_args(&args(&["--part", "part-1", "--token"])),
            ParsedArgs::BadUsage
        );
        assert_eq!(
            parse_args(&args(&["--part", "part-1", "--token", "abc", "--bogus"])),
            ParsedArgs::BadUsage
        );
        assert_eq!(
            parse_args(&args(&[
                "--part", "part-1", "--token", "abc", "dir1", "dir2"
            ])),
            ParsedArgs::BadUsage
        );
        assert_eq!(
            parse_args(&args(&["--part", "part-1"])),
            ParsedArgs::BadUsage
        );
        assert_eq!(parse_args(&args(&["--token", "abc"])), ParsedArgs::BadUsage);
    }

    #[test]
    fn cli_parsing_accepts_help_and_a_well_formed_run() {
        assert_eq!(parse_args(&args(&["--help"])), ParsedArgs::Help);
        assert_eq!(parse_args(&args(&["-h"])), ParsedArgs::Help);
        assert_eq!(
            parse_args(&args(&["--part", "part-1", "--token", "abc", "dir1"])),
            ParsedArgs::Run {
                part: "part-1".to_string(),
                token: "abc".to_string(),
                skill_dir: Some("dir1".to_string()),
            }
        );
    }

    // (g) skill_root()'s failure path (PLANNING_SKILL_ROOT unset, cwd and
    // exe path both outside any planning/scripts-containing tree) is
    // exercised at the real compiled-binary subprocess level in
    // tests/verify_skill_load_flow.rs, mirroring the goals 8/9/10 AR-26
    // precedent exactly. skill_root_from's own pure logic is unit-tested
    // here instead of hitting the real environment.
    #[test]
    fn skill_root_from_returns_none_when_nothing_resolves() {
        let resolved = skill_root_from(
            Some(Path::new("/definitely/not/a/skill/root")),
            Some(Path::new("/also/not/one")),
            Some(Path::new("/still/not/one")),
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn skill_root_from_prefers_a_valid_env_root() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let resolved = skill_root_from(
            Some(repo_root),
            Some(Path::new("/not/a/root")),
            Some(Path::new("/also/not/a/root")),
        );
        assert_eq!(resolved, Some(repo_root.to_path_buf()));
    }

    // (h) per AR-36/AR-37: a --part value containing a literal regex
    // metacharacter that would make an equivalent ERE pattern syntactically
    // invalid (an unbalanced '[') is treated as an ordinary literal
    // substring -- no panic, crash, or undefined exit code for any --part
    // input.
    #[test]
    fn a_part_value_with_a_regex_metacharacter_is_a_safe_literal_substring_when_present() {
        let part = "part-1[";
        let content = "<!-- SKILL-LOAD-PROOF part=part-1[ token=deadbeef -->\n";
        assert_eq!(extract_token(content, part), Some("deadbeef".to_string()));

        let skill_dir = PathBuf::from("/skill");
        let part_file = skill_dir.join("parts/part-1[.md");
        let outcome = evaluate(part, "deadbeef", &skill_dir, &part_file, content);
        assert_eq!(outcome.exit_code(), 0);
    }

    #[test]
    fn a_part_value_with_a_regex_metacharacter_is_a_safe_literal_substring_when_absent() {
        let part = "part-1[";
        let content = "no load-proof line here at all\n";
        assert_eq!(extract_token(content, part), None);

        let skill_dir = PathBuf::from("/skill");
        let part_file = skill_dir.join("parts/part-1[.md");
        let outcome = evaluate(part, "deadbeef", &skill_dir, &part_file, content);
        assert_eq!(outcome.exit_code(), 65);
    }
}
