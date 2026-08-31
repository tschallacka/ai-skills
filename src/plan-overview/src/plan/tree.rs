// MODE: DEV
// PACKAGE: PROD
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanDocument {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTree {
    pub root: PathBuf,
    pub documents: Vec<PlanDocument>,
    pub missing_optional: Vec<PathBuf>,
}

impl PlanTree {
    pub fn document(&self, relative_path: impl AsRef<Path>) -> Option<&PlanDocument> {
        let path = self.root.join(relative_path);
        self.documents.iter().find(|document| document.path == path)
    }
}

pub fn read_plan_tree(root: impl AsRef<Path>) -> io::Result<PlanTree> {
    let root = root.as_ref().to_path_buf();
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("plan directory not found: {}", root.display()),
        ));
    }

    let mut paths = Vec::new();
    collect_markdown(&root, &mut paths)?;
    paths.sort();
    let mut documents = Vec::with_capacity(paths.len());
    for path in paths {
        documents.push(PlanDocument {
            contents: fs::read_to_string(&path)?,
            path,
        });
    }

    let optional = [
        "work-unit-inventory.md",
        "adversarial-review.md",
        "adversarial-review-history.md",
    ];
    let missing_optional = optional
        .iter()
        .map(|name| root.join(name))
        .filter(|path| !documents.iter().any(|document| document.path == *path))
        .collect();

    Ok(PlanTree {
        root,
        documents,
        missing_optional,
    })
}

fn collect_markdown(directory: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_markdown(&path, paths)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            paths.push(path);
        }
    }
    Ok(())
}
