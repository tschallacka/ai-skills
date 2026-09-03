// MODE: DEV
// PACKAGE: PROD
use plan_overview::plan::extract::extract_state_as;
use plan_overview::plan::tree::read_plan_tree;
use std::env;
use std::path::PathBuf;

const COMMAND: &str = "overview-state.sh";

fn usage(code: i32) -> ! {
    print!(
        "Usage: {COMMAND} [--plan-dir] <plan-directory>\n       {COMMAND} --help\n\nEmits the reviewing state of one plan as a single JSON document on stdout.\n"
    );
    std::process::exit(code);
}

fn main() {
    let mut plan_dir = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => usage(0),
            "--plan-dir" => plan_dir = args.next().map(PathBuf::from),
            "--" => break,
            value if value.starts_with('-') => {
                eprintln!("{COMMAND}: unknown option: {value}");
                usage(64);
            }
            value => {
                if plan_dir.is_some() {
                    usage(64);
                }
                plan_dir = Some(PathBuf::from(value));
            }
        }
    }
    let plan_dir = plan_dir.unwrap_or_else(|| usage(64));
    if !plan_dir.is_dir() {
        eprintln!(
            "{COMMAND}: plan directory not found: {}",
            plan_dir.display()
        );
        std::process::exit(66);
    }
    if !plan_dir.join("plan-description.md").is_file() {
        eprintln!(
            "{COMMAND}: plan-description.md not found: {}",
            plan_dir.join("plan-description.md").display()
        );
        std::process::exit(66);
    }
    let tree = read_plan_tree(&plan_dir).unwrap_or_else(|error| {
        eprintln!("{COMMAND}: {error}");
        std::process::exit(66);
    });
    let state = extract_state_as(&tree, COMMAND).unwrap_or_else(|error| {
        eprintln!("{COMMAND}: {error}");
        std::process::exit(66);
    });
    print!("{state}");
}
