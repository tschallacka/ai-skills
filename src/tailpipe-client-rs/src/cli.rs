// MODE: DEV
// PACKAGE: PROD
//! Argument parsing for the ingest/list/read/search/tail/save subcommands.

use std::path::PathBuf;
use tailpipe_server_rs::protocol::SearchMode;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ingest {
        endpoint: PathBuf,
        stream: String,
    },
    List {
        endpoint: PathBuf,
    },
    Read {
        endpoint: PathBuf,
        stream: String,
        from: u64,
        to: u64,
    },
    Search {
        endpoint: PathBuf,
        stream: String,
        mode: SearchMode,
        query: String,
    },
    /// `since: None` means "start from the stream's current end", mirroring
    /// chat-client-rs tail's own default; `Some(id)` replays from that id.
    Tail {
        endpoint: PathBuf,
        stream: String,
        since: Option<u64>,
    },
    Save {
        endpoint: PathBuf,
        stream: String,
        out: Option<PathBuf>,
    },
}

pub fn parse_args(argv: &[String]) -> Result<Command, String> {
    let (subcommand, rest) = argv
        .split_first()
        .ok_or("missing subcommand (ingest/list/read/search/tail/save)")?;

    let mut endpoint = None;
    let mut stream = None;
    let mut from = None;
    let mut to = None;
    let mut mode = None;
    let mut query = None;
    let mut since = None;
    let mut out = None;

    let mut i = 0;
    while i < rest.len() {
        let flag = &rest[i];
        let mut value = || {
            i += 1;
            rest.get(i).cloned().ok_or(format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--endpoint" => endpoint = Some(PathBuf::from(value()?)),
            "--stream" => stream = Some(value()?),
            "--from" => {
                from = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|_| "--from must be a number")?,
                )
            }
            "--to" => {
                to = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|_| "--to must be a number")?,
                )
            }
            "--mode" => {
                mode = Some(match value()?.as_str() {
                    "exact" => SearchMode::Exact,
                    "regex" => SearchMode::Regex,
                    other => return Err(format!("--mode must be exact or regex, not {other}")),
                })
            }
            "--query" => query = Some(value()?),
            "--since" => {
                since = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|_| "--since must be a number")?,
                )
            }
            "--out" => out = Some(PathBuf::from(value()?)),
            other => return Err(format!("unrecognized flag: {other}")),
        }
        i += 1;
    }

    let endpoint = endpoint.ok_or("missing --endpoint")?;
    let require_stream = || stream.clone().ok_or("missing --stream".to_string());

    match subcommand.as_str() {
        "ingest" => Ok(Command::Ingest {
            endpoint,
            stream: require_stream()?,
        }),
        "list" => Ok(Command::List { endpoint }),
        "read" => Ok(Command::Read {
            endpoint,
            stream: require_stream()?,
            from: from.ok_or("read needs --from")?,
            to: to.ok_or("read needs --to")?,
        }),
        "search" => Ok(Command::Search {
            endpoint,
            stream: require_stream()?,
            mode: mode.ok_or("search needs --mode exact|regex")?,
            query: query.ok_or("search needs --query")?,
        }),
        "tail" => Ok(Command::Tail {
            endpoint,
            stream: require_stream()?,
            since,
        }),
        "save" => Ok(Command::Save {
            endpoint,
            stream: require_stream()?,
            out,
        }),
        other => Err(format!("unrecognized subcommand: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| part.to_string()).collect()
    }

    #[test]
    fn ingest_parses() {
        let command =
            parse_args(&args(&["ingest", "--endpoint", "/tmp/e", "--stream", "s"])).unwrap();
        assert_eq!(
            command,
            Command::Ingest {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
            }
        );
    }

    #[test]
    fn list_parses() {
        let command = parse_args(&args(&["list", "--endpoint", "/tmp/e"])).unwrap();
        assert_eq!(
            command,
            Command::List {
                endpoint: "/tmp/e".into(),
            }
        );
    }

    #[test]
    fn read_parses() {
        let command = parse_args(&args(&[
            "read",
            "--endpoint",
            "/tmp/e",
            "--stream",
            "s",
            "--from",
            "1",
            "--to",
            "5",
        ]))
        .unwrap();
        assert_eq!(
            command,
            Command::Read {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                from: 1,
                to: 5,
            }
        );
    }

    #[test]
    fn search_parses_both_modes() {
        let exact = parse_args(&args(&[
            "search",
            "--endpoint",
            "/tmp/e",
            "--stream",
            "s",
            "--mode",
            "exact",
            "--query",
            "q",
        ]))
        .unwrap();
        assert_eq!(
            exact,
            Command::Search {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                mode: SearchMode::Exact,
                query: "q".into(),
            }
        );
        let regex = parse_args(&args(&[
            "search",
            "--endpoint",
            "/tmp/e",
            "--stream",
            "s",
            "--mode",
            "regex",
            "--query",
            "q+",
        ]))
        .unwrap();
        assert_eq!(
            regex,
            Command::Search {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                mode: SearchMode::Regex,
                query: "q+".into(),
            }
        );
    }

    #[test]
    fn tail_defaults_since_to_none_but_accepts_an_explicit_value() {
        let default =
            parse_args(&args(&["tail", "--endpoint", "/tmp/e", "--stream", "s"])).unwrap();
        assert_eq!(
            default,
            Command::Tail {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                since: None,
            }
        );
        let explicit = parse_args(&args(&[
            "tail",
            "--endpoint",
            "/tmp/e",
            "--stream",
            "s",
            "--since",
            "3",
        ]))
        .unwrap();
        assert_eq!(
            explicit,
            Command::Tail {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                since: Some(3),
            }
        );
    }

    #[test]
    fn save_parses_with_and_without_out() {
        let bare = parse_args(&args(&["save", "--endpoint", "/tmp/e", "--stream", "s"])).unwrap();
        assert_eq!(
            bare,
            Command::Save {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                out: None,
            }
        );
        let with_out = parse_args(&args(&[
            "save",
            "--endpoint",
            "/tmp/e",
            "--stream",
            "s",
            "--out",
            "/tmp/out.gz",
        ]))
        .unwrap();
        assert_eq!(
            with_out,
            Command::Save {
                endpoint: "/tmp/e".into(),
                stream: "s".into(),
                out: Some("/tmp/out.gz".into()),
            }
        );
    }

    #[test]
    fn an_unrecognized_subcommand_is_refused_by_name() {
        let error = parse_args(&args(&["bogus", "--endpoint", "/tmp/e"])).unwrap_err();
        assert!(error.contains("bogus"));
    }

    #[test]
    fn an_unrecognized_flag_is_refused_by_name() {
        let error =
            parse_args(&args(&["list", "--endpoint", "/tmp/e", "--nope", "x"])).unwrap_err();
        assert!(error.contains("--nope"));
    }
}
