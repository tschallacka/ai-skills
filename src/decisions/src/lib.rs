// MODE: DEV
// PACKAGE: PROD
//! The decisions register's tools as a library, so a sibling crate (the MCP
//! adapter) can depend on it by path instead of duplicating register logic.

pub mod migrate;
pub mod mutate;
pub mod query;
pub mod register;

pub use migrate::SUPPORTED;
pub use mutate::{add, answer, close, stub, NewQuestion};
pub use query::{list, Filter};
pub use register::{Choice, Priority, Question, Register, Status};
