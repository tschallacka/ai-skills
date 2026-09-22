// MODE: DEV
// PACKAGE: PROD
//! Reads the workspace's own dependency graph from `cargo metadata`.
//!
//! Deliberate simplification: this parses the metadata JSON natively with
//! `serde_json` rather than shelling out to a query tool. `git` and
//! `cargo metadata` themselves are still real subprocesses: their own
//! resolution behavior must be preserved exactly.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
}

#[derive(Deserialize)]
pub struct Package {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
pub struct Dependency {
    pub name: String,
}

pub enum MetadataError {
    /// `command -v cargo` fails: a distinct reason for a missing tool, not
    /// a failing one.
    CargoNotOnPath,
    /// `cargo metadata` ran but exited non-zero.
    CommandFailed,
    /// `cargo metadata` exited 0 but produced no stdout.
    Empty,
    /// The JSON did not parse.
    Unparseable,
}

pub fn read(repo_root: &Path) -> Result<Metadata, MetadataError> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .current_dir(repo_root)
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(MetadataError::CargoNotOnPath)
        }
        Err(_) => return Err(MetadataError::CommandFailed),
    };
    if !output.status.success() {
        return Err(MetadataError::CommandFailed);
    }
    if output.stdout.is_empty() {
        return Err(MetadataError::Empty);
    }
    serde_json::from_slice(&output.stdout).map_err(|_| MetadataError::Unparseable)
}

/// `(dependent, dependency)` pairs, one per workspace-internal dependency
/// edge: for every package, for every one of its dependencies whose name is
/// itself a workspace member, emit that pair.
pub fn build_edges(metadata: &Metadata) -> Vec<(String, String)> {
    let members: std::collections::HashSet<&str> =
        metadata.packages.iter().map(|p| p.name.as_str()).collect();
    let mut edges = Vec::new();
    for package in &metadata.packages {
        for dependency in &package.dependencies {
            if members.contains(dependency.name.as_str()) {
                edges.push((package.name.clone(), dependency.name.clone()));
            }
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Metadata {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn edges_only_include_workspace_internal_dependencies() {
        let metadata = parse(
            r#"{"packages": [
                {"name": "app", "dependencies": [{"name": "lib"}, {"name": "serde"}]},
                {"name": "lib", "dependencies": []}
            ]}"#,
        );
        let edges = build_edges(&metadata);
        assert_eq!(edges, vec![("app".to_string(), "lib".to_string())]);
    }

    #[test]
    fn a_package_with_no_dependencies_field_defaults_to_empty() {
        let metadata = parse(r#"{"packages": [{"name": "solo"}]}"#);
        assert!(build_edges(&metadata).is_empty());
        assert_eq!(metadata.packages.len(), 1);
    }

    #[test]
    fn members_total_is_the_package_count() {
        let metadata = parse(r#"{"packages": [{"name": "a"}, {"name": "b"}, {"name": "c"}]}"#);
        assert_eq!(metadata.packages.len(), 3);
    }
}
