// MODE: DEV
// PACKAGE: PROD

/// Every fallible operation in this crate carries its own real exit code,
/// mirroring the specific codes ci-failures.sh itself uses at each failure
/// site (64 bad usage/detached HEAD, 66 a resolution or API failure, 69 a
/// required CLI missing) rather than one generic error type that would blur
/// them together.
#[derive(Debug, PartialEq, Eq)]
pub struct CliError {
    pub message: String,
    pub code: u8,
}

impl CliError {
    pub fn new(message: impl Into<String>, code: u8) -> Self {
        CliError {
            message: message.into(),
            code,
        }
    }

    /// 64: bad usage, an invalid CI_FAILURES_FORGE value, or detached HEAD
    /// with no target to disambiguate.
    pub fn bad_usage(message: impl Into<String>) -> Self {
        Self::new(message, 64)
    }

    /// 66: could not resolve the target, or an underlying API call failed
    /// (no such PR/MR, no runs/pipelines for a branch, could not tell which
    /// forge a remote is, or a subprocess itself failed).
    pub fn resolution(message: impl Into<String>) -> Self {
        Self::new(message, 66)
    }

    /// 69: the required gh or glab CLI is not present.
    pub fn missing_tool(message: impl Into<String>) -> Self {
        Self::new(message, 69)
    }
}

impl From<String> for CliError {
    fn from(message: String) -> Self {
        CliError::resolution(message)
    }
}
