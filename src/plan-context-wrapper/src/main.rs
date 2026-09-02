// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const COMMAND: &str = "plan-context-wrapper.sh";

fn usage(code: u8) -> ! {
    println!("Usage: {COMMAND} <variables-file> <plan-context.sh arguments...>");
    println!("       {COMMAND} --help");
    std::process::exit(code.into());
}

fn context_reader_from(
    executable: &Path,
    explicit: Option<&Path>,
    scripts_root: Option<&Path>,
) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    if let Some(parent) = executable.parent() {
        let binary = parent.join("plan-context");
        if binary.is_file() {
            return binary;
        }
    }
    if let Some(root) = scripts_root {
        return root.join("plan-context.sh");
    }
    PathBuf::from("plan-context")
}

fn context_reader() -> PathBuf {
    let executable = env::current_exe().unwrap_or_default();
    let explicit = env::var_os("PLANNING_PLAN_CONTEXT");
    let scripts_root = env::var_os("PLANNING_SCRIPTS_ROOT");
    context_reader_from(
        &executable,
        explicit.as_deref().map(Path::new),
        scripts_root.as_deref().map(Path::new),
    )
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(variables_file) = args.next() else {
        usage(64);
    };
    if variables_file == "--help" || variables_file == "-h" {
        usage(0);
    }
    let Some(first_reader_arg) = args.next() else {
        usage(64);
    };
    let variables_file = PathBuf::from(variables_file);
    if !variables_file.is_file() {
        eprintln!("variables file not found: {}", variables_file.display());
        return ExitCode::from(66);
    }

    let reader = context_reader();
    let mut command = Command::new("bash");
    command
        .arg("-c")
        .arg("set -euo pipefail; source \"$1\"; shift; reader=\"$1\"; shift; exec \"$reader\" \"$@\"")
        .arg("plan-context-wrapper")
        .arg(&variables_file)
        .arg(&reader)
        .arg(first_reader_arg);
    command.args(args);
    match command.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).try_into().unwrap_or(1)),
        Err(error) => {
            eprintln!("{COMMAND}: {error}");
            ExitCode::from(66)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::context_reader_from;
    use std::path::Path;

    #[test]
    fn explicit_reader_wins() {
        let path = context_reader_from(
            Path::new("/tmp/bin/wrapper"),
            Some(Path::new("/custom/reader")),
            Some(Path::new("/scripts")),
        );
        assert_eq!(path, Path::new("/custom/reader"));
    }

    #[test]
    fn scripts_root_is_used_when_sibling_is_absent() {
        let path = context_reader_from(
            Path::new("/tmp/bin/wrapper"),
            None,
            Some(Path::new("/scripts")),
        );
        assert_eq!(path, Path::new("/scripts/plan-context.sh"));
    }

    #[test]
    fn scripts_root_is_the_transition_fallback() {
        let path = context_reader_from(
            Path::new("/missing/wrapper"),
            None,
            Some(Path::new("/scripts")),
        );
        assert_eq!(path, Path::new("/scripts/plan-context.sh"));
    }

    #[test]
    fn path_lookup_is_the_final_fallback() {
        let path = context_reader_from(Path::new("/missing/wrapper"), None, None);
        assert_eq!(path, Path::new("plan-context"));
    }
}
