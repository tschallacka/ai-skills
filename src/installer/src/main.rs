// MODE: DEV
// PACKAGE: PROD
//! installer — the binary a downloaded release package hands off to once the
//! tiny curl-piped bootstrap script has fetched and extracted it. Runs from
//! inside the extracted tree; knows nothing about fetching itself.
//!
//! This is an early slice, not full parity with install.sh yet: it installs
//! one skill directory fresh, atomically, with a .version marker. It does not
//! yet do install.sh's digest-based backup/merge (60-install.sh's
//! content_digest/record_digests), the interactive TUI, per-agent permission
//! grants, or manifest-driven multi-skill selection. Those port next.

mod install;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
installer — installs skills from the release tree it ships inside.

Usage:
  installer --platform                     print the resolved target triple
  installer install --skill NAME --source DIR --target DIR
                                            install one skill fresh
  installer --help
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("installer: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(argv: &[String]) -> Result<ExitCode, String> {
    match argv.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => {
            print!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Some("--platform") => {
            let target = installer_platform::current().map_err(|e| e.to_string())?;
            println!("{target}");
            Ok(ExitCode::SUCCESS)
        }
        Some("install") => run_install(&argv[1..]),
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

/// The extracted release tree's own `skills/` directory, next to wherever this
/// binary itself was run from. `--source` overrides it; nothing else needs to
/// know the layout the bootstrap script's tar extracted.
fn default_source() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("resolving own path: {e}"))?;
    let dir = exe
        .parent()
        .ok_or("installer binary has no parent directory")?;
    Ok(dir.join("skills"))
}

fn run_install(argv: &[String]) -> Result<ExitCode, String> {
    let mut skill: Option<String> = None;
    let mut source: Option<PathBuf> = None;
    let mut target: Option<PathBuf> = None;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--skill" => {
                i += 1;
                skill = Some(argv.get(i).ok_or("--skill needs a value")?.clone());
            }
            "--source" => {
                i += 1;
                source = Some(PathBuf::from(argv.get(i).ok_or("--source needs a value")?));
            }
            "--target" => {
                i += 1;
                target = Some(PathBuf::from(argv.get(i).ok_or("--target needs a value")?));
            }
            other => return Err(format!("install: unknown option: {other}")),
        }
        i += 1;
    }

    let skill = skill.ok_or("install: --skill is required")?;
    let source = match source {
        Some(s) => s,
        None => default_source()?,
    };
    let target = target.ok_or("install: --target is required")?;

    install::install_skill_fresh(&source, &skill, &target).map_err(|e| e.to_string())?;
    println!("installed {skill} -> {}", target.join(&skill).display());
    Ok(ExitCode::SUCCESS)
}
