// MODE: DEV
// PACKAGE: PROD

use tailpipe_client_rs::cli::{parse_args, Command};
use tailpipe_client_rs::client::{connect_and_request, tail};
use tailpipe_client_rs::ingest::run_ingest;
use tailpipe_server_rs::protocol::{Request, Response};

fn print_response(response: Response) {
    match response {
        Response::Ingested { id } => println!("{id}"),
        Response::Streams { names } => {
            for name in names {
                println!("{name}");
            }
        }
        Response::Lines { lines } => {
            for line in lines {
                println!("{}\t{}", line.id, line.text);
            }
        }
        Response::Saved { path } => println!("{path}"),
        Response::Error { code, message } => {
            eprintln!("tailpipe-client-rs: {code}: {message}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse_args(&argv) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("tailpipe-client-rs: {message}");
            std::process::exit(64);
        }
    };

    match command {
        Command::Ingest { endpoint, stream } => {
            let stdin = std::io::stdin();
            if let Err(error) = run_ingest(&endpoint, &stream, stdin.lock(), |id| {
                eprintln!("ingested: {id}");
            }) {
                eprintln!("tailpipe-client-rs: {error}");
                std::process::exit(1);
            }
        }
        Command::List { endpoint } => match connect_and_request(&endpoint, &Request::List) {
            Ok(response) => print_response(response),
            Err(error) => {
                eprintln!("tailpipe-client-rs: {error}");
                std::process::exit(1);
            }
        },
        Command::Read {
            endpoint,
            stream,
            from,
            to,
        } => match connect_and_request(&endpoint, &Request::Read { stream, from, to }) {
            Ok(response) => print_response(response),
            Err(error) => {
                eprintln!("tailpipe-client-rs: {error}");
                std::process::exit(1);
            }
        },
        Command::Search {
            endpoint,
            stream,
            mode,
            query,
        } => match connect_and_request(
            &endpoint,
            &Request::Search {
                stream,
                mode,
                query,
            },
        ) {
            Ok(response) => print_response(response),
            Err(error) => {
                eprintln!("tailpipe-client-rs: {error}");
                std::process::exit(1);
            }
        },
        Command::Tail {
            endpoint,
            stream,
            since,
        } => {
            // since=None means "start from the stream's current end":
            // resolve it with a List+Read-less approach by reading the
            // stream's own current length via a Read of the whole range and
            // taking the highest id seen, 0 if the stream does not exist
            // yet.
            let resolved_since = match since {
                Some(value) => value,
                None => match connect_and_request(
                    &endpoint,
                    &Request::Read {
                        stream: stream.clone(),
                        from: 0,
                        to: u64::MAX,
                    },
                ) {
                    Ok(Response::Lines { lines }) => {
                        lines.iter().map(|line| line.id).max().unwrap_or(0)
                    }
                    _ => 0,
                },
            };
            if let Err(error) = tail(&endpoint, &stream, resolved_since, |response| {
                if let Response::Lines { lines } = response {
                    for line in lines {
                        println!("{}\t{}", line.id, line.text);
                    }
                }
            }) {
                eprintln!("tailpipe-client-rs: {error}");
                std::process::exit(1);
            }
        }
        Command::Save {
            endpoint,
            stream,
            out,
        } => match connect_and_request(&endpoint, &Request::Save { stream }) {
            Ok(Response::Saved { path }) => {
                if let Some(out) = out {
                    if let Err(error) = std::fs::copy(&path, &out) {
                        eprintln!("tailpipe-client-rs: could not copy to {out:?}: {error}");
                        std::process::exit(1);
                    }
                    println!("{}", out.display());
                } else {
                    println!("{path}");
                }
            }
            Ok(other) => print_response(other),
            Err(error) => {
                eprintln!("tailpipe-client-rs: {error}");
                std::process::exit(1);
            }
        },
    }
}
