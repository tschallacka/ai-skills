// MODE: DEV
// PACKAGE: PROD
//! tailpipe-server-rs: binds its endpoint, spawns one thread per accepted
//! connection (mirroring src/chat-server-rs/src/main.rs's own accept loop)
//! that decodes and dispatches requests against the shared Hub, and one
//! dedicated eviction-sweep thread on a timer.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tailpipe_server_rs::hub::Hub;
use tailpipe_server_rs::protocol::{Request, Response};
use tailpipe_server_rs::transport::{Listener, Stream};

const USAGE: &str = r#"tailpipe-server-rs <endpoint-path> [--idle-timeout-ms N] [--snapshot-dir PATH]

  <endpoint-path>       where the Unix socket (or, on Windows, the discovery
                        file) is bound.
  --idle-timeout-ms N   idle threshold before a stream is evicted (default:
                        900000, 15 minutes). Lower values exist for testing.
  --snapshot-dir PATH   where evicted/saved streams are written as gzip
                        files (default: $TAILPIPE_HOME, or
                        $XDG_CONFIG_HOME/tsch-ai-skills/tailpipe/snapshots,
                        or ~/.config/tsch-ai-skills/tailpipe/snapshots).
"#;

const DEFAULT_IDLE_TIMEOUT_MS: u64 = 15 * 60 * 1000;
/// How often the eviction thread wakes to check every stream's idle time.
const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

struct Args {
    endpoint: PathBuf,
    idle_timeout: Duration,
    snapshot_dir: PathBuf,
}

fn default_snapshot_dir() -> PathBuf {
    if let Ok(home) = std::env::var("TAILPIPE_HOME") {
        return PathBuf::from(home).join("snapshots");
    }
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".config")
        });
    config_home
        .join("tsch-ai-skills")
        .join("tailpipe")
        .join("snapshots")
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut endpoint = None;
    let mut idle_timeout_ms = DEFAULT_IDLE_TIMEOUT_MS;
    let mut snapshot_dir = default_snapshot_dir();

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--idle-timeout-ms" => {
                i += 1;
                let value = argv.get(i).ok_or("--idle-timeout-ms needs a value")?;
                idle_timeout_ms = value
                    .parse()
                    .map_err(|_| "--idle-timeout-ms must be a number")?;
            }
            "--snapshot-dir" => {
                i += 1;
                let value = argv.get(i).ok_or("--snapshot-dir needs a value")?;
                snapshot_dir = PathBuf::from(value);
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if endpoint.is_none() => endpoint = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1;
    }

    Ok(Args {
        endpoint: endpoint.ok_or("missing <endpoint-path>")?,
        idle_timeout: Duration::from_millis(idle_timeout_ms),
        snapshot_dir,
    })
}

/// Reads one Request line, dispatches or long-polls it, and writes the
/// response(s). One connection, one Request -- except Tail, which keeps the
/// connection open and keeps writing as new lines arrive.
fn serve_connection(hub: &Arc<Hub>, mut stream: Stream) {
    if !stream.authenticate().unwrap_or(false) {
        return;
    }
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let request: Request = match serde_json::from_str(line.trim_end()) {
        Ok(request) => request,
        Err(error) => {
            let _ = writeln!(
                stream,
                "{}",
                serde_json::to_string(&Response::Error {
                    code: "bad_request".into(),
                    message: error.to_string(),
                })
                .unwrap()
            );
            return;
        }
    };

    if let Request::Tail {
        stream: name,
        since,
    } = request
    {
        hub.tail_since(
            &name,
            since,
            |received| {
                let response = Response::Lines {
                    lines: vec![received],
                };
                let _ = writeln!(stream, "{}", serde_json::to_string(&response).unwrap());
            },
            || false,
        );
        return;
    }

    let response = hub.dispatch(request);
    let _ = writeln!(stream, "{}", serde_json::to_string(&response).unwrap());
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("tailpipe-server-rs: {message}");
            std::process::exit(if message == USAGE { 0 } else { 64 });
        }
    };

    let listener = match Listener::bind(&args.endpoint) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!(
                "tailpipe-server-rs: could not bind {:?}: {error}",
                args.endpoint
            );
            std::process::exit(1);
        }
    };

    let hub = Arc::new(Hub::new(args.snapshot_dir));

    let sweep_hub = Arc::clone(&hub);
    let idle_timeout = args.idle_timeout;
    std::thread::spawn(move || loop {
        std::thread::sleep(SWEEP_INTERVAL);
        sweep_hub.sweep_idle(idle_timeout);
    });

    loop {
        match listener.accept() {
            Ok(stream) => {
                let hub = Arc::clone(&hub);
                std::thread::spawn(move || serve_connection(&hub, stream));
            }
            Err(error) => {
                eprintln!("tailpipe-server-rs: accept failed: {error}");
            }
        }
    }
}
