// MODE: DEV
// PACKAGE: PROD
use plan_crypt::sha256::{hex, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const COMMAND: &str = "generate-skill-docs.sh";

const USAGE: &str = "Usage: generate-skill-docs.sh [--check] [<skill-directory>]\n       generate-skill-docs.sh --help\n";

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
enum ParsedArgs {
    Help,
    /// Bad usage (implicit rc=64): prints only the Usage text, with no
    /// extra message -- this script never names what was wrong.
    BadUsage,
    Run {
        check_only: bool,
        skill_dir: Option<String>,
    },
}

/// Pure argument parsing: never exits the process, so tests can assert on
/// the returned value directly. Only position 1 is ever checked for a flag
/// (`-h`/`--help`/`--check`), at most one argument may remain after that,
/// and it must not start with `-`:
///   case "${1:-}" in -h|--help) usage 0;; --check) check_only=true; shift;; esac
///   [ "$#" -le 1 ] || usage
///   case "${1:-}" in -*) usage;; esac
fn parse_args(args: &[String]) -> ParsedArgs {
    let mut check_only = false;
    let mut rest = args;
    if let Some(first) = rest.first() {
        if first == "-h" || first == "--help" {
            return ParsedArgs::Help;
        }
        if first == "--check" {
            check_only = true;
            rest = &rest[1..];
        }
    }
    if rest.len() > 1 {
        return ParsedArgs::BadUsage;
    }
    if rest.first().is_some_and(|first| first.starts_with('-')) {
        return ParsedArgs::BadUsage;
    }
    ParsedArgs::Run {
        check_only,
        skill_dir: rest.first().cloned(),
    }
}

const PARTS: [&str; 4] = ["part-1", "part-2", "part-3", "part-4"];
const PART_TITLES: [&str; 4] = [
    "1 of 4 — setup, operating rules, gates, and establishing the plan boundary",
    "2 of 4 — create the plan directory",
    "3 of 4 — mandatory classification and independent review",
    "4 of 4 — resume and update the plan",
];

/// Every SKILL_SECTION whose targets= list contains `wanted`, concatenated in
/// source order, verbatim (each kept line plus its own trailing newline).
fn skill_section_body(source: &str, wanted: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in source.lines() {
        if let Some(header) = line.strip_prefix("<!-- SKILL_SECTION:START ") {
            inside = section_targets(header).iter().any(|t| t == wanted);
            continue;
        }
        if line.starts_with("<!-- SKILL_SECTION:END ") {
            inside = false;
            continue;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Parses `<name> targets=<a,b,c> -->` (the header text after the literal
/// `<!-- SKILL_SECTION:START ` prefix) into its comma-separated targets list.
fn section_targets(header: &str) -> Vec<String> {
    let inner = header.strip_suffix("-->").unwrap_or(header).trim_end();
    let Some(ws_pos) = inner.find(char::is_whitespace) else {
        return Vec::new();
    };
    let rest = inner[ws_pos..].trim_start();
    let Some(list) = rest.strip_prefix("targets=") else {
        return Vec::new();
    };
    list.split(',').map(|s| s.to_string()).collect()
}

/// The YAML block between the first two literal `---` lines, verbatim.
fn front_matter(source: &str) -> String {
    let mut out = String::new();
    let mut n = 0;
    for line in source.lines() {
        if line == "---" {
            n += 1;
            out.push_str(line);
            out.push('\n');
            if n == 2 {
                break;
            }
            continue;
        }
        if n == 1 {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn hash_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex(&digest.finish())
}

/// T86. Inserts a load-sanity line in the last fifth of the body, derived
/// from the body's own SHA-256. Per AR-28: `raw_body` is trimmed of ALL
/// trailing newline/blank-line characters first, so the hash/count/
/// insertion-point arithmetic runs over that trimmed content -- not the raw
/// section text, which may carry a trailing blank line before its
/// SKILL_SECTION:END marker. The proof block is inserted strictly AFTER
/// line `at`, never before or in place of it.
fn plant_load_proof(part: &str, raw_body: &str) -> String {
    let body = raw_body.trim_end_matches('\n');
    let total = body.lines().count() as i64;
    let mut floor = total * 4 / 5;
    if floor >= total {
        floor = total - 1;
    }
    if floor < 0 {
        floor = 0;
    }
    let hash = hash_hex(body.as_bytes());
    let token = &hash[0..16];
    let mut span = total - floor + 1;
    if span <= 0 {
        span = 1;
    }
    let offset_dec = u32::from_str_radix(&hash[16..24], 16).unwrap_or(0);
    let at = floor + (offset_dec as i64 % span);
    let mut out = String::new();
    for (index, line) in body.lines().enumerate() {
        let lineno = (index + 1) as i64;
        out.push_str(line);
        out.push('\n');
        if lineno == at {
            out.push('\n');
            out.push_str(&format!(
                "<!-- SKILL-LOAD-PROOF part={part} token={token} -->\n"
            ));
            out.push('\n');
        }
    }
    out
}

/// MODE banner, the load-sanity instructions (T86), then the section body
/// with its planted proof line. Refuses (exit 65) when the part's section
/// body is empty or whitespace-only.
fn emit_part(source: &str, source_file: &Path, index: usize) -> Result<String, String> {
    let name = PARTS[index];
    let title = PART_TITLES[index];
    let body = skill_section_body(source, name);
    if body.chars().all(char::is_whitespace) {
        return Err(format!(
            "no content for part {name} (targets={name} matched nothing in {})",
            source_file.display()
        ));
    }
    let mut out = String::new();
    out.push_str("<!-- MODE: PROD -->\n");
    out.push_str(
        "> Generated from `skill-source.txt` by `scripts/generate-skill-docs.sh` — do not edit.\n",
    );
    out.push_str(&format!("> Part {title}.\n"));
    out.push_str(">\n");
    out.push_str("> Before treating this part as read: find the line below matching\n");
    out.push_str(&format!(
        "> `<!-- SKILL-LOAD-PROOF part={name} token=... -->` — its position moves on every\n"
    ));
    out.push_str(&format!(
        "> regeneration — and run `planning/scripts/verify-skill-load.sh --part {name}\n"
    ));
    out.push_str("> --token <the-token-you-found>` before continuing. Naming a token is not\n");
    out.push_str("> enough; the command must succeed. If it refuses, you have not finished\n");
    out.push_str("> reading this part.\n\n");
    out.push_str(&plant_load_proof(name, &body));
    Ok(out)
}

/// SKILL.md: front matter, a short pointer table, and the same load-sanity
/// contract stated once. This planning-specific prose is NOT meant to be
/// generic.
fn emit_index(source: &str) -> String {
    let mut out = front_matter(source);
    out.push_str("<!-- MODE: PROD -->\n\n");
    out.push_str("# Planning\n\n");
    out.push_str("Use this skill to turn an initiative into a directory of Markdown files\n");
    out.push_str("that another agent can resume and execute without reconstructing missing\n");
    out.push_str("context. Do not use it for a small, self-contained change or a temporary\n");
    out.push_str("in-chat plan.\n\n");
    out.push_str("The full skill is generated from a single authored source\n");
    out.push_str("(`skill-source.txt`) into this short index plus the parts below, because\n");
    out.push_str("the whole thing is 89,860 bytes and at least one harness this repo runs\n");
    out.push_str("under silently truncates a file read past roughly 25,000 tokens with no\n");
    out.push_str("notice anywhere (`.agents/knowledge/agent-read-limits.md`). Read the part\n");
    out.push_str("that applies to what you are doing now; each is well under that budget on\n");
    out.push_str("its own.\n\n");
    out.push_str("| Part | Covers |\n|---|---|\n");
    out.push_str("| [parts/part-1.md](parts/part-1.md) | Setup, operating rules, tool/context-limit discipline, hard planning gates, establishing the plan boundary |\n");
    out.push_str("| [parts/part-2.md](parts/part-2.md) | Creating the plan directory |\n");
    out.push_str(
        "| [parts/part-3.md](parts/part-3.md) | Mandatory classification and independent review |\n",
    );
    out.push_str("| [parts/part-4.md](parts/part-4.md) | Resuming and updating a plan |\n\n");
    out.push_str("Every part carries a load-sanity check (T86): a hidden line at a random\n");
    out.push_str("position near its end, and a command (`planning/scripts/verify-skill-load.sh\n");
    out.push_str("with --part <N> --token <token>`) that must succeed before the part counts\n");
    out.push_str("as read. Stating a token from memory or from this index is not that command\n");
    out.push_str("succeeding; the check exists because reporting a token is claimable and\n");
    out.push_str("running the command against the real file is not.\n");
    out
}

/// Trims all trailing newlines from `content` and appends exactly one.
fn normalize_trailing_newline(content: &str) -> String {
    format!("{}\n", content.trim_end_matches('\n'))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (check_only, skill_dir_arg) = match parse_args(&args) {
        ParsedArgs::Help => usage(0),
        ParsedArgs::BadUsage => usage(64),
        ParsedArgs::Run {
            check_only,
            skill_dir,
        } => (check_only, skill_dir),
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
    let source_file = skill_dir.join("skill-source.txt");
    let source = fs::read_to_string(&source_file).unwrap_or_else(|_| {
        die(
            format!("source skill not found: {}", source_file.display()),
            66,
        )
    });

    let parts_dir = skill_dir.join("parts");
    if !check_only {
        fs::create_dir_all(&parts_dir).unwrap_or_else(|error| die(error.to_string(), 70));
    }

    let mut stale = false;

    let index_content = normalize_trailing_newline(&emit_index(&source));
    write_or_check(
        &skill_dir.join("SKILL.md"),
        &index_content,
        &skill_dir,
        check_only,
        &mut stale,
    );

    for (index, part) in PARTS.iter().enumerate() {
        let rendered =
            emit_part(&source, &source_file, index).unwrap_or_else(|message| die(message, 65));
        let content = normalize_trailing_newline(&rendered);
        write_or_check(
            &parts_dir.join(format!("{part}.md")),
            &content,
            &skill_dir,
            check_only,
            &mut stale,
        );
    }

    if check_only {
        if stale {
            process::exit(1);
        }
        println!("SKILL.md and parts/ are up to date with skill-source.txt");
    } else {
        println!(
            "Wrote {} and {}/parts/{{{}}}.md",
            skill_dir.join("SKILL.md").display(),
            skill_dir.display(),
            PARTS.join(",")
        );
    }
}

fn write_or_check(
    target: &Path,
    content: &str,
    skill_dir: &Path,
    check_only: bool,
    stale: &mut bool,
) {
    if check_only {
        let committed = fs::read(target).ok();
        if committed.as_deref() != Some(content.as_bytes()) {
            let relative = target.strip_prefix(skill_dir).unwrap_or(target);
            eprintln!("{} is stale; run {COMMAND}", relative.display());
            *stale = true;
        }
        return;
    }
    fs::write(target, content).unwrap_or_else(|error| die(error.to_string(), 70));
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_SOURCE: &str = "---\ntitle: x\n---\n<!-- SKILL_SECTION:START alpha targets=part-1 -->\nAlpha body line one.\nAlpha body line two.\n<!-- SKILL_SECTION:END alpha -->\n<!-- SKILL_SECTION:START beta targets=part-1,part-2 -->\nBeta body line.\n<!-- SKILL_SECTION:END beta -->\n";

    // (a) skill_section_body concatenates every matching SKILL_SECTION, in
    // source order, verbatim, across a synthetic multi-section fixture.
    #[test]
    fn skill_section_body_concatenates_matching_sections_in_order() {
        let body = skill_section_body(FIXTURE_SOURCE, "part-1");
        assert_eq!(
            body,
            "Alpha body line one.\nAlpha body line two.\nBeta body line.\n"
        );
    }

    // (b) a section listing two targets is picked up by both.
    #[test]
    fn a_section_with_two_targets_is_matched_by_both() {
        let part1 = skill_section_body(FIXTURE_SOURCE, "part-1");
        let part2 = skill_section_body(FIXTURE_SOURCE, "part-2");
        assert!(part1.contains("Beta body line."));
        assert_eq!(part2, "Beta body line.\n");
    }

    // (c) front_matter extracts exactly the text between the first two '---'
    // lines and nothing after the second.
    #[test]
    fn front_matter_extracts_between_first_two_markers() {
        assert_eq!(front_matter(FIXTURE_SOURCE), "---\ntitle: x\n---\n");
    }

    // (d) plant_load_proof is deterministic and the token is derivable from
    // the body's own SHA-256.
    #[test]
    fn plant_load_proof_is_deterministic_and_token_matches_sha256() {
        let body = "line one\nline two\nline three\nline four\nline five\n";
        let out1 = plant_load_proof("part-1", body);
        let out2 = plant_load_proof("part-1", body);
        assert_eq!(out1, out2);
        let expected_hash = hash_hex(body.trim_end_matches('\n').as_bytes());
        let expected_token = &expected_hash[0..16];
        assert!(out1.contains(&format!("token={expected_token}")));
    }

    // (e) floor/span arithmetic clamps correctly for a very short body.
    #[test]
    fn floor_clamp_applies_for_a_very_short_body() {
        // total=1: floor = 1*4/5 = 0, which is already < total, so no clamp
        // fires -- confirms the arithmetic runs without panicking and
        // inserts sanely (or not at all) for a body this short.
        let out = plant_load_proof("part-1", "only line\n");
        assert!(out.starts_with("only line\n"));
    }

    // (f) emit_part refuses (exit-65-equivalent Err) when a part's section
    // body is empty or whitespace-only.
    #[test]
    fn emit_part_refuses_an_empty_part() {
        let source = "---\n---\n<!-- SKILL_SECTION:START alpha targets=part-2 -->\nsomething\n<!-- SKILL_SECTION:END alpha -->\n";
        let result = emit_part(source, Path::new("/tmp/skill-source.txt"), 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no content for part part-1"));
    }

    // (g) emit_index's output contains the exact hardcoded planning-specific
    // prose and the four-row pointer table.
    #[test]
    fn emit_index_contains_hardcoded_prose_and_table() {
        let out = emit_index("---\n---\n");
        assert!(out.contains("# Planning"));
        assert!(out.contains("89,860 bytes"));
        assert!(out.contains("[parts/part-1.md](parts/part-1.md)"));
        assert!(out.contains("[parts/part-4.md](parts/part-4.md)"));
    }

    // (h) write_or_check's --check path reports stale for a deliberately
    // different committed file and does not write.
    #[test]
    fn write_or_check_reports_stale_and_does_not_write() {
        let dir = tempdir();
        let target = dir.join("SKILL.md");
        fs::write(&target, "old content\n").unwrap();
        let mut stale = false;
        write_or_check(&target, "new content\n", &dir, true, &mut stale);
        assert!(stale);
        assert_eq!(fs::read_to_string(&target).unwrap(), "old content\n");
        fs::remove_dir_all(&dir).ok();
    }

    // (i) CLI parsing rejects more than one positional argument and an
    // unrecognized flag, both with exit 64.
    #[test]
    fn cli_error_paths_exit_64() {
        assert!(matches!(
            parse_args(&["a".to_string(), "b".to_string()]),
            ParsedArgs::BadUsage
        ));
        assert!(matches!(
            parse_args(&["--bogus".to_string()]),
            ParsedArgs::BadUsage
        ));
        // Position-only-first: a flag AFTER the positional is also refused,
        // since only position 1 is ever inspected for --check/-h/--help.
        assert!(matches!(
            parse_args(&["somedir".to_string(), "--check".to_string()]),
            ParsedArgs::BadUsage
        ));
        // The accepted forms still parse cleanly.
        assert!(matches!(
            parse_args(&["--check".to_string()]),
            ParsedArgs::Run {
                check_only: true,
                skill_dir: None
            }
        ));
        assert!(matches!(
            parse_args(&["--check".to_string(), "somedir".to_string()]),
            ParsedArgs::Run {
                check_only: true,
                skill_dir: Some(ref dir)
            } if dir == "somedir"
        ));
    }

    // (j) skill_root()'s failure path is exercised at the real subprocess
    // level; here we test the pure resolution function directly.
    #[test]
    fn skill_root_returns_none_when_nothing_resolves() {
        let dir = tempdir();
        let outside = dir.join("no-planning-scripts-here");
        fs::create_dir_all(&outside).unwrap();
        let resolved = skill_root_from(
            None,
            Some(&outside.join("generate-skill-docs")),
            Some(&outside),
        );
        assert!(resolved.is_none());
        fs::remove_dir_all(&dir).ok();
    }

    // (k) per AR-28: a body with trailing blank lines produces the identical
    // token/at as the same body with those lines stripped.
    #[test]
    fn trailing_blank_lines_do_not_affect_token_or_insertion_point() {
        let without_trailing = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";
        let with_trailing = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\n\n";
        assert_eq!(
            plant_load_proof("part-1", without_trailing),
            plant_load_proof("part-1", with_trailing)
        );
    }

    // (l) the proof block's three inserted lines appear strictly after line
    // `at`, never before or replacing it.
    #[test]
    fn proof_block_lands_strictly_after_line_at() {
        let body = "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n";
        let out = plant_load_proof("part-1", body);
        let lines: Vec<&str> = out.lines().collect();
        let proof_index = lines
            .iter()
            .position(|line| line.starts_with("<!-- SKILL-LOAD-PROOF"))
            .expect("proof line present");
        // The line immediately before the blank-then-proof pair must be a
        // real body line (l1..l10), never itself blank/absent, confirming
        // the proof follows a real content line rather than replacing one.
        assert!(
            proof_index >= 2,
            "proof line must follow at least one body line and its blank separator"
        );
        assert!(lines[proof_index - 1].is_empty());
        assert!(!lines[proof_index - 2].is_empty());
    }

    fn tempdir() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "generate-skill-docs-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
