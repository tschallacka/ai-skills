// MODE: DEV
// PACKAGE: PROD
use crate::error::CliError;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge {
    Gh,
    Glab,
}

/// Anything real enough to stand in for the two probes detect_forge falls
/// back to when neither the override nor the remote URL settle it: "is this
/// CLI on PATH and does it recognize the current repo." Injected so unit
/// tests never actually shell out to a real gh/glab.
pub trait ForgeProbe {
    fn gh_available(&self) -> bool;
    fn glab_available(&self) -> bool;
}

pub struct RealProbe;

impl ForgeProbe for RealProbe {
    fn gh_available(&self) -> bool {
        Command::new("gh")
            .arg("repo")
            .arg("view")
            .output()
            .is_ok_and(|out| out.status.success())
    }
    fn glab_available(&self) -> bool {
        Command::new("glab")
            .arg("repo")
            .arg("view")
            .output()
            .is_ok_and(|out| out.status.success())
    }
}

/// An explicit CI_FAILURES_FORGE override wins outright (refused if it names
/// anything else); otherwise the origin remote's own host decides;
/// otherwise whichever CLI is present and already speaks for this remote,
/// gh first only because it was written first, not because it is preferred.
pub fn detect_forge(
    forge_override: Option<&str>,
    remote_url: Option<&str>,
    probe: &dyn ForgeProbe,
) -> Result<Forge, CliError> {
    if let Some(value) = forge_override {
        return match value {
            "gh" => Ok(Forge::Gh),
            "glab" => Ok(Forge::Glab),
            other => Err(CliError::bad_usage(format!(
                "ci-failures: CI_FAILURES_FORGE must be gh or glab, not {other}"
            ))),
        };
    }
    if let Some(url) = remote_url {
        if url.contains("github.com") {
            return Ok(Forge::Gh);
        }
        if url.contains("gitlab.com") {
            return Ok(Forge::Glab);
        }
    }
    if probe.gh_available() {
        return Ok(Forge::Gh);
    }
    if probe.glab_available() {
        return Ok(Forge::Glab);
    }
    Err(CliError::resolution(format!(
        "ci-failures: could not tell which forge {} is; set CI_FAILURES_FORGE=gh or glab",
        remote_url.unwrap_or("this remote")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Probe {
        gh: bool,
        glab: bool,
    }
    impl ForgeProbe for Probe {
        fn gh_available(&self) -> bool {
            self.gh
        }
        fn glab_available(&self) -> bool {
            self.glab
        }
    }
    const NEITHER: Probe = Probe {
        gh: false,
        glab: false,
    };

    #[test]
    fn override_gh_wins_outright() {
        assert_eq!(
            detect_forge(Some("gh"), Some("https://gitlab.com/x/y"), &NEITHER),
            Ok(Forge::Gh)
        );
    }

    #[test]
    fn override_glab_wins_outright() {
        assert_eq!(
            detect_forge(Some("glab"), Some("https://github.com/x/y"), &NEITHER),
            Ok(Forge::Glab)
        );
    }

    #[test]
    fn an_invalid_override_is_refused() {
        let error = detect_forge(Some("bogus"), None, &NEITHER).unwrap_err();
        assert!(error
            .message
            .contains("CI_FAILURES_FORGE must be gh or glab, not bogus"));
        assert_eq!(error.code, 64);
    }

    #[test]
    fn a_github_remote_selects_gh() {
        assert_eq!(
            detect_forge(
                None,
                Some("git@github.com:tschallacka/ai-skills.git"),
                &NEITHER
            ),
            Ok(Forge::Gh)
        );
    }

    #[test]
    fn a_gitlab_remote_selects_glab() {
        assert_eq!(
            detect_forge(None, Some("https://gitlab.com/group/project.git"), &NEITHER),
            Ok(Forge::Glab)
        );
    }

    #[test]
    fn a_self_hosted_remote_falls_back_to_the_gh_probe_first() {
        let probe = Probe {
            gh: true,
            glab: true,
        };
        assert_eq!(
            detect_forge(None, Some("https://git.example.com/x/y"), &probe),
            Ok(Forge::Gh)
        );
    }

    #[test]
    fn a_self_hosted_remote_falls_back_to_glab_when_only_glab_is_present() {
        let probe = Probe {
            gh: false,
            glab: true,
        };
        assert_eq!(
            detect_forge(None, Some("https://git.example.com/x/y"), &probe),
            Ok(Forge::Glab)
        );
    }

    #[test]
    fn no_remote_and_no_cli_present_is_refused_by_name() {
        let error = detect_forge(None, None, &NEITHER).unwrap_err();
        assert!(error.message.contains("could not tell which forge"));
        assert!(error.message.contains("this remote"));
        assert_eq!(error.code, 66);
    }

    #[test]
    fn no_matching_remote_pattern_and_no_cli_names_the_remote() {
        let error = detect_forge(None, Some("https://git.example.com/x/y"), &NEITHER).unwrap_err();
        assert!(error.message.contains("git.example.com"));
    }
}
