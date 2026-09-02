// MODE: DEV
// PACKAGE: PROD
//! Rust orchestration for the planning validation passes.

use planning_validator_commands::CommandRegistry;
use planning_validator_common::Findings;
use planning_validator_comparisons::ComparisonRegistry;
use planning_validator_docs::{
    validate_existence, validate_obsolete, validate_plan_documents, validate_step_numbers,
};
use planning_validator_goals::{
    validate_goals, validate_step_naming, validate_steps, GoalTableRegistry,
};
use planning_validator_inventory::Inventory;
use planning_validator_placeholders::PlaceholderValidator;
use planning_validator_propagation::{
    validate_companions, validate_completion, validate_freshness, validate_leaves, validate_reach,
    validate_roster, validate_symbols,
};
use planning_validator_serve::{ServeRegistry, Unit as ServeUnit};
use planning_validator_stale::validate_stale;
use planning_validator_ui::validate as validate_ui;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

const USAGE: &str = "Usage: validate-plan.sh [--complete] [--propagation|--no-propagation] [--stale <file-of-phrases>|default] [--repo-root DIR] [--plan-dir] <plan-directory>";

#[derive(Debug, Default, Eq, PartialEq)]
struct Options {
    complete: bool,
    propagation: bool,
    stale: Option<String>,
    repo_root: Option<PathBuf>,
    plan: Option<PathBuf>,
}

fn main() {
    let options = match parse(env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(0) => std::process::exit(0),
        Err(code) => {
            eprintln!("{USAGE}");
            std::process::exit(code);
        }
    };
    let plan = options.plan.expect("parser requires a plan");
    let script_name = "validate-plan";
    if let Some(code) = validate_obsolete(&plan, script_name) {
        std::process::exit(code);
    }
    let mut findings = Findings::default();
    if validate_existence(&plan, &mut findings) != 0 {
        std::process::exit(1);
    }
    let skill = skill_root();
    let documents = validate_plan_documents(&plan, options.complete, &mut findings);
    validate_step_numbers(&plan, &mut findings);
    let placeholders = PlaceholderValidator::from_registry(&skill.join("placeholders.json"));
    let placeholder_result =
        placeholders.validate_plan(&plan, &documents.plan_docs, options.complete, &mut findings);
    validate_stale(
        &plan,
        &documents.plan_docs,
        options.stale.is_some(),
        options.stale.as_deref().map(Path::new),
        &mut findings,
    );
    let inventory = Inventory::parse(&plan.join("work-unit-inventory.md"), &mut findings);
    inventory.validate_dependency_graph(&mut findings);
    inventory.validate_target_paths(options.repo_root.as_deref(), &mut findings);
    let goals = GoalTableRegistry::from_file(&skill.join("goal-tables.json"), &mut findings);
    validate_goals(&plan, &inventory, &goals, options.complete, &mut findings);
    validate_steps(&plan, &inventory, &mut findings);
    validate_step_naming(&plan, &inventory, &mut findings);
    let unit_types = inventory
        .units
        .iter()
        .map(|unit| (unit.id.clone(), unit.kind.clone()))
        .collect::<HashMap<_, _>>();
    validate_ui(
        &plan,
        &documents.ui_affected,
        options.complete,
        &unit_types,
        &mut findings,
    );
    let serve_goals = inventory
        .goals
        .iter()
        .map(|(goal, ids)| {
            (
                goal.clone(),
                ids.iter()
                    .filter_map(|id| inventory.units.iter().find(|unit| &unit.id == id))
                    .map(|unit| ServeUnit {
                        step: unit.step.clone(),
                        kind: unit.kind.clone(),
                    })
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    ServeRegistry::from_file(&skill.join("state-change-registry.json")).validate_goals(
        &plan,
        &serve_goals,
        &mut findings,
    );
    CommandRegistry::from_files(
        &plan.join("commands.json"),
        &skill.join("never-executable-extensions.json"),
    )
    .validate_plan(&plan, options.complete, &mut findings);
    validate_completion(&plan, &inventory, options.complete, &mut findings);
    if options.propagation {
        validate_symbols(&plan, &inventory, &mut findings);
        validate_reach(&plan, &inventory, &mut findings);
        validate_companions(&plan, &inventory, &mut findings);
        validate_leaves(&inventory, &mut findings);
        validate_roster(&plan, &inventory, &mut findings);
        if let Some(repo_root) = options.repo_root.as_deref() {
            validate_freshness(repo_root, &plan, &inventory, &mut findings);
        }
    }
    ComparisonRegistry::from_file(&skill.join("artifact-comparisons.json"))
        .validate_companions(&plan, &mut findings);
    if findings.errors > 0 {
        eprintln!("Plan validation failed with {} error(s).", findings.errors);
        eprintln!(
            "Gates: structurally valid=no (errors above)  adversarially approved={}  implementation complete={}",
            review_gate(&plan, documents.review_approved),
            if options.complete {
                "yes"
            } else {
                "not checked (pass --complete)"
            }
        );
        std::process::exit(1);
    }
    let goals_count = inventory.goals.len();
    if placeholder_result.warnings > 0 {
        println!("Plan validation incomplete: {} work units across {} goals, {} placeholder(s) still to fill in {} document(s).", inventory.units.len(), goals_count, placeholder_result.warnings, placeholder_result.warning_docs.split_whitespace().count());
        println!("Structure is sound; fill the placeholders above before presenting the plan, or run with --complete to have them reported as errors.");
    } else {
        println!(
            "Plan validation passed: {} work units across {} goals.",
            inventory.units.len(),
            goals_count
        );
    }
    println!(
        "Gates: structurally valid={}  adversarially approved={}  implementation complete={}",
        if findings.errors == 0 { "yes" } else { "no" },
        review_gate(&plan, documents.review_approved),
        if options.complete {
            "yes"
        } else {
            "not checked (pass --complete)"
        }
    );
}

fn review_gate(plan: &Path, approved: bool) -> &'static str {
    if !plan.join("adversarial-review.md").is_file() {
        "no review file"
    } else if approved {
        "yes"
    } else {
        "no"
    }
}

fn parse(args: Vec<String>) -> Result<Options, i32> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        eprintln!("{USAGE}");
        return Err(0);
    }
    let mut options = Options {
        propagation: true,
        ..Options::default()
    };
    let mut positional = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--complete" => options.complete = true,
            "--propagation" => options.propagation = true,
            "--no-propagation" => options.propagation = false,
            "--stale" | "--repo-root" | "--plan-dir" => {
                index += 1;
                if index >= args.len() {
                    return Err(64);
                }
                if args[index - 1] == "--stale" {
                    options.stale = Some(args[index].clone());
                } else if args[index - 1] == "--repo-root" {
                    options.repo_root = Some(args[index].clone().into());
                } else {
                    positional.push(args[index].clone());
                }
            }
            value if value.starts_with("--stale=") => options.stale = Some(value[8..].to_owned()),
            value if value.starts_with("--repo-root=") => {
                options.repo_root = Some(value[12..].into())
            }
            value if value.starts_with("--plan-dir=") => positional.push(value[11..].to_owned()),
            "--" => positional.extend(args[index + 1..].iter().cloned()),
            value if value.starts_with('-') => return Err(64),
            value => positional.push(value.to_owned()),
        };
        index += 1;
    }
    if positional.len() != 1 {
        return Err(64);
    }
    options.plan = Some(positional.remove(0).into());
    Ok(options)
}

fn skill_root() -> PathBuf {
    env::var_os("PLANNING_SKILL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("planning"))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_plan_dir_and_propagation_switch() {
        let options = parse(vec![
            "--no-propagation".into(),
            "--plan-dir".into(),
            "/tmp/example".into(),
        ])
        .unwrap();
        assert!(!options.propagation);
        assert_eq!(options.plan.unwrap().to_str(), Some("/tmp/example"));
    }

    #[test]
    fn rejects_multiple_plan_directories() {
        assert_eq!(parse(vec!["one".into(), "two".into()]), Err(64));
    }
}
