use std::fmt::Display;

use crate::errors::BitcoinMessageError;
use bitflags::bitflags;

bitflags! {
    /// Service identifier flags. See [bitcoin docs](https://developer.bitcoin.org/reference/p2p_networking.html#version).
    pub struct ServiceIdentifier: u64 {
        /// This node is not a full node. It may not be able to provide any data except for the transactions it originates.
        const UNNAMED = 0x00;

        /// This is a full node and can be asked for full blocks. It should implement all protocol features available in its self-reported protocol version.
        const NODE_NETWORK = 0x01;

        /// This is a full node capable of responding to the getutxo protocol request. This is not supported by any currently-maintained Bitcoin node.
        const NODE_GETUTXO = 0x02;

        /// This is a full node capable and willing to handle bloom-filtered connections.
        const NODE_BLOOM = 0x04;

        /// This is a full node that can be asked for blocks and transactions including witness data.
        const NODE_WITNESS = 0x08;

        /// This is a full node that supports Xtreme Thinblocks. This is not supported by any currently-maintained Bitcoin node.
        const NODE_XTHIN = 0x10;

        /// This is the same as NODE_NETWORK but the node has at least the last 288 blocks (last 2 days).
        const NODE_NETWORK_LIMITED = 0x0400;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Enum corresponding to the `command_name` from Message header.
pub enum Command {
    /// `version` command_name
    Version,

    /// `verack command_name
    VerAck,
}

impl Command {
    /// Converts [`Command`] into byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Command::Version => "version",
            Command::VerAck => "verack",
        };

        write!(f, "{}", s)
    }
}

impl TryFrom<&str> for Command {
    type Error = BitcoinMessageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "version" => Ok(Command::Version),
            "verack" => Ok(Command::VerAck),
            x => Err(BitcoinMessageError::CommandNameUnknown(x.to_string())),
        }
    }
}

impl From<Command> for String {
    fn from(c: Command) -> Self {
        c.to_string()
    }
}

impl From<Command> for Vec<u8> {
    fn from(c: Command) -> Self {
        c.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::Arbitrary;

    use super::*;

    impl Arbitrary for ServiceIdentifier {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self::from_bits_truncate(u64::arbitrary(g))
        }
    }

    impl Arbitrary for Command {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            *g.choose(&[Command::Version, Command::VerAck]).unwrap()
        }
    }

    #[test]
    fn command_as_string() {
        assert_eq!(Command::Version.to_string(), "version");
        assert_eq!(Command::VerAck.to_string(), "verack");
    }

    #[test]
    fn string_as_command() {
        assert_eq!(Command::try_from("version").unwrap(), Command::Version);
        assert_eq!(Command::try_from("verack").unwrap(), Command::VerAck);
    }

    #[test]
    fn command_as_bytes() {
        assert_eq!(Command::Version.to_bytes(), b"version");
        assert_eq!(Command::VerAck.to_bytes(), b"verack");
    }
}
