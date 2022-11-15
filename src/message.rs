use crate::{checksum, enums::ServiceIdentifier, errors::BitcoinMessageError, BitcoinSerialize};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use getset::Getters;
use std::{io::Write, net::SocketAddr};

/// Max payload size, as per Bitcoin protocol docs
const MAX_SIZE: usize = 32 * 1024 * 1024;

const COMMAND_NAME_SIZE: usize = 12;

pub const START_STRING_MAINNET: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];

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

impl BitcoinSerialize for Message {
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError> {
        let mut buf = Vec::with_capacity(24);
        buf.write_all(&self.start_string)?;
        write!(&mut buf, "{}", self.command_name)?;
        for _ in 0..(COMMAND_NAME_SIZE - self.command_name.len()) {
            buf.write_u8(0x00)?;
        }
        let payload = self.payload.to_bytes()?;
        buf.write_u32::<LittleEndian>(payload.len() as u32)?;
        buf.write_all(&checksum(&payload))?;

        Ok(buf)
    }

    fn from_bytes() -> Self {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum Payload {
    Empty,
    Version(VersionData),
}

impl BitcoinSerialize for Payload {
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError> {
        match self {
            Payload::Empty => Ok(vec![]),
            Payload::Version(data) => data.to_bytes(),
        }
    }

    fn from_bytes() -> Self {
        todo!()
    }
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
    addr_recv_services: ServiceIdentifier,

    #[getset(get = "pub")]
    addr_recv_socket_address: SocketAddr,

    #[getset(get = "pub")]
    addr_trans_services: ServiceIdentifier,

    #[getset(get = "pub")]
    addr_trans_socket_address: SocketAddr,

    #[getset(get = "pub")]
    nonce: u64,

    #[getset(get = "pub")]
    user_agent: String,

    #[getset(get = "pub")]
    start_height: i32,

    #[getset(get = "pub")]
    relay: bool,
}

impl VersionData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        services: u64,
        timestamp: i64,
        addr_recv_services: ServiceIdentifier,
        addr_recv_socket_address: SocketAddr,
        addr_trans_services: ServiceIdentifier,
        addr_trans_socket_address: SocketAddr,
        nonce: u64,
        user_agent: String,
        start_height: i32,
        relay: bool,
    ) -> Self {
        Self {
            version: 70015, // This lib implements only version 70015
            services,
            timestamp,
            addr_recv_services,
            addr_recv_socket_address,
            addr_trans_services,
            addr_trans_socket_address,
            nonce,
            user_agent,
            start_height,
            relay,
        }
    }
}

impl BitcoinSerialize for VersionData {
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError> {
        let mut buf = Vec::new(); // TODO: Estimate capacity?
        buf.write_i32::<LittleEndian>(self.version)?;
        buf.write_u64::<LittleEndian>(self.services)?;
        buf.write_i64::<LittleEndian>(self.timestamp)?;
        buf.write_u64::<LittleEndian>(self.addr_recv_services as u64)?;
        buf.write_u128::<BigEndian>(u128::from_ne_bytes(
            match self.addr_recv_socket_address.ip() {
                std::net::IpAddr::V4(x) => x.to_ipv6_mapped(),
                std::net::IpAddr::V6(x) => x,
            }
            .octets(),
        ))?;
        buf.write_u16::<BigEndian>(self.addr_recv_socket_address.port())?;
        buf.write_u64::<LittleEndian>(self.addr_trans_services as u64)?;
        buf.write_u128::<BigEndian>(u128::from_ne_bytes(
            match self.addr_trans_socket_address.ip() {
                std::net::IpAddr::V4(x) => x.to_ipv6_mapped(),
                std::net::IpAddr::V6(x) => x,
            }
            .octets(),
        ))?;
        buf.write_u16::<BigEndian>(self.addr_trans_socket_address.port())?;
        buf.write_u64::<LittleEndian>(self.nonce)?;
        buf.write_u8(0x00)?; // TODO: implement user_agent
        buf.write_i32::<LittleEndian>(self.start_height)?;
        buf.write_u8(self.relay.into())?;

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
