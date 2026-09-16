// MODE: DEV
// PACKAGE: PROD
use plan_overview::plan::extract::extract_state;
use plan_overview::plan::state::parse_state;
use plan_overview::plan::tree::read_plan_tree;
use plan_overview::render::router::render_page;
use plan_overview::watch::{coalesce_events, watch_plan_dir, ChangeEvent, DEBOUNCE_WINDOW};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

const USAGE: &str = "usage: plan-overview --plan-dir DIR [--out FILE] [--serve] [--host HOST] [--port N] [--watch] [--refresh MS]";

#[derive(Debug, Default)]
struct Args {
    help: bool,
    plan_dir: PathBuf,
    out: Option<PathBuf>,
    serve: bool,
    host: Option<String>,
    port: Option<u16>,
    watch: bool,
    refresh: Option<Duration>,
}

fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args::default();
    let mut positional = None;
    let mut it = argv.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--plan-dir" => {
                args.plan_dir = PathBuf::from(it.next().ok_or("--plan-dir needs a value")?)
            }
            "--out" => args.out = Some(PathBuf::from(it.next().ok_or("--out needs a value")?)),
            // T130: --refresh overrides watch.rs's own DEBOUNCE_WINDOW -- the
            // window run()'s watch loop uses to coalesce a burst of file
            // changes (via coalesce_events) into one re-render rather than
            // one per change. It is meaningless without --watch, so it is
            // validated against that once parsing finishes (below), the same
            // "refuse by name" stance B99 took rather than silently ignoring
            // a value with nothing to apply it to.
            "--refresh" => {
                let value = it.next().ok_or("--refresh needs a value")?;
                let millis: u64 = value.parse().map_err(|_| {
                    format!("--refresh value must be a whole number of milliseconds, got {value:?}")
                })?;
                args.refresh = Some(Duration::from_millis(millis));
            }
            "--watch" => args.watch = true,
            "--serve" => args.serve = true,
            "--host" => args.host = Some(it.next().ok_or("--host needs a value")?),
            "--port" => {
                args.port = Some(
                    it.next()
                        .ok_or("--port needs a value")?
                        .parse()
                        .map_err(|_| "--port is not a port".to_string())?,
                )
            }
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
    if args.refresh.is_some() && !args.watch {
        return Err("--refresh only applies together with --watch".into());
    }
    if args.host.is_some() && !args.serve {
        return Err("--host only applies together with --serve".into());
    }
    Ok(args)
}

// Re-extracts and re-serializes plan state from disk, the same first two
// steps run() itself takes on startup (read_plan_tree -> extract_state). Used
// after a watch.rs ChangeEvent fires, to publish a fresh state_json without
// re-rendering the (unchanged, per T130's own scope) served HTML shell.
fn rerender_state_json(plan_dir: &Path) -> Result<String, String> {
    let tree = read_plan_tree(plan_dir).map_err(|error| error.to_string())?;
    extract_state(&tree)
}

// The --out (non-serve) equivalent: a watch cycle here has no StateStream
// subscriber to publish to, so it re-renders the full HTML artifact and
// rewrites it in place instead.
fn rerender_artifact(plan_dir: &Path) -> Result<String, String> {
    let state_json = rerender_state_json(plan_dir)?;
    let state = parse_state(&state_json).map_err(|error| error.to_string())?;
    Ok(render_page(&state, "/overview"))
}

// Blocks for the next change, then drains anything else that arrives within
// `window` so a burst of saves becomes one re-render rather than one per
// file -- the debounce behaviour coalesce_events (watch.rs, already unit
// tested) is built to express. Returns false once the watcher's sender is
// dropped (the scan thread exited), which is this loop's only exit path.
fn wait_for_batch(events: &Receiver<ChangeEvent>, window: Duration) -> bool {
    let first = match events.recv() {
        Ok(event) => event,
        Err(_) => return false,
    };
    let mut batch = vec![first];
    let deadline = Instant::now() + window;
    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match events.recv_timeout(deadline - now) {
            Ok(event) => batch.push(event),
            Err(_) => break,
        }
    }
    if let Some(merged) = coalesce_events(&batch, window).into_iter().next_back() {
        eprintln!(
            "plan-overview: {} file(s) changed, refreshing",
            merged.changed.len()
        );
    }
    true
}

fn run(args: Args) -> Result<(), String> {
    let tree = read_plan_tree(&args.plan_dir).map_err(|error| error.to_string())?;
    let state_json = extract_state(&tree)?;
    let window = args.refresh.unwrap_or(DEBOUNCE_WINDOW);

    if args.serve {
        let stream = plan_overview::serve::state_stream(state_json);
        if args.watch {
            let root = args.plan_dir.clone();
            let publish_stream = stream.clone();
            let (events, _watch_stop) =
                watch_plan_dir(root.clone()).map_err(|error| error.to_string())?;
            std::thread::spawn(move || {
                while wait_for_batch(&events, window) {
                    match rerender_state_json(&root) {
                        Ok(fresh) => publish_stream.publish(fresh),
                        Err(error) => {
                            eprintln!("plan-overview: watch re-render failed: {error}")
                        }
                    }
                }
            });
        }
        let host = args.host.as_deref().unwrap_or("127.0.0.1");
        let _server =
            plan_overview::serve::serve_on_host_port(stream, host, args.port.unwrap_or(0))
                .map_err(|error| error.to_string())?;
        loop {
            std::thread::park();
        }
    }

    let state = parse_state(&state_json).map_err(|error| error.to_string())?;
    let artifact = render_page(&state, "/overview");
    let output = args
        .out
        .unwrap_or_else(|| args.plan_dir.join("overview.html"));
    std::fs::write(&output, artifact).map_err(|error| error.to_string())?;

    if args.watch {
        let root = args.plan_dir.clone();
        let (events, _watch_stop) =
            watch_plan_dir(root.clone()).map_err(|error| error.to_string())?;
        while wait_for_batch(&events, window) {
            match rerender_artifact(&root) {
                Ok(fresh) => {
                    if let Err(error) = std::fs::write(&output, fresh) {
                        eprintln!("plan-overview: watch write failed: {error}");
                    }
                }
                Err(error) => eprintln!("plan-overview: watch re-render failed: {error}"),
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| p.to_string()).collect()
    }

    // T130: --watch and --refresh now drive real behaviour (run()'s watch
    // loop, wired to watch.rs/StateStream), so both are accepted here --
    // B99's own refusal tests covered the opposite, pre-T130 state.
    #[test]
    fn watch_is_accepted_with_no_value() {
        let args = parse_args(argv(&["--plan-dir", "p", "--watch"])).unwrap();
        assert!(args.watch);
        assert!(args.refresh.is_none());
    }

    #[test]
    fn refresh_with_watch_sets_the_debounce_window_in_milliseconds() {
        let args = parse_args(argv(&["--plan-dir", "p", "--watch", "--refresh", "50"])).unwrap();
        assert!(args.watch);
        assert_eq!(args.refresh, Some(Duration::from_millis(50)));
    }

    #[test]
    fn a_refresh_missing_its_value_is_still_refused_naming_the_value() {
        let error = parse_args(argv(&["--plan-dir", "p", "--watch", "--refresh"])).unwrap_err();
        assert!(error.contains("--refresh needs a value"), "{error}");
    }

    #[test]
    fn a_refresh_value_that_is_not_a_number_is_refused() {
        let error =
            parse_args(argv(&["--plan-dir", "p", "--watch", "--refresh", "soon"])).unwrap_err();
        assert!(error.contains("--refresh"), "{error}");
        assert!(error.contains("soon"), "{error}");
    }

    #[test]
    fn refresh_without_watch_is_refused() {
        let error = parse_args(argv(&["--plan-dir", "p", "--refresh", "50"])).unwrap_err();
        assert!(error.contains("--refresh"), "{error}");
        assert!(error.contains("--watch"), "{error}");
    }

    #[test]
    fn serve_and_watch_may_be_combined() {
        let args = parse_args(argv(&["--plan-dir", "p", "--serve", "--watch"])).unwrap();
        assert!(args.serve);
        assert!(args.watch);
    }
}
