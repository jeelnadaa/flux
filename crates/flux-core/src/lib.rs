//! Flux engine: decoding, DSP, rendering, analysis, export, devices, storage and the
//! JSON command API shared by the desktop UI, the MCP server and the tests.

pub mod analysis;
pub mod api;
pub mod device;
pub mod dsp;
pub mod export;
pub mod fx;
pub mod jobs;
pub mod live;
pub mod live_presets;
pub mod media;
pub mod methods;
pub mod project;
pub mod render;
pub mod server;
pub mod smart;
pub mod store;
pub mod tools;
pub mod voiceclone;
pub mod voicemodels;
