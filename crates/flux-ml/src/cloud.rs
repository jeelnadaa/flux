//! Cloud job client, local mocking, auto-deletion TTL, and privacy consent management.

use serde::{Deserialize, Serialize};

/// State of an asynchronous cloud or background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudJobStatus {
    /// Queued and waiting for an available worker.
    Queued,
    /// Currently processing.
    Running,
    /// Completed successfully with downloadable assets.
    Completed,
    /// Failed with an error.
    Failed,
}

/// A job in the queue matching the prototype `#jobs` screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudJobRecord {
    /// Unique identifier for the job.
    pub id: String,
    /// Human-readable title (e.g. "Stems: Harbor Lights").
    pub title: String,
    /// Subtitle details (e.g. "On device · 3 min left", "Cloud · 1 min left · Deletes in 24 h").
    pub details: String,
    /// Percentage completed (0 to 100).
    pub progress_pct: u32,
    /// Current execution state.
    pub status: CloudJobStatus,
    /// Whether this job is currently actively spinning/animating.
    pub is_active: bool,
    /// Time-to-live in hours before ephemeral cloud assets are permanently deleted.
    pub auto_delete_hours: u32,
}

/// Per-feature cloud privacy consent toggles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrivacyConsentSettings {
    /// Allow cloud for Stems: Best quality.
    pub stems_best_cloud: bool,
    /// Allow cloud for higher quality TTS voices.
    pub tts_cloud: bool,
    /// Allow cloud for dubbing (required for dubbing).
    pub dubbing_cloud: bool,
    /// Allow anonymous crash reports (no audio or transcripts).
    pub crash_reports: bool,
    /// Allow anonymous usage telemetry.
    pub usage_stats: bool,
}

/// Mock cloud client simulating encrypted presigned upload, job queuing, progress updates, and TTL auto-deletion.
#[derive(Debug, Clone)]
pub struct MockCloudClient {
    jobs: Vec<CloudJobRecord>,
    consent: PrivacyConsentSettings,
}

impl Default for MockCloudClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCloudClient {
    /// Creates a mock client pre-populated with standard jobs matching `#jobs`.
    pub fn new() -> Self {
        Self {
            jobs: vec![
                CloudJobRecord {
                    id: "job1".into(),
                    title: "Stems: Harbor Lights".into(),
                    details: "On device · 3 min left".into(),
                    progress_pct: 62,
                    status: CloudJobStatus::Running,
                    is_active: true,
                    auto_delete_hours: 0,
                },
                CloudJobRecord {
                    id: "job2".into(),
                    title: "Transcript: Interview ep. 12".into(),
                    details: "On device · Waiting".into(),
                    progress_pct: 0,
                    status: CloudJobStatus::Queued,
                    is_active: false,
                    auto_delete_hours: 0,
                },
                CloudJobRecord {
                    id: "job3".into(),
                    title: "Dub to Spanish".into(),
                    details: "Cloud · 1 min left · Deletes in 24 h".into(),
                    progress_pct: 84,
                    status: CloudJobStatus::Running,
                    is_active: true,
                    auto_delete_hours: 24,
                },
                CloudJobRecord {
                    id: "job4".into(),
                    title: "Export: Harbor Lights master.flac".into(),
                    details: "Done 4 min ago".into(),
                    progress_pct: 100,
                    status: CloudJobStatus::Completed,
                    is_active: false,
                    auto_delete_hours: 0,
                },
            ],
            consent: PrivacyConsentSettings::default(),
        }
    }

    /// Returns a list of all jobs.
    pub fn list_jobs(&self) -> &[CloudJobRecord] {
        &self.jobs
    }

    /// Cancels or removes a job.
    pub fn cancel_job(&mut self, job_id: &str) -> bool {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == job_id) {
            self.jobs.remove(pos);
            true
        } else {
            false
        }
    }

    /// Simulates permanent auto-deletion of all cloud assets after 24h TTL.
    pub fn purge_expired_cloud_data(&mut self) -> usize {
        let initial_len = self.jobs.len();
        self.jobs
            .retain(|j| j.auto_delete_hours == 0 || j.status != CloudJobStatus::Completed);
        initial_len - self.jobs.len()
    }

    /// Deletes all cloud data immediately upon user request.
    pub fn delete_all_cloud_data(&mut self) {
        self.jobs.retain(|j| j.auto_delete_hours == 0);
    }

    /// Returns current privacy settings.
    pub fn consent_settings(&self) -> &PrivacyConsentSettings {
        &self.consent
    }

    /// Updates privacy settings.
    pub fn update_consent_settings(&mut self, settings: PrivacyConsentSettings) {
        self.consent = settings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_cloud_client_jobs_and_purge() {
        let mut client = MockCloudClient::new();
        assert_eq!(client.list_jobs().len(), 4);

        // Cancel job1
        assert!(client.cancel_job("job1"));
        assert_eq!(client.list_jobs().len(), 3);

        // Delete all cloud data
        client.delete_all_cloud_data();
        assert!(!client.list_jobs().iter().any(|j| j.id == "job3"));
    }
}
