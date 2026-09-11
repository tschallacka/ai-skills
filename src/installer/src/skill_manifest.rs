// MODE: DEV
// PACKAGE: PROD
//! The authoritative "which files does this skill ship" answer, when it can
//! be gotten: install.sh's own `skill_files()` (installer/src/50-manifest.sh)
//! is a hand-maintained, per-skill bash function -- not a data file, so
//! there is nothing for this installer to parse into a Rust table without
//! re-deriving install.sh's own logic and risking it drifting out of sync.
//! Rather than approximate it (install.rs's `should_ship` MODE-marker
//! heuristic, kept as the fallback below), this extracts `skill_files()`'s
//! source text out of `install.sh` and interprets it directly -- no `bash`
//! subprocess, no shell escaping. `skill_files()` is a hand-maintained
//! function, but it is written in a narrow, stable dialect: per-skill
//! `case` arms built only from `printf`/heredoc literal file lists, a
//! nested `case "$(uname -s):$(uname -m)"` platform dispatch, a
//! `[ "$package" = dev ] || return 0` tier gate, a `for … *.sh` script-glob
//! loop, and one directory walk. Every one of those seven shapes is parsed
//! and interpreted natively here (`Stmt`/`parse_arms`/`interpret`); any
//! construct this does not recognize -- anywhere in the function, not just
//! the arm asked for -- fails the whole parse closed (`None`), so a future
//! change to `skill_files()` this parser cannot follow falls back to the
//! MODE-marker heuristic below rather than silently mis-answering.
//!
//! `skill_artifact_files()` (the one helper `skill_files()` calls for a
//! binary row) is not parsed at all: its own two behaviors -- pass every
//! argument through unfiltered under `DEV_BUILD=1`, else drop any argument
//! that does not exist under `$SOURCE_ROOT/$skill/` -- collapse to the same
//! existence check `install.rs`'s own caller already performs on every
//! manifest row regardless of source (see `install.rs`'s
//! `relative_paths_for`), so a `skill_artifact_files skill file...` call is
//! interpreted exactly like a plain literal file list here.
//!
//! `install.sh` is never shipped inside a `build-release.sh` tarball --
//! `bootstrap.sh` downloads the skill payload alone -- so this is a no-op
//! (`None`) for the common end-user path, where install.rs's own MODE-
//! marker filter already does the right thing because that tarball is
//! already prod-only.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Pulls one `name() { ... }` function's exact source text out of
/// `content`, matched on a closing `}` alone on its own line -- true for
/// every function in this codebase's own style (confirmed for
/// `skill_files` specifically), including a body containing `case`/`esac`,
/// heredocs, and nested `if`/`fi`, none of which put a bare `}` at the
/// start of a line.
fn extract_function(content: &str, name: &str) -> Option<String> {
    let marker = format!("\n{name}() {{\n");
    let start = content.find(&marker)? + 1;
    let body = &content[start..];
    let end = body.find("\n}\n")?;
    Some(body[..end + 2].to_string())
}

/// One statement inside a skill's `case "$1" in …)` arm, in the narrow
/// dialect `skill_files()` is written in.
#[derive(Debug, Clone, PartialEq)]
enum Stmt {
    /// `printf '%s\n' a b c`, a `cat <<'EOF' … EOF` heredoc, or a
    /// `skill_artifact_files skill a b c` call (its own skill-name argument
    /// already dropped) -- all three are just a fixed list of relative
    /// paths to add to the manifest.
    Literal(Vec<String>),
    /// `[ "$package" = dev ] || return 0` -- stop this arm here unless the
    /// requested package is `dev`.
    PackageGate,
    /// `case "$(uname -s):$(uname -m)" in PATTERN) files ;; … esac` -- the
    /// `*)` error-and-return default arm is parsed (to stay in sync with
    /// the line cursor) and then discarded, since a host `installer-platform`
    /// itself refuses is never one this installer would still be running on.
    PlatformDispatch(Vec<(String, Vec<String>)>),
    /// `for file in "$SOURCE_ROOT/<dir>"*.sh; do … printf '%s\n' "<prefix>$(basename "$file")"; done`
    GlobScripts { dir: String, prefix: String },
    /// `if [ -d "$SOURCE_ROOT/<dir>" ]; then (cd … && find . -type f -print) | sed 's#^\./#<prefix>#'; fi`
    WalkDir { dir: String, prefix: String },
}

fn is_bare_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Splits `s` on whitespace, honoring single/double quotes the way bash's
/// own word-splitting does for the plain, unexpanded literal tokens
/// `skill_files()` ever hands `printf`/`skill_artifact_files` -- none of
/// which contain a variable, so this needs no expansion, only quote
/// stripping.
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            None => match c {
                '\'' | '"' => quote = Some(c),
                c if c.is_whitespace() => {
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                }
                c => cur.push(c),
            },
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

const PRINTF_PREFIX: &str = "printf '%s\\n' ";

/// `printf '%s\n' a b c` (already continuation-joined into one line) into
/// its literal file tokens; `None` for the differently-shaped error printf
/// (`>&2`) inside a platform-dispatch default arm, which this never treats
/// as a manifest row.
fn parse_printf_literal(joined: &str) -> Option<Vec<String>> {
    let rest = joined.strip_prefix(PRINTF_PREFIX)?;
    if rest.contains(">&2") {
        return None;
    }
    Some(tokenize(rest))
}

/// Bash's own backslash-newline continuation: joins `lines[start..]` into
/// one logical line wherever a line ends with `\`, and returns the index
/// just past the last physical line consumed.
fn join_continuation(lines: &[&str], start: usize) -> Option<(String, usize)> {
    let mut joined = String::new();
    let mut i = start;
    loop {
        let line = lines.get(i)?.trim();
        if let Some(rest) = line.strip_suffix('\\') {
            joined.push_str(rest.trim_end());
            joined.push(' ');
            i += 1;
        } else {
            joined.push_str(line);
            i += 1;
            break;
        }
    }
    Some((joined, i))
}

fn heredoc_delimiter(line: &str) -> Option<&str> {
    line.strip_prefix("cat <<'")?.strip_suffix('\'')
}

/// `for file in "$SOURCE_ROOT/<dir>"*.sh; do` -- `dir` keeps its trailing
/// `/`.
fn for_glob_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("for file in \"$SOURCE_ROOT/")?;
    let dir = rest.strip_suffix("\"*.sh; do")?;
    Some(dir.to_string())
}

/// `[ -f "$file" ] && printf '%s\n' "<prefix>$(basename "$file")"` --
/// `PRINTF_PREFIX`'s quoting differs here (a double-quoted format arg with
/// an embedded `$(basename …)`, not a bare token list), so this is matched
/// on its own rather than reusing `parse_printf_literal`.
fn for_glob_body_prefix(line: &str) -> Option<String> {
    let rest = line.strip_prefix("[ -f \"$file\" ] && printf '%s\\n' \"")?;
    let prefix = rest.strip_suffix("$(basename \"$file\")\"")?;
    Some(prefix.to_string())
}

fn if_dir_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("if [ -d \"$SOURCE_ROOT/")?;
    let dir = rest.strip_suffix("\" ]; then")?;
    Some(dir.to_string())
}

fn find_cd_line(line: &str, dir: &str) -> bool {
    line == format!("(cd \"$SOURCE_ROOT/{dir}\" && find . -type f -print) \\")
}

fn sed_prefix_line(line: &str) -> Option<String> {
    let rest = line.strip_prefix("| sed 's#^\\./#")?;
    let prefix = rest.strip_suffix("#'")?;
    Some(prefix.to_string())
}

/// Parses every `case "$1" in …)` arm out of `skill_files()`'s own source
/// text into its `Stmt` sequence, or `None` the moment anything does not
/// match the seven recognized shapes -- deliberately global rather than
/// per-arm: a construct this cannot follow anywhere in the function is a
/// sign the function has moved on from the dialect this was written
/// against, and every skill should fall back to the MODE-marker heuristic
/// together rather than some getting silently exact answers and others not.
fn parse_arms(skill_files_src: &str) -> Option<HashMap<String, Vec<Stmt>>> {
    let marker = "case \"$1\" in\n";
    let start = skill_files_src.find(marker)? + marker.len();
    let body = &skill_files_src[start..];
    let lines: Vec<&str> = body.lines().collect();

    let mut i = 0usize;
    let mut arms = HashMap::new();
    loop {
        let t = *lines.get(i)?;
        let t = t.trim();
        if t.is_empty() || t.starts_with('#') {
            i += 1;
            continue;
        }
        if t == "esac" {
            return Some(arms);
        }
        let name = t.strip_suffix(')')?;
        if !is_bare_identifier(name) {
            return None;
        }
        i += 1;

        let mut stmts = Vec::new();
        loop {
            let t = *lines.get(i)?;
            let t = t.trim();
            if t == ";;" {
                i += 1;
                break;
            }
            if t.is_empty() || t.starts_with('#') || t == "local file" {
                i += 1;
                continue;
            }

            if let Some(delim) = heredoc_delimiter(t) {
                i += 1;
                let mut items = Vec::new();
                loop {
                    let line = *lines.get(i)?;
                    if line == delim {
                        i += 1;
                        break;
                    }
                    let item = line.trim();
                    if !item.is_empty() {
                        items.push(item.to_string());
                    }
                    i += 1;
                }
                stmts.push(Stmt::Literal(items));
                continue;
            }

            if t.starts_with("printf ") {
                let (joined, next) = join_continuation(&lines, i)?;
                let files = parse_printf_literal(&joined)?;
                stmts.push(Stmt::Literal(files));
                i = next;
                continue;
            }

            if t == "[ \"$package\" = dev ] || return 0" {
                stmts.push(Stmt::PackageGate);
                i += 1;
                continue;
            }

            if t == "case \"$(uname -s):$(uname -m)\" in" {
                i += 1;
                let mut dispatch = Vec::new();
                loop {
                    let at = *lines.get(i)?;
                    let at = at.trim();
                    if at == "esac" {
                        i += 1;
                        break;
                    }
                    let pattern = at.strip_suffix(')')?.to_string();
                    i += 1;

                    let mut action = String::new();
                    loop {
                        let al = *lines.get(i)?;
                        let al = al.trim();
                        action.push_str(al);
                        action.push(' ');
                        i += 1;
                        if al.ends_with(";;") {
                            break;
                        }
                    }
                    if pattern == "*" {
                        continue; // the error-and-return default arm
                    }
                    let action = action.trim().strip_suffix(";;")?.trim();
                    let files = if let Some(rest) = action.strip_prefix(PRINTF_PREFIX) {
                        if rest.contains(">&2") {
                            return None;
                        }
                        tokenize(rest)
                    } else {
                        let rest = action.strip_prefix("skill_artifact_files ")?;
                        let mut toks = tokenize(rest);
                        if toks.is_empty() {
                            return None;
                        }
                        toks.remove(0); // the skill-name argument
                        toks
                    };
                    dispatch.push((pattern, files));
                }
                stmts.push(Stmt::PlatformDispatch(dispatch));
                continue;
            }

            if let Some(dir) = for_glob_header(t) {
                i += 1;
                let prefix = for_glob_body_prefix(lines.get(i)?.trim())?;
                i += 1;
                if lines.get(i)?.trim() != "done" {
                    return None;
                }
                i += 1;
                stmts.push(Stmt::GlobScripts { dir, prefix });
                continue;
            }

            if let Some(dir) = if_dir_header(t) {
                i += 1;
                if !find_cd_line(lines.get(i)?.trim(), &dir) {
                    return None;
                }
                i += 1;
                let prefix = sed_prefix_line(lines.get(i)?.trim())?;
                i += 1;
                if lines.get(i)?.trim() != "fi" {
                    return None;
                }
                i += 1;
                stmts.push(Stmt::WalkDir { dir, prefix });
                continue;
            }

            return None; // an eighth shape this does not know
        }
        arms.insert(name.to_string(), stmts);
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some(b'*'), _) => go(&p[1..], t) || (!t.is_empty() && go(p, &t[1..])),
            (Some(pc), Some(tc)) if pc == tc => go(&p[1..], &t[1..]),
            _ => false,
        }
    }
    go(pattern.as_bytes(), text.as_bytes())
}

/// A `case "$(uname -s):$(uname -m)" in` arm's pattern (`|`-joined
/// alternatives, each an install.sh-style glob with only `*` as a special
/// character) against a synthetic `os:arch` string.
fn case_pattern_matches(pattern: &str, text: &str) -> bool {
    pattern.split('|').any(|alt| glob_match(alt.trim(), text))
}

/// One valid `$(uname -s):$(uname -m)` value for `target` -- chosen to
/// match the corresponding glob arm in `skill_files()`'s own platform
/// dispatch (installer-platform's own `resolve()` doc comment: "Mirrors
/// `normalize_platform()` in install.sh… so the two stay in agreement").
fn synthetic_uname_pair(target: installer_platform::Target) -> &'static str {
    use installer_platform::Target::*;
    match target {
        X86_64UnknownLinuxMusl => "Linux:x86_64",
        Aarch64UnknownLinuxMusl => "Linux:aarch64",
        X86_64AppleDarwin => "Darwin:x86_64",
        Aarch64AppleDarwin => "Darwin:arm64",
        X86_64PcWindowsMsvc => "Windows_NT:x86_64",
    }
}

fn walk_files(dir: &Path, prefix: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let Ok(file_type) = entry.file_type() else { continue };
        let relative = prefix.join(entry.file_name());
        if file_type.is_dir() {
            walk_files(&entry.path(), &relative, out);
        } else if file_type.is_file() {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn interpret(stmts: &[Stmt], source_root: &Path, package_dev: bool) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Literal(files) => out.extend(files.iter().cloned()),
            Stmt::PackageGate => {
                if !package_dev {
                    return Some(out);
                }
            }
            Stmt::PlatformDispatch(arms) => {
                let target = installer_platform::current().ok()?;
                let synthetic = synthetic_uname_pair(target);
                let (_, files) = arms
                    .iter()
                    .find(|(pattern, _)| case_pattern_matches(pattern, synthetic))?;
                out.extend(files.iter().cloned());
            }
            Stmt::GlobScripts { dir, prefix } => {
                let Ok(entries) = fs::read_dir(source_root.join(dir)) else {
                    continue;
                };
                let mut names: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        (e.file_type().ok()?.is_file() && name.ends_with(".sh")).then_some(name)
                    })
                    .collect();
                names.sort();
                out.extend(names.into_iter().map(|n| format!("{prefix}{n}")));
            }
            Stmt::WalkDir { dir, prefix } => {
                let scan_dir = source_root.join(dir);
                if !scan_dir.is_dir() {
                    continue;
                }
                let mut files = Vec::new();
                walk_files(&scan_dir, Path::new(""), &mut files);
                files.sort();
                out.extend(files.into_iter().map(|rel| format!("{prefix}{rel}")));
            }
        }
    }
    Some(out)
}

/// The exact relative paths `install.sh`'s own `skill_files(skill, package)`
/// would print for this host, or `None` when that answer cannot be
/// obtained -- no `install.sh` next to `source_root`, its `skill_files`
/// function could not be extracted, its source has moved on from the
/// dialect `parse_arms` understands, this host is not one
/// `installer-platform` resolves, or `skill` has no arm at all. The caller
/// falls back to its own heuristic in every `None` case rather than
/// failing the install over it.
///
/// `package_dev` is `--package prod|dev`/`PACKAGE_SELECTION`,
/// `skill_files`'s own `$2` -- which file TIER ships (an end user's files,
/// or a maintainer's additional ones). It is independent of
/// `--dev-build`/`DEV_BUILD`, install.sh's separate "prefer this host's
/// freshly-built binary over the shipped one" switch: that question has no
/// answer to give here at all, since it is about which binary
/// `source_file()` resolves a manifest row TO, not which rows the manifest
/// names in the first place.
pub fn skill_files_via_install_sh(source_root: &Path, skill: &str, package_dev: bool) -> Option<Vec<String>> {
    let install_sh = source_root.join("install.sh");
    let content = fs::read_to_string(&install_sh).ok()?;
    let skill_files_fn = extract_function(&content, "skill_files")?;
    let arms = parse_arms(&skill_files_fn)?;
    let stmts = arms.get(skill)?;
    interpret(stmts, source_root, package_dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_function_pulls_exactly_one_function_body() {
        let content = "\nfoo() {\n    echo bar\n}\n\nbaz() {\n    echo qux\n}\n";
        let foo = extract_function(content, "foo").unwrap();
        assert!(foo.contains("echo bar"));
        assert!(!foo.contains("echo qux"));
    }

    #[test]
    fn extract_function_handles_a_body_with_nested_braces_in_expansions() {
        let content = "\nfoo() {\n    local x=\"${1:-default}\"\n    echo \"$x\"\n}\n";
        let foo = extract_function(content, "foo").unwrap();
        assert!(foo.contains("${1:-default}"));
    }

    #[test]
    fn no_install_sh_next_to_source_root_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(skill_files_via_install_sh(dir.path(), "todo", false).is_none());
    }

    fn write_install_sh(dir: &Path, skill_files_body: &str) {
        fs::write(
            dir.join("install.sh"),
            format!("\nskill_files() {{\n{skill_files_body}\n}}\n"),
        )
        .unwrap();
    }

    #[test]
    fn a_minimal_install_sh_produces_its_own_skill_files_answer() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md requires.tsv\n            [ \"$package\" = dev ] || return 0\n            printf '%s\\n' MAINTAINER.md\n            ;;\n    esac",
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md", "requires.tsv"]);

        let dev_files = skill_files_via_install_sh(dir.path(), "widget", true).unwrap();
        assert_eq!(dev_files, vec!["SKILL.md", "requires.tsv", "MAINTAINER.md"]);
    }

    #[test]
    fn a_heredoc_is_read_as_a_literal_file_list() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            cat <<'EOF'\nSKILL.md\ndocs/README.md\nEOF\n            ;;\n    esac",
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md", "docs/README.md"]);
    }

    #[test]
    fn a_multiline_printf_with_backslash_continuation_joins_its_tokens() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md requires.tsv \\\n                schema.json extra.tsv\n            ;;\n    esac",
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md", "requires.tsv", "schema.json", "extra.tsv"]);
    }

    #[test]
    fn platform_dispatch_picks_this_hosts_own_arm() {
        let target = installer_platform::current().expect("test host must be a supported platform");
        let synthetic = synthetic_uname_pair(target);
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            &format!(
                "    case \"$1\" in\n        widget)\n            case \"$(uname -s):$(uname -m)\" in\n                {synthetic})\n                    printf '%s\\n' 'bin/{target}/widget' ;;\n                *)\n                    printf 'skill_files: no widget artifact for %s:%s\\n' \"$(uname -s)\" \"$(uname -m)\" >&2\n                    return 69 ;;\n            esac\n            ;;\n    esac",
            ),
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec![format!("bin/{target}/widget")]);
    }

    #[test]
    fn platform_dispatch_falls_through_when_no_arm_matches_the_glob() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            case \"$(uname -s):$(uname -m)\" in\n                Plan9:riscv64)\n                    printf '%s\\n' 'bin/plan9/widget' ;;\n                *)\n                    printf 'skill_files: no widget artifact for %s:%s\\n' \"$(uname -s)\" \"$(uname -m)\" >&2\n                    return 69 ;;\n            esac\n            ;;\n    esac",
        );
        assert!(skill_files_via_install_sh(dir.path(), "widget", false).is_none());
    }

    #[test]
    fn skill_artifact_files_call_drops_its_own_skill_name_argument() {
        let target = installer_platform::current().expect("test host must be a supported platform");
        let synthetic = synthetic_uname_pair(target);
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            &format!(
                "    case \"$1\" in\n        widget)\n            case \"$(uname -s):$(uname -m)\" in\n                {synthetic})\n                    skill_artifact_files widget bin/{target}/widget bin/{target}/widget-mcp ;;\n                *)\n                    return 69 ;;\n            esac\n            ;;\n    esac",
            ),
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(
            files,
            vec![format!("bin/{target}/widget"), format!("bin/{target}/widget-mcp")]
        );
    }

    #[test]
    fn glob_scripts_lists_only_sh_files_sorted() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md\n            local file\n            for file in \"$SOURCE_ROOT/widget/scripts/\"*.sh; do\n                [ -f \"$file\" ] && printf '%s\\n' \"scripts/$(basename \"$file\")\"\n            done\n            ;;\n    esac",
        );
        fs::create_dir_all(dir.path().join("widget/scripts")).unwrap();
        fs::write(dir.path().join("widget/scripts/b.sh"), "").unwrap();
        fs::write(dir.path().join("widget/scripts/a.sh"), "").unwrap();
        fs::write(dir.path().join("widget/scripts/not-a-script.txt"), "").unwrap();
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md", "scripts/a.sh", "scripts/b.sh"]);
    }

    #[test]
    fn glob_scripts_is_empty_when_the_directory_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md\n            local file\n            for file in \"$SOURCE_ROOT/widget/scripts/\"*.sh; do\n                [ -f \"$file\" ] && printf '%s\\n' \"scripts/$(basename \"$file\")\"\n            done\n            ;;\n    esac",
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md"]);
    }

    #[test]
    fn walk_dir_lists_a_directorys_files_recursively_with_a_prefix_when_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            if [ -d \"$SOURCE_ROOT/widget/tests/fixtures\" ]; then\n                (cd \"$SOURCE_ROOT/widget/tests/fixtures\" && find . -type f -print) \\\n                    | sed 's#^\\./#tests/fixtures/#'\n            fi\n            ;;\n    esac",
        );
        fs::create_dir_all(dir.path().join("widget/tests/fixtures/nested")).unwrap();
        fs::write(dir.path().join("widget/tests/fixtures/a.md"), "").unwrap();
        fs::write(dir.path().join("widget/tests/fixtures/nested/b.md"), "").unwrap();
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["tests/fixtures/a.md", "tests/fixtures/nested/b.md"]);
    }

    #[test]
    fn walk_dir_is_a_no_op_when_the_directory_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md\n            if [ -d \"$SOURCE_ROOT/widget/tests/fixtures\" ]; then\n                (cd \"$SOURCE_ROOT/widget/tests/fixtures\" && find . -type f -print) \\\n                    | sed 's#^\\./#tests/fixtures/#'\n            fi\n            ;;\n    esac",
        );
        let files = skill_files_via_install_sh(dir.path(), "widget", false).unwrap();
        assert_eq!(files, vec!["SKILL.md"]);
    }

    #[test]
    fn an_unrecognized_construct_fails_the_whole_parse_closed() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            eval \"$(some_future_construct)\"\n            ;;\n    esac",
        );
        assert!(skill_files_via_install_sh(dir.path(), "widget", false).is_none());
    }

    #[test]
    fn a_skill_with_no_arm_is_none() {
        let dir = tempfile::tempdir().unwrap();
        write_install_sh(
            dir.path(),
            "    case \"$1\" in\n        widget)\n            printf '%s\\n' SKILL.md\n            ;;\n    esac",
        );
        assert!(skill_files_via_install_sh(dir.path(), "gadget", false).is_none());
    }

}
