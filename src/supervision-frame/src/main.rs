// MODE: DEV
// PACKAGE: PROD
use planning_register::now;
use std::env;
use std::fs;
use std::path::Path;

fn usage(code: i32) -> ! {
    println!("supervision-frame.sh — bounded supervision-frame emitter + grant log.");
    println!("\nUsage:");
    println!("  supervision-frame.sh write <frame-file> --subagent NAME --persona ID \\");
    println!(
        "      --status ok|blocked|escalated|out-of-bounds [--read-discipline ok|violated] \\"
    );
    println!(
        "      [--wholesale-reads N] [--skill-loaded none|NAME] [--needs-escalation none|CASE] \\"
    );
    println!("      [--grant-requested none|COMMAND] [--verdict TEXT]");
    println!("  supervision-frame.sh grant <grant-log-file> <subagent> <persona> \\");
    println!("      --case TEXT --command TEXT");
    println!("  supervision-frame.sh show <frame-file>");
    println!("  supervision-frame.sh check <frame-file> <budget>");
    std::process::exit(code);
}
fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{}", message.as_ref());
    std::process::exit(code);
}
fn require_arg(args: &[String], i: &mut usize) -> String {
    *i += 1;
    args.get(*i).cloned().unwrap_or_else(|| usage(64))
}

fn write_frame(args: &[String]) {
    let file = args.first().cloned().unwrap_or_else(|| usage(64));
    let mut subagent = "".into();
    let mut persona = "".into();
    let mut status = "".into();
    let mut discipline = "ok".into();
    let mut reads = "0".into();
    let mut skill = "none".into();
    let mut needs = "none".into();
    let mut grant = "none".into();
    let mut verdict = "".into();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--subagent" => subagent = require_arg(args, &mut i),
            "--persona" => persona = require_arg(args, &mut i),
            "--status" => status = require_arg(args, &mut i),
            "--read-discipline" => discipline = require_arg(args, &mut i),
            "--wholesale-reads" => reads = require_arg(args, &mut i),
            "--skill-loaded" => skill = require_arg(args, &mut i),
            "--needs-escalation" => needs = require_arg(args, &mut i),
            "--grant-requested" => grant = require_arg(args, &mut i),
            "--verdict" => verdict = require_arg(args, &mut i),
            _ => usage(64),
        }
        i += 1;
    }
    if subagent.is_empty() || persona.is_empty() || status.is_empty() {
        usage(64)
    }
    if !["ok", "blocked", "escalated", "out-of-bounds"].contains(&status.as_str()) {
        die(format!("supervision-frame: invalid status \"{status}\" (ok|blocked|escalated|out-of-bounds)"),64)
    }
    let content=format!("subagent: {subagent}\npersona: {persona}\nstatus: {status}\nread_discipline: {discipline}\nwholesale_reads: {reads}\nskill_loaded: {skill}\nneeds_escalation: {needs}\ngrant_requested: {grant}\nverdict: {verdict}\n");
    let budget = env::var("FRAME_BUDGET")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2048);
    if content.len() > budget {
        die(
            format!(
                "supervision-frame: frame {subagent} exceeds byte budget {budget} (is {})",
                content.len()
            ),
            64,
        )
    }
    let path = Path::new(&file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| die(e.to_string(), 66));
    }
    let tmp = path.with_file_name(format!(".supervision-frame.{}", std::process::id()));
    fs::write(&tmp, content).unwrap_or_else(|e| die(e.to_string(), 66));
    fs::rename(&tmp, path).unwrap_or_else(|e| die(e.to_string(), 66));
}
fn grant(args: &[String]) {
    if args.len() < 3 {
        usage(64)
    }
    let file = &args[0];
    let subagent = &args[1];
    let persona = &args[2];
    let mut case_text = "";
    let mut command = "";
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--case" => {
                i += 1;
                case_text = args.get(i).map(String::as_str).unwrap_or("")
            }
            "--command" => {
                i += 1;
                command = args.get(i).map(String::as_str).unwrap_or("")
            }
            _ => usage(64),
        }
        i += 1;
    }
    if case_text.is_empty() || command.is_empty() {
        usage(64)
    }
    let path = Path::new(file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| die(e.to_string(), 66));
    }
    use std::io::Write;
    let mut out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|e| die(e.to_string(), 66));
    writeln!(
        out,
        "grant\t{}\t{}\t{}\t{}\t{}",
        now(),
        subagent,
        persona,
        case_text,
        command
    )
    .unwrap_or_else(|e| die(e.to_string(), 66));
}
fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().cloned().unwrap_or_default();
    if command == "--help" || command == "-h" {
        usage(0)
    }
    if !args.is_empty() {
        args.remove(0);
    }
    match command.as_str() {
        "write" => write_frame(&args),
        "grant" => grant(&args),
        "show" => {
            if args.len() != 1 {
                usage(64)
            }
            let p = Path::new(&args[0]);
            if !p.is_file() {
                die(
                    format!("supervision-frame: no frame at {}", p.display()),
                    66,
                )
            }
            print!(
                "{}",
                fs::read_to_string(p).unwrap_or_else(|e| die(e.to_string(), 66))
            );
        }
        "check" => {
            if args.len() != 2 {
                usage(64)
            }
            let p = Path::new(&args[0]);
            if !p.is_file() {
                die(
                    format!("supervision-frame: no frame at {}", p.display()),
                    66,
                )
            }
            let size = fs::metadata(p).unwrap().len();
            let budget = args[1].parse::<u64>().unwrap_or_else(|_| usage(64));
            if size > budget {
                die(
                    format!("supervision-frame: frame over budget {budget} (is {size})"),
                    64,
                )
            }
            println!("ok: {size} bytes (budget {budget})")
        }
        _ => usage(64),
    }
}
