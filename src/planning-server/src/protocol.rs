// MODE: DEV
// PACKAGE: PROD
//! Wire protocol between planning-client/planning-mcp and the planning-server
//! daemon: one Request per connection line, one Response back (NDJSON, the
//! same one-message-per-line shape ai-text-editor's own transport.rs uses).
//!
//! Two bounded reads, a growing set of guarded writes (three of them --
//! UpdateWorkUnit/RemoveWorkUnit/UpdatePlanContent -- generic wire shapes
//! covering several CLI modes each, rather than one Request variant per
//! flag), and one read-only validator. Every guarded Request carries the
//! PlanRevision (as its hex string) the caller last observed for the
//! document it targets; the server checks it against the document's current
//! on-disk hash before applying any write (see revision.rs).

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
    /// The flexible positional/flag mix update-work-unit itself takes
    /// (`args`, verbatim, after `plan_dir` and `unit_id`) rather than one
    /// field per flag: `--scope`/`--file`/`--type`/`--depends-on`/
    /// `--description`, the two-positional shorthand, or `--goal`/`--step`
    /// to move the unit are all the same guarded write against the same
    /// file, so nothing is gained by giving each its own Request shape.
    /// Guards work-unit-inventory.md only, matching AddWorkUnit's own
    /// documented MVP simplification: a move also rewrites the unit's step
    /// file and both goals' progress trackers, and those are not separately
    /// guarded here.
    UpdateWorkUnit {
        plan_dir: String,
        unit_id: String,
        args: Vec<String>,
        revision: String,
    },
    /// Guards work-unit-inventory.md, the same simplification as
    /// UpdateWorkUnit/AddWorkUnit -- the cascade also rewrites coverage
    /// rows, the owning goal's roster, the step file and its testing twin,
    /// and rebuilds both progress trackers, none of which are separately
    /// guarded here.
    RemoveWorkUnit {
        plan_dir: String,
        unit_id: String,
        confirm_cascade: bool,
        revision: String,
    },
    /// One generic wire shape for every update-plan-content mode except
    /// review-status/testing-requirement (which predate this and keep their
    /// own dedicated variants): `mode` is the long flag name without its
    /// leading `--` (e.g. "description-paragraph", "title",
    /// "decomposition-review"), and `args` is the rest of that mode's own
    /// positional arguments, verbatim and in CLI order, after `plan_dir`.
    /// The guard target is resolved from `mode`/`args` the same way
    /// update-plan-content's own `document_path` resolves its write target
    /// (see handlers::update_plan_content_target) -- eighteen modes would
    /// otherwise mean eighteen near-identical Request variants and handler
    /// functions for what is, underneath, one guarded subprocess call with
    /// a different flag and argument list each time.
    UpdatePlanContent {
        plan_dir: String,
        mode: String,
        args: Vec<String>,
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
    /// Unguarded: create-adversarial-review itself refuses outright when
    /// adversarial-review.md already exists (exit 73), so this can never
    /// lose a concurrent write the way an overwrite could -- the file
    /// either does not exist yet (nothing to guard against) or the call
    /// fails cleanly with no write at all.
    CreateAdversarialReview { plan_dir: String },
    /// `args`, verbatim, after `plan_dir`: `--file`/`--cycle`/`--check`.
    /// Guards adversarial-review.md.
    UpdateAdversarialReview {
        plan_dir: String,
        args: Vec<String>,
        revision: String,
    },
    /// `args`, verbatim, after `plan_dir` and `finding_id`: the finding and
    /// resolution text, plus `--status`/`--work-unit`. Guards
    /// adversarial-review.md; re-minting fix-keys.json when `--work-unit`
    /// is given is not separately guarded (the same MVP simplification as
    /// AddWorkUnit's own secondary file).
    AddAdversarialFinding {
        plan_dir: String,
        finding_id: String,
        args: Vec<String>,
        revision: String,
    },
    /// `args`, verbatim, after `plan_dir` and `finding_id`:
    /// `--status`/`--claimed-by`. Guards adversarial-review.md.
    ResolveFinding {
        plan_dir: String,
        finding_id: String,
        args: Vec<String>,
        revision: String,
    },
    /// Unguarded: mint-fix-keys fully regenerates fix-keys.json from the
    /// plan's current findings every time rather than applying an
    /// incremental edit, so a revision guard against its OWN previous
    /// bytes would not protect anything a plain re-run does not already
    /// risk -- two concurrent mints simply leave the later write standing,
    /// exactly as running the CLI twice back to back would.
    MintFixKeys { plan_dir: String },
    /// Read-only, unguarded, like ValidatePlan: verifies fixes.md's claims
    /// against fix-keys.json and reports pass/fail plus the full report.
    VerifyFixKeys {
        plan_dir: String,
        claimed_by: Option<String>,
    },
    /// Unguarded: fixes.md is an append-only audit trail that may not exist
    /// yet at all before the first claim (the same "nothing to guard
    /// against yet" gap AddWorkUnit's own new step file already carries,
    /// documented rather than worked around with a new bootstrap
    /// mechanism this crate's guard does not otherwise need).
    AddFixClaim {
        plan_dir: String,
        finding_id: String,
        work_unit: String,
        key: String,
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
    fn the_generic_write_requests_round_trip_through_json() {
        let requests = [
            Request::UpdateWorkUnit {
                plan_dir: "/plans/demo".to_string(),
                unit_id: "W05".to_string(),
                args: vec!["--scope".to_string(), "new scope".to_string()],
                revision: "abc".to_string(),
            },
            Request::RemoveWorkUnit {
                plan_dir: "/plans/demo".to_string(),
                unit_id: "W05".to_string(),
                confirm_cascade: true,
                revision: "abc".to_string(),
            },
            Request::UpdatePlanContent {
                plan_dir: "/plans/demo".to_string(),
                mode: "title".to_string(),
                args: vec!["goal:01-example".to_string(), "New title".to_string()],
                revision: "abc".to_string(),
            },
        ];
        for request in requests {
            let encoded = encode_request(&request);
            assert_eq!(decode_request(&encoded).unwrap(), request);
        }
    }

    #[test]
    fn the_adversarial_review_workflow_requests_round_trip_through_json() {
        let requests = [
            Request::CreateAdversarialReview {
                plan_dir: "/plans/demo".to_string(),
            },
            Request::UpdateAdversarialReview {
                plan_dir: "/plans/demo".to_string(),
                args: vec!["--cycle".to_string(), "2".to_string()],
                revision: "abc".to_string(),
            },
            Request::AddAdversarialFinding {
                plan_dir: "/plans/demo".to_string(),
                finding_id: "AR-01".to_string(),
                args: vec!["missing".to_string(), "add it".to_string()],
                revision: "abc".to_string(),
            },
            Request::ResolveFinding {
                plan_dir: "/plans/demo".to_string(),
                finding_id: "AR-01".to_string(),
                args: vec!["--status".to_string(), "resolved".to_string()],
                revision: "abc".to_string(),
            },
            Request::MintFixKeys {
                plan_dir: "/plans/demo".to_string(),
            },
            Request::VerifyFixKeys {
                plan_dir: "/plans/demo".to_string(),
                claimed_by: Some("session-1".to_string()),
            },
            Request::AddFixClaim {
                plan_dir: "/plans/demo".to_string(),
                finding_id: "AR-01".to_string(),
                work_unit: "W01".to_string(),
                key: "a".repeat(64),
            },
        ];
        for request in requests {
            let encoded = encode_request(&request);
            assert_eq!(decode_request(&encoded).unwrap(), request);
        }
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
