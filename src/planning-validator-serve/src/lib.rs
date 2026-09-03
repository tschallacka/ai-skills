// MODE: DEV
// PACKAGE: PROD
//! The "still serves" validation pass formerly provided by
//! `validate-plan-serve-lib.sh`.

use planning_validator_common::Findings;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Unit {
    pub step: String,
    pub kind: String,
}

#[derive(Debug, Default)]
pub struct ServeRegistry {
    indicators: Vec<String>,
    serve_phrases: Vec<String>,
}

impl ServeRegistry {
    pub fn from_file(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return Self::default();
        };
        Self {
            indicators: strings(&value, "indicators"),
            serve_phrases: strings(&value, "serve_phrases"),
        }
    }

    pub fn validate_goals(
        &self,
        plan: &Path,
        goals: &[(String, Vec<Unit>)],
        findings: &mut Findings,
    ) {
        for (goal_name, units) in goals {
            let touched = units.iter().any(|unit| {
                let step = plan.join(goal_name).join("steps").join(&unit.step);
                contains_indicator(&step, &self.indicators)
                    || contains_indicator(&companion(&step), &self.indicators)
            });
            if !touched {
                continue;
            }
            let served = units.iter().any(|unit| {
                if unit.kind != "test" && unit.kind != "verification" {
                    return false;
                }
                let step = plan.join(goal_name).join("steps").join(&unit.step);
                let acceptance = section(&step, "## Acceptance criteria");
                let companion_acceptance = section(&companion(&step), "## Automated tests");
                let combined = format!("{acceptance}\n{companion_acceptance}").to_ascii_lowercase();
                self.serve_phrases
                    .iter()
                    .any(|phrase| combined.contains(&phrase.to_ascii_lowercase()))
            });
            if !served {
                findings.warn(format!(
                    "{goal_name} changes module state, schema, or configuration, but no verification acceptance condition checks that the application still serves; add one (e.g. a plain request returns HTTP 200)"
                ));
            }
        }
    }
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn companion(step: &Path) -> PathBuf {
    let mut companion = step.to_path_buf();
    if companion
        .extension()
        .is_some_and(|extension| extension == "md")
    {
        companion.set_extension("");
        let name = companion
            .file_name()
            .map(|name| format!("{}-testing.md", name.to_string_lossy()))
            .unwrap_or_default();
        companion.set_file_name(name);
    }
    companion
}

fn contains_indicator(path: &Path, indicators: &[String]) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    indicators
        .iter()
        .filter(|indicator| !indicator.is_empty())
        .any(|indicator| lower.contains(&indicator.to_ascii_lowercase()))
}

fn section(path: &Path, heading: &str) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return String::new();
    };
    let mut active = false;
    let mut output = String::new();
    for line in text.lines() {
        if line == heading {
            active = true;
            continue;
        }
        if active && line.starts_with("## ") {
            break;
        }
        if active {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{ServeRegistry, Unit};
    use planning_validator_common::Findings;
    use std::fs;

    #[test]
    fn state_change_without_live_acceptance_is_advisory() {
        let root = std::env::temp_dir().join(format!("validator-serve-{}", std::process::id()));
        let plan = root.join("plan");
        let _ = fs::create_dir_all(plan.join("01-goal/steps"));
        fs::write(
            root.join("state-change-registry.json"),
            r#"{"indicators":["schema"],"serve_phrases":["HTTP 200"]}"#,
        )
        .unwrap();
        let step = plan.join("01-goal/steps/01-step.md");
        fs::write(
            &step,
            "schema migration\n## Acceptance criteria\n\nfile exists\n",
        )
        .unwrap();
        let registry = ServeRegistry::from_file(&root.join("state-change-registry.json"));
        let mut findings = Findings::default();
        registry.validate_goals(
            &plan,
            &[(
                "01-goal".into(),
                vec![Unit {
                    step: "01-step.md".into(),
                    kind: "source".into(),
                }],
            )],
            &mut findings,
        );
        assert_eq!(findings.warnings, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn verification_acceptance_phrase_satisfies_state_change() {
        let root = std::env::temp_dir().join(format!("validator-serve-ok-{}", std::process::id()));
        let plan = root.join("plan");
        let _ = fs::create_dir_all(plan.join("01-goal/steps"));
        fs::write(
            root.join("state-change-registry.json"),
            r#"{"indicators":["schema"],"serve_phrases":["HTTP 200"]}"#,
        )
        .unwrap();
        fs::write(
            plan.join("01-goal/steps/01-test.md"),
            "## Acceptance criteria\n\nA request returns HTTP 200.\n",
        )
        .unwrap();
        fs::write(
            plan.join("01-goal/steps/01-test-testing.md"),
            "schema migration\n## Automated tests\n\ncheck\n",
        )
        .unwrap();
        let registry = ServeRegistry::from_file(&root.join("state-change-registry.json"));
        let mut findings = Findings::default();
        registry.validate_goals(
            &plan,
            &[(
                "01-goal".into(),
                vec![Unit {
                    step: "01-test.md".into(),
                    kind: "verification".into(),
                }],
            )],
            &mut findings,
        );
        assert_eq!(findings.warnings, 0);
        let _ = fs::remove_dir_all(root);
    }
}
