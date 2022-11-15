/// Enumarations defining specific status and flags
pub mod enums;

/// Specific errors used by this crate.
pub mod errors;

/// Bitcoin protocol message implementation stub
pub mod message;

mod utils;

/// Protocol version implemented by this crate
pub const PROTOCOL_VERSION: i32 = 70015;
