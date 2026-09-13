// MODE: DEV
// PACKAGE: PROD
//! Runtime tool requirements -- ported from installer/src/20-runtime-tools.sh,
//! but reads each skill's own `requires.tsv` directly rather than replicating
//! install.sh's build-time code generation (installer/build.sh bakes
//! requires.tsv into literal `case` statements in runtime_requirements() and
//! friends). requires.tsv itself ships per skill in the release payload
//! (it is `MODE: PROD` and listed in skill_files()), so there is nothing
//! generated for this installer to regenerate: parsing the TSV at runtime
//! is the whole story, and it can never drift from the source of truth the
//! bash generator reads from either.
//!
//! Every `verify` row in installer/tools.tsv reduces to `command -v <tool>`
//! (checked: no shipped row does anything else) with one exception this
//! module still has to know about: rjq. install.sh never demands a system
//! rjq -- `prepend_bundled_rjq` in 20-runtime-tools.sh puts the source
//! tree's own prebuilt `planning/bin/<target-triple>/rjq` ahead of PATH
//! before any dependency check runs, so a host with no system-wide rjq
//! still satisfies the requirement as long as the release shipped one for
//! this host's triple. This installer adds one further rung install.sh does
//! not have: rjq is a jq-compatible reimplementation, so a system `jq` on
//! PATH also satisfies the requirement when neither rjq nor a bundled
//! artifact is found. `tool_available` is where all three rungs live; every
//! other tool is a plain PATH lookup. Install hints
//! (`runtime_requirement_install_hint`) live in `tools.rs`, not here, which
//! reads `installer/tools.tsv` for the picker's `d` key -- `d`/`r`/`m`
//! (dependency hints, reverify, integration-mode cycling) are all ported;
//! see `ui::model::PickerState`.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Hard,
    Soft,
}

pub struct Requirement {
    /// A bare tool id (`"rjq"`) or an any-of group id shared by several rows
    /// (`Some("group-id")`); the label is `tool` for the former, `"any of
    /// <members>"` for the latter.
    pub tool: String,
    pub group: Option<String>,
    pub strength: Strength,
    pub why: String,
}

/// One row of requires.tsv still applicable to this host, before any
/// same-group rows are folded into one any-of entry.
struct Row {
    tool: String,
    strength: Strength,
    why: String,
    group: Option<String>,
}

fn parse_row(line: &str) -> Option<Row> {
    let mut cols = line.split('\t');
    let tool = cols.next()?.to_string();
    let condition = cols.next()?;
    let strength = match cols.next()? {
        "hard" => Strength::Hard,
        "soft" => Strength::Soft,
        _ => return None,
    };
    let why = cols.next().unwrap_or("").to_string();
    let group = cols.next().filter(|g| !g.is_empty()).map(str::to_string);
    if !condition_applies(condition, host_os(), host_arch()) {
        return None;
    }
    Some(Row {
        tool,
        strength,
        why,
        group,
    })
}

/// `condition` is a bash `case` pattern against `"$os:$arch"`: each side of
/// the `:` is matched independently (`*` matches anything on that side),
/// `|` joins alternatives (`Linux:x86_64|Linux:amd64`), everything else must
/// match literally. Matched per-segment rather than as one `os:arch` glob
/// because `*` legitimately appears on both sides of the same alternative
/// (`*:*`), which a single-wildcard glob can't express.
fn condition_applies(condition: &str, os: &str, arch: &str) -> bool {
    condition.split('|').any(|alt| {
        let alt = alt.trim();
        match alt.split_once(':') {
            Some((os_pattern, arch_pattern)) => {
                glob_match(os_pattern, os) && glob_match(arch_pattern, arch)
            }
            None => false,
        }
    })
}

fn glob_match(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((prefix, suffix)) => {
            text.len() >= prefix.len() + suffix.len()
                && text.starts_with(prefix)
                && text.ends_with(suffix)
        }
    }
}

fn host_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "Darwin"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(windows) {
        "Windows_NT"
    } else {
        "unknown"
    }
}

fn host_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    }
}

/// Every requirement `skill` carries on this host, groups already folded:
/// rows sharing a non-empty group id become one any-of `Requirement`, using
/// the first member's strength/why (installer/build.sh's generator assumes
/// the same, since a mixed-strength group has no single answer to "is this
/// requirement met").
pub fn requirements_for(source_root: &Path, skill: &str) -> Vec<Requirement> {
    let path = source_root.join(skill).join("requires.tsv");
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut out: Vec<Requirement> = Vec::new();
    for line in content.lines() {
        if line.is_empty() || line.starts_with('#') || line.starts_with("tool\t") {
            continue;
        }
        let Some(row) = parse_row(line) else { continue };
        if let Some(group) = &row.group {
            if let Some(existing) = out.iter_mut().find(|r| r.group.as_deref() == Some(group)) {
                existing.tool.push_str(", ");
                existing.tool.push_str(&row.tool);
                continue;
            }
        }
        out.push(Requirement {
            tool: row.tool,
            group: row.group,
            strength: row.strength,
            why: row.why,
        });
    }
    out
}

pub fn tool_on_path(tool: &str) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| is_executable(&dir.join(tool)))
}

fn is_executable(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(candidate)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// The bundled rjq binary this release shipped for the running host, if any
/// -- `planning/bin/<target-triple>/rjq[.exe]`, the same path
/// `bundled_rjq_artifact`/`prepend_bundled_rjq` in 20-runtime-tools.sh
/// resolve, kept in sync with them by going through `installer_platform`
/// (the same crate `build-installer-release.sh`'s own target list is built
/// from) rather than a second copy of the os/arch match.
fn bundled_rjq_path(source_root: &Path) -> Option<PathBuf> {
    let target = installer_platform::current().ok()?;
    let name = if target.is_windows() {
        "rjq.exe"
    } else {
        "rjq"
    };
    let path = source_root
        .join("planning")
        .join("bin")
        .join(target.as_str())
        .join(name);
    is_executable(&path).then_some(path)
}

/// A tool is available when it is on PATH, or -- for rjq only -- via two
/// further rungs, checked in the order install.sh itself checks them:
///
/// 1. this release's own bundled artifact first, same as
///    `prepend_bundled_rjq` putting `planning/bin/<triple>/` ahead of PATH
///    before any dependency check runs, so a bundled rjq wins even over a
///    different rjq already on PATH;
/// 2. PATH itself second;
/// 3. a system `jq` last, since rjq is a jq-compatible reimplementation --
///    wherever rjq would satisfy this requirement, jq does too, but only
///    once install.sh's own two rungs have both come up empty.
///
/// Every other tool has no such fallback: install.sh's own generated
/// `runtime_tool_verify()` is a plain `command -v` for everything but rjq.
fn tool_available(source_root: &Path, tool: &str) -> bool {
    if tool != "rjq" {
        return tool_on_path(tool);
    }
    bundled_rjq_path(source_root).is_some() || tool_on_path("rjq") || tool_on_path("jq")
}

pub fn requirement_met(source_root: &Path, req: &Requirement) -> bool {
    if req.group.is_some() {
        req.tool
            .split(", ")
            .any(|tool| tool_available(source_root, tool))
    } else {
        tool_available(source_root, &req.tool)
    }
}

pub fn requirement_label(req: &Requirement) -> String {
    if req.group.is_some() {
        format!("any of {}", req.tool)
    } else {
        req.tool.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillState {
    Ok,
    /// A soft requirement is missing: installable, but named capability is
    /// lost.
    Degraded,
    /// A hard requirement is missing: not installable.
    Blocked,
}

pub struct SkillStatus {
    pub state: SkillState,
    /// The first hard-missing tool for Blocked, or the first soft-missing
    /// one for Degraded -- install.sh's IUI_SKILL_BLOCKER, named in both
    /// states since a Degraded row still needs to say what triggered it.
    pub blocker: Option<String>,
    pub requirements: Vec<(Requirement, bool)>,
}

/// blocked beats degraded: an unmet hard requirement stops the install, an
/// unmet soft one only costs capability -- same rule as install.sh's
/// iui_skill_state.
pub fn skill_status(source_root: &Path, skill: &str) -> SkillStatus {
    let reqs = requirements_for(source_root, skill);
    let mut state = SkillState::Ok;
    let mut blocker = None;
    let mut evaluated = Vec::with_capacity(reqs.len());
    for req in reqs {
        let met = requirement_met(source_root, &req);
        if !met {
            match req.strength {
                Strength::Hard => {
                    state = SkillState::Blocked;
                    if blocker.is_none() || state == SkillState::Blocked {
                        blocker = Some(requirement_label(&req));
                    }
                }
                Strength::Soft => {
                    if state == SkillState::Ok {
                        state = SkillState::Degraded;
                    }
                    if blocker.is_none() {
                        blocker = Some(requirement_label(&req));
                    }
                }
            }
        }
        evaluated.push((req, met));
    }
    SkillStatus {
        state,
        blocker,
        requirements: evaluated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    fn write_requires(dir: &Path, skill: &str, content: &str) {
        let skill_dir = dir.join(skill);
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("requires.tsv")).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    // tool_available("rjq", ...) reads the process-global PATH, so every test
    // that overrides it takes this lock first -- same reasoning as
    // plan_migration.rs's ENV_LOCK.
    static PATH_LOCK: Mutex<()> = Mutex::new(());

    fn write_fake_tool(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    #[test]
    fn a_skill_with_no_requires_tsv_has_no_requirements() {
        let dir = tempfile::tempdir().unwrap();
        assert!(requirements_for(dir.path(), "no-such-skill").is_empty());
    }

    #[test]
    fn header_and_comment_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(
            dir.path(),
            "s",
            "# MODE: PROD\ntool\tcondition\tstrength\twhy\nbash\t*:*\thard\tneeds bash\n",
        );
        let reqs = requirements_for(dir.path(), "s");
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "bash");
    }

    #[test]
    fn a_condition_not_matching_this_host_is_excluded() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(
            dir.path(),
            "s",
            "tool\tcondition\tstrength\twhy\nnever\tPlan9:riscv64\thard\tunreachable\n",
        );
        assert!(requirements_for(dir.path(), "s").is_empty());
    }

    #[test]
    fn a_wildcard_condition_always_matches() {
        assert!(condition_applies("*:*", "Linux", "x86_64"));
        assert!(condition_applies("Darwin:*", "Darwin", "arm64"));
        assert!(!condition_applies("Darwin:*", "Linux", "x86_64"));
    }

    #[test]
    fn alternation_matches_either_side() {
        assert!(condition_applies(
            "Linux:x86_64|Linux:amd64",
            "Linux",
            "amd64"
        ));
        assert!(!condition_applies(
            "Linux:x86_64|Linux:amd64",
            "Darwin",
            "arm64"
        ));
    }

    #[test]
    fn same_group_rows_fold_into_one_any_of_requirement() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(
            dir.path(),
            "s",
            "tool\tcondition\tstrength\twhy\tgroup\n\
             python3\t*:*\tsoft\trenders faster\tinterp\n\
             node\t*:*\tsoft\trenders faster\tinterp\n",
        );
        let reqs = requirements_for(dir.path(), "s");
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].tool, "python3, node");
        assert_eq!(requirement_label(&reqs[0]), "any of python3, node");
    }

    #[test]
    fn tool_on_path_finds_a_real_binary_and_rejects_a_fake_one() {
        let _guard = PATH_LOCK.lock().unwrap();
        assert!(tool_on_path("ls") || tool_on_path("cmd.exe"));
        assert!(!tool_on_path("definitely-not-a-real-tool-xyz"));
    }

    #[test]
    fn a_skill_with_every_requirement_met_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(dir.path(), "s", "tool\tcondition\tstrength\twhy\n");
        let status = skill_status(dir.path(), "s");
        assert_eq!(status.state, SkillState::Ok);
        assert!(status.blocker.is_none());
    }

    #[test]
    fn a_missing_hard_requirement_blocks_the_skill() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(
            dir.path(),
            "s",
            "tool\tcondition\tstrength\twhy\ndefinitely-not-a-real-tool-xyz\t*:*\thard\tneeds it\n",
        );
        let status = skill_status(dir.path(), "s");
        assert_eq!(status.state, SkillState::Blocked);
        assert_eq!(
            status.blocker.as_deref(),
            Some("definitely-not-a-real-tool-xyz")
        );
    }

    #[test]
    fn a_missing_soft_requirement_degrades_without_blocking() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(
            dir.path(),
            "s",
            "tool\tcondition\tstrength\twhy\ndefinitely-not-a-real-tool-xyz\t*:*\tsoft\tnice to have\n",
        );
        let status = skill_status(dir.path(), "s");
        assert_eq!(status.state, SkillState::Degraded);
    }

    #[test]
    fn a_blocked_state_wins_over_a_degraded_one() {
        let dir = tempfile::tempdir().unwrap();
        write_requires(
            dir.path(),
            "s",
            "tool\tcondition\tstrength\twhy\n\
             definitely-not-a-real-tool-xyz\t*:*\tsoft\tnice to have\n\
             also-not-real-xyz\t*:*\thard\tneeds it\n",
        );
        let status = skill_status(dir.path(), "s");
        assert_eq!(status.state, SkillState::Blocked);
    }

    #[test]
    fn a_missing_rjq_falls_back_to_a_system_jq() {
        let _guard = PATH_LOCK.lock().unwrap();
        let source = tempfile::tempdir().unwrap(); // no bundled planning/bin/*/rjq
        write_requires(
            source.path(),
            "s",
            "tool\tcondition\tstrength\twhy\nrjq\t*:*\thard\tneeds json\n",
        );
        let fake_path_dir = tempfile::tempdir().unwrap();
        write_fake_tool(fake_path_dir.path(), "jq");
        let original = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", fake_path_dir.path());
        let status = skill_status(source.path(), "s");
        std::env::set_var("PATH", original);
        assert_eq!(status.state, SkillState::Ok);
    }

    #[test]
    fn no_rjq_and_no_jq_still_blocks() {
        let _guard = PATH_LOCK.lock().unwrap();
        let source = tempfile::tempdir().unwrap();
        write_requires(
            source.path(),
            "s",
            "tool\tcondition\tstrength\twhy\nrjq\t*:*\thard\tneeds json\n",
        );
        let empty_path_dir = tempfile::tempdir().unwrap();
        let original = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", empty_path_dir.path());
        let status = skill_status(source.path(), "s");
        std::env::set_var("PATH", original);
        assert_eq!(status.state, SkillState::Blocked);
    }
}
