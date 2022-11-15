use std::net::SocketAddrV6;

use crate::{errors::BitcoinMessageError, BitcoinSerialize};
use byteorder::{LittleEndian, WriteBytesExt};
use getset::Getters;

/// Max payload size, as per Bitcoin protocol docs
const MAX_SIZE: usize = 32 * 1024 * 1024;

const COMMAND_NAME_SIZE: usize = 12;

#[derive(Getters, Debug, Clone)]
pub struct Message {
    #[getset(get = "pub")]
    start_string: [u8; 4],

    #[getset(get = "pub")]
    command_name: String,

    #[getset(get = "pub")]
    payload: Payload,
}

impl Message {
    pub fn new<T: AsRef<[u8]>, S: AsRef<str> + ?Sized>(
        start_string: T,
        command_name: &S,
        payload: Payload,
    ) -> Result<Self, BitcoinMessageError> {
        let command_name = command_name.as_ref();
        if command_name.len() > COMMAND_NAME_SIZE {
            return Err(BitcoinMessageError::CommandNameTooLong);
        }
        if !command_name.is_ascii() {
            return Err(BitcoinMessageError::CommandNameNonAscii);
        }

        let start_string = start_string.as_ref();
        if start_string.len() < 4 {
            return Err(BitcoinMessageError::StartStringTooShort);
        }

        let mut buf = [0u8; 4];
        buf.copy_from_slice(&start_string[..4]);

        Ok(Self {
            start_string: buf,
            command_name: command_name.to_string(),
            payload,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Payload {
    Empty,
    Version(VersionData),
}

#[derive(Getters, Debug, Clone)]
pub struct VersionData {
    #[getset(get = "pub")]
    version: i32,

    #[getset(get = "pub")]
    services: u64,

    #[getset(get = "pub")]
    timestamp: i64,

    #[getset(get = "pub")]
    addr_recv_services: u64,

    #[getset(get = "pub")]
    addr_recv_socket_address: SocketAddrV6,

    #[getset(get = "pub")]
    addr_trans_services: u64,

    #[getset(get = "pub")]
    addr_trans_socket_address: SocketAddrV6,

    #[getset(get = "pub")]
    nonce: u64,

    #[getset(get = "pub")]
    user_agent: String,

    #[getset(get = "pub")]
    start_height: i32,

    #[getset(get = "pub")]
    relay: bool,
}

impl BitcoinSerialize for VersionData {
    fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = Vec::new(); // TODO: Estimate capacity?
        buf.write_i32::<LittleEndian>(self.version)?;
        buf.write_u64::<LittleEndian>(self.services)?;
        buf.write_i64::<LittleEndian>(self.timestamp)?;

        Ok(buf)
    }

    fn from_bytes() -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    const DUMMY_START_STRING: [u8; 4] = [0; 4];

    #[quickcheck]
    fn message_new_returns_err_on_too_long_command_name(name: String) -> TestResult {
        if !name.is_ascii() || name.len() <= COMMAND_NAME_SIZE {
            return TestResult::discard();
        }

        TestResult::from_bool(matches!(
            Message::new(DUMMY_START_STRING, &name, Payload::Empty),
            Err(BitcoinMessageError::CommandNameTooLong)
        ))
    }

    #[quickcheck]
    fn message_new_returns_err_on_non_ascii_command_name(name: String) -> TestResult {
        // size will be checked first
        if name.is_ascii() || name.len() > COMMAND_NAME_SIZE {
            return TestResult::discard();
        }

        TestResult::from_bool(matches!(
            Message::new(DUMMY_START_STRING, &name, Payload::Empty),
            Err(BitcoinMessageError::CommandNameNonAscii)
        ))
    }

    #[quickcheck]
    fn message_new_returns_err_on_too_short_start_string(data: Vec<u8>) -> TestResult {
        if data.len() >= 4 {
            return TestResult::discard();
        }

        TestResult::from_bool(matches!(
            Message::new(data, "", Payload::Empty),
            Err(BitcoinMessageError::StartStringTooShort)
        ))
    }
}
