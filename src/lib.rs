//! `bitcoin-handshake` is a stub crate for communicating with bitcoin network. It defines [Bitcoin protocol](https://developer.bitcoin.org/reference/p2p_networking.html#p2p-network) messages as Rust data structures.

#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

/// Enumarations defining specific status and flags
pub mod enums;

/// Specific errors used by this crate.
pub mod errors;

/// Bitcoin protocol message implementation stub
pub mod message;

mod utils;

/// Protocol version implemented by this crate
pub const PROTOCOL_VERSION: i32 = 70015;

/// The port of Bitcoin's mainnet
pub const PORT_MAINNET: u16 = 8333;

pub use enums::*;
pub use errors::*;
pub use message::*;
