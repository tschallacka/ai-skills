// MODE: DEV
// PACKAGE: PROD
use crate::error::CliError;
use crate::extract::extract;
use crate::target::{classify_target, ResolvedTarget};
use std::process::Command;

fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("ci-failures: could not run {program}: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "-q", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

fn pr_head_branch(repo_slug: &str, pr_number: &str) -> Result<String, CliError> {
    let out = run_command(
        "gh",
        &[
            "pr",
            "view",
            pr_number,
            "--repo",
            repo_slug,
            "--json",
            "headRefName",
        ],
    )
    .map_err(|_| CliError::resolution(format!("ci-failures: no PR {pr_number}")))?;
    let value: serde_json::Value = serde_json::from_str(&out)
        .map_err(|_| CliError::resolution(format!("ci-failures: no PR {pr_number}")))?;
    value
        .get("headRefName")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| CliError::resolution(format!("ci-failures: no PR {pr_number}")))
}

/// The newest run for a branch, ignoring the artifact-render workflow: it is
/// almost always green and never the reason a check suite is red.
fn latest_run_for(repo_slug: &str, branch: &str) -> Result<String, CliError> {
    let out = run_command(
        "gh",
        &[
            "run",
            "list",
            "--repo",
            repo_slug,
            "--branch",
            branch,
            "--limit",
            "20",
            "--json",
            "databaseId,name",
        ],
    )?;
    let runs: Vec<serde_json::Value> =
        serde_json::from_str(&out).map_err(|error| CliError::resolution(error.to_string()))?;
    let id = runs
        .iter()
        .find(|run| run.get("name").and_then(|n| n.as_str()) != Some("render-artifacts"))
        .and_then(|run| run.get("databaseId"))
        .and_then(|id| id.as_u64());
    id.map(|id| id.to_string())
        .ok_or_else(|| CliError::resolution(format!("ci-failures: no runs for branch {branch}")))
}

/// Resolves the target to a run id, mirroring gh_resolve_run's own logic:
/// pr/N and a short bare number both resolve a PR to its head branch first;
/// empty resolves the current git branch (refusing on detached HEAD); a
/// non-numeric target is used as a branch name directly; a long bare number
/// is a run id with no further resolution.
pub fn resolve_run(repo_slug: &str, target: &str) -> Result<String, CliError> {
    match classify_target(target) {
        ResolvedTarget::RunOrPipelineId(id) => Ok(id),
        ResolvedTarget::PrOrMrNumber(number) => {
            let branch = pr_head_branch(repo_slug, &number)?;
            latest_run_for(repo_slug, &branch)
        }
        ResolvedTarget::Branch(branch) => latest_run_for(repo_slug, &branch),
        ResolvedTarget::CurrentBranch => {
            let branch = current_branch().ok_or_else(|| {
                CliError::bad_usage("ci-failures: detached HEAD; name a run, PR or branch")
            })?;
            latest_run_for(repo_slug, &branch)
        }
    }
}

fn json_field(text: &str, field: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| value.get(field).cloned())
}

struct Job {
    id: String,
    conclusion: String,
    name: String,
}

fn jobs_for(repo_slug: &str, run_id: &str, want_all: bool) -> Result<Vec<Job>, CliError> {
    let out = run_command(
        "gh",
        &["run", "view", run_id, "--repo", repo_slug, "--json", "jobs"],
    )?;
    let value: serde_json::Value =
        serde_json::from_str(&out).map_err(|error| CliError::resolution(error.to_string()))?;
    let jobs = value
        .get("jobs")
        .and_then(|jobs| jobs.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(jobs
        .into_iter()
        .filter_map(|job| {
            let id = job.get("databaseId")?.as_u64()?.to_string();
            let conclusion = job
                .get("conclusion")
                .and_then(|c| c.as_str())
                .unwrap_or("running")
                .to_string();
            let name = job.get("name").and_then(|n| n.as_str())?.to_string();
            (want_all || conclusion == "failure").then_some(Job {
                id,
                conclusion,
                name,
            })
        })
        .collect())
}

fn print_job(repo_slug: &str, raw_dir: Option<&str>, job: &Job) {
    println!("\n== {}  [{}]  job {}", job.name, job.conclusion, job.id);
    let log = Command::new("gh")
        .args([
            "api",
            "--allow-escape-sequences",
            &format!("repos/{repo_slug}/actions/jobs/{}/logs", job.id),
        ])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();
    if log.is_empty() {
        println!("    (no log; a job that never started has none)");
        return;
    }
    if let Some(dir) = raw_dir {
        let cleaned = crate::extract::strip_ansi_and_cr(&log);
        let path = format!("{dir}/{}.log", job.id);
        if std::fs::create_dir_all(dir).is_ok() && std::fs::write(&path, cleaned).is_ok() {
            println!("    raw: {path}");
        }
    }
    let found = extract(&log);
    if found.is_empty() {
        println!("    (nothing matched the failure patterns; read the raw log)");
    } else {
        print!("{found}");
    }
}

pub fn run(
    repo_slug: &str,
    target: &str,
    raw_dir: Option<&str>,
    want_all: bool,
) -> Result<(), CliError> {
    if Command::new("gh").arg("--version").output().is_err() {
        return Err(CliError::missing_tool("ci-failures: gh is required"));
    }
    let run_id = resolve_run(repo_slug, target)?;
    let status_out = run_command(
        "gh",
        &[
            "run", "view", &run_id, "--repo", repo_slug, "--json", "status",
        ],
    )?;
    let status = json_field(&status_out, "status")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let conclusion_out = run_command(
        "gh",
        &[
            "run",
            "view",
            &run_id,
            "--repo",
            repo_slug,
            "--json",
            "conclusion",
        ],
    )?;
    let conclusion = json_field(&conclusion_out, "conclusion")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "pending".to_string());

    println!(
        "forge: gh\nrun {run_id}  {status}/{conclusion}  https://github.com/{repo_slug}/actions/runs/{run_id}"
    );

    let jobs = jobs_for(repo_slug, &run_id, want_all)?;
    if jobs.is_empty() {
        if conclusion == "failure" {
            println!(
                "\nno job reports failure yet, though the run does: it may still be settling,\nor the failure is at the workflow level (a cancelled or skipped required job).\nRe-run with --all to see every job."
            );
            return Ok(());
        }
        println!("\nno failing jobs.");
        return Ok(());
    }
    for job in &jobs {
        print_job(repo_slug, raw_dir, job);
    }
    Ok(())
}
