// MODE: DEV
// PACKAGE: PROD
//! plan-crypt — the two cryptographic primitives the planning skill's
//! adversarial-review fix-key gate needs, in one static binary.
//!
//! It replaces a `sha256sum` -> `shasum` -> `openssl` fallback chain and an
//! `openssl rand` call with a guessable shell fallback. A static binary asks
//! the target machine for nothing, so shipping it *lowers* the skill's declared
//! requirements: `openssl` leaves `planning/requires.tsv` with this change.
//!
//! Two subcommands, both writing one lowercase hex line to stdout:
//!   sha256              digest stdin
//!   random-hex <bytes>  <bytes> bytes from the OS CSPRNG
//!
//! The message is read from stdin, never from argv: a secret in a command line
//! is visible to every process on the machine.

use plan_crypt::{random, sha256};

use std::io::{self, Read, Write};
use std::process::ExitCode;

const USAGE: &str = "\
Usage: plan-crypt sha256                 read stdin, print its lowercase hex SHA-256
       plan-crypt random-hex <bytes>     print <bytes> bytes of OS entropy as lowercase hex
       plan-crypt --version
       plan-crypt --help

Exit codes: 0 ok, 64 usage error, 74 I/O error.
";

/// The largest random-hex request served. Nothing in the planning skill asks
/// for more than 32 bytes; the cap turns a typo'd argument into a usage error
/// rather than an allocation the size of the argument.
const MAX_RANDOM_BYTES: usize = 1024;

const EX_USAGE: u8 = 64;
const EX_IOERR: u8 = 74;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Usage(message)) => {
            eprintln!("plan-crypt: {message}");
            eprint!("{USAGE}");
            ExitCode::from(EX_USAGE)
        }
        Err(Failure::Io(message)) => {
            eprintln!("plan-crypt: {message}");
            ExitCode::from(EX_IOERR)
        }
    }
}

enum Failure {
    Usage(String),
    Io(String),
}

fn run(args: &[String]) -> Result<(), Failure> {
    match args.first().map(String::as_str) {
        Some("--help") | Some("-h") => {
            print!("{USAGE}");
            Ok(())
        }
        Some("--version") => {
            println!("plan-crypt {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("sha256") => {
            if args.len() != 1 {
                return Err(Failure::Usage(
                    "sha256 takes no arguments; the message is read from stdin".into(),
                ));
            }
            digest_stdin()
        }
        Some("random-hex") => {
            let count = match args.len() {
                2 => parse_count(&args[1])?,
                _ => {
                    return Err(Failure::Usage(
                        "random-hex takes exactly one argument".into(),
                    ))
                }
            };
            random_hex(count)
        }
        Some(other) => Err(Failure::Usage(format!("unknown subcommand: {other}"))),
        None => Err(Failure::Usage("no subcommand given".into())),
    }
}

fn parse_count(raw: &str) -> Result<usize, Failure> {
    let count: usize = raw
        .parse()
        .map_err(|_| Failure::Usage(format!("not a byte count: {raw}")))?;
    if count == 0 || count > MAX_RANDOM_BYTES {
        return Err(Failure::Usage(format!(
            "byte count must be 1..={MAX_RANDOM_BYTES}, got {count}"
        )));
    }
    Ok(count)
}

/// Streamed in 64 KiB reads rather than slurped: the caller may pipe a file,
/// and the digest is incremental anyway.
fn digest_stdin() -> Result<(), Failure> {
    let mut hasher = sha256::Sha256::new();
    let mut stdin = io::stdin().lock();
    let mut buffer = vec![0u8; 65536];
    loop {
        let read = stdin
            .read(&mut buffer)
            .map_err(|e| Failure::Io(format!("reading stdin: {e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    emit(&sha256::hex(&hasher.finish()))
}

fn random_hex(count: usize) -> Result<(), Failure> {
    let mut bytes = vec![0u8; count];
    random::fill(&mut bytes)
        .map_err(|e| Failure::Io(format!("reading the OS random source: {e}")))?;
    emit(&sha256::hex(&bytes))
}

/// One line, then an explicit flush: `print!` swallows a failed write on a
/// closed or full stdout, and a truncated digest must not exit 0.
fn emit(line: &str) -> Result<(), Failure> {
    let mut out = io::stdout().lock();
    writeln!(out, "{line}").map_err(|e| Failure::Io(format!("writing stdout: {e}")))?;
    out.flush()
        .map_err(|e| Failure::Io(format!("flushing stdout: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_count_rejects_what_it_should() {
        assert!(matches!(parse_count("0"), Err(Failure::Usage(_))));
        assert!(matches!(parse_count("-1"), Err(Failure::Usage(_))));
        assert!(matches!(parse_count("1025"), Err(Failure::Usage(_))));
        assert!(matches!(parse_count("eight"), Err(Failure::Usage(_))));
        assert!(matches!(parse_count(""), Err(Failure::Usage(_))));
        assert_eq!(parse_count("8").ok(), Some(8));
        assert_eq!(parse_count("1024").ok(), Some(1024));
    }

    #[test]
    fn unknown_and_missing_subcommands_are_usage_errors() {
        assert!(matches!(run(&[]), Err(Failure::Usage(_))));
        assert!(matches!(
            run(&["sha512".to_string()]),
            Err(Failure::Usage(_))
        ));
        assert!(matches!(
            run(&["sha256".to_string(), "extra".to_string()]),
            Err(Failure::Usage(_))
        ));
        assert!(matches!(
            run(&["random-hex".to_string()]),
            Err(Failure::Usage(_))
        ));
    }
}
