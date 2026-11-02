//! Sample-accurate seek index for variable bitrate and packet-based audio streams.

/// A single sync point entry in the seek index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekEntry {
    /// Exact audio frame index in the decoded stream.
    pub frame_index: u64,
    /// Byte offset in the underlying stream or container.
    pub byte_offset: u64,
    /// Presentation timestamp (PTS) or packet timestamp.
    pub packet_pts: u64,
}

/// A collection of seek entries mapping audio frames to stream byte positions.
#[derive(Debug, Clone, Default)]
pub struct SeekIndex {
    entries: Vec<SeekEntry>,
}

impl SeekIndex {
    /// Creates a new empty `SeekIndex`.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adds a seek point entry to the index.
    pub fn add_entry(&mut self, frame_index: u64, byte_offset: u64, packet_pts: u64) {
        if let Some(last) = self.entries.last() {
            if last.frame_index >= frame_index {
                return;
            }
        }
        self.entries.push(SeekEntry {
            frame_index,
            byte_offset,
            packet_pts,
        });
    }

    /// Returns the number of indexed seek points.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Finds the nearest sync entry at or before `target_frame` using binary search.
    pub fn find_sync_frame(&self, target_frame: u64) -> Option<&SeekEntry> {
        if self.entries.is_empty() {
            return None;
        }

        match self
            .entries
            .binary_search_by(|entry| entry.frame_index.cmp(&target_frame))
        {
            Ok(idx) => Some(&self.entries[idx]),
            Err(0) => None,
            Err(idx) => Some(&self.entries[idx - 1]),
        }
    }

    /// Returns a slice of all indexed entries.
    pub fn entries(&self) -> &[SeekEntry] {
        &self.entries
    }
}
