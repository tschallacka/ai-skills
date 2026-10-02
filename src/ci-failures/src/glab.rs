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

/// The project path, told apart by whether a scheme is present (a URL form,
/// https://host/group/sub/project.git, has the project path after the FIRST
/// slash past the host) from an scp-like form (git@host:group/sub/project.git,
/// which has no host slash to strip at all) -- applying both stripping steps
/// to the scp-like form would eat its first path segment as if it were a
/// host, exactly the bug ci-failures-glab-lib.sh's own comment records
/// having found.
pub fn project_path(
    repo_override: Option<&str>,
    remote_url: Option<&str>,
) -> Result<String, String> {
    if let Some(value) = repo_override {
        return Ok(value.to_string());
    }
    let url = remote_url
        .filter(|url| !url.is_empty())
        .ok_or_else(|| "ci-failures: no origin remote; set CI_FAILURES_REPO".to_string())?;
    let path = if let Some(rest) = url.split_once("://") {
        rest.1.split_once('/').map(|(_, p)| p).unwrap_or("")
    } else {
        let after_at = url.split_once('@').map(|(_, r)| r).unwrap_or(url);
        after_at.split_once(':').map(|(_, p)| p).unwrap_or(after_at)
    };
    Ok(path.strip_suffix(".git").unwrap_or(path).to_string())
}

pub fn encoded_project(project_path: &str) -> String {
    project_path.replace('/', "%2F")
}

fn pr_head_branch(project: &str, mr_number: &str) -> Result<String, CliError> {
    let out = run_command(
        "glab",
        &[
            "api",
            &format!("projects/{project}/merge_requests/{mr_number}"),
        ],
    )
    .map_err(|_| CliError::resolution(format!("ci-failures: no MR {mr_number}")))?;
    let value: serde_json::Value = serde_json::from_str(&out)
        .map_err(|_| CliError::resolution(format!("ci-failures: no MR {mr_number}")))?;
    value
        .get("source_branch")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| CliError::resolution(format!("ci-failures: no MR {mr_number}")))
}

fn latest_pipeline_for(project: &str, branch: &str) -> Result<String, CliError> {
    let out = run_command(
        "glab",
        &[
            "api",
            &format!("projects/{project}/pipelines?ref={branch}&order_by=id&sort=desc&per_page=1"),
        ],
    )?;
    let runs: Vec<serde_json::Value> =
        serde_json::from_str(&out).map_err(|error| CliError::resolution(error.to_string()))?;
    runs.first()
        .and_then(|run| run.get("id"))
        .and_then(|id| id.as_u64())
        .map(|id| id.to_string())
        .ok_or_else(|| {
            CliError::resolution(format!("ci-failures: no pipelines for branch {branch}"))
        })
}

/// Resolves the target to a pipeline id, mirroring glab_resolve_pipeline's
/// own logic -- identical shape to gh's resolve_run, against GitLab's own
/// merge_requests/pipelines endpoints instead.
pub fn resolve_pipeline(project: &str, target: &str) -> Result<String, CliError> {
    match classify_target(target) {
        ResolvedTarget::RunOrPipelineId(id) => Ok(id),
        ResolvedTarget::PrOrMrNumber(number) => {
            let branch = pr_head_branch(project, &number)?;
            latest_pipeline_for(project, &branch)
        }
        ResolvedTarget::Branch(branch) => latest_pipeline_for(project, &branch),
        ResolvedTarget::CurrentBranch => {
            let branch = current_branch().ok_or_else(|| {
                CliError::bad_usage("ci-failures: detached HEAD; name a pipeline, MR or branch")
            })?;
            latest_pipeline_for(project, &branch)
        }
    }
}

struct Job {
    id: String,
    status: String,
    name: String,
}

fn jobs_for(project: &str, pipeline_id: &str, want_all: bool) -> Result<Vec<Job>, CliError> {
    let out = run_command(
        "glab",
        &[
            "api",
            &format!("projects/{project}/pipelines/{pipeline_id}/jobs?per_page=100"),
        ],
    )?;
    let jobs: Vec<serde_json::Value> =
        serde_json::from_str(&out).map_err(|error| CliError::resolution(error.to_string()))?;
    Ok(jobs
        .into_iter()
        .filter_map(|job| {
            let id = job.get("id")?.as_u64()?.to_string();
            let status = job.get("status").and_then(|s| s.as_str())?.to_string();
            let name = job.get("name").and_then(|n| n.as_str())?.to_string();
            (want_all || status == "failed").then_some(Job { id, status, name })
        })
        .collect())
}

fn print_job(project: &str, raw_dir: Option<&str>, job: &Job) {
    println!("\n== {}  [{}]  job {}", job.name, job.status, job.id);
    // The trace endpoint returns the raw job log as plain text, not JSON --
    // piped straight through, not parsed.
    let log = Command::new("glab")
        .args(["api", &format!("projects/{project}/jobs/{}/trace", job.id)])
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
    repo_override: Option<&str>,
    remote_url: Option<&str>,
    target: &str,
    raw_dir: Option<&str>,
    want_all: bool,
) -> Result<(), CliError> {
    if Command::new("glab").arg("--version").output().is_err() {
        return Err(CliError::missing_tool("ci-failures: glab is required"));
    }
    let path = project_path(repo_override, remote_url)?;
    let project = encoded_project(&path);
    let pipeline_id = resolve_pipeline(&project, target)?;

    let status_out = run_command(
        "glab",
        &[
            "api",
            &format!("projects/{project}/pipelines/{pipeline_id}"),
        ],
    )?;
    let status = serde_json::from_str::<serde_json::Value>(&status_out)
        .ok()
        .and_then(|v| v.get("status").and_then(|s| s.as_str().map(str::to_string)))
        .unwrap_or_default();

    println!(
        "forge: glab\npipeline {pipeline_id}  {status}  https://gitlab.com/{path}/-/pipelines/{pipeline_id}"
    );

    let jobs = jobs_for(&project, &pipeline_id, want_all)?;
    if jobs.is_empty() {
        if status == "failed" {
            println!(
                "\nno job reports failed yet, though the pipeline does: it may still be\nsettling, or the failure is at the pipeline level. Re-run with --all to\nsee every job."
            );
            return Ok(());
        }
        println!("\nno failing jobs.");
        return Ok(());
    }
    for job in &jobs {
        print_job(&project, raw_dir, job);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_form_project_path_strips_scheme_host_and_git_suffix() {
        assert_eq!(
            project_path(None, Some("https://gitlab.com/group/sub/project.git")).unwrap(),
            "group/sub/project"
        );
    }

    #[test]
    fn scp_like_form_project_path_strips_user_host_and_git_suffix() {
        assert_eq!(
            project_path(None, Some("git@gitlab.com:group/sub/project.git")).unwrap(),
            "group/sub/project"
        );
    }

    #[test]
    fn an_explicit_repo_override_wins_outright() {
        assert_eq!(
            project_path(
                Some("explicit/project"),
                Some("https://gitlab.com/other/project.git")
            )
            .unwrap(),
            "explicit/project"
        );
    }

    #[test]
    fn no_remote_and_no_override_is_refused() {
        let error = project_path(None, None).unwrap_err();
        assert!(error.contains("no origin remote"));
    }

    #[test]
    fn encoded_project_percent_encodes_every_slash() {
        assert_eq!(
            encoded_project("group/sub/project"),
            "group%2Fsub%2Fproject"
        );
    }
}
