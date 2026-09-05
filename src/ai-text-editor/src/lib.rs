// MODE: DEV
// PACKAGE: PROD
//! Core library for the agent-oriented local editor.

pub const PROTOCOL_VERSION: u32 = 1;

/// The threshold above which a tab becomes a bounded large tab, used by the
/// server default, the client's cold-start `resources` answer, and the
/// published capabilities contract.
pub const DEFAULT_LARGE_THRESHOLD_BYTES: u64 = 256 * 1024 * 1024;

pub mod auth;
pub mod client;
pub mod document;
pub mod history;
pub mod index;
pub mod jobs;
pub mod journal;
pub mod large_file;
pub mod metadata;
pub mod navigation;
pub mod protocol;
pub mod resources;
pub mod revision;
pub mod search;
pub mod session;
pub mod transport;
