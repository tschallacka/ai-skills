// MODE: DEV
// PACKAGE: PROD
//! Wire protocol between planning-client/planning-mcp and the planning-server
//! daemon: one Request per connection line, one Response back (NDJSON, the
//! same one-message-per-line shape ai-text-editor's own transport.rs uses).
//!
//! Seven wire-level operations: two bounded reads, four guarded writes, and
//! one read-only validator. Every guarded Request carries the PlanRevision
//! (as its hex string) the caller last observed for the document it targets;
//! the server checks it against the document's current on-disk hash before
//! applying any write (see revision.rs).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op")]
pub enum Request {
    /// Any of plan-context.sh's own resolvable document ids: plan, review,
    /// goal:<goal>, step:<goal>/<step>, unit:<WNN>, inventory, coverage,
    /// progress, goal-progress:<goal>, adversarial-review, stories, bugs,
    /// fixes, fix-keys, approval.
    ReadPlanDocument {
        plan_dir: String,
        document_id: String,
        view: Option<String>,
    },
    /// Sugar for ReadPlanDocument's own unit:<WNN> case.
    ReadWorkUnit { plan_dir: String, unit_id: String },
    UpdateStep {
        plan_dir: String,
        goal: String,
        step: String,
        status: String,
        revision: String,
    },
    AddWorkUnit {
        plan_dir: String,
        id: String,
        unit_type: String,
        file: String,
        scope: String,
        subscope: String,
        change: String,
        depends_on: String,
        goal: String,
        step: String,
        revision: String,
    },
    SetReviewStatus {
        plan_dir: String,
        status: String,
        revision: String,
    },
    SetTestingRequirement {
        plan_dir: String,
        goal: String,
        required: bool,
        rationale: String,
        revision: String,
    },
    /// Read-only: no revision at all, since it applies no write.
    ValidatePlan { plan_dir: String, complete: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status")]
pub enum Response {
    /// A successful bounded read: the view text and the revision it was
    /// read at (so a subsequent guarded write can submit it as its own
    /// `revision` field without a second round trip).
    Document { content: String, revision: String },
    /// A successful guarded write: the new revision the write produced.
    Written { revision: String },
    /// ValidatePlan's own result: pass/fail plus the full report text.
    Validated { passed: bool, report: String },
    /// A guarded write refused because `revision` no longer matches what is
    /// on disk -- re-read and retry with `actual`.
    Stale { expected: String, actual: String },
    /// Any other refusal (bad usage, missing file, and the like), with a
    /// message meant to be shown to the caller as-is.
    Error { message: String },
    /// This Request variant has no handler yet (present only until W79
    /// lands each real handler; proves the dispatch loop itself is correct
    /// independent of any operation's own logic).
    Unimplemented,
}

pub fn encode_response(response: &Response) -> String {
    serde_json::to_string(response).unwrap_or_else(|error| {
        serde_json::to_string(&Response::Error {
            message: format!("could not encode response: {error}"),
        })
        .expect("Response::Error always encodes")
    })
}

pub fn decode_request(line: &str) -> Result<Request, String> {
    serde_json::from_str(line).map_err(|error| format!("malformed request: {error}"))
}

pub fn encode_request(request: &Request) -> String {
    serde_json::to_string(request).expect("Request always encodes")
}

pub fn decode_response(line: &str) -> Result<Response, String> {
    serde_json::from_str(line).map_err(|error| format!("malformed response: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_request_round_trips_through_json() {
        let request = Request::ReadPlanDocument {
            plan_dir: "/plans/demo".to_string(),
            document_id: "goal:01-example".to_string(),
            view: Some("full".to_string()),
        };
        let encoded = encode_request(&request);
        let decoded = decode_request(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn a_malformed_line_is_refused_by_name_not_a_panic() {
        let result = decode_request("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn a_response_round_trips_through_json() {
        let response = Response::Written {
            revision: "abc123".to_string(),
        };
        let encoded = encode_response(&response);
        assert_eq!(decode_response(&encoded).unwrap(), response);
    }

    #[test]
    fn every_response_variant_encodes_and_is_stable_shaped() {
        let responses = [
            Response::Document {
                content: "text".into(),
                revision: "abc".into(),
            },
            Response::Written {
                revision: "def".into(),
            },
            Response::Validated {
                passed: true,
                report: "ok".into(),
            },
            Response::Stale {
                expected: "a".into(),
                actual: "b".into(),
            },
            Response::Error {
                message: "bad".into(),
            },
            Response::Unimplemented,
        ];
        for response in responses {
            let text = encode_response(&response);
            assert!(!text.is_empty());
            assert!(!text.contains('\n'), "encoded response must be one line");
        }
    }
}
