// MODE: DEV
// PACKAGE: PROD
//! Core library for the agent-oriented local editor.

pub const PROTOCOL_VERSION: u32 = 1;

pub mod auth;
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
pub mod transport;
