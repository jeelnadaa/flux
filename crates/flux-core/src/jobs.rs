//! Background jobs (imports, stems, exports, batch). Each has progress, a stage label and a
//! cancel flag; results are JSON.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone, Debug)]
pub struct JobInfo {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub state: String,
    pub progress: f32,
    pub stage: String,
    pub stages: Vec<(String, String)>,
    pub result: Value,
    pub error: Option<String>,
    pub started: u64,
    pub finished: Option<u64>,
}

pub struct Job {
    pub info: JobInfo,
    pub cancel: Arc<AtomicBool>,
}

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
    pub fn stage(&self, name: &str, state: &str) {
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
