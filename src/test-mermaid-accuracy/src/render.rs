// MODE: DEV
// PACKAGE: PROD

//! The mmdc render check: the only proof mermaid itself accepts a block.
//! `mmdc` comes from the dev flake and is not a suite dependency, so this
//! reports UNCONFIGURED when it is absent rather than passing quietly.
//!
//! AR-126: the profile directory used for chromium's `--user-data-dir` is a
//! hardcoded-short root under `/tmp`, NEVER derived from `TMPDIR` (a unix
//! socket path is capped near 104 bytes) -- the compiled binary never
//! sources lib-test.sh, so `T_SOCKET_TMPDIR` is never set on this path
//! either way.
//!
//! AR-135: the mmdc-absent branch never calls group_done (reproduced by the
//! caller checking `Outcome::Unconfigured` and skipping that print), while
//! both other outcomes do.

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Diagram {
    pub where_: String,
    pub path: PathBuf,
    pub is_sequence: bool,
}

pub enum Outcome {
    /// mmdc not on PATH at all.
    Unconfigured,
    /// mmdc present but even the control flowchart could not render.
    BrowserUnconfigured,
    /// mmdc present and at least the control flowchart rendered.
    Rendered {
        rendered: u32,
        sequence_skew_warning: Option<String>,
        findings: Vec<String>,
        fail_findings: Vec<String>,
    },
}

fn mmdc_on_path() -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join("mmdc").is_file()))
        .unwrap_or(false)
}

fn socket_tmpdir_root() -> &'static str {
    // AR-126: hardcoded, never TMPDIR-derived.
    "/tmp"
}

fn render_once(diagram: &Path, output: &Path, work: &Path) -> Result<(), String> {
    let profile = tempfile_dir(socket_tmpdir_root(), "profile");
    let puppeteer_json = work.join("puppeteer.json");
    let _ = std::fs::write(
        &puppeteer_json,
        format!(
            "{{ \"args\": [\"--no-sandbox\", \"--disable-dev-shm-usage\", \"--user-data-dir={}\"] }}",
            profile.display()
        ),
    );
    let render_log = work.join("render.log");
    let result = Command::new("mmdc")
        .arg("-q")
        .arg("-p")
        .arg(&puppeteer_json)
        .arg("-i")
        .arg(diagram)
        .arg("-o")
        .arg(output)
        .output();
    let _ = std::fs::remove_dir_all(&profile);
    match result {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let mut combined = out.stdout;
            combined.extend_from_slice(&out.stderr);
            let _ = std::fs::write(&render_log, &combined);
            Err(String::from_utf8_lossy(&combined).into_owned())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// A short, unique directory under `root/<prefix>.XXXXXX`-shaped, matching
/// `mktemp -d`'s own short-path contract without depending on TMPDIR.
fn tempfile_dir(root: &str, prefix: &str) -> PathBuf {
    let unique = format!(
        "{prefix}.{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    );
    let dir = Path::new(root).join(unique);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn run(work: &Path, diagrams: &[Diagram]) -> Outcome {
    if !mmdc_on_path() {
        return Outcome::Unconfigured;
    }

    let control_flow = work.join("control-flowchart.mmd");
    let _ = std::fs::write(&control_flow, "flowchart TD\n    A-->B\n");
    if render_once(&control_flow, &work.join("control-flowchart.svg"), work).is_err() {
        return Outcome::BrowserUnconfigured;
    }

    let control_seq = work.join("control-sequence.mmd");
    let _ = std::fs::write(&control_seq, "sequenceDiagram\n    A->>B: hi\n");
    let mut sequence_skew = false;
    let mut sequence_skew_warning = None;
    if render_once(&control_seq, &work.join("control-sequence.svg"), work).is_err() {
        sequence_skew = true;
        sequence_skew_warning = Some(
            "mermaid-accuracy: WARN: this browser build cannot render any sequenceDiagram (mermaid-cli/chromium skew); sequence blocks are checked structurally, and CI mermaid-render is the syntax authority for them".to_string(),
        );
    }

    let mut rendered = 0u32;
    let mut findings = Vec::new();
    let mut fail_findings = Vec::new();
    for diagram in diagrams {
        rendered += 1;
        let output = diagram.path.with_extension("mmd.svg");
        if render_once(&diagram.path, &output, work).is_ok() {
            continue;
        }
        let attempt2 = render_once(&diagram.path, &output, work);
        if attempt2.is_ok() {
            continue;
        }
        let log = attempt2.unwrap_err();
        if sequence_skew && log.contains("svg element not in render tree") && diagram.is_sequence {
            findings.push(format!(
                "mermaid-accuracy: WARN: {} not rendered -- the local browser build rejects every sequenceDiagram; see the probe warning above",
                diagram.where_
            ));
            continue;
        }
        let truncated: String = log
            .chars()
            .map(|c| if c == '\n' { ' ' } else { c })
            .take(200)
            .collect();
        fail_findings.push(format!(
            "mmdc rejected the diagram at {}: {truncated}",
            diagram.where_
        ));
    }

    Outcome::Rendered {
        rendered,
        sequence_skew_warning,
        findings,
        fail_findings,
    }
}
