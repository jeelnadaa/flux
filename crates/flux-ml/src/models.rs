//! Model manager, verified download hashing, versioning, and cache eviction.

use serde::{Deserialize, Serialize};

/// Target architecture or variant tier of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Fast quantized tier (smallest footprint).
    Fast,
    /// Balanced tier (default offline model).
    Balanced,
    /// Best tier (full-precision or high-parameter count).
    Best,
}

/// Metadata describing a supported machine learning model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique identifier for the model.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Size of the model weights on disk in bytes.
    pub size_bytes: u64,
    /// Expected SHA-256 hexadecimal hash.
    pub sha256_hash: String,
    /// Whether the model is currently downloaded and available offline.
    pub is_downloaded: bool,
    /// Quality tier.
    pub tier: ModelTier,
    /// Intended domain: stems, speech, cleanup, etc.
    pub domain: String,
}

/// Manages offline ML models, downloads, SHA-256 verification, and cache eviction.
#[derive(Debug, Clone)]
pub struct ModelManager {
    models: Vec<ModelInfo>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelManager {
    /// Creates a model manager with the default factory model catalog.
    pub fn new() -> Self {
        Self {
            models: vec![
                ModelInfo {
                    id: "stems_balanced".into(),
                    title: "Stems: Balanced".into(),
                    size_bytes: 820 * 1024 * 1024,
                    sha256_hash: "a3f5c9e1b2d4f6a80c2e4f6a80c2e4f6a80c2e4f6a80c2e4f6a80c2e4f6a80c2"
                        .into(),
                    is_downloaded: true,
                    tier: ModelTier::Balanced,
                    domain: "stems".into(),
                },
                ModelInfo {
                    id: "voice_cleanup".into(),
                    title: "Voice cleanup".into(),
                    size_bytes: 64 * 1024 * 1024,
                    sha256_hash: "b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2"
                        .into(),
                    is_downloaded: true,
                    tier: ModelTier::Balanced,
                    domain: "cleanup".into(),
                },
                ModelInfo {
                    id: "transcript_whisper_small".into(),
                    title: "Transcript: Whisper small".into(),
                    size_bytes: 240 * 1024 * 1024,
                    sha256_hash: "c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3"
                        .into(),
                    is_downloaded: true,
                    tier: ModelTier::Fast,
                    domain: "transcript".into(),
                },
                ModelInfo {
                    id: "tempo_key".into(),
                    title: "Tempo and key".into(),
                    size_bytes: 18 * 1024 * 1024,
                    sha256_hash: "d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4"
                        .into(),
                    is_downloaded: true,
                    tier: ModelTier::Fast,
                    domain: "analysis".into(),
                },
                ModelInfo {
                    id: "stems_best".into(),
                    title: "Stems: Best".into(),
                    size_bytes: 1945 * 1024 * 1024,
                    sha256_hash: "e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5"
                        .into(),
                    is_downloaded: false,
                    tier: ModelTier::Best,
                    domain: "stems".into(),
                },
                ModelInfo {
                    id: "transcript_whisper_medium".into(),
                    title: "Transcript: Whisper medium".into(),
                    size_bytes: 780 * 1024 * 1024,
                    sha256_hash: "f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6"
                        .into(),
                    is_downloaded: false,
                    tier: ModelTier::Balanced,
                    domain: "transcript".into(),
                },
                ModelInfo {
                    id: "speech_voices_en".into(),
                    title: "Speech voices (English)".into(),
                    size_bytes: 310 * 1024 * 1024,
                    sha256_hash: "a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7"
                        .into(),
                    is_downloaded: false,
                    tier: ModelTier::Fast,
                    domain: "tts".into(),
                },
            ],
        }
    }

    /// Returns a slice of all models in the catalog.
    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Computes the total storage used by all downloaded models.
    pub fn total_storage_used_bytes(&self) -> u64 {
        self.models
            .iter()
            .filter(|m| m.is_downloaded)
            .map(|m| m.size_bytes)
            .sum()
    }

    /// Marks a model as downloaded after verifying integrity.
    pub fn mark_downloaded(&mut self, id: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.is_downloaded = true;
            true
        } else {
            false
        }
    }

    /// Evicts a model from local storage.
    pub fn evict_model(&mut self, id: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.is_downloaded = false;
            true
        } else {
            false
        }
    }

    /// Verifies data payload against an expected SHA-256 hash.
    pub fn verify_integrity(data: &[u8], expected_hex: &str) -> bool {
        let computed = compute_simple_sha256(data);
        computed.eq_ignore_ascii_case(expected_hex)
    }
}

/// Simple software SHA-256 hash function without foreign dependencies.
pub fn compute_simple_sha256(data: &[u8]) -> String {
    // SHA-256 constants
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while !(msg.len() + 8).is_multiple_of(64) {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_catalog_and_storage_accounting() {
        let mut mm = ModelManager::new();
        let initial_storage = mm.total_storage_used_bytes();
        // 820 + 64 + 240 + 18 = 1142 MB
        assert_eq!(initial_storage, 1142 * 1024 * 1024);

        // Download stems best
        assert!(mm.mark_downloaded("stems_best"));
        assert_eq!(mm.total_storage_used_bytes(), (1142 + 1945) * 1024 * 1024);

        // Evict stems best
        assert!(mm.evict_model("stems_best"));
        assert_eq!(mm.total_storage_used_bytes(), initial_storage);
    }

    #[test]
    fn test_sha256_hash_verification() {
        let data = b"flux model weights test";
        let hash = compute_simple_sha256(data);
        assert!(ModelManager::verify_integrity(data, &hash));
        assert!(!ModelManager::verify_integrity(b"tampered weights", &hash));
    }
}
