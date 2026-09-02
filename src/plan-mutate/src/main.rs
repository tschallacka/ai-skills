// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const USAGE: &str = r#"Usage:
  ${0##*/} add-goal <plan> <goal-name> <title> <outcome>
  ${0##*/} add-work-unit <plan> --id <WNN> --type <type> --file <file|N/A> --scope <scope>
      --subscope <subscope|N/A> --change <change> --depends-on <depends-on|—>
      --goal <goal> --step <step>
  ${0##*/} add-testing <goal-directory> <step-name> <verification-instructions>
  ${0##*/} add-progress <goal-directory> <step-name> <description>
  ${0##*/} rebuild-progress <goal-directory>
  ${0##*/} add-coverage <plan> <outcome-or-proof> <WNN[,WNN...]> <notes>
  ${0##*/} add-finding <plan> <AR-NN> <finding> <resolution> [open|in-progress|resolved]
  ${0##*/} update-work-unit <plan> <WNN> [<new-primary-scope>] [<new-file>]
      [--scope <text>] [--file <path>] [--type <type>]
      [--depends-on <WNN[,WNN...]|—>] [--description <text>]
  ${0##*/} update-work-unit <plan> <WNN> --goal <goal> --step <step-name>
      (move between goals: edges and coverage links survive untouched)
  ${0##*/} set-unit-scope <plan> <WNN> <new-primary-scope>
      (alias of update-work-unit --scope; kept for existing callers)
  ${0##*/} remove-work-unit <plan> <WNN>
  ${0##*/} remove-plan <plan-directory>
  ${0##*/} cleanup-plans [--list] [<plan-name> ...] [--yes]
  ${0##*/} set-step <goal-directory> <step-name> <incomplete|in-progress|completed>
  ${0##*/} set-goal <plan> <goal-name> <incomplete|in-progress|completed>
  ${0##*/} set-review <plan> <pending|approved>
  ${0##*/} set-decomposition <plan> <incomplete|completed>
  ${0##*/} update-adversarial-review <plan> [--file CSV]
  ${0##*/} rebuild-plan-progress <plan>
  ${0##*/} content <update-plan-content.sh flags...>
      narrative and table edits: -dp/-ds/-gp/-gs/-sp/-ss/-rp/-rs paragraphs
      and sections, -ap append, -tp table, -ia/-ib insert, --delete-paragraph,
      -t title, -f field, -tr testing requirement
  ${0##*/} add-ui-story <plan> <story args...>
  ${0##*/} add-ui-story-links <plan> <link args...>
  ${0##*/} add-fix-claim <plan> <claim args...>
  ${0##*/} mint-fix-keys <plan> <session-id>
  ${0##*/} verify-fix-keys <plan> [--claimed-by <session>]
  ${0##*/} create-adversarial-review <plan>
  ${0##*/} register-command <plan> <key> <command> <when>
  ${0##*/} validate <plan>
All durable plan mutations must use this dispatcher or the named helper it
dispatches. Direct edits to .plans are prohibited by the planning protocol.
"#;

fn usage(code: i32) -> ! {
    print!("{USAGE}");
    process::exit(code)
}

fn skill_root() -> Option<PathBuf> {
    if let Some(root) = env::var_os("PLANNING_SKILL_ROOT") {
        let root = PathBuf::from(root);
        if root.join("planning/scripts").is_dir() {
            return Some(root);
        }
    }
    let mut places = Vec::new();
    if let Ok(exe) = env::current_exe() {
        places.extend(exe.ancestors().map(Path::to_path_buf));
    }
    if let Ok(cwd) = env::current_dir() {
        places.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    places
        .into_iter()
        .find(|root| root.join("planning/scripts").is_dir())
}

fn rust_verbs() -> &'static [(&'static str, &'static str)] {
    &[
        ("add-finding", "add-adversarial-finding"),
        ("add-coverage", "add-coverage"),
        ("add-fix-claim", "add-fix-claim"),
        ("add-goal", "add-goal"),
        ("add-ui-story", "add-ui-story"),
        ("add-ui-story-links", "add-ui-story-links"),
        ("add-work-unit", "add-work-unit"),
        ("add-testing", "create-step-testing"),
        ("create-adversarial-review", "create-adversarial-review"),
        ("mint-fix-keys", "mint-fix-keys"),
        ("rebuild-plan-progress", "rebuild-plan-progress"),
        ("register-command", "register-command"),
        ("remove-plan", "remove-plan"),
        ("remove-work-unit", "remove-work-unit"),
        ("set-unit-scope", "update-work-unit"),
        ("set-step", "update-step"),
        ("set-goal", "update-plan-progress"),
        ("set-review", "update-plan-content"),
        ("set-decomposition", "update-plan-content"),
        ("update-adversarial-review", "update-adversarial-review"),
        ("update-work-unit", "update-work-unit"),
        ("verify-fix-keys", "verify-fix-keys"),
    ]
}

fn shell_verbs() -> &'static [(&'static str, &'static str)] {
    &[
        ("add-progress", "plan-mutate.sh"),
        ("rebuild-progress", "plan-mutate.sh"),
        ("content", "update-plan-content.sh"),
        ("cleanup-plans", "cleanup-plans.sh"),
        ("validate", "validate-plan.sh"),
    ]
}

fn dispatch(root: &Path, command: &str, args: &[String]) -> ! {
    let rust_name = rust_verbs()
        .iter()
        .find(|(verb, _)| *verb == command)
        .map(|(_, name)| *name);
    let shell_name = shell_verbs()
        .iter()
        .find(|(verb, _)| *verb == command)
        .map(|(_, name)| *name);
    if rust_name.is_none() && shell_name.is_none() {
        usage(64)
    }
    let (name, path, target_args) = if let Some(name) = rust_name {
        let mut candidates = vec![root.join("bin").join(name)];
        if let Ok(entries) = std::fs::read_dir(root.join("bin")) {
            for entry in entries.flatten() {
                candidates.push(entry.path().join(name));
            }
        }
        candidates.push(
            root.join("src")
                .join(name)
                .join("target/release")
                .join(name),
        );
        candidates.push(root.join("src").join(name).join("target/debug").join(name));
        let path = candidates
            .into_iter()
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| root.join("planning/scripts").join(format!("{name}.sh")));
        (name, path, args.to_vec())
    } else {
        let name = shell_name.expect("shell verb checked above");
        let mut target_args = args.to_vec();
        if command == "add-progress" || command == "rebuild-progress" {
            target_args.insert(0, command.to_string());
        } else if command == "set-review" {
            target_args.insert(0, "--review-status".to_string());
        } else if command == "set-decomposition" {
            target_args.insert(0, "--decomposition-review".to_string());
        }
        (name, root.join("planning/scripts").join(name), target_args)
    };
    if !path.is_file() {
        eprintln!("plan-mutate.sh: command not found: {name}");
        process::exit(69)
    }
    let status = Command::new(&path)
        .args(target_args)
        .status()
        .unwrap_or_else(|error| {
            eprintln!("plan-mutate.sh: could not run {name}: {error}");
            process::exit(69)
        });
    process::exit(status.code().unwrap_or(1))
}

fn main() {
    let mut args: Vec<_> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0)
    }
    let command = args.first().cloned().unwrap_or_else(|| usage(64));
    args.remove(0);
    let root = skill_root().unwrap_or_else(|| {
        eprintln!("plan-mutate.sh: could not locate the planning skill root");
        process::exit(69)
    });
    dispatch(&root, &command, &args)
}

#[cfg(test)]
mod tests {
    use super::rust_verbs;

    #[test]
    fn every_rust_dispatch_verb_is_unique() {
        let mut names = rust_verbs()
            .iter()
            .map(|(verb, _)| *verb)
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), rust_verbs().len());
    }
}
