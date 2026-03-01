pub mod config;

pub use pot_o_core::*;
pub use ai3_lib::*;
pub use pot_o_mining::*;
pub use pot_o_extensions::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_PORT: u16 = 8900;
