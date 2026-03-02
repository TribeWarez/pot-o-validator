pub mod config;

pub use ai3_lib::*;
pub use pot_o_core::*;
pub use pot_o_extensions::*;
pub use pot_o_mining::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_PORT: u16 = 8900;
