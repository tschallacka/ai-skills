// MODE: DEV
// PACKAGE: PROD
//! Placeholder validation formerly provided by `validate-plan-placeholders-lib.sh`.

use planning_validator_common::Findings;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct PlaceholderResult {
    pub warnings: usize,
    pub warning_docs: String,
}

#[derive(Debug, Default)]
pub struct PlaceholderValidator {
    surfaces: std::collections::BTreeMap<String, String>,
}

impl PlaceholderValidator {
    pub fn from_registry(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return Self::default();
        };
        let mut surfaces = std::collections::BTreeMap::new();
        if let Some(entries) = value.get("placeholders").and_then(Value::as_array) {
            for entry in entries {
                let (Some(token), Some(surface)) = (
                    entry.get("token").and_then(Value::as_str),
                    entry.get("surface").and_then(Value::as_str),
                ) else {
                    continue;
                };
                surfaces.insert(token.to_owned(), surface.to_owned());
            }
        }
        Self { surfaces }
    }

    pub fn token_surface(&self, token: &str) -> Option<&str> {
        self.surfaces.get(token).map(String::as_str)
    }

    pub fn check_file(
        &self,
        surface: Surface,
        label: &str,
        file: &Path,
        complete_mode: bool,
        findings: &mut Findings,
        result: &mut PlaceholderResult,
    ) {
        let Ok(text) = fs::read_to_string(file) else {
            return;
        };
        for token in registered_tokens(&text) {
            if self.token_surface(&token).is_none() {
                continue;
            }
            match surface {
                Surface::Generated => findings.fail(format!(
                    "{label} contains a registered placeholder that no author will fill: {token}"
                )),
                Surface::Authored if complete_mode => findings.fail(format!(
                    "{label} still contains a registered placeholder: {token}"
                )),
                Surface::Authored => {
                    findings.warn(format!(
                        "{label} contains a registered placeholder (fill before completion): {token}"
                    ));
                    result.warnings += 1;
                    if !result
                        .warning_docs
                        .split_whitespace()
                        .any(|existing| existing == label)
                    {
                        result.warning_docs.push(' ');
                        result.warning_docs.push_str(label);
                    }
                }
            }
        }
    }

    pub fn validate_plan(
        &self,
        plan: &Path,
        plan_docs: &[PathBuf],
        complete_mode: bool,
        findings: &mut Findings,
    ) -> PlaceholderResult {
        let mut result = PlaceholderResult::default();
        for document in plan_docs {
            if document.is_file() {
                if let Some(label) = document.file_name().and_then(|name| name.to_str()) {
                    self.check_file(
                        Surface::Authored,
                        label,
                        document,
                        complete_mode,
                        findings,
                        &mut result,
                    );
                }
            }
        }
        let progress = plan.join("progress.md");
        if progress.is_file() {
            self.check_file(
                Surface::Generated,
                "plan progress.md",
                &progress,
                complete_mode,
                findings,
                &mut result,
            );
        }
        let mut goals = sorted_goal_dirs(plan);
        for goal in goals.drain(..) {
            let goal_progress = goal.join("progress.md");
            if goal_progress.is_file() {
                let label = format_file_label(&goal, "progress.md");
                self.check_file(
                    Surface::Generated,
                    &label,
                    &goal_progress,
                    complete_mode,
                    findings,
                    &mut result,
                );
            }
            let goal_file = goal.join("goal.md");
            if goal_file.is_file() {
                for token in registered_tokens_in_section(&goal_file, "## Goal-size exception") {
                    if self.token_surface(&token).is_some() {
                        findings.fail(
                            format_file_label(&goal, "goal-size exception")
                                + &format!(
                            " contains a registered placeholder that no author will fill: {token}"
                        ),
                        );
                    }
                }
            }
        }
        let cache_dir = plan.join("ui-story-runs");
        if cache_dir.is_dir() {
            let mut caches = fs::read_dir(cache_dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "md"))
                .collect::<Vec<_>>();
            caches.sort();
            for cache in caches {
                if let Some(name) = cache.file_name().and_then(|name| name.to_str()) {
                    self.check_file(
                        Surface::Generated,
                        &format!("{name} run cache"),
                        &cache,
                        complete_mode,
                        findings,
                        &mut result,
                    );
                }
            }
        }
        result
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Surface {
    Authored,
    Generated,
}

fn registered_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let bytes = line.as_bytes();
        let mut start = 0;
        while let Some(relative) = line[start..].find('<') {
            let open = start + relative;
            let Some(end_relative) = line[open + 1..].find('>') else {
                break;
            };
            let end = open + 1 + end_relative;
            let candidate = &line[open..=end];
            if !candidate[1..candidate.len() - 1].contains('<')
                && candidate[1..candidate.len() - 1]
                    .bytes()
                    .any(|byte| byte.is_ascii_alphabetic())
            {
                tokens.insert(candidate.to_owned());
            }
            start = open + 1;
            if start >= bytes.len() {
                break;
            }
        }
    }
    tokens
}

fn registered_tokens_in_section(path: &Path, heading: &str) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let mut section = String::new();
    let mut active = false;
    for line in text.lines() {
        if line == heading {
            active = true;
            continue;
        }
        if active && line.starts_with("## ") {
            break;
        }
        if active {
            section.push_str(line);
            section.push('\n');
        }
    }
    registered_tokens(&section)
}

fn sorted_goal_dirs(plan: &Path) -> Vec<PathBuf> {
    let mut goals = fs::read_dir(plan)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    goals.sort();
    goals
}

fn format_file_label(goal: &Path, suffix: &str) -> String {
    format!(
        "{} {suffix}",
        goal.file_name().unwrap_or_default().to_string_lossy()
    )
}

#[cfg(test)]
mod tests {
    use super::{registered_tokens, PlaceholderResult, PlaceholderValidator, Surface};
    use planning_validator_common::Findings;
    use std::fs;

    #[test]
    fn scans_registered_shape_outside_fences_and_deduplicates() {
        assert_eq!(
            registered_tokens("<one> <one> <two words>\n```\n<three>\n```\n"),
            ["<one>", "<two words>"]
                .into_iter()
                .map(String::from)
                .collect()
        );
    }

    #[test]
    fn authored_warning_promotes_only_in_complete_mode() {
        let root =
            std::env::temp_dir().join(format!("validator-placeholders-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let registry = root.join("placeholders.json");
        let file = root.join("goal.md");
        fs::write(
            &registry,
            r#"{"placeholders":[{"token":"<todo>","surface":"authored"}]}"#,
        )
        .unwrap();
        fs::write(&file, "<todo>\n").unwrap();
        let validator = PlaceholderValidator::from_registry(&registry);
        let mut findings = Findings::default();
        let mut result = PlaceholderResult::default();
        validator.check_file(
            Surface::Authored,
            "goal.md",
            &file,
            false,
            &mut findings,
            &mut result,
        );
        assert_eq!(result.warnings, 1);
        assert_eq!(findings.errors, 0);
        let mut strict = Findings::default();
        validator.check_file(
            Surface::Authored,
            "goal.md",
            &file,
            true,
            &mut strict,
            &mut result,
        );
        assert_eq!(strict.errors, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn generated_placeholder_fails_regardless_of_surface() {
        let root = std::env::temp_dir().join(format!(
            "validator-placeholders-generated-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&root);
        let registry = root.join("placeholders.json");
        let file = root.join("progress.md");
        fs::write(
            &registry,
            r#"{"placeholders":[{"token":"<todo>","surface":"authored"}]}"#,
        )
        .unwrap();
        fs::write(&file, "<todo>\n").unwrap();
        let validator = PlaceholderValidator::from_registry(&registry);
        let mut findings = Findings::default();
        validator.check_file(
            Surface::Generated,
            "plan progress.md",
            &file,
            false,
            &mut findings,
            &mut PlaceholderResult::default(),
        );
        assert_eq!(findings.errors, 1);
        assert!(findings.messages[0].text.contains("no author will fill"));
        let _ = fs::remove_dir_all(root);
    }
}
