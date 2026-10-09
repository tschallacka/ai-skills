// MODE: DEV
// PACKAGE: PROD
//! The per-stream append-only line store: monotonic id allocation, in-memory
//! retention, and the id-range read and exact-text/regex search this plan's
//! Read/Search requests need. `last_activity` is updated by every read or
//! write, which is what `eviction::sweep_idle_streams` checks against the
//! idle threshold.

use crate::protocol::{Line, SearchMode};
use regex::Regex;
use std::time::Instant;

pub struct StreamLog {
    lines: Vec<Line>,
    next_id: u64,
    last_activity: Instant,
}

impl StreamLog {
    pub fn new() -> Self {
        StreamLog {
            lines: Vec::new(),
            next_id: 1,
            last_activity: Instant::now(),
        }
    }

    /// Appends `text` under the next monotonic id and returns it.
    pub fn ingest(&mut self, text: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.lines.push(Line { id, text });
        self.touch();
        id
    }

    /// Every retained line with `from <= id <= to`.
    pub fn read_range(&mut self, from: u64, to: u64) -> Vec<Line> {
        self.touch();
        self.lines
            .iter()
            .filter(|line| line.id >= from && line.id <= to)
            .cloned()
            .collect()
    }

    /// Every retained line matching `query` under `mode`. A malformed regex
    /// is reported rather than silently matching nothing.
    pub fn search(&mut self, mode: &SearchMode, query: &str) -> Result<Vec<Line>, String> {
        self.touch();
        match mode {
            SearchMode::Exact => Ok(self
                .lines
                .iter()
                .filter(|line| line.text.contains(query))
                .cloned()
                .collect()),
            SearchMode::Regex => {
                let re = Regex::new(query).map_err(|error| error.to_string())?;
                Ok(self
                    .lines
                    .iter()
                    .filter(|line| re.is_match(&line.text))
                    .cloned()
                    .collect())
            }
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
}

impl Default for StreamLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_gap_free_and_increasing() {
        let mut log = StreamLog::new();
        let ids: Vec<u64> = (0..5).map(|n| log.ingest(format!("line {n}"))).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_range_read_returns_exactly_the_requested_ids() {
        let mut log = StreamLog::new();
        for n in 0..5 {
            log.ingest(format!("line {n}"));
        }
        let got: Vec<u64> = log.read_range(2, 4).iter().map(|line| line.id).collect();
        assert_eq!(got, vec![2, 3, 4]);
    }

    #[test]
    fn exact_search_finds_a_planted_line() {
        let mut log = StreamLog::new();
        log.ingest("hello world".into());
        log.ingest("goodbye".into());
        let found = log.search(&SearchMode::Exact, "hello").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "hello world");
    }

    #[test]
    fn regex_search_finds_a_planted_line_exact_would_miss() {
        let mut log = StreamLog::new();
        log.ingest("error: boom".into());
        log.ingest("all good".into());
        let found = log.search(&SearchMode::Regex, "^error:").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "error: boom");
    }
}
