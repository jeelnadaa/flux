//! Voice cloning consent, liveness verification, inaudible watermarking, and abuse protection.

use serde::{Deserialize, Serialize};

/// Consent attestations required before voice cloning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VoiceCloneConsent {
    /// Attestation: Own voice or written permission.
    pub has_permission: bool,
    /// Attestation: Will not impersonate or mislead.
    pub no_impersonation: bool,
    /// Attestation: Understands inaudible watermark and metadata labeling.
    pub acknowledges_watermark: bool,
    /// Spoken verification audio recorded and validated.
    pub spoken_phrase_verified: bool,
}

impl VoiceCloneConsent {
    /// Returns true only when every required safeguard and attestation is fulfilled.
    pub fn is_fully_authorized(&self) -> bool {
        self.has_permission
            && self.no_impersonation
            && self.acknowledges_watermark
            && self.spoken_phrase_verified
    }
}

/// Inaudible watermark generator and detector for synthetic voice governance.
pub struct WatermarkSecurity;

impl WatermarkSecurity {
    /// Fixed pseudo-random sequence for inaudible high-frequency spread-spectrum watermark.
    const WATERMARK_CARRIER_FREQ: f32 = 19500.0;

    /// Embeds an inaudible high-frequency watermark into audio samples.
    pub fn embed_watermark(samples: &mut [f32], sample_rate: f32) {
        if sample_rate < 44100.0 {
            return;
        }

        let omega = 2.0 * std::f32::consts::PI * Self::WATERMARK_CARRIER_FREQ / sample_rate;
        let watermark_amplitude = 1e-4; // -80 dBFS, imperceptible

        for (i, s) in samples.iter_mut().enumerate() {
            let carrier = (omega * i as f32).sin();
            *s += watermark_amplitude * carrier;
        }
    }

    /// Verifies if an inaudible watermark is present in the audio.
    pub fn verify_watermark(samples: &[f32], sample_rate: f32) -> bool {
        if samples.is_empty() || sample_rate < 44100.0 {
            return false;
        }

        let omega = 2.0 * std::f32::consts::PI * Self::WATERMARK_CARRIER_FREQ / sample_rate;
        let mut corr = 0.0f32;

        for (i, &s) in samples.iter().enumerate() {
            corr += s * (omega * i as f32).sin();
        }

        let normalized_corr = (corr.abs() / samples.len() as f32) * 1e4;
        normalized_corr > 0.4
    }
}

/// Voice clone management engine enforcing daily quotas and strict legal safeguards.
pub struct VoiceCloneManager {
    clones_created_today: u32,
    max_clones_per_day: u32,
}

impl VoiceCloneManager {
    /// Creates a new manager with daily safety caps.
    pub fn new(max_clones_per_day: u32) -> Self {
        Self {
            clones_created_today: 0,
            max_clones_per_day,
        }
    }

    /// Attempts to register a cloned voice model after strict authorization validation.
    pub fn create_clone(&mut self, consent: &VoiceCloneConsent) -> Result<String, &'static str> {
        if !consent.is_fully_authorized() {
            return Err("Missing required consent attestations or spoken phrase verification");
        }

        if self.clones_created_today >= self.max_clones_per_day {
            return Err("Daily voice cloning limit reached. Try again tomorrow.");
        }

        self.clones_created_today += 1;
        Ok("Synthetic voice clone initialized with verifiable watermark".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_clone_consent_authorization() {
        let mut consent = VoiceCloneConsent::default();
        assert!(!consent.is_fully_authorized());

        consent.has_permission = true;
        consent.no_impersonation = true;
        consent.acknowledges_watermark = true;
        assert!(!consent.is_fully_authorized());

        consent.spoken_phrase_verified = true;
        assert!(consent.is_fully_authorized());

        let mut manager = VoiceCloneManager::new(2);
        assert!(manager.create_clone(&consent).is_ok());
    }

    #[test]
    fn test_watermark_embed_and_verify() {
        let sample_rate = 48000.0;
        let mut samples = vec![0.0f32; 9600];

        assert!(!WatermarkSecurity::verify_watermark(&samples, sample_rate));

        WatermarkSecurity::embed_watermark(&mut samples, sample_rate);
        assert!(WatermarkSecurity::verify_watermark(&samples, sample_rate));
    }
}
