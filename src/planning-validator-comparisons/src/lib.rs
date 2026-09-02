// MODE: DEV
// PACKAGE: PROD
//! Artifact-comparison validation formerly provided by
//! `validate-plan-comparisons-lib.sh`.

use planning_validator_common::{trim, Findings};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct ComparisonRegistry {
    registry_path: Option<PathBuf>,
    comparisons: Vec<String>,
    nondeterministic_extensions: std::collections::BTreeMap<String, String>,
}

impl ComparisonRegistry {
    pub fn from_file(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self {
                registry_path: Some(path.to_path_buf()),
                ..Self::default()
            };
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return Self {
                registry_path: Some(path.to_path_buf()),
                ..Self::default()
            };
        };
        let comparisons = value
            .get("comparisons")
            .and_then(Value::as_object)
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();
        let nondeterministic_extensions = value
            .get("nondeterministic_extensions")
            .and_then(Value::as_object)
            .map(|object| {
                object
                    .iter()
                    .filter_map(|(extension, reason)| {
                        reason
                            .as_str()
                            .map(|reason| (extension.clone(), reason.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            registry_path: Some(path.to_path_buf()),
            comparisons,
            nondeterministic_extensions,
        }
    }

    pub fn validate_companions(&self, plan: &Path, findings: &mut Findings) {
        if self.comparisons.is_empty() {
            let registry = self
                .registry_path
                .as_deref()
                .unwrap_or_else(|| Path::new("artifact-comparisons.json"));
            if !registry.is_file() {
                findings.fail(format!(
                    "Artifact comparison registry not found: {}",
                    registry.display()
                ));
            } else {
                findings.fail("Artifact comparison registry lists no comparisons");
            }
            return;
        }
        for companion in testing_companions(plan) {
            let Some(section) = comparison_rows(&companion) else {
                continue;
            };
            for (artifact, comparison) in section {
                if artifact == "Artifact" || artifact.is_empty() || comparison.is_empty() {
                    continue;
                }
                if !self.comparisons.iter().any(|legal| legal == &comparison) {
                    findings.fail(format!(
                        "{}: comparison '{}' for {} is not in artifact-comparisons.json (legal:{})",
                        companion.display(),
                        comparison,
                        artifact,
                        self.comparisons.join(" ")
                    ));
                    continue;
                }
                if comparison != "exact" {
                    continue;
                }
                let Some(extension) = artifact.rsplit_once('.').map(|(_, ext)| ext) else {
                    continue;
                };
                let extension = extension.to_ascii_lowercase();
                let Some(reason) = self.nondeterministic_extensions.get(&extension) else {
                    continue;
                };
                findings.fail(format!(
                    "{}: {} cannot be compared 'exact' -- {}. Use one of the non-exact comparisons in artifact-comparisons.json and say what tolerance the proof allows.",
                    companion.display(), artifact, reason
                ));
            }
        }
    }
}

fn testing_companions(plan: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_files(plan, &mut paths);
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("-testing.md"))
            && !path
                .components()
                .any(|component| component.as_os_str() == "context")
    });
    paths.sort();
    paths
}

fn collect_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, paths);
        } else if path.is_file() {
            paths.push(path);
        }
    }
}

fn comparison_rows(path: &Path) -> Option<Vec<(String, String)>> {
    let text = fs::read_to_string(path).ok()?;
    let mut inside = false;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line == "## Artifact comparisons" {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if !inside || !line.starts_with('|') || line.contains("---") {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        if fields.len() < 3 {
            continue;
        }
        rows.push((trim(fields[1]), trim(fields[2])));
    }
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::{comparison_rows, ComparisonRegistry};
    use planning_validator_common::Findings;
    use std::fs;

    #[test]
    fn reads_only_rows_under_the_comparison_heading() {
        let root =
            std::env::temp_dir().join(format!("validator-comparisons-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let file = root.join("step-testing.md");
        fs::write(
            &file,
            "| Artifact | Comparison |\n|---|---|\n| outside | exact |\n\n## Artifact comparisons\n\n| `report.pdf` | text-layer |\n|---|---|\n\n## Next\n| ignored | exact |\n",
        )
        .unwrap();
        assert_eq!(
            comparison_rows(&file).unwrap(),
            vec![("report.pdf".into(), "text-layer".into())]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn exact_nondeterministic_extension_is_refused() {
        let root =
            std::env::temp_dir().join(format!("validator-comparisons-gate-{}", std::process::id()));
        let plan = root.join("plan");
        let skill = root.join("skill");
        let _ = fs::create_dir_all(plan.join("goal/steps"));
        let _ = fs::create_dir_all(&skill);
        fs::write(
            skill.join("artifact-comparisons.json"),
            r#"{"comparisons":{"exact":""},"nondeterministic_extensions":{"pdf":"creation timestamp"}}"#,
        )
        .unwrap();
        let companion = plan.join("goal/steps/01-testing.md");
        fs::write(
            &companion,
            "## Artifact comparisons\n\n| Artifact | Comparison |\n|---|---|\n| `invoice.PDF` | exact |\n",
        )
        .unwrap();
        let registry = ComparisonRegistry::from_file(&skill.join("artifact-comparisons.json"));
        let mut findings = Findings::default();
        registry.validate_companions(&plan, &mut findings);
        assert_eq!(findings.errors, 1);
        assert!(findings.messages[0].text.contains("creation timestamp"));
        let _ = fs::remove_dir_all(root);
    }
}
