//! PoT-O Validator library: re-exports of core, ai3-lib, mining, and extensions, plus config.
//!
//! Use this crate when building tooling or tests that need the same types as the validator binary.

pub mod config;

pub use ai3_lib::*;
pub use pot_o_core::*;
pub use pot_o_extensions::*;
pub use pot_o_mining::*;

/// Crate version (from Cargo.toml).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default HTTP listen port for the validator API.
pub const DEFAULT_PORT: u16 = 8900;
