//! Audio capture, input metering, pre-roll buffer, and crash-safe recording.

pub mod chunked_writer;
pub mod meter;
pub mod preroll;

pub use chunked_writer::{ChunkHeader, ChunkedRecordWriter};
pub use meter::{ChannelMeter, InputLevelMeter};
pub use preroll::PreRollBuffer;
