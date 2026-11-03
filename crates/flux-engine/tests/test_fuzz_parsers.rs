//! Robustness and hostile-input fuzzing test for all audio decoders and metadata parsers (P1.5-c).

use flux_engine::decoder::{decode_buffer, open_decoder};
use std::io::Cursor;

#[test]
fn test_parsers_reject_empty_and_truncated_inputs() {
    let empty_bytes = b"";
    assert!(decode_buffer(empty_bytes).is_err());
    assert!(open_decoder(Cursor::new(empty_bytes.to_vec())).is_err());

    // Truncated headers
    for len in 1..=32 {
        let truncated = vec![0x42u8; len];
        let res = decode_buffer(&truncated);
        assert!(
            res.is_err(),
            "Truncated input of length {} should fail gracefully",
            len
        );
    }
}

#[test]
fn test_parsers_reject_corrupted_magic_headers() {
    let bad_magics: &[&[u8]] = &[
        b"RIFF\x00\x00\x00\x00NOTWAVE",
        b"RF64\x00\x00\x00\x00BADMAGIC",
        b"FORM\x00\x00\x00\x00NOTAIFF",
        b"fLaX\x00\x00\x00\x22\x00\x00",
        b"ID3\xFF\xFF\x00\x00\x00\x00\x00",
        b"OggX\x00\x02\x00\x00\x00\x00\x00",
        b"\xFF\x00\x00\x00\x00\x00\x00\x00",
    ];

    for bad in bad_magics {
        let res = decode_buffer(bad);
        assert!(
            res.is_err(),
            "Malformed magic should be rejected without panicking"
        );
    }
}

#[test]
fn test_parsers_reject_decompression_bombs_and_huge_lengths() {
    // 1. WAV claiming 4 GB data chunk but only offering 16 bytes
    let mut wav_bomb = Vec::new();
    wav_bomb.extend_from_slice(b"RIFF\xFF\xFF\xFF\x7FWAVEfmt ");
    wav_bomb.extend_from_slice(&16u32.to_le_bytes()); // fmt size
    wav_bomb.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav_bomb.extend_from_slice(&2u16.to_le_bytes()); // 2 channels
    wav_bomb.extend_from_slice(&48000u32.to_le_bytes()); // 48 kHz
    wav_bomb.extend_from_slice(&(48000u32 * 4).to_le_bytes());
    wav_bomb.extend_from_slice(&4u16.to_le_bytes());
    wav_bomb.extend_from_slice(&16u16.to_le_bytes());
    wav_bomb.extend_from_slice(b"data\xFF\xFF\xFF\x7F"); // 2 GB data claim
    wav_bomb.extend_from_slice(&[0u8; 16]); // only 16 bytes payload

    let res = decode_buffer(&wav_bomb);
    // Must reject or cleanly handle without allocating 2 GB RAM or hanging
    assert!(res.is_err() || res.unwrap().num_frames() <= 4);

    // 2. FLAC claiming 16 MB metadata block with only 4 bytes payload
    let mut flac_bomb = Vec::new();
    flac_bomb.extend_from_slice(b"fLaC");
    flac_bomb.push(0x00); // STREAMINFO
    flac_bomb.extend_from_slice(&[0xFF, 0xFF, 0xFF]); // 16 MB length
    flac_bomb.extend_from_slice(&[0u8; 8]);

    assert!(decode_buffer(&flac_bomb).is_err());

    // 3. Ogg Opus claiming huge packet length in segment table
    let mut ogg_bomb = Vec::new();
    ogg_bomb.extend_from_slice(b"OggS\x00\x02");
    ogg_bomb.extend_from_slice(&0u64.to_le_bytes()); // granule pos
    ogg_bomb.extend_from_slice(&1234u32.to_le_bytes()); // serial
    ogg_bomb.extend_from_slice(&0u32.to_le_bytes()); // seq
    ogg_bomb.extend_from_slice(&0u32.to_le_bytes()); // crc
    ogg_bomb.push(255); // 255 segments
    ogg_bomb.extend_from_slice(&[255u8; 255]); // ~65 KB claim without payload

    assert!(decode_buffer(&ogg_bomb).is_err());
}

#[test]
fn test_parsers_reject_invalid_audio_parameters() {
    // WAV with 0 channels
    let mut wav_0ch = Vec::new();
    wav_0ch.extend_from_slice(b"RIFF\x24\x00\x00\x00WAVEfmt ");
    wav_0ch.extend_from_slice(&16u32.to_le_bytes());
    wav_0ch.extend_from_slice(&1u16.to_le_bytes());
    wav_0ch.extend_from_slice(&0u16.to_le_bytes()); // 0 channels!
    wav_0ch.extend_from_slice(&48000u32.to_le_bytes());
    wav_0ch.extend_from_slice(&0u32.to_le_bytes());
    wav_0ch.extend_from_slice(&0u16.to_le_bytes());
    wav_0ch.extend_from_slice(&16u16.to_le_bytes());
    wav_0ch.extend_from_slice(b"data\x00\x00\x00\x00");

    assert!(decode_buffer(&wav_0ch).is_err());

    // WAV with 0 Hz sample rate
    let mut wav_0sr = wav_0ch;
    wav_0sr[22] = 1; // 1 channel
    wav_0sr[24] = 0; // 0 Hz sample rate
    wav_0sr[25] = 0;
    wav_0sr[26] = 0;
    wav_0sr[27] = 0;

    assert!(decode_buffer(&wav_0sr).is_err());
}

#[test]
fn test_seeded_pseudorandom_fuzz_stream_decoders() {
    // 500 deterministic pseudorandom hostile bitstreams
    let mut prng_state = 0xDEADBEEFCAFEBABEu64;

    for len in (16..1024).step_by(2) {
        let mut mutated = Vec::with_capacity(len);
        for _ in 0..len {
            prng_state ^= prng_state << 13;
            prng_state ^= prng_state >> 7;
            prng_state ^= prng_state << 17;
            mutated.push(prng_state as u8);
        }

        // Must fail cleanly without panics, hangs, or UB
        let _ = decode_buffer(&mutated);
        let _ = open_decoder(Cursor::new(mutated));
    }
}
