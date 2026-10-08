// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

pub fn project_root_for(input: Option<&str>) -> Result<PathBuf, String> {
    let directory = input.unwrap_or(".");
    let path = Path::new(directory);
    if !path.is_dir() {
        return Err(format!("directory does not exist: {directory}"));
    }
    if let Some(root) = git_value(path, &["rev-parse", "--show-toplevel"]) {
        if !root.is_empty() {
            return canonical_directory(Path::new(&root));
        }
    }
    canonical_directory(path)
}

pub fn home_plans() -> Result<PathBuf, String> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            Some(PathBuf::from(drive).join(path).join(".config"))
        })
        .ok_or_else(|| "unable to resolve home directory; set PLANS_ROOT".to_string())?;
    Ok(base.join("tsch-ai-skills").join("plans"))
}

pub fn global_scoped_root(project: &Path) -> Result<PathBuf, String> {
    let plans = home_plans()?;
    let base = project
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let (owner, repo) = git_remote_namespace(project).unwrap_or_default();
    if !owner.is_empty() && !repo.is_empty() {
        let candidate = plans.join(&owner).join(&repo);
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }
    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".into());
    Ok(plans.join(user).join(base))
}

pub fn canonical_directory(path: &Path) -> Result<PathBuf, String> {
    canonicalize(path).map_err(|_| format!("directory does not exist: {}", path.display()))
}

/// `fs::canonicalize` without the `\\?\` verbatim prefix Windows adds.
///
/// The prefixed form is a valid path to Rust, but nothing that reads what we
/// write can use it: bash, git and every manifest a shell sources see
/// `\\?\C:\Users\x` and cannot open it. `C:\Users\x` is the same directory.
/// Elsewhere this is plain `fs::canonicalize`.
pub fn canonicalize(path: &Path) -> std::io::Result<PathBuf> {
    fs::canonicalize(path).map(simplified)
}

/// Strips the verbatim prefix from a canonical Windows path when what is left
/// is an ordinary drive or UNC path; every other path is returned as it came.
pub fn simplified(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            let bytes = rest.as_bytes();
            if bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'\\'
            {
                return PathBuf::from(rest.to_string());
            }
        }
    }
    path
}

/// A program's file name on this platform: `plan-context` is
/// `plan-context.exe` on Windows and itself elsewhere. Anything that looks for
/// a sibling binary by joining a bare name onto a directory needs this, or it
/// finds nothing on Windows and falls back to a name the OS cannot resolve.
pub fn exe_name(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// The `bash` to run scripts with.
///
/// Elsewhere that is `bash`, found through PATH. On Windows a bare
/// `Command::new("bash")` is not: Rust looks in the system directories before
/// PATH, so it finds `C:\Windows\System32\bash.exe`, the WSL launcher, ahead of
/// the Git for Windows bash the user has. PATH is walked here, skipping the
/// launcher stubs.
pub fn bash_program() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path) = env::var_os("PATH") {
            for dir in env::split_paths(&path) {
                let candidate = dir.join("bash.exe");
                if candidate.is_file() && !is_wsl_launcher(&candidate) {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("bash")
}

/// True for the WSL launcher stubs Windows ships as `bash.exe`: the one in
/// System32 and the app-execution alias under WindowsApps.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_wsl_launcher(path: &Path) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    lowered.contains("\\windows\\system32\\") || lowered.contains("\\windowsapps\\")
}

/// A path as bash wants to receive it as an argument: with slashes on Windows
/// (MSYS converts a `C:/x` argument and mangles a `C:\x` one), untouched
/// elsewhere.
pub fn for_bash(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text
    }
}

/// A command that runs `program`. A `.sh` script is started through bash on
/// Windows, where the file itself cannot be executed (CreateProcess answers
/// "%1 is not a valid Win32 application"); everywhere else, and for a real
/// executable, the file is started directly.
pub fn command_for(program: &Path) -> Command {
    if cfg!(windows)
        && program
            .extension()
            .is_some_and(|extension| extension == "sh")
    {
        let mut command = Command::new(bash_program());
        command.arg(for_bash(program));
        command
    } else {
        Command::new(program)
    }
}

pub fn require_safe_value(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.contains('|') {
        return Err(format!(
            "{label} must not contain a Markdown table separator (|)"
        ));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{label} must be one line"));
    }
    Ok(())
}

pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(format!("Target directory not found: {}", parent.display()));
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let temporary = parent.join(format!(".{name}.{}", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| format!("Cannot create a temp file in: {}", parent.display()))?;
    if let Ok(metadata) = fs::metadata(path) {
        file.set_permissions(metadata.permissions())
            .map_err(|error| error.to_string())?;
    }
    file.write_all(content).map_err(|error| error.to_string())?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        error.to_string()
    })
}

pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".into();
    }
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_./-:#".contains(&byte))
    {
        return value.into();
    }
    let mut quoted = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"_./-:#".contains(&byte) {
            quoted.push(byte as char);
        } else {
            quoted.push('\\');
            quoted.push(byte as char);
        }
    }
    quoted
}

pub fn write_env_manifest(path: &Path, values: &[(&str, String)]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("manifest has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".env.tmp.{}", std::process::id()));
    let mut content = String::from("# Generated by plan-env.sh; do not edit.\n");
    for (key, value) in values {
        content.push_str(key);
        content.push('=');
        content.push_str(&shell_quote(value));
        content.push('\n');
    }
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|error| error.to_string())?;
        }
        drop(file);
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn git_value(directory: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn git_remote_namespace(project: &Path) -> Option<(String, String)> {
    let remote = git_value(project, &["remote", "get-url", "origin"])?;
    parse_git_remote_namespace(&remote)
}

pub fn snapshot_repo(plan: &Path) -> Option<PathBuf> {
    let manifest = plan.join(".env");
    let manifest = fs::read_to_string(manifest).ok()?;
    let line = manifest
        .lines()
        .find(|line| line.starts_with("PLAN_SNAPSHOT_REPO="))?;
    let value = line.strip_prefix("PLAN_SNAPSHOT_REPO=")?;
    if value.is_empty() || value.contains(['$', '`', ';', '|', '&', '<', '>']) {
        return None;
    }
    let value = shell_unquote(value);
    (!value.is_empty()).then(|| PathBuf::from(value))
}

/// The value a shell would read back from one `KEY=value` assignment written
/// by either quoting style in this repository: single-quoted (plan-env), or
/// bare with a backslash before each unsafe character ([`shell_quote`]).
pub fn shell_unquote(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return value[1..value.len() - 1].replace("'\\''", "'");
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(escaped) = chars.next() {
                out.push(escaped);
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn git_snapshot(plan: &Path) {
    let Some(plan) = canonicalize(plan).ok() else {
        return;
    };
    let Some(repo) = snapshot_repo(&plan) else {
        return;
    };
    // A linked worktree's own .git is a file, an ordinary repository root's
    // is a directory -- both mean a real git repository is present; only a
    // genuinely missing .git should skip the snapshot.
    if !repo.join(".git").exists() {
        return;
    }
    let command = env::args()
        .next()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "command".into());
    let command = if command.ends_with(".sh") {
        command
    } else {
        format!("{command}.sh")
    };
    // -f: a cone-mode plans-branch worktree still carries the host project's
    // own tracked .gitignore excluding .plans/, so staging would otherwise
    // silently fail even with the directory-vs-file check above relaxed;
    // harmless for the existing separate-repo mode, where nothing is ever
    // gitignored by a parent project.
    let _ = Command::new("git")
        .args(["-C"])
        .arg(&repo)
        .args(["add", "-A", "-f", "--"])
        .arg(&plan)
        .output();
    let _ = Command::new("git")
        .args(["-C"])
        .arg(&repo)
        .args([
            "-c",
            "user.name=plan-skill",
            "-c",
            "user.email=plan-skill@localhost",
            "commit",
            "-q",
            "-m",
        ])
        .arg(format!("snapshot before {command}"))
        .output();
}

pub fn parse_git_remote_namespace(remote: &str) -> Option<(String, String)> {
    if remote.starts_with('/') || remote.starts_with("./") || remote.starts_with("../") {
        return None;
    }
    let is_drive_letter_prefix = remote
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic())
        && remote.as_bytes().get(1) == Some(&b':');
    if is_drive_letter_prefix {
        return None;
    }
    let path = remote
        .strip_prefix("ssh://")
        .and_then(|value| value.split_once('/').map(|(_, path)| path))
        .or_else(|| {
            remote
                .split_once("git@")
                .and_then(|(_, value)| value.split_once(':').map(|(_, path)| path))
        })
        .or_else(|| {
            remote
                .split_once("://")
                .and_then(|(_, value)| value.split_once('/').map(|(_, path)| path))
        })
        .unwrap_or(remote);
    let mut repo = path.trim_end_matches('/').to_string();
    if repo.ends_with(".git") {
        repo.truncate(repo.len() - 4);
    }
    let (owner, name) = repo.rsplit_once('/')?;
    if name.is_empty() || owner.is_empty() {
        return None;
    }
    Some((owner.to_string(), name.to_string()))
}

/// The `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills` directory, shared by
/// [`registers_scoped_root`], [`plans_branch_scoped_root`] and
/// [`tsch_config_path`]. Does not create it.
fn tsch_ai_skills_base() -> Result<PathBuf, String> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            Some(PathBuf::from(drive).join(path).join(".config"))
        })
        .ok_or_else(|| "unable to resolve a home directory for tsch-ai-skills state".to_string())?;
    Ok(base.join("tsch-ai-skills"))
}

/// `project`'s own owner/repo (from its git remote) or, with no remote, its
/// OS user and the project directory's own base name -- the two-component
/// addressing shared by [`registers_scoped_root`], [`plans_branch_scoped_root`]
/// and [`tsch_config_path`]. Unlike [`global_scoped_root`], this never checks
/// whether a candidate directory already exists.
fn owner_repo_or_user_project(project: &Path) -> (String, String) {
    if let Some((owner, repo)) = git_remote_namespace(project) {
        if !owner.is_empty() && !repo.is_empty() {
            return (owner, repo);
        }
    }
    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".into());
    let project_dir = project
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    (user, project_dir)
}

pub fn registers_scoped_root(project: &Path) -> Result<PathBuf, String> {
    let base = tsch_ai_skills_base()?;
    let (first, second) = owner_repo_or_user_project(project);
    Ok(base.join("registers").join(first).join(second))
}

pub fn plans_branch_scoped_root(project: &Path) -> Result<PathBuf, String> {
    let base = tsch_ai_skills_base()?;
    let (first, second) = owner_repo_or_user_project(project);
    Ok(base.join("plans-branch").join(first).join(second))
}

pub fn worktree_recognized(path: &Path) -> bool {
    let Some(candidate_common) = git_value(path, &["rev-parse", "--git-common-dir"]) else {
        return false;
    };
    let Ok(candidate_common) = canonicalize(&path.join(candidate_common)) else {
        return false;
    };
    let Ok(project) = project_root_for(None) else {
        return false;
    };
    let Some(project_common) = git_value(&project, &["rev-parse", "--git-common-dir"]) else {
        return false;
    };
    let Ok(project_common) = canonicalize(&project.join(project_common)) else {
        return false;
    };
    candidate_common == project_common
}

/// Whether `directory` is checked out on `branch` right now (B404). A caller
/// resolving a registers-style scoped worktree at some fixed, computed path
/// (e.g. [`registers_scoped_root`]) uses this to recognise, FIRST, the case
/// where it is already running from inside the real worktree directly --
/// which may live nowhere near that fixed path -- rather than asking to
/// create a second, colliding one at the computed location. `directory` not
/// being a git repository at all (or having no commits yet) answers `false`,
/// never a panic or a prompt.
pub fn is_on_branch(directory: &Path, branch: &str) -> bool {
    git_value(directory, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() == Some(branch)
}

/// Runs a git subcommand with its output CAPTURED rather than inherited, so a
/// caller of [`create_sparse_worktree`] never sees git's own informational
/// chatter ("Preparing worktree...", "HEAD is now at...") leak into its own
/// stdout/stderr on success -- confirmed empirically: `Command::status`
/// shares the parent's stdio by default, and that chatter showed up verbatim
/// in a caller's own captured stderr, defeating a "no prompt output" check
/// that had nothing to do with this function's own prompting. On failure,
/// git's stderr is folded into the returned error so the named step still
/// explains itself.
fn run_git_capturing(step: &str, mut command: Command) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{step}: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!("{step} failed"))
    } else {
        Err(format!("{step} failed: {stderr}"))
    }
}

pub fn create_sparse_worktree(
    project: &Path,
    destination: &Path,
    branch: &str,
    cone: bool,
    patterns: &[&str],
) -> Result<(), String> {
    if git_value(project, &["rev-parse", "--verify", branch]).is_none() {
        let upstream = format!("origin/{branch}");
        let base = if git_value(project, &["rev-parse", "--verify", &upstream]).is_some() {
            upstream
        } else {
            "HEAD".to_string()
        };
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(project)
            .args(["branch", branch, &base]);
        run_git_capturing(&format!("git branch {branch} {base}"), command)?;
    }
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(project)
        .args(["worktree", "add", "--no-checkout"])
        .arg(destination)
        .arg(branch);
    run_git_capturing(&format!("git worktree add --no-checkout {branch}"), command)?;

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(destination)
        .args(["sparse-checkout", "init"]);
    if !cone {
        command.arg("--no-cone");
    }
    run_git_capturing("git sparse-checkout init", command)?;

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(destination)
        .args(["sparse-checkout", "set"])
        .args(patterns);
    run_git_capturing("git sparse-checkout set", command)?;

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(destination)
        .args(["checkout", branch]);
    run_git_capturing(&format!("git checkout {branch}"), command)
}

pub enum PromptOutcome {
    Answered(bool),
    DefaultedNonInteractive(bool),
}

pub fn prompt_yes_no_with_default(
    prompt: &str,
    interactive_default: bool,
    non_interactive_default: bool,
) -> PromptOutcome {
    prompt_yes_no_with_default_from(
        prompt,
        interactive_default,
        non_interactive_default,
        io::stdin().is_terminal(),
        &mut io::stdin(),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn prompt_yes_no_with_default_from(
    prompt: &str,
    interactive_default: bool,
    non_interactive_default: bool,
    is_terminal: bool,
    reader: &mut impl Read,
) -> PromptOutcome {
    if is_terminal {
        eprint!(
            "{prompt} [y/n, default: {}] ",
            if interactive_default { "y" } else { "n" }
        );
        let mut line = String::new();
        let _ = io::BufReader::new(reader).read_line(&mut line);
        match line.trim() {
            "y" | "Y" | "yes" => PromptOutcome::Answered(true),
            "n" | "N" | "no" => PromptOutcome::Answered(false),
            _ => PromptOutcome::Answered(interactive_default),
        }
    } else {
        let mut discarded = String::new();
        let _ = reader.read_to_string(&mut discarded);
        eprintln!("{prompt} (non-interactive; defaulting to {non_interactive_default})");
        PromptOutcome::DefaultedNonInteractive(non_interactive_default)
    }
}

/// A sibling file named after `candidate`'s own file name, suffixed
/// `.declined`, in `candidate`'s own parent directory.
pub fn declined_marker_path(candidate: &Path) -> PathBuf {
    let mut name = candidate
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    name.push_str(".declined");
    candidate.with_file_name(name)
}

pub fn is_declined(candidate: &Path) -> bool {
    declined_marker_path(candidate).is_file()
}

pub fn mark_declined(candidate: &Path) -> io::Result<()> {
    let marker = declined_marker_path(candidate);
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&marker, b"")
}

#[derive(Default, Serialize, Deserialize)]
pub struct TschConfig {
    pub registers_access: Option<String>,
    pub registers_branch: Option<String>,
    pub plans_storage: Option<String>,
    pub plans_branch: Option<String>,
}

pub fn tsch_config_path(project: &Path) -> Result<PathBuf, String> {
    let base = tsch_ai_skills_base()?;
    let (first, second) = owner_repo_or_user_project(project);
    Ok(base
        .join("config")
        .join(first)
        .join(format!("{second}.json")))
}

pub fn read_tsch_config(project: &Path) -> TschConfig {
    let Ok(path) = tsch_config_path(project) else {
        return TschConfig::default();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return TschConfig::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn write_tsch_config_patch(
    project: &Path,
    registers_access: Option<&str>,
    registers_branch: Option<&str>,
    plans_storage: Option<&str>,
    plans_branch: Option<&str>,
) -> Result<(), String> {
    let path = tsch_config_path(project)?;
    let mut config = read_tsch_config(project);
    if let Some(value) = registers_access {
        config.registers_access = Some(value.to_string());
    }
    if let Some(value) = registers_branch {
        config.registers_branch = Some(value.to_string());
    }
    if let Some(value) = plans_storage {
        config.plans_storage = Some(value.to_string());
    }
    if let Some(value) = plans_branch {
        config.plans_branch = Some(value.to_string());
    }
    let content = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, content).map_err(|error| error.to_string())
}

pub fn prompt_branch_name(prompt: &str, default: &str) -> Option<String> {
    prompt_branch_name_from(prompt, default, &mut io::stdin())
}

#[cfg_attr(not(test), allow(dead_code))]
fn prompt_branch_name_from(prompt: &str, default: &str, reader: &mut impl Read) -> Option<String> {
    eprint!("{prompt} (default: {default}) ");
    let mut line = String::new();
    let _ = io::BufReader::new(reader).read_line(&mut line);
    let trimmed = line.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        atomic_write, command_for, create_sparse_worktree, declined_marker_path, exe_name,
        git_remote_namespace, global_scoped_root, is_declined, is_on_branch, is_wsl_launcher,
        mark_declined, parse_git_remote_namespace, plans_branch_scoped_root,
        prompt_branch_name_from, prompt_yes_no_with_default_from, read_tsch_config,
        registers_scoped_root, require_safe_value, shell_quote, shell_unquote, simplified,
        tsch_config_path, worktree_recognized, write_tsch_config_patch, PromptOutcome,
    };
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Mutex;

    // registers_scoped_root/plans_branch_scoped_root/tsch_config_path read the
    // process-global XDG_CONFIG_HOME/USER/USERNAME env vars, so every test that
    // sets them takes this lock first, matching the PATH_LOCK convention in
    // installer/src/requirements.rs.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // worktree_recognized resolves "the calling project" from the process's
    // current working directory, so every test that chdirs takes this lock.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed in {}", dir.display());
    }

    fn init_repo_with_commit(dir: &Path) {
        run_git(dir, &["init", "-q"]);
        run_git(
            dir,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "init",
            ],
        );
    }

    #[test]
    fn no_remote_is_not_a_namespace() {
        assert!(git_remote_namespace(Path::new("/tmp")).is_none());
    }

    #[test]
    fn remote_keeps_nested_namespace() {
        assert_eq!(
            parse_git_remote_namespace("git@git.example:group/subgroup/project.git"),
            Some(("group/subgroup".into(), "project".into()))
        );
        assert_eq!(
            parse_git_remote_namespace("https://git.example/group/subgroup/project"),
            Some(("group/subgroup".into(), "project".into()))
        );
    }

    #[test]
    fn safe_values_reject_table_breakout_and_newlines() {
        assert!(require_safe_value("label", "plain").is_ok());
        assert!(require_safe_value("label", "bad|cell").is_err());
        assert!(require_safe_value("label", "bad\nvalue").is_err());
    }

    #[test]
    fn unquoting_reverses_both_quoting_styles() {
        assert_eq!(shell_unquote("'a b'"), "a b");
        assert_eq!(shell_unquote("'it'\\''s'"), "it's");
        assert_eq!(shell_unquote(r"C:\\Users\\x"), r"C:\Users\x");
        for original in ["plain", "with space", r"C:\Users\runner\x", "a'b", "$x;y"] {
            assert_eq!(shell_unquote(&shell_quote(original)), original);
        }
    }

    #[test]
    fn a_path_that_is_not_verbatim_is_left_alone() {
        let path = std::path::PathBuf::from("/some/dir");
        assert_eq!(simplified(path.clone()), path);
    }

    #[cfg(windows)]
    #[test]
    fn the_verbatim_prefix_comes_off_a_drive_and_a_unc_path() {
        assert_eq!(
            simplified(std::path::PathBuf::from(r"\\?\C:\Users\x")),
            std::path::PathBuf::from(r"C:\Users\x")
        );
        assert_eq!(
            simplified(std::path::PathBuf::from(r"\\?\UNC\host\share\x")),
            std::path::PathBuf::from(r"\\host\share\x")
        );
        assert_eq!(
            simplified(std::path::PathBuf::from(r"\\?\Volume{1}\x")),
            std::path::PathBuf::from(r"\\?\Volume{1}\x")
        );
    }

    #[test]
    fn the_wsl_launchers_are_recognised_and_git_bash_is_not() {
        assert!(is_wsl_launcher(Path::new(r"C:\Windows\System32\bash.exe")));
        assert!(is_wsl_launcher(Path::new(
            r"C:\Users\me\AppData\Local\Microsoft\WindowsApps\bash.exe"
        )));
        assert!(!is_wsl_launcher(Path::new(
            r"C:\Program Files\Git\bin\bash.exe"
        )));
    }

    #[test]
    fn a_program_name_carries_the_platform_suffix() {
        assert_eq!(
            exe_name("plan-context"),
            format!("plan-context{}", std::env::consts::EXE_SUFFIX)
        );
    }

    #[test]
    fn a_shell_script_command_goes_through_bash_only_on_windows() {
        let command = command_for(Path::new("/x/run.sh"));
        if cfg!(windows) {
            assert!(command
                .get_program()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("bash"));
        } else {
            assert_eq!(command.get_program(), "/x/run.sh");
        }
        assert_eq!(command_for(Path::new("/x/tool")).get_program(), "/x/tool");
    }

    #[test]
    fn atomic_write_replaces_content() {
        let path = std::env::temp_dir().join(format!("planning-core-{}", std::process::id()));
        fs::write(&path, "old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_filesystem_path_remote_is_never_a_namespace() {
        assert!(parse_git_remote_namespace("/home/user/some-repo").is_none());
        assert!(parse_git_remote_namespace("./relative-repo").is_none());
        assert!(parse_git_remote_namespace("../sibling-repo").is_none());
        assert!(parse_git_remote_namespace(r"C:\Users\me\repo").is_none());
    }

    #[test]
    fn registers_scoped_root_returns_owner_repo_shape_on_first_call() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        run_git(
            project.path(),
            &[
                "remote",
                "add",
                "origin",
                "git@example.com:acme/widgets.git",
            ],
        );
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        let result = registers_scoped_root(project.path());
        std::env::remove_var("XDG_CONFIG_HOME");
        let result = result.unwrap();
        assert!(
            !result.is_dir(),
            "must not require the directory to pre-exist"
        );
        assert_eq!(
            result,
            home.path()
                .join("tsch-ai-skills")
                .join("registers")
                .join("acme")
                .join("widgets")
        );
    }

    #[test]
    fn registers_scoped_root_falls_back_to_user_and_projectdir_with_no_remote() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        std::env::set_var("USER", "alex");
        let result = registers_scoped_root(project.path());
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("USER");
        let expected_dir = project.path().file_name().unwrap();
        assert_eq!(
            result.unwrap(),
            home.path()
                .join("tsch-ai-skills")
                .join("registers")
                .join("alex")
                .join(expected_dir)
        );
    }

    #[test]
    fn plans_branch_scoped_root_differs_from_registers_and_global() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        run_git(
            project.path(),
            &[
                "remote",
                "add",
                "origin",
                "git@example.com:acme/widgets.git",
            ],
        );
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        let registers = registers_scoped_root(project.path()).unwrap();
        let plans_branch = plans_branch_scoped_root(project.path()).unwrap();
        let global = global_scoped_root(project.path()).unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            plans_branch,
            home.path()
                .join("tsch-ai-skills")
                .join("plans-branch")
                .join("acme")
                .join("widgets")
        );
        assert_ne!(plans_branch, registers);
        assert_ne!(plans_branch, global);
    }

    #[test]
    fn worktree_recognized_is_true_for_a_real_worktree_of_the_calling_project() {
        let _cwd = CWD_LOCK.lock().unwrap();
        let project = tempfile::tempdir().unwrap();
        init_repo_with_commit(project.path());
        let worktree_parent = tempfile::tempdir().unwrap();
        let worktree_path = worktree_parent.path().join("wt");
        run_git(
            project.path(),
            &[
                "worktree",
                "add",
                "-b",
                "feature",
                worktree_path.to_str().unwrap(),
            ],
        );
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project.path()).unwrap();
        let recognized = worktree_recognized(&worktree_path);
        std::env::set_current_dir(original).unwrap();
        assert!(recognized);
    }

    #[test]
    fn worktree_recognized_is_false_for_a_path_that_is_not_a_git_repo_at_all() {
        let _cwd = CWD_LOCK.lock().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        let not_a_repo = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project.path()).unwrap();
        let recognized = worktree_recognized(not_a_repo.path());
        std::env::set_current_dir(original).unwrap();
        assert!(!recognized);
    }

    #[test]
    fn worktree_recognized_is_false_for_an_unrelated_repo() {
        let _cwd = CWD_LOCK.lock().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        let unrelated = tempfile::tempdir().unwrap();
        run_git(unrelated.path(), &["init", "-q"]);
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project.path()).unwrap();
        let recognized = worktree_recognized(unrelated.path());
        std::env::set_current_dir(original).unwrap();
        assert!(!recognized);
    }

    #[test]
    fn is_on_branch_is_true_for_the_checked_out_branch_and_false_for_another() {
        let project = tempfile::tempdir().unwrap();
        init_repo_with_commit(project.path());
        run_git(project.path(), &["branch", "registers"]);
        run_git(project.path(), &["checkout", "-q", "registers"]);
        assert!(is_on_branch(project.path(), "registers"));
        assert!(!is_on_branch(project.path(), "main"));
        assert!(!is_on_branch(project.path(), "master"));
    }

    #[test]
    fn is_on_branch_is_false_for_a_path_that_is_not_a_git_repo_at_all() {
        let not_a_repo = tempfile::tempdir().unwrap();
        assert!(!is_on_branch(not_a_repo.path(), "registers"));
    }

    #[test]
    fn is_on_branch_recognises_a_worktree_checked_out_on_that_branch_from_outside_it() {
        // The scenario B404 reports: a dedicated worktree on the registers
        // branch, checked without ever changing into it.
        let project = tempfile::tempdir().unwrap();
        init_repo_with_commit(project.path());
        let worktree_parent = tempfile::tempdir().unwrap();
        let worktree_path = worktree_parent.path().join("registers-worktree");
        run_git(
            project.path(),
            &[
                "worktree",
                "add",
                "-b",
                "registers",
                worktree_path.to_str().unwrap(),
            ],
        );
        assert!(is_on_branch(&worktree_path, "registers"));
        assert!(!is_on_branch(project.path(), "registers"));
    }

    #[test]
    fn create_sparse_worktree_non_cone_checks_out_only_named_files() {
        let project = tempfile::tempdir().unwrap();
        init_repo_with_commit(project.path());
        fs::write(project.path().join("KEEP.json"), "{}").unwrap();
        fs::write(project.path().join("OTHER.txt"), "x").unwrap();
        run_git(project.path(), &["add", "-A"]);
        run_git(
            project.path(),
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-q",
                "-m",
                "add files",
            ],
        );
        let destination_parent = tempfile::tempdir().unwrap();
        let destination = destination_parent.path().join("wt");
        create_sparse_worktree(
            project.path(),
            &destination,
            "registers",
            false,
            &["KEEP.json"],
        )
        .unwrap();
        assert!(destination.join("KEEP.json").is_file());
        assert!(!destination.join("OTHER.txt").exists());
    }

    #[test]
    fn create_sparse_worktree_cone_checks_out_a_whole_directory() {
        let project = tempfile::tempdir().unwrap();
        init_repo_with_commit(project.path());
        fs::create_dir_all(project.path().join(".plans").join("sample")).unwrap();
        fs::write(
            project.path().join(".plans").join("sample").join("goal.md"),
            "# goal",
        )
        .unwrap();
        // Cone mode always includes files at the repository root, so the
        // excluded case needs its own subdirectory to prove anything.
        fs::create_dir_all(project.path().join("src")).unwrap();
        fs::write(project.path().join("src").join("other.rs"), "x").unwrap();
        run_git(project.path(), &["add", "-A"]);
        run_git(
            project.path(),
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-q",
                "-m",
                "add plans",
            ],
        );
        let destination_parent = tempfile::tempdir().unwrap();
        let destination = destination_parent.path().join("wt");
        create_sparse_worktree(project.path(), &destination, "plans", true, &[".plans"]).unwrap();
        assert!(destination
            .join(".plans")
            .join("sample")
            .join("goal.md")
            .is_file());
        assert!(!destination.join("src").join("other.rs").exists());
    }

    #[test]
    fn prompt_yes_no_interactive_reads_yes_no_and_defaults_on_an_empty_line() {
        let outcome = prompt_yes_no_with_default_from(
            "use a worktree?",
            true,
            false,
            true,
            &mut Cursor::new(b"y\n".to_vec()),
        );
        assert!(matches!(outcome, PromptOutcome::Answered(true)));

        let outcome = prompt_yes_no_with_default_from(
            "use a worktree?",
            true,
            false,
            true,
            &mut Cursor::new(b"n\n".to_vec()),
        );
        assert!(matches!(outcome, PromptOutcome::Answered(false)));

        let outcome = prompt_yes_no_with_default_from(
            "use a worktree?",
            true,
            false,
            true,
            &mut Cursor::new(b"\n".to_vec()),
        );
        assert!(matches!(outcome, PromptOutcome::Answered(true)));
    }

    #[test]
    fn prompt_yes_no_non_interactive_defaults_without_acting_on_piped_input() {
        let outcome = prompt_yes_no_with_default_from(
            "use a worktree?",
            true,
            false,
            false,
            &mut Cursor::new(b"y\n".to_vec()),
        );
        assert!(matches!(
            outcome,
            PromptOutcome::DefaultedNonInteractive(false)
        ));
    }

    #[test]
    fn declined_marker_path_is_stable_across_calls() {
        let candidate = PathBuf::from("/tmp/some/registers");
        assert_eq!(
            declined_marker_path(&candidate),
            declined_marker_path(&candidate)
        );
        assert_eq!(
            declined_marker_path(&candidate),
            PathBuf::from("/tmp/some/registers.declined")
        );
    }

    #[test]
    fn mark_declined_is_idempotent_and_is_declined_reflects_it() {
        let dir = tempfile::tempdir().unwrap();
        let candidate = dir.path().join("registers");
        assert!(!is_declined(&candidate));
        mark_declined(&candidate).unwrap();
        assert!(is_declined(&candidate));
        mark_declined(&candidate).unwrap();
        assert!(is_declined(&candidate));
    }

    #[test]
    fn prompt_branch_name_from_trims_and_declines_on_blank_input() {
        assert_eq!(
            prompt_branch_name_from(
                "branch name?",
                "registers",
                &mut Cursor::new(b"  custom-name  \n".to_vec())
            ),
            Some("custom-name".to_string())
        );
        assert_eq!(
            prompt_branch_name_from(
                "branch name?",
                "registers",
                &mut Cursor::new(b"\n".to_vec())
            ),
            None
        );
        assert_eq!(
            prompt_branch_name_from(
                "branch name?",
                "registers",
                &mut Cursor::new(b"   \n".to_vec())
            ),
            None
        );
    }

    #[test]
    fn read_tsch_config_defaults_on_missing_or_malformed_file() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        let missing = read_tsch_config(project.path());
        let path = tsch_config_path(project.path()).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();
        let malformed = read_tsch_config(project.path());
        std::env::remove_var("XDG_CONFIG_HOME");
        assert!(missing.registers_access.is_none());
        assert!(missing.plans_branch.is_none());
        assert!(malformed.registers_access.is_none());
    }

    #[test]
    fn write_tsch_config_patch_only_touches_named_fields() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        write_tsch_config_patch(
            project.path(),
            Some("dedicated-worktree"),
            Some("registers"),
            None,
            None,
        )
        .unwrap();
        write_tsch_config_patch(
            project.path(),
            None,
            None,
            Some("same-repo-branch"),
            Some("plans"),
        )
        .unwrap();
        let config = read_tsch_config(project.path());
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            config.registers_access,
            Some("dedicated-worktree".to_string())
        );
        assert_eq!(config.registers_branch, Some("registers".to_string()));
        assert_eq!(config.plans_storage, Some("same-repo-branch".to_string()));
        assert_eq!(config.plans_branch, Some("plans".to_string()));
    }

    #[test]
    fn write_tsch_config_patch_second_write_of_the_same_field_wins() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        write_tsch_config_patch(project.path(), Some("main-checkout"), None, None, None).unwrap();
        write_tsch_config_patch(project.path(), Some("dedicated-worktree"), None, None, None)
            .unwrap();
        let config = read_tsch_config(project.path());
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            config.registers_access,
            Some("dedicated-worktree".to_string())
        );
    }

    #[test]
    fn write_tsch_config_patch_creates_the_missing_directory_and_file() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        let path = tsch_config_path(project.path()).unwrap();
        assert!(!path.exists());
        write_tsch_config_patch(project.path(), Some("dedicated-worktree"), None, None, None)
            .unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert!(path.is_file());
    }

    #[test]
    fn tsch_config_path_differs_from_registers_scoped_root_only_in_its_root_segment() {
        let _env = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        run_git(project.path(), &["init", "-q"]);
        run_git(
            project.path(),
            &[
                "remote",
                "add",
                "origin",
                "git@example.com:acme/widgets.git",
            ],
        );
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        let config_path = tsch_config_path(project.path()).unwrap();
        let registers_path = registers_scoped_root(project.path()).unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            config_path,
            home.path()
                .join("tsch-ai-skills")
                .join("config")
                .join("acme")
                .join("widgets.json")
        );
        assert_eq!(
            registers_path,
            home.path()
                .join("tsch-ai-skills")
                .join("registers")
                .join("acme")
                .join("widgets")
        );
    }
}
