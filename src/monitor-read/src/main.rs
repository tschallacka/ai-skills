// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const COMMAND: &str = "monitor-read.sh";

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("monitor-read: {}", message.as_ref());
    std::process::exit(code);
}

fn usage(code: i32) -> ! {
    print!(
        "MODE: PROD\n{COMMAND} — gated, bounded, paginated monitor reader (pull-on-exception).\n\nWillie (the end-user-facing monitor, maintainer persona) supervises personas\nby reading ONLY each subagent's bounded supervision frame (written by\nsupervision-frame.sh), never raw logs. This reader serves those frames\nbounded and paginated so a green frame costs ~zero context and Willie pulls\ndeeper only when a frame flags escalated/out-of-bounds/blocked.\n\nUsage:\n  {COMMAND} show <frame-file>                      # bounded frame text\n  {COMMAND} status <frame-file>                    # one-line: subagent,status\n  {COMMAND} summary <dir>                          # all frames under <dir>\n  {COMMAND} grants <grant-log> [--last N]          # grant log (case+command)\n  {COMMAND} verify <frame-file>                    # fail-closed identity\n  {COMMAND} --help\n\nGating: Willie is the maintainer. Reading a frame requires ROLE_ID resolving\nto `maintainer`; non-maintainer callers are refused (fail closed). Budget is\nenforced per frame via supervision-frame.sh check.\n"
    );
    std::process::exit(code);
}

fn require_maintainer() {
    let role = env::var("ROLE_ID").unwrap_or_default().to_ascii_lowercase();
    if role != "maintainer" && role != "willie" {
        die(
            format!(
                "FAIL-CLOSED identity: only the maintainer (Willie) may read supervision frames; got ROLE_ID=\"{}\"",
                env::var("ROLE_ID").unwrap_or_default()
            ),
            64,
        );
    }
}

fn frame(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| die(format!("no frame at {path}"), 66))
}

fn check_budget(path: &str) {
    let budget = env::var("FRAME_BUDGET")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(2048);
    let size = fs::metadata(path)
        .unwrap_or_else(|_| die(format!("no frame at {path}"), 66))
        .len() as usize;
    if size > budget {
        die(format!("frame {path} over budget; refusing to load"), 64);
    }
}

fn show(args: &[String]) {
    if args.len() != 1 {
        usage(64);
    }
    require_maintainer();
    let path = &args[0];
    let content = frame(path);
    check_budget(path);
    print!("{content}");
}

fn status(args: &[String]) {
    if args.len() != 1 {
        usage(64);
    }
    require_maintainer();
    let content = frame(&args[0]);
    let mut output = String::new();
    for line in content.lines() {
        if line.starts_with("subagent:") || line.starts_with("status:") {
            if let Some((key, value)) = line.split_once(": ") {
                output.push_str(key);
                output.push('=');
                output.push_str(value);
                output.push(' ');
            }
        }
    }
    println!("{output}");
}

fn summary(args: &[String]) {
    if args.len() != 1 {
        usage(64);
    }
    require_maintainer();
    let dir = Path::new(&args[0]);
    if !dir.is_dir() {
        die(format!("no frames dir {}", args[0]), 66);
    }
    let mut paths = fs::read_dir(dir)
        .unwrap_or_else(|error| die(error.to_string(), 66))
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .collect::<Vec<_>>();
    paths.sort_by_key(|entry| entry.file_name());
    for entry in paths {
        print!("{}: ", entry.file_name().to_string_lossy());
        status(&[entry.path().to_string_lossy().into_owned()]);
    }
}

fn grants(args: &[String]) {
    let mut log = None;
    let mut last = 20usize;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--last" => {
                if index + 1 >= args.len() {
                    usage(64);
                }
                let value = &args[index + 1];
                last = match value.parse::<usize>() {
                    Ok(value) => value,
                    Err(_) => match value
                        .strip_prefix('-')
                        .filter(|value| !value.is_empty())
                        .and_then(|value| value.parse::<usize>().ok())
                    {
                        Some(value) => value,
                        None => {
                            eprintln!("tail: invalid number of lines: '{value}'");
                            std::process::exit(1);
                        }
                    },
                };
                index += 2;
            }
            value if value.starts_with('-') => usage(64),
            value => {
                if log.replace(value.to_string()).is_some() {
                    usage(64);
                }
                index += 1;
            }
        }
    }
    require_maintainer();
    let path = log.unwrap_or_else(|| usage(64));
    let content = fs::read(&path).unwrap_or_else(|_| die(format!("no grant log at {path}"), 66));
    let start = if last == 0 || content.is_empty() {
        content.len()
    } else {
        let target = last + usize::from(content.ends_with(b"\n"));
        let mut seen = 0;
        let mut start = 0;
        for index in (0..content.len()).rev() {
            if content[index] == b'\n' {
                seen += 1;
                if seen == target {
                    start = index + 1;
                    break;
                }
            }
        }
        start
    };
    io::stdout()
        .write_all(&content[start..])
        .unwrap_or_else(|error| die(error.to_string(), 73));
}

fn verify(args: &[String]) {
    if args.len() != 1 {
        usage(64);
    }
    require_maintainer();
    let content = frame(&args[0]);
    let status = content
        .lines()
        .find_map(|line| line.strip_prefix("status:").map(str::trim))
        .unwrap_or_default();
    match status {
        "ok" => println!("green: {status}"),
        "escalated" | "out-of-bounds" | "blocked" => {
            println!("PULL-ON-EXCEPTION: status={status}")
        }
        _ => die(format!("unknown status \"{status}\""), 64),
    }
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0);
    }
    let command = args.first().cloned().unwrap_or_default();
    if !args.is_empty() {
        args.remove(0);
    }
    match command.as_str() {
        "show" => show(&args),
        "status" => status(&args),
        "summary" => summary(&args),
        "grants" => grants(&args),
        "verify" => verify(&args),
        _ => usage(64),
    }
}
