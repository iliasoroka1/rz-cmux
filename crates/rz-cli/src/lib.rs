//! rz — inter-agent communication over cmux.
//!
//! Uses cmux's v2 JSON-RPC socket API for direct surface-to-surface
//! messaging. No files, no focus switching.

pub mod bootstrap;
pub mod cmux;
pub mod log;
pub mod status;

pub use rz_cmux_protocol::{Envelope, MessageKind, SENTINEL};
