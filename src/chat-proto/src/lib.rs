// MODE: DEV
// PACKAGE: PROD
//! Shared IRC-grammar message model for the chat server and client.

pub mod home_tag;
pub mod message;
pub mod spool;

pub use home_tag::home_tag;
pub use message::{fetch_end, numeric, numerics, Message, ParseError, Tag, FETCH_END};
