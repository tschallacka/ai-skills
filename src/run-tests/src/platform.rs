// MODE: DEV
// PACKAGE: PROD

//! The few places where a Windows host differs from a unix one, kept in one
//! module so the rest of the crate reads the same on both.
//!
//! What goes wrong on Windows, all of it observed in CI rather than assumed:
//!
//! - `Command::new("bash")` (and `find`, `sort`, `timeout`) is not resolved
//!   through PATH first. Rust searches the system directories before it, so
//!   the name finds `C:\Windows\System32\bash.exe` (the WSL launcher, which
//!   fails without a distro), `find.exe` and `sort.exe` (unrelated Windows
//!   tools that reject GNU arguments) and `timeout.exe` (a "wait N seconds"
//!   command), not the Git for Windows tools a caller put first on PATH.
//! - A file is only a program when it ends in `.exe`, so `dir.join("cargo")`
//!   is never a file there.
//! - A `.sh` file cannot be started; it has to be handed to bash.
//! - There is no `/tmp`, no `ps`, no `date -u`, and PATH entries are joined
//!   with `;`.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Where `program` would run from on PATH, or None. Walks PATH itself, so the
/// answer is the first match a shell would give, and applies the platform's
/// executable suffix. `bash` skips the WSL launcher directories on Windows.
pub fn resolve(program: &str) -> Option<PathBuf> {
    let name = format!("{program}{}", env::consts::EXE_SUFFIX);
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file() && !(program == "bash" && is_wsl_launcher(candidate)))
}

pub fn which(program: &str) -> bool {
    resolve(program).is_some()
}

fn is_wsl_launcher(path: &Path) -> bool {
    let lowered = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', "\\");
    lowered.contains("\\windows\\system32\\") || lowered.contains("\\windowsapps\\")
}

/// The bash to run scripts with: what `RUN_TESTS_BASH` names when it is set,
/// otherwise `bash`, resolved through PATH on Windows (see the module doc).
pub fn bash_program(configured: &str) -> PathBuf {
    if configured == "bash" {
        if let Some(found) = resolve("bash") {
            return found;
        }
    }
    PathBuf::from(configured)
}

/// The `RUN_TESTS_BASH` interpreter, or plain `bash`.
pub fn bash_from_env() -> String {
    env::var("RUN_TESTS_BASH").unwrap_or_else(|_| "bash".to_string())
}

/// A path in the form bash accepts on every platform: forward slashes.
pub fn to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A command that runs the script at `path`. Unix starts the file itself
/// (its `#!` line and mode bits stay in charge); Windows cannot start a
/// script, so it goes through bash.
pub fn script_command(path: &Path) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new(bash_program(&bash_from_env()));
        command.arg(to_posix(path));
        command
    }
    #[cfg(not(windows))]
    {
        Command::new(path)
    }
}

/// Turns a path some bash printed (`/d/a/x`) into one a Windows program
/// understands (`D:\a\x`). Anything that is not such a path is returned as is.
pub fn to_native_path(printed: &str) -> String {
    #[cfg(windows)]
    {
        if printed.starts_with('/') {
            if let Some(cygpath) = resolve("cygpath") {
                if let Ok(out) = Command::new(cygpath).arg("-w").arg(printed).output() {
                    let converted = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if out.status.success() && !converted.is_empty() {
                        return converted;
                    }
                }
            }
        }
    }
    printed.to_string()
}

/// `dir` in front of the current PATH, in the platform's own separator.
pub fn prepend_to_path(dir: &str) -> Option<String> {
    let mut parts = vec![PathBuf::from(dir)];
    parts.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    env::join_paths(parts)
        .ok()
        .map(|joined| joined.to_string_lossy().into_owned())
}

/// The machine-wide temporary directory the lock file and the scratch scan
/// live under: `/tmp` where there is one, the platform's own otherwise.
pub fn system_tmp() -> PathBuf {
    if cfg!(windows) {
        env::temp_dir()
    } else {
        PathBuf::from("/tmp")
    }
}

/// "YYYY-MM-DDTHH:MM:SSZ" for now, without shelling out to `date -u`.
pub fn utc_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    utc_stamp_at(seconds)
}

fn utc_stamp_at(seconds: u64) -> String {
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let within = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        within / 3600,
        within % 3600 / 60,
        within % 60
    )
}

/// Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_the_first_of_january_1970() {
        assert_eq!(utc_stamp_at(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_known_instant_formats_the_way_date_dash_u_does() {
        assert_eq!(utc_stamp_at(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn a_leap_day_is_kept() {
        // 2024-02-29T12:00:00Z
        assert_eq!(utc_stamp_at(1_709_208_000), "2024-02-29T12:00:00Z");
    }

    #[test]
    fn backslashes_become_forward_slashes_for_bash() {
        assert_eq!(to_posix(Path::new("a\\b\\c.sh")), "a/b/c.sh");
    }

    #[test]
    fn the_wsl_launcher_is_recognised_by_its_directory() {
        assert!(is_wsl_launcher(Path::new(
            "C:\\Windows\\System32\\bash.exe"
        )));
        assert!(!is_wsl_launcher(Path::new(
            "C:\\Program Files\\Git\\bin\\bash.exe"
        )));
    }
}
