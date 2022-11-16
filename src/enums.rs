use std::fmt::Display;

use crate::errors::BitcoinMessageError;
use bitflags::bitflags;

bitflags! {
    pub struct ServiceIdentifier: u64 {
        const UNNAMED = 0x00;
        const NODE_NETWORK = 0x01;
        const NODE_GETUTXO = 0x02;
        const NODE_BLOOM = 0x04;
        const NODE_WITNESS = 0x08;
        const NODE_XTHIN = 0x10;
        const NODE_NETWORK_LIMITED = 0x0400;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Version,
    VerAck,
}

impl Command {
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
