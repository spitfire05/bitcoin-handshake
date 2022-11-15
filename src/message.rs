use crate::{
    enums::ServiceIdentifier,
    errors::BitcoinMessageError,
    utils::{checksum, CHECKSUM_SIZE},
    PROTOCOL_VERSION,
};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use getset::Getters;
use std::{
    io::{Read, Write},
    net::{Ipv6Addr, SocketAddr},
};

/// `start_string` bytes for mainnnet
pub const START_STRING_MAINNET: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];

/// Max payload size, as per Bitcoin protocol docs
const MAX_SIZE: usize = 32 * 1024 * 1024;

pub const COMMAND_NAME_SIZE: usize = 12;

/// Trait defining a data structure that can be serialized to bitcoin protocol "wire" data without any outside input.
pub trait BitcoinSerialize {
    /// Performs the serialization.
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError>;
}

pub trait BitcoinDeserialize {
    /// Constructs `Self` from binary data.
    fn from_bytes(data: &mut impl Read) -> Result<Self, BitcoinMessageError>
    where
        Self: std::marker::Sized;
}

/// Defines a Bitcoin protocol message.
#[derive(Getters, Debug, Clone)]
pub struct Message {
    /// Magic bytes indicating the originating network; used to seek to next message when stream state is unknown.
    #[getset(get = "pub")]
    start_string: [u8; 4],

    /// ASCII string which identifies what message type is contained in the payload.
    #[getset(get = "pub")]
    command_name: String,

    /// The payload of this message.
    #[getset(get = "pub")]
    payload: Payload,
}

impl Message {
    /// Creates new [`Message`]. The `command_name` parameter will be checked for being ASCII string up to [`COMMAND_NAME_SIZE`] bytes.
    ///
    /// This method can return [`BitcoinMessageError::CommandNameTooLong`] if `command_name` is longer than [`COMMAND_NAME_SIZE`].
    /// [`BitcoinMessageError::CommandNameNonAscii`] will be returned if there are non-ASCII characters inside `command_name`.
    pub fn new(
        start_string: [u8; 4],
        command_name: impl Into<String>,
        payload: Payload,
    ) -> Result<Self, BitcoinMessageError> {
        let command_name: String = command_name.into();
        if command_name.len() > COMMAND_NAME_SIZE {
            return Err(BitcoinMessageError::CommandNameTooLong);
        }
        if !command_name.is_ascii() {
            return Err(BitcoinMessageError::CommandNameNonAscii);
        }

        Ok(Self {
            start_string,
            command_name,
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
        buf.write_all(&payload)?;

        Ok(buf)
    }
}

impl BitcoinDeserialize for Message {
    fn from_bytes(data: &mut impl Read) -> Result<Self, BitcoinMessageError>
    where
        Self: std::marker::Sized,
    {
        let mut start_string = [0u8; 4];
        data.read_exact(&mut start_string)?;
        let mut command_name_bytes = vec![0u8; COMMAND_NAME_SIZE];
        data.read_exact(&mut command_name_bytes)?;
        let command_name = String::from_utf8(command_name_bytes)?;
        let command_name = command_name.replace('\0', "");
        let payload_len = data.read_u32::<LittleEndian>()? as usize;
        if payload_len > MAX_SIZE {
            return Err(BitcoinMessageError::PayloadTooBig);
        }
        let mut checksum = vec![0u8; CHECKSUM_SIZE];
        data.read_exact(&mut checksum)?;
        // TODO: verify checksum
        let payload = Payload::from_bytes(data, &command_name)?;

        Ok(Self {
            start_string,
            command_name,
            payload,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Payload {
    Empty,
    Version(VersionData),
}

impl Payload {
    // special case as it needs to know the command name
    pub fn from_bytes(
        data: &mut impl Read,
        command_name: &(impl AsRef<str> + ?Sized),
    ) -> Result<Self, BitcoinMessageError> {
        let command_name = command_name.as_ref();
        match command_name {
            "version" => Ok(Payload::Version(VersionData::from_bytes(data)?)),
            _ => Err(BitcoinMessageError::CommandNameUnknown(
                command_name.to_string(),
            )),
        }
    }
}

impl BitcoinSerialize for Payload {
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError> {
        let data = match self {
            Payload::Empty => Ok(vec![]),
            Payload::Version(data) => data.to_bytes(),
        };
        if let Ok(ref d) = data {
            if d.len() > MAX_SIZE {
                return Err(BitcoinMessageError::PayloadTooBig);
            }
        }

        data
    }
}

#[derive(Getters, Debug, Clone)]
pub struct VersionData {
    #[getset(get = "pub")]
    version: i32,

    #[getset(get = "pub")]
    services: ServiceIdentifier,

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
        services: ServiceIdentifier,
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
            version: PROTOCOL_VERSION, // any data created by this lib, not deserialized from wire, will always be PROTCOL_VERSION
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
        buf.write_u64::<LittleEndian>(self.services.bits())?;
        buf.write_i64::<LittleEndian>(self.timestamp)?;
        buf.write_u64::<LittleEndian>(self.addr_recv_services.bits())?;
        buf.write_u128::<BigEndian>(u128::from_ne_bytes(
            match self.addr_recv_socket_address.ip() {
                std::net::IpAddr::V4(x) => x.to_ipv6_mapped(),
                std::net::IpAddr::V6(x) => x,
            }
            .octets(),
        ))?;
        buf.write_u16::<BigEndian>(self.addr_recv_socket_address.port())?;
        buf.write_u64::<LittleEndian>(self.addr_trans_services.bits())?;
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
}

impl BitcoinDeserialize for VersionData {
    fn from_bytes(data: &mut impl Read) -> Result<Self, BitcoinMessageError>
    where
        Self: std::marker::Sized,
    {
        let version = data.read_i32::<LittleEndian>()?;
        log::trace!("Deserialing version `{}`", version);
        let services = ServiceIdentifier::from_bits_truncate(data.read_u64::<LittleEndian>()?);
        let timestamp = data.read_i64::<LittleEndian>()?;
        let addr_recv_services =
            ServiceIdentifier::from_bits_truncate(data.read_u64::<LittleEndian>()?);
        let recv_ip: Ipv6Addr = data.read_u128::<BigEndian>()?.into();
        let recv_port = data.read_u16::<BigEndian>()?;
        let addr_recv_socket_address: SocketAddr = (recv_ip, recv_port).into();
        let addr_trans_services =
            ServiceIdentifier::from_bits_truncate(data.read_u64::<LittleEndian>()?);
        let trans_ip: Ipv6Addr = data.read_u128::<BigEndian>()?.into();
        let trans_port = data.read_u16::<BigEndian>()?;
        let addr_trans_socket_address: SocketAddr = (trans_ip, trans_port).into();
        let nonce = data.read_u64::<LittleEndian>()?;
        let user_agent_len = data.read_u8()?;
        let mut user_agent_bytes = vec![0u8; user_agent_len as usize];
        data.read_exact(&mut user_agent_bytes)?;
        let user_agent = String::from_utf8(user_agent_bytes)?;
        let start_height = data.read_i32::<LittleEndian>()?;
        let relay: bool = data.read_u8()? != 0x00;

        Ok(Self {
            version,
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
        })
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
}
