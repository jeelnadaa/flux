use flux_engine::capture::{ChunkedRecordWriter, InputLevelMeter, PreRollBuffer};
use flux_engine::decoder::{AudioDecoder, WavDecoder};
use std::io::Cursor;

#[test]
fn test_input_level_meter_accuracy_and_clipping() {
    let mut meter = InputLevelMeter::new(2, 48000);

    // Channel 0: 0.5 amplitude sine / constant (-6.02 dBFS)
    // Channel 1: 1.05 amplitude (clipped)
    let n = 480; // 10ms
    let ch0 = vec![0.5f32; n];
    let mut ch1 = vec![0.1f32; n];
    ch1[50] = 1.05; // single clipping sample

    let readings = meter.process(&[&ch0, &ch1]);
    assert_eq!(readings.len(), 2);

    // Channel 0 assertions
    let peak_0 = readings[0].peak_db;
    assert!(
        (peak_0 - (-6.0206)).abs() < 0.05,
        "Peak ch0 expected ~ -6.02 dBFS, got {peak_0}"
    );
    let rms_0 = readings[0].rms_db;
    assert!(
        (rms_0 - (-6.0206)).abs() < 0.05,
        "RMS ch0 expected ~ -6.02 dBFS, got {rms_0}"
    );
    assert!(!readings[0].clipped);
    assert!(!readings[0].clip_held);

    // Channel 1 assertions
    assert!(readings[1].clipped, "Channel 1 should register clipping");
    assert!(readings[1].clip_held, "Channel 1 should hold clip flag");
    assert!(readings[1].peak_db > 0.0, "Channel 1 peak > 0 dBFS");

    // Process silent block: clipping flag clears, but clip_held persists within hold duration
    let silence = vec![0.0f32; n];
    let readings_silent = meter.process(&[&silence, &silence]);
    assert!(
        !readings_silent[1].clipped,
        "Immediate clipping flag should clear"
    );
    assert!(
        readings_silent[1].clip_held,
        "Clip held should persist during hold window"
    );

    // Clear clips manually
    meter.clear_clips();
    let readings_cleared = meter.process(&[&silence, &silence]);
    assert!(
        !readings_cleared[1].clip_held,
        "Clip held should be cleared"
    );
}

#[test]
fn test_preroll_buffer_circular_ring_continuity() {
    // 0.5 second buffer at 1000 Hz = 500 frames capacity
    let mut preroll = PreRollBuffer::new(2, 1000, 0.5);
    assert_eq!(preroll.capacity(), 500);
    assert!(preroll.is_empty());

    // Push 300 frames: values 0..300
    let ch0_first: Vec<f32> = (0..300).map(|i| i as f32).collect();
    let ch1_first: Vec<f32> = (0..300).map(|i| -(i as f32)).collect();
    preroll.append(&[&ch0_first, &ch1_first]);
    assert_eq!(preroll.len(), 300);

    // Push another 400 frames: values 300..700 (total pushed = 700 > 500 capacity)
    // Oldest 200 frames (0..200) should be dropped; buffer should contain 200..700 (500 frames)
    let ch0_second: Vec<f32> = (300..700).map(|i| i as f32).collect();
    let ch1_second: Vec<f32> = (300..700).map(|i| -(i as f32)).collect();
    preroll.append(&[&ch0_second, &ch1_second]);
    assert_eq!(preroll.len(), 500);

    // Drain buffer and verify chronological integrity
    let drained = preroll.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].len(), 500);
    assert_eq!(drained[1].len(), 500);

    for (idx, &sample) in drained[0].iter().enumerate() {
        let expected = (200 + idx) as f32;
        assert_eq!(sample, expected, "Ch0 frame {idx} mismatch");
    }
    for (idx, &sample) in drained[1].iter().enumerate() {
        let expected = -((200 + idx) as f32);
        assert_eq!(sample, expected, "Ch1 frame {idx} mismatch");
    }

    // PreRollBuffer should now be empty
    assert!(preroll.is_empty());
}

#[test]
fn test_chunked_writer_crash_recovery_and_wav_conversion() {
    let mut writer = ChunkedRecordWriter::new(48000, 2);
    let mut stream_bytes = Vec::new();

    let frames_per_chunk = 480;
    let mut expected_audio = Vec::new();

    // Write 10 valid chunks
    for chunk_idx in 0..10 {
        let ch0: Vec<f32> = (0..frames_per_chunk)
            .map(|i| ((chunk_idx * frames_per_chunk + i) as f32 * 0.001).sin())
            .collect();
        let ch1: Vec<f32> = (0..frames_per_chunk)
            .map(|i| ((chunk_idx * frames_per_chunk + i) as f32 * 0.001).cos())
            .collect();

        expected_audio.push((ch0.clone(), ch1.clone()));
        writer
            .write_chunk(&mut stream_bytes, &[&ch0, &ch1])
            .unwrap();
    }

    let valid_stream_len = stream_bytes.len();

    // Simulate abrupt crash during chunk 11: write partial header and random corrupted bytes
    let ch0_partial = vec![0.42f32; frames_per_chunk];
    let ch1_partial = vec![0.84f32; frames_per_chunk];
    let mut temp = Vec::new();
    writer
        .write_chunk(&mut temp, &[&ch0_partial, &ch1_partial])
        .unwrap();
    // Append only half of chunk 11 (corrupt trailing mid-write)
    stream_bytes.extend_from_slice(&temp[..temp.len() / 2]);

    // Recover from stream: should gracefully recover all 10 chunks and discard the corrupt trailing chunk
    let mut cursor = Cursor::new(&stream_bytes);
    let recovered = ChunkedRecordWriter::recover_from_stream(&mut cursor).unwrap();

    assert_eq!(recovered.channels.len(), 2);
    assert_eq!(recovered.num_frames(), 10 * frames_per_chunk);
    assert_eq!(recovered.sample_rate, 48000);

    // Verify samples bit-exact
    for (chunk_idx, (exp_ch0, exp_ch1)) in expected_audio.iter().enumerate() {
        let offset = chunk_idx * frames_per_chunk;
        for i in 0..frames_per_chunk {
            assert_eq!(recovered.channels[0][offset + i], exp_ch0[i]);
            assert_eq!(recovered.channels[1][offset + i], exp_ch1[i]);
        }
    }

    // Convert recovered valid stream to standard WAV
    let mut valid_cursor = Cursor::new(&stream_bytes[..valid_stream_len]);
    let mut wav_bytes = Cursor::new(Vec::new());
    ChunkedRecordWriter::convert_chunks_to_wav(&mut valid_cursor, &mut wav_bytes).unwrap();

    // Decode WAV using standard WavDecoder and verify bit-exact
    let wav_data = wav_bytes.into_inner();
    let mut wav_dec = WavDecoder::new(Cursor::new(wav_data)).unwrap();
    let decoded = wav_dec.decode_all().unwrap();

    assert_eq!(decoded.channels.len(), 2);
    assert_eq!(decoded.num_frames(), 10 * frames_per_chunk);
    for ch in 0..2 {
        for frame in 0..decoded.num_frames() {
            assert_eq!(decoded.channels[ch][frame], recovered.channels[ch][frame]);
        }
    }
}

#[test]
fn test_recording_loopback_continuity_across_chunks() {
    let mut writer = ChunkedRecordWriter::new(48000, 1);
    let mut stream_bytes = Vec::new();

    let chunk_size = 256;
    let total_chunks = 20;
    let total_frames = chunk_size * total_chunks;

    // Continuous 1000 Hz sine wave across chunks
    let mut full_signal = Vec::with_capacity(total_frames);
    for i in 0..total_frames {
        let t = i as f64 / 48000.0;
        let s = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() as f32;
        full_signal.push(s);
    }

    for chunk_idx in 0..total_chunks {
        let start = chunk_idx * chunk_size;
        let slice = &full_signal[start..start + chunk_size];
        writer.write_chunk(&mut stream_bytes, &[slice]).unwrap();
    }

    let mut cursor = Cursor::new(stream_bytes);
    let recovered = ChunkedRecordWriter::recover_from_stream(&mut cursor).unwrap();

    assert_eq!(recovered.num_frames(), total_frames);
    for (i, &expected) in full_signal.iter().enumerate().take(total_frames) {
        assert_eq!(
            recovered.channels[0][i], expected,
            "Discontinuity at frame {i}"
        );
    }
}
