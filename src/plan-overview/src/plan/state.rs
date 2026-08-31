// MODE: DEV
// PACKAGE: PROD
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub identity: Identity,
    pub goals: Vec<Goal>,
    pub steps: Vec<Step>,
    pub edges: Vec<Edge>,
    pub testing_marks: Vec<TestingMark>,
    pub coverage: Vec<Coverage>,
    pub findings: Vec<Finding>,
    pub cycles: u32,
    pub review_target: u32,
    pub generated_at: String,
    pub generated_by: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub title: String,
    pub ui_affected: String,
    pub review_status: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Goal {
    pub id: String,
    pub outcome: String,
    pub testing_requirement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub goal: String,
    pub step: String,
    pub unit: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub target: String,
    pub companion: Option<String>,
    pub status: String,
    pub instructions: String,
    pub criteria: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestingMark {
    pub goal: String,
    pub step: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coverage {
    pub outcome: String,
    pub units: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub item: String,
    pub change: String,
    pub status: String,
    pub work_unit: String,
    pub cycle: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub unknown_fields: Vec<String>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_state(input: &str) -> Result<State, ParseError> {
    let value: Value = serde_json::from_str(input).map_err(|error| ParseError {
        message: format!(
            "malformed state at line {}, column {}: {error}",
            error.line(),
            error.column()
        ),
        unknown_fields: Vec::new(),
    })?;
    let object = value.as_object().ok_or_else(|| ParseError {
        message: "malformed state: expected a JSON object".into(),
        unknown_fields: Vec::new(),
    })?;
    let known = [
        "identity",
        "goals",
        "steps",
        "edges",
        "testingMarks",
        "coverage",
        "findings",
        "cycles",
        "reviewTarget",
        "generatedAt",
        "generatedBy",
    ];
    let unknown_fields: Vec<String> = object
        .keys()
        .filter(|key| !known.contains(&key.as_str()))
        .cloned()
        .collect();
    if !unknown_fields.is_empty() {
        return Err(ParseError {
            message: format!("unknown state fields: {}", unknown_fields.join(", ")),
            unknown_fields,
        });
    }
    serde_json::from_value(value).map_err(|error| ParseError {
        message: format!("malformed state: {error}"),
        unknown_fields: Vec::new(),
    })
}
