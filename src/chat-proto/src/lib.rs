// MODE: DEV
// PACKAGE: PROD
//! Shared IRC-grammar message model for the chat server and client.

pub mod message;

pub use message::{fetch_end, numeric, numerics, Message, ParseError, FETCH_END};
