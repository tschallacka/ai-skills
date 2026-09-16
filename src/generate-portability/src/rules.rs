// MODE: DEV
// PACKAGE: PROD

//! portability-rules.json's data model, parsed in-process via serde_json --
//! the compiled binary has no rjq dependency at all, unlike the bash
//! original. Replaces bash's own "rjq is required" fatal check (exit 69,
//! meaningless once JSON parsing happens in-process) with a new, analogous
//! failure mode: a missing or unparseable rules file exits 66 (EX_NOINPUT,
//! matching the exit code bash's own `[ -f "$rules" ]` check already uses).

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Rules {
    pub target: Target,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub shell: String,
    pub userland: String,
    pub locale: String,
    pub verified: Verified,
}

#[derive(Debug, Deserialize)]
pub struct Verified {
    #[serde(rename = "bash-3.2")]
    pub bash_3_2: String,
    #[serde(rename = "bsd-userland")]
    pub bsd_userland: String,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub id: String,
    pub construct: String,
    pub detect: Option<String>,
    pub breaks: String,
    pub symptom: String,
    pub replacement: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Exit-66-shaped: a missing file or invalid JSON is a clear `Err`, never a
/// panic.
pub fn load(rules_path: &Path) -> Result<Rules, String> {
    let contents = std::fs::read_to_string(rules_path)
        .map_err(|error| format!("missing {}: {error}", rules_path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("{} is not valid JSON: {error}", rules_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "generate-portability-rules-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_is_a_clear_err() {
        let dir = scratch("missing");
        let err = load(&dir.join("nope.json")).unwrap_err();
        assert!(err.contains("missing"));
    }

    #[test]
    fn invalid_json_is_a_clear_err() {
        let dir = scratch("invalid");
        let path = dir.join("rules.json");
        fs::write(&path, "{ not json").unwrap();
        let err = load(&path).unwrap_err();
        assert!(err.contains("not valid JSON"));
    }

    #[test]
    fn a_well_formed_file_loads_with_a_null_detect() {
        let dir = scratch("valid");
        let path = dir.join("rules.json");
        fs::write(
            &path,
            r#"{
                "target": {
                    "shell": "bash",
                    "userland": "GNU or BSD",
                    "locale": "C",
                    "verified": {"bash-3.2": "yes", "bsd-userland": "yes"}
                },
                "rules": [
                    {"id": "r1", "construct": "c", "detect": null, "breaks": "b", "symptom": "s", "replacement": "r"}
                ]
            }"#,
        )
        .unwrap();
        let rules = load(&path).unwrap();
        assert_eq!(rules.rules.len(), 1);
        assert!(rules.rules[0].detect.is_none());
        assert!(rules.rules[0].note.is_none());
    }
}
