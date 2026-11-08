//! Background jobs (imports, stems, exports, batch). Each has progress, a stage label and a
//! cancel flag; results are JSON.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Serialize, Clone, Debug)]
pub struct JobInfo {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub state: String,
    pub progress: f32,
    pub stage: String,
    pub stages: Vec<(String, String)>,
    /// What the current stage is doing right now, such as "812 MB of 2.40 GB". Empty when there's nothing to say.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    pub result: Value,
    pub error: Option<String>,
    pub started: u64,
    pub finished: Option<u64>,
}

pub struct Job {
    pub info: JobInfo,
    pub cancel: Arc<AtomicBool>,
    /// What the job has printed, newest last. In memory only, gone when Flux closes.
    log: VecDeque<String>,
    /// When the job last said anything, so a page can tell quiet from stuck.
    last_output: Instant,
}

/// Lines a job keeps. A long install prints a few hundred.
const LOG_LINES: usize = 400;

#[derive(Clone, Default)]
pub struct Jobs {
    inner: Arc<Mutex<BTreeMap<String, Job>>>,
}

/// Handle given to the worker thread.
#[derive(Clone)]
pub struct JobCtx {
    pub id: String,
    jobs: Jobs,
    cancel: Arc<AtomicBool>,
}

impl JobCtx {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    /// Report progress; returns false when the job was cancelled.
    pub fn progress(&self, p: f32) -> bool {
        self.jobs
            .update(&self.id, |j| j.progress = p.clamp(0.0, 1.0));
        !self.cancelled()
    }
    /// Add a line to what the job has printed (`job.log`). Replaces the current detail.
    pub fn log(&self, line: &str) {
        if let Some(j) = self.jobs.inner.lock().unwrap().get_mut(&self.id) {
            j.log.push_back(line.chars().take(500).collect());
            while j.log.len() > LOG_LINES {
                j.log.pop_front();
            }
            j.info.detail.clear();
            j.last_output = Instant::now();
        }
    }
    /// Say what the current stage is doing right now, without adding a line.
    pub fn detail(&self, text: &str) {
        if let Some(j) = self.jobs.inner.lock().unwrap().get_mut(&self.id) {
            j.info.detail = text.to_string();
            j.last_output = Instant::now();
        }
    }
    pub fn stage(&self, name: &str, state: &str) {
        // A stage can be marked done twice; the log says so once.
        let again = self
            .jobs
            .get(&self.id)
            .is_some_and(|j| j.stages.iter().any(|s| s.0 == name && s.1 == "done"));
        match state {
            "active" => self.log(name),
            "done" if !again => self.log(&format!("Done: {name}")),
            _ => {}
        }
        self.jobs.update(&self.id, |j| {
            j.stage = name.to_string();
            if let Some(s) = j.stages.iter_mut().find(|s| s.0 == name) {
                s.1 = state.to_string();
            } else {
                j.stages.push((name.to_string(), state.to_string()));
            }
        });
    }
}

impl Jobs {
    pub fn spawn<F>(&self, kind: &str, title: &str, stages: &[&str], f: F) -> String
    where
        F: FnOnce(&JobCtx) -> anyhow::Result<Value> + Send + 'static,
    {
        let id = crate::store::new_id();
        let cancel = Arc::new(AtomicBool::new(false));
        let info = JobInfo {
            id: id.clone(),
            kind: kind.into(),
            title: title.into(),
            state: "running".into(),
            progress: 0.0,
            stage: stages.first().map(|s| s.to_string()).unwrap_or_default(),
            detail: String::new(),
            stages: stages
                .iter()
                .map(|s| (s.to_string(), "waiting".to_string()))
                .collect(),
            result: Value::Null,
            error: None,
            started: crate::project::now_secs(),
            finished: None,
        };
        self.inner.lock().unwrap().insert(
            id.clone(),
            Job {
                info,
                cancel: cancel.clone(),
                log: VecDeque::new(),
                last_output: Instant::now(),
            },
        );
        let ctx = JobCtx {
            id: id.clone(),
            jobs: self.clone(),
            cancel,
        };
        let jobs = self.clone();
        std::thread::Builder::new()
            .name(format!("flux-job-{kind}"))
            .spawn(move || {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&ctx)));
                let cancelled = ctx.cancelled();
                jobs.update(&ctx.id, |j| {
                    j.finished = Some(crate::project::now_secs());
                    match r {
                        _ if cancelled => j.state = "cancelled".into(),
                        Ok(Ok(v)) => {
                            j.state = "done".into();
                            j.progress = 1.0;
                            j.result = v;
                            for s in &mut j.stages {
                                s.1 = "done".into();
                            }
                        }
                        Ok(Err(e)) => {
                            j.state = "failed".into();
                            j.error = Some(format!("{e:#}"));
                        }
                        Err(_) => {
                            j.state = "failed".into();
                            j.error = Some("the job stopped unexpectedly".into());
                        }
                    }
                });
            })
            .expect("job thread");
        id
    }
    fn update(&self, id: &str, f: impl FnOnce(&mut JobInfo)) {
        if let Some(j) = self.inner.lock().unwrap().get_mut(id) {
            f(&mut j.info);
        }
    }
    /// What a job has printed, its current detail, and how long it has been quiet.
    pub fn log_of(&self, id: &str) -> Option<Value> {
        self.inner.lock().unwrap().get(id).map(|j| {
            serde_json::json!({
                "state": j.info.state,
                "lines": j.log,
                "detail": j.info.detail,
                "quiet_s": j.last_output.elapsed().as_secs(),
            })
        })
    }
    pub fn get(&self, id: &str) -> Option<JobInfo> {
        self.inner.lock().unwrap().get(id).map(|j| j.info.clone())
    }
    pub fn list(&self) -> Vec<JobInfo> {
        let mut v: Vec<JobInfo> = self
            .inner
            .lock()
            .unwrap()
            .values()
            .map(|j| j.info.clone())
            .collect();
        v.sort_by(|a, b| b.started.cmp(&a.started).then(b.id.cmp(&a.id)));
        v
    }
    pub fn cancel(&self, id: &str) -> bool {
        if let Some(j) = self.inner.lock().unwrap().get(id) {
            j.cancel.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }
    pub fn active(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .values()
            .filter(|j| j.info.state == "running")
            .count()
    }
    pub fn clear_finished(&self) {
        self.inner
            .lock()
            .unwrap()
            .retain(|_, j| j.info.state == "running");
    }
    /// Block until the job finishes (tests, MCP `wait`).
    pub fn wait(&self, id: &str, timeout: std::time::Duration) -> Option<JobInfo> {
        let t0 = std::time::Instant::now();
        loop {
            let j = self.get(id)?;
            if j.state != "running" || t0.elapsed() > timeout {
                return Some(j);
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
}

#[cfg(test)]
mod log_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_job_keeps_its_last_lines_and_what_it_is_doing_now() {
        let jobs = Jobs::default();
        let id = jobs.spawn("t", "t", &[], |ctx| {
            for i in 0..500 {
                ctx.log(&format!("line {i}"));
            }
            ctx.detail("9 MB of 10 MB");
            Ok(Value::Null)
        });
        jobs.wait(&id, Duration::from_secs(5));
        let l = jobs.log_of(&id).unwrap();
        let lines = l["lines"].as_array().unwrap();
        assert_eq!(lines.len(), LOG_LINES);
        assert_eq!(lines[0], "line 100");
        assert_eq!(lines[LOG_LINES - 1], "line 499");
        assert_eq!(l["detail"], "9 MB of 10 MB");
        assert_eq!(jobs.get(&id).unwrap().detail, "9 MB of 10 MB");
    }

    #[test]
    fn stages_are_logged_and_a_new_line_ends_the_detail() {
        let jobs = Jobs::default();
        let id = jobs.spawn("t", "t", &["Step"], |ctx| {
            ctx.stage("Step", "active");
            ctx.detail("1 MB of 2 MB");
            ctx.stage("Step", "active: a package");
            ctx.stage("Step", "done");
            ctx.stage("Step", "done");
            Ok(Value::Null)
        });
        jobs.wait(&id, Duration::from_secs(5));
        let l = jobs.log_of(&id).unwrap();
        assert_eq!(l["lines"], serde_json::json!(["Step", "Done: Step"]));
        assert_eq!(l["detail"], "");
    }

    #[test]
    fn an_unknown_job_has_no_log() {
        assert!(Jobs::default().log_of("nope").is_none());
    }
}
