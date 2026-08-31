// MODE: DEV
// PACKAGE: PROD
mod pages;
mod plan;
mod render;
mod serve;
mod watch;

use plan::extract::extract_state;
use plan::state::parse_state;
use plan::tree::read_plan_tree;
use render::router::{route, Route};
use render::shell::render_shell;
use std::path::PathBuf;

const USAGE: &str =
    "usage: plan-overview --plan-dir DIR [--out FILE] [--refresh N] [--watch] [--serve] [--port N]";

#[derive(Debug, Default)]
struct Args {
    help: bool,
    plan_dir: PathBuf,
    out: Option<PathBuf>,
    refresh: Option<u32>,
    watch: bool,
    serve: bool,
    port: Option<u16>,
}

fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args::default();
    let mut positional = None;
    let mut it = argv.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--plan-dir" => args.plan_dir = PathBuf::from(it.next().ok_or("--plan-dir needs a value")?),
            "--out" => args.out = Some(PathBuf::from(it.next().ok_or("--out needs a value")?)),
            "--refresh" => args.refresh = Some(it.next().ok_or("--refresh needs a value")?.parse().map_err(|_| "--refresh is not a number".to_string())?),
            "--watch" => args.watch = true,
            "--serve" => args.serve = true,
            "--port" => args.port = Some(it.next().ok_or("--port needs a value")?.parse().map_err(|_| "--port is not a port".to_string())?),
            // A help request is not a usage error: it prints on stdout and
            // exits 0, the way every other entry point in this repository does.
            "--help" | "-h" => {
                args.help = true;
                return Ok(args);
            }
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value if positional.is_none() => positional = Some(value.to_string()),
            _ => return Err("only one plan directory may be supplied".into()),
        }
    }
    if args.plan_dir.as_os_str().is_empty() {
        args.plan_dir = positional
            .map(PathBuf::from)
            .ok_or_else(|| "--plan-dir needs a value".to_string())?;
    }
    if args.serve && args.out.is_some() {
        return Err("--serve cannot be combined with --out".into());
    }
    Ok(args)
}

fn render_page(state: &plan::state::State, hash: &str) -> String {
    let page = match route(hash, state) {
        Route::Overview { .. } => pages::overview::render_overview(state),
        Route::Goal { id } => pages::goal::render_goal(state, &id),
        Route::Unit { id, .. } => pages::unit::render_unit(state, &id),
        Route::Finding { id } => pages::findings::render_finding(state, &id),
        Route::Test { id } => pages::tests::render_test(state, &id),
        Route::Coverage => pages::coverage::render_coverage(state),
        Route::History => pages::history::render_history(state),
        Route::Graph => pages::graph::render_graph(state),
    };
    render_shell(state, &page)
}

fn run(args: Args) -> Result<(), String> {
    let tree = read_plan_tree(&args.plan_dir).map_err(|error| error.to_string())?;
    let state_json = extract_state(&tree)?;
    let state = parse_state(&state_json).map_err(|error| error.to_string())?;
    let artifact = render_page(&state, "#overview");
    if args.serve {
        let _server = serve::serve_on_port(artifact, state_json, args.port.unwrap_or(0))
            .map_err(|error| error.to_string())?;
        loop {
            std::thread::park();
        }
    }
    let output = args
        .out
        .unwrap_or_else(|| args.plan_dir.join("overview.html"));
    std::fs::write(output, artifact).map_err(|error| error.to_string())?;
    Ok(())
}

fn main() {
    match parse_args(std::env::args().skip(1)) {
        Ok(args) => {
            if args.help {
                println!("{USAGE}");
                return;
            }
            if let Err(error) = run(args) {
                eprintln!("plan-overview: {error}");
                std::process::exit(66);
            }
        }
        Err(error) => {
            eprintln!("plan-overview: {error}");
            std::process::exit(64);
        }
    }
}
