// MODE: DEV
// PACKAGE: PROD
//! Shared IRC-grammar message model for the chat server and client.

pub mod message;

pub use message::{numeric, numerics, fetch_end, Message, ParseError, FETCH_END};
