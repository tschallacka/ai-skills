// MODE: DEV
// PACKAGE: PROD
//! `chat-spool-watch`: run it as a Claude Code Monitor, and a notification wakes an
//! idle agent whose chat interrupts have sat unread. See the library for why.

use chat_spool_watch::{heartbeat, read_snapshot, step, Config, State};
use std::io::Write;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime};

const PROGRAM: &str = "chat-spool-watch";

const USAGE: &str = "chat-spool-watch - say so when chat interrupts have gone unread

Watches the chat interrupt spool for one Claude Code session and prints ONE line
when notices have sat in it, unread, for a while. A line on a Claude Code
Monitor's stdout is a notification, so this wakes an agent that has gone idle and
is not making the tool calls that would show it the notices. It prints nothing
while the spool is empty or is emptied in time, and it never empties it.

Usage:
  chat-spool-watch [--session ID] [--state DIR] [--after SECONDS] [--repeat SECONDS]
                   [--max-alerts N] [--poll SECONDS] [--max-runtime SECONDS] [--once]
  chat-spool-watch --help

  --session ID        the Claude Code session id (default: $CLAUDE_CODE_SESSION_ID)
  --state DIR         the chat state directory (default: $AI_CHAT_HOME, else the
                      XDG config home's tsch-ai-skills/chat)
  --after SECONDS     how long notices must be unread before it speaks (default 300)
  --repeat SECONDS    quiet time before it speaks again about the same unread
                      spool (default: the --after value)
  --max-alerts N      the most times it speaks about one unread stretch (default 3)
  --poll SECONDS      how often it looks (default 5)
  --max-runtime SECONDS  exit quietly after this long (default: run until stopped)
  --once              exit after the first line, for a background Bash command

It touches <state>/interrupts/<session>/.watcher on every look, so anything can
tell a watcher is armed; it removes it on a clean exit. Arm it with, for example,
Monitor(command: \"chat-spool-watch\", timeout_ms: 1800000). A Monitor is killed at
its timeout, and nothing re-arms it.

Exit codes: 0 done (after --once, --max-runtime, or a closed stdout); 64 bad usage
or no session id.
";

struct Options {
    session: String,
    state: Option<String>,
    after: Duration,
    repeat: Option<Duration>,
    max_alerts: u32,
    poll: Duration,
    max_runtime: Option<Duration>,
    once: bool,
}

fn seconds(text: &str, flag: &str) -> Result<Duration, String> {
    let value: f64 = text
        .parse()
        .map_err(|_| format!("{flag} needs a number of seconds, not {text:?}"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{flag} needs a number of seconds of at least 0"));
    }
    Ok(Duration::from_secs_f64(value))
}

fn parse(args: &[String]) -> Result<Option<Options>, String> {
    let mut opts = Options {
        session: std::env::var("CLAUDE_CODE_SESSION_ID").unwrap_or_default(),
        state: None,
        after: Duration::from_secs(300),
        repeat: None,
        max_alerts: 3,
        poll: Duration::from_secs(5),
        max_runtime: None,
        once: false,
    };
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--once" => opts.once = true,
            "--session" => opts.session = value("--session")?,
            "--state" => opts.state = Some(value("--state")?),
            "--after" => opts.after = seconds(&value("--after")?, "--after")?,
            "--repeat" => opts.repeat = Some(seconds(&value("--repeat")?, "--repeat")?),
            "--poll" => opts.poll = seconds(&value("--poll")?, "--poll")?,
            "--max-runtime" => {
                opts.max_runtime = Some(seconds(&value("--max-runtime")?, "--max-runtime")?)
            }
            "--max-alerts" => {
                opts.max_alerts = value("--max-alerts")?
                    .parse()
                    .map_err(|_| "--max-alerts needs a whole number".to_string())?
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if opts.session.trim().is_empty() {
        return Err(
            "no session id: pass --session, or run where CLAUDE_CODE_SESSION_ID is set".to_string(),
        );
    }
    if opts.poll.is_zero() {
        opts.poll = Duration::from_millis(50);
    }
    Ok(Some(opts))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse(&args) {
        Ok(Some(opts)) => opts,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{PROGRAM}: {message}");
            return ExitCode::from(64);
        }
    };
    let state_dir = opts
        .state
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(chat_proto::spool::default_state_dir);
    let dir = chat_proto::spool::dir(&state_dir, &opts.session);
    let config = Config {
        after: opts.after,
        repeat: opts.repeat.unwrap_or(opts.after),
        max_alerts: opts.max_alerts,
    };
    // stderr goes to the Monitor's output file, not into a notification.
    eprintln!("{PROGRAM}: watching {}", dir.display());

    let started = Instant::now();
    let mut state = State::default();
    let stdout = std::io::stdout();
    loop {
        heartbeat(&dir);
        let snapshot = read_snapshot(&dir);
        if let Some(line) = step(
            &mut state,
            &snapshot,
            Instant::now(),
            SystemTime::now(),
            &config,
        ) {
            let mut out = stdout.lock();
            if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() || opts.once {
                break;
            }
        }
        if opts.max_runtime.is_some_and(|max| started.elapsed() >= max) {
            break;
        }
        std::thread::sleep(opts.poll);
    }
    let _ = std::fs::remove_file(dir.join(chat_proto::spool::HEARTBEAT));
    ExitCode::SUCCESS
}
