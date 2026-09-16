// MODE: DEV
// PACKAGE: PROD

/// What a caller-supplied target string means, before either forge backend
/// makes a single API call. Mirrors gh_resolve_run's and
/// glab_resolve_pipeline's own identical structure in the bash source: pr/N
/// and a short bare number both name a PR/MR, a long bare number names a
/// run/pipeline id directly, empty names the current branch, and anything
/// else is a branch name. The magnitude threshold (9+ digits is a run id,
/// shorter is a PR/MR number) is a stated heuristic in the bash source
/// itself, preserved exactly rather than "fixed" -- this port's scope is
/// behavioral parity, not judgment changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTarget {
    /// A run or pipeline id, usable directly with no further resolution.
    RunOrPipelineId(String),
    /// A PR or MR number (GitHub calls it a PR, GitLab an MR; pr/N is the
    /// spelling both forges share in this tool), needing its head/source
    /// branch resolved via one more API call before the latest run/pipeline
    /// for that branch can be found.
    PrOrMrNumber(String),
    /// A plain branch name, resolved directly via the "latest run/pipeline
    /// for this branch" call.
    Branch(String),
    /// No target was given at all: use the current git branch.
    CurrentBranch,
}

const RUN_ID_MAGNITUDE_THRESHOLD: usize = 9;

pub fn classify_target(target: &str) -> ResolvedTarget {
    if let Some(number) = target.strip_prefix("pr/") {
        return ResolvedTarget::PrOrMrNumber(number.to_string());
    }
    if target.is_empty() {
        return ResolvedTarget::CurrentBranch;
    }
    if !target.bytes().all(|byte| byte.is_ascii_digit()) {
        return ResolvedTarget::Branch(target.to_string());
    }
    if target.len() >= RUN_ID_MAGNITUDE_THRESHOLD {
        ResolvedTarget::RunOrPipelineId(target.to_string())
    } else {
        ResolvedTarget::PrOrMrNumber(target.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pr_prefixed_target_names_a_pr_or_mr_number_unambiguously() {
        assert_eq!(
            classify_target("pr/47"),
            ResolvedTarget::PrOrMrNumber("47".to_string())
        );
    }

    #[test]
    fn an_empty_target_means_the_current_branch() {
        assert_eq!(classify_target(""), ResolvedTarget::CurrentBranch);
    }

    #[test]
    fn a_non_numeric_target_is_a_branch_name() {
        assert_eq!(
            classify_target("fix/some-branch"),
            ResolvedTarget::Branch("fix/some-branch".to_string())
        );
    }

    #[test]
    fn a_short_bare_number_is_a_pr_or_mr_number() {
        assert_eq!(
            classify_target("47"),
            ResolvedTarget::PrOrMrNumber("47".to_string())
        );
    }

    #[test]
    fn a_long_bare_number_is_a_run_or_pipeline_id() {
        assert_eq!(
            classify_target("33894205595"),
            ResolvedTarget::RunOrPipelineId("33894205595".to_string())
        );
    }

    #[test]
    fn the_magnitude_threshold_is_exactly_nine_digits() {
        assert_eq!(
            classify_target("12345678"),
            ResolvedTarget::PrOrMrNumber("12345678".to_string())
        );
        assert_eq!(
            classify_target("123456789"),
            ResolvedTarget::RunOrPipelineId("123456789".to_string())
        );
    }
}
