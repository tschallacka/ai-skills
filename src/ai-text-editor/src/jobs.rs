// MODE: DEV
// PACKAGE: PROD
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use thiserror::Error;

const DEFAULT_RETENTION: Duration = Duration::from_secs(10 * 60);
const MAX_RETENTION: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
    Released,
    Evicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub id: u64,
    pub state: JobState,
    pub owner: String,
    pub detached: bool,
    pub resume_token: Option<String>,
    pub progress: Option<Value>,
    pub result: Vec<Value>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum JobError {
    #[error("job does not exist")]
    Missing,
    #[error("resume token is invalid or permanently expired")]
    InvalidToken,
    #[error("job is not transferable")]
    NotTransferable,
    #[error("job has already been released or evicted")]
    Terminal,
    #[error("job retention exceeds the one-hour maximum without dangerous acknowledgement")]
    RetentionTooLong,
    #[error("operation lost the job state race")]
    RaceLost,
}

#[derive(Debug)]
struct Job {
    snapshot: JobSnapshot,
    expires_at: Option<SystemTime>,
    cancel_or_complete: bool,
}

#[derive(Debug)]
pub struct JobRegistry {
    next_id: u64,
    jobs: HashMap<u64, Job>,
    retention: Duration,
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self {
            next_id: 1,
            jobs: HashMap::new(),
            retention: DEFAULT_RETENTION,
        }
    }
}

impl JobRegistry {
    pub fn with_retention(
        retention: Duration,
        dangerous_acknowledged: bool,
    ) -> Result<Self, JobError> {
        if retention > MAX_RETENTION && !dangerous_acknowledged {
            return Err(JobError::RetentionTooLong);
        }
        Ok(Self {
            retention: retention.min(MAX_RETENTION),
            ..Self::default()
        })
    }

    pub fn start(&mut self, owner: impl Into<String>, detached: bool) -> JobSnapshot {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let token = format!("job-{id}-{}", token_fragment(id));
        let snapshot = JobSnapshot {
            id,
            state: JobState::Queued,
            owner: owner.into(),
            detached,
            resume_token: Some(token),
            progress: None,
            result: Vec::new(),
        };
        self.jobs.insert(
            id,
            Job {
                snapshot: snapshot.clone(),
                expires_at: None,
                cancel_or_complete: false,
            },
        );
        snapshot
    }

    pub fn running(&mut self, id: u64) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.snapshot.state != JobState::Queued {
            return Err(JobError::RaceLost);
        }
        job.snapshot.state = JobState::Running;
        Ok(job.snapshot.clone())
    }

    pub fn progress(&mut self, id: u64, progress: Value) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if !matches!(job.snapshot.state, JobState::Queued | JobState::Running) {
            return Err(JobError::Terminal);
        }
        job.snapshot.progress = Some(progress);
        Ok(job.snapshot.clone())
    }

    pub fn complete(&mut self, id: u64, result: Vec<Value>) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.cancel_or_complete {
            return Err(JobError::RaceLost);
        }
        if !matches!(job.snapshot.state, JobState::Queued | JobState::Running) {
            return Err(JobError::Terminal);
        }
        job.cancel_or_complete = true;
        job.snapshot.state = JobState::Completed;
        job.snapshot.result = result;
        job.expires_at = Some(SystemTime::now() + self.retention);
        Ok(job.snapshot.clone())
    }

    pub fn fail(&mut self, id: u64, message: impl Into<String>) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.cancel_or_complete {
            return Err(JobError::RaceLost);
        }
        if !matches!(job.snapshot.state, JobState::Queued | JobState::Running) {
            return Err(JobError::Terminal);
        }
        job.cancel_or_complete = true;
        job.snapshot.state = JobState::Failed;
        job.snapshot.result = vec![serde_json::json!({"error": message.into()})];
        job.expires_at = Some(SystemTime::now() + self.retention);
        Ok(job.snapshot.clone())
    }

    pub fn cancel(&mut self, id: u64) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.cancel_or_complete {
            return Err(JobError::RaceLost);
        }
        if !matches!(job.snapshot.state, JobState::Queued | JobState::Running) {
            return Err(JobError::Terminal);
        }
        job.cancel_or_complete = true;
        job.snapshot.state = JobState::Cancelled;
        job.expires_at = Some(SystemTime::now() + self.retention);
        Ok(job.snapshot.clone())
    }

    pub fn disconnect(&mut self, id: u64) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.snapshot.detached {
            return Ok(job.snapshot.clone());
        }
        if job.cancel_or_complete {
            return Err(JobError::RaceLost);
        }
        job.cancel_or_complete = true;
        job.snapshot.state = JobState::Cancelled;
        job.expires_at = Some(SystemTime::now() + self.retention);
        Ok(job.snapshot.clone())
    }

    pub fn transfer(
        &mut self,
        id: u64,
        token: &str,
        new_owner: impl Into<String>,
    ) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.snapshot.resume_token.as_deref() != Some(token) {
            return Err(JobError::InvalidToken);
        }
        if matches!(job.snapshot.state, JobState::Released | JobState::Evicted) {
            return Err(JobError::Terminal);
        }
        job.snapshot.owner = new_owner.into();
        job.snapshot.detached = true;
        Ok(job.snapshot.clone())
    }

    pub fn get(&mut self, id: u64, token: Option<&str>) -> Result<JobSnapshot, JobError> {
        self.evict_expired();
        let job = self.jobs.get(&id).ok_or(JobError::Missing)?;
        if let Some(token) = token {
            if job.snapshot.resume_token.as_deref() != Some(token) {
                return Err(JobError::InvalidToken);
            }
        }
        Ok(job.snapshot.clone())
    }

    pub fn release(&mut self, id: u64, token: &str) -> Result<JobSnapshot, JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Missing)?;
        if job.snapshot.resume_token.as_deref() != Some(token) {
            return Err(JobError::InvalidToken);
        }
        if matches!(job.snapshot.state, JobState::Released | JobState::Evicted) {
            return Err(JobError::Terminal);
        }
        job.snapshot.state = JobState::Released;
        job.snapshot.resume_token = None;
        job.expires_at = None;
        Ok(job.snapshot.clone())
    }

    /// True while any job on this tab is still queued or running — a
    /// detached large-edit job in particular can outlive the connection that
    /// started it, and an idle-timeout watchdog must not shut the server
    /// down out from under one.
    pub fn has_active(&self) -> bool {
        self.jobs
            .values()
            .any(|job| matches!(job.snapshot.state, JobState::Queued | JobState::Running))
    }

    fn evict_expired(&mut self) {
        let now = SystemTime::now();
        for job in self.jobs.values_mut() {
            if job.expires_at.is_some_and(|expires| expires <= now)
                && !matches!(job.snapshot.state, JobState::Released | JobState::Evicted)
            {
                job.snapshot.state = JobState::Evicted;
                job.snapshot.resume_token = None;
                job.expires_at = None;
            }
        }
    }
}

fn token_fragment(id: u64) -> String {
    blake3::hash(format!("{id}-{}", std::process::id()).as_bytes()).to_hex()[..16].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_transfer_and_release_invalidate_token() {
        let mut jobs = JobRegistry::default();
        let queued = jobs.start("client", false);
        assert_eq!(queued.state, JobState::Queued);
        jobs.running(queued.id).unwrap();
        let transferred = jobs
            .transfer(queued.id, queued.resume_token.as_deref().unwrap(), "other")
            .unwrap();
        assert!(transferred.detached);
        let completed = jobs
            .complete(queued.id, vec![serde_json::json!({"ok": true})])
            .unwrap();
        jobs.release(queued.id, completed.resume_token.as_deref().unwrap())
            .unwrap();
        assert_eq!(
            jobs.get(queued.id, queued.resume_token.as_deref())
                .unwrap_err(),
            JobError::InvalidToken
        );
    }

    #[test]
    fn cancel_wins_the_state_lock_and_disconnect_cancels_owned_jobs() {
        let mut jobs = JobRegistry::default();
        let queued = jobs.start("client", false);
        jobs.running(queued.id).unwrap();
        assert_eq!(jobs.cancel(queued.id).unwrap().state, JobState::Cancelled);
        assert_eq!(
            jobs.complete(queued.id, Vec::new()).unwrap_err(),
            JobError::RaceLost
        );
        let detached = jobs.start("client", false);
        assert_eq!(
            jobs.disconnect(detached.id).unwrap().state,
            JobState::Cancelled
        );
        let kept = jobs.start("client", true);
        assert_eq!(jobs.disconnect(kept.id).unwrap().state, JobState::Queued);
    }

    #[test]
    fn retention_limit_requires_dangerous_acknowledgement() {
        assert_eq!(
            JobRegistry::with_retention(MAX_RETENTION + Duration::from_secs(1), false).unwrap_err(),
            JobError::RetentionTooLong
        );
        assert!(JobRegistry::with_retention(MAX_RETENTION + Duration::from_secs(1), true).is_ok());
    }
}
