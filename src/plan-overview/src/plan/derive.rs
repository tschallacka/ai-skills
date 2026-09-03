// MODE: DEV
// PACKAGE: PROD
use super::state::State;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    pub completed: u32,
    pub total: u32,
}

impl Count {
    pub fn percentage(self) -> u32 {
        self.completed
            .saturating_mul(100)
            .checked_div(self.total)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalCount {
    pub goal: String,
    pub steps: Count,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counts {
    pub goals: Count,
    pub steps: Count,
    pub work_units: Count,
    pub steps_complete: u32,
    pub findings: Count,
    pub findings_open: u32,
    pub findings_resolved: Count,
    pub resolved_percentage: u32,
    pub review_depth: Count,
    pub per_goal: Vec<GoalCount>,
}

pub fn derive_counts(state: &State) -> Counts {
    let completed_steps = state
        .steps
        .iter()
        .filter(|step| step.status == "completed")
        .count() as u32;
    let units: BTreeSet<&str> = state
        .steps
        .iter()
        .map(|step| step.unit.as_str())
        .filter(|unit| !unit.is_empty())
        .collect();
    let resolved = state
        .findings
        .iter()
        .filter(|finding| finding.status == "resolved")
        .count() as u32;
    let per_goal = state
        .goals
        .iter()
        .map(|goal| {
            let steps: Vec<_> = state
                .steps
                .iter()
                .filter(|step| step.goal == goal.id)
                .collect();
            GoalCount {
                goal: goal.id.clone(),
                steps: Count {
                    completed: steps
                        .iter()
                        .filter(|step| step.status == "completed")
                        .count() as u32,
                    total: steps.len() as u32,
                },
            }
        })
        .collect();
    Counts {
        goals: Count {
            completed: 0,
            total: state.goals.len() as u32,
        },
        steps: Count {
            completed: completed_steps,
            total: state.steps.len() as u32,
        },
        work_units: Count {
            completed: 0,
            total: units.len() as u32,
        },
        findings: Count {
            completed: resolved,
            total: state.findings.len() as u32,
        },
        steps_complete: completed_steps,
        findings_open: state.findings.len() as u32 - resolved,
        findings_resolved: Count {
            completed: resolved,
            total: state.findings.len() as u32,
        },
        resolved_percentage: Count {
            completed: resolved,
            total: state.findings.len() as u32,
        }
        .percentage(),
        review_depth: Count {
            completed: state.cycles,
            total: state.review_target,
        },
        per_goal,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub donut_circumference: f64,
    pub donut_offset: f64,
    pub ring_circumference: f64,
    pub work_offset: f64,
    pub findings_offset: f64,
    pub feedback_offset: f64,
}

pub fn derive_geometry(counts: &Counts) -> Geometry {
    let donut_circumference = std::f64::consts::TAU * 52.0;
    let ring_circumference = std::f64::consts::TAU * 16.0;
    Geometry {
        donut_circumference,
        donut_offset: offset(donut_circumference, counts.steps),
        ring_circumference,
        work_offset: offset(ring_circumference, counts.work_units),
        findings_offset: offset(ring_circumference, counts.findings_resolved),
        feedback_offset: offset(ring_circumference, counts.review_depth),
    }
}

fn offset(circumference: f64, count: Count) -> f64 {
    if count.total == 0 {
        circumference
    } else {
        circumference * (1.0 - (count.completed as f64 / count.total as f64)).clamp(0.0, 1.0)
    }
}
