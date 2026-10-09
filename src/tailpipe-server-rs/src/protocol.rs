// MODE: DEV
// PACKAGE: PROD
//! The wire protocol: one JSON object per line, in both directions.
//!
//! `Tail` is a long-poll request, not a single round trip: the server does
//! not answer it immediately. It holds the connection open and writes one
//! `Response::Line` per new line as it is ingested into the named stream,
//! starting after id `since`, until the client disconnects -- mirroring the
//! chat skill's own live, connection-held tail rather than request/response
//! pairing. Every other variant is an ordinary single round trip.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchMode {
    Exact,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Request {
    Ingest {
        stream: String,
        line: String,
    },
    List,
    Read {
        stream: String,
        from: u64,
        to: u64,
    },
    Search {
        stream: String,
        mode: SearchMode,
        query: String,
    },
    Save {
        stream: String,
    },
    /// Long-poll: see this module's own doc comment.
    Tail {
        stream: String,
        since: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Line {
    pub id: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Response {
    Ingested { id: u64 },
    Streams { names: Vec<String> },
    Lines { lines: Vec<Line> },
    Saved { path: String },
    Error { code: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trips<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let text = serde_json::to_string(value).unwrap();
        let back: T = serde_json::from_str(&text).unwrap();
        assert_eq!(value, &back);
    }

    #[test]
    fn every_request_variant_round_trips() {
        round_trips(&Request::Ingest {
            stream: "s".into(),
            line: "hello".into(),
        });
        round_trips(&Request::List);
        round_trips(&Request::Read {
            stream: "s".into(),
            from: 1,
            to: 5,
        });
        round_trips(&Request::Search {
            stream: "s".into(),
            mode: SearchMode::Regex,
            query: "ab+c".into(),
        });
        round_trips(&Request::Save { stream: "s".into() });
        round_trips(&Request::Tail {
            stream: "s".into(),
            since: 3,
        });
    }

    #[test]
    fn every_response_variant_round_trips() {
        round_trips(&Response::Ingested { id: 1 });
        round_trips(&Response::Streams {
            names: vec!["a".into(), "b".into()],
        });
        round_trips(&Response::Lines {
            lines: vec![Line {
                id: 1,
                text: "hello".into(),
            }],
        });
        round_trips(&Response::Saved {
            path: "/tmp/s-1.gz".into(),
        });
        round_trips(&Response::Error {
            code: "not_found".into(),
            message: "no such stream".into(),
        });
    }
}
