//! Payment processing layers, exposed independently for testing.
//!
//! This is a structure-only scaffold: every method is deliberately unimplemented.
//! The CLI will be connected to these layers when their behavior is implemented.

#![allow(
    dead_code,
    clippy::unimplemented,
    clippy::missing_panics_doc,
    reason = "Temporary allowances for structure-only method placeholders and unread fields"
)]

pub mod application;
pub mod csv_io;
pub mod domain;
pub mod infrastructure;
