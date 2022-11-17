use crate::{
    enums::{Command, ServiceIdentifier},
    errors::BitcoinMessageError,
    utils::{self, checksum, CHECKSUM_SIZE},
    PROTOCOL_VERSION,
};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use getset::Getters;
use std::{
    io::{Cursor, Read, Write},
    net::{Ipv6Addr, SocketAddr},
};

/// `start_string` bytes for mainnnet
pub const START_STRING_MAINNET: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];

/// Maximum `user_agent` length in [`VersionData`]
pub const MAX_USER_AGENT_LEN: usize = 256;

/// Max payload size, as per Bitcoin protocol docs
const MAX_SIZE: usize = 32 * 1024 * 1024;
const COMMAND_NAME_SIZE: usize = 12;

/// Trait defining a data structure that can be serialized to bitcoin protocol "wire" data without any outside input.
pub trait BitcoinSerialize {
    /// Performs the serialization.
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError>;
}

/// Trait defining a data structure that can be deserialized from bitcoin protocol "wire" data without any outside input.
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

    /// Identifies what message type is contained in the payload.
    #[getset(get = "pub")]
    command: Command,

    /// The payload of this message.
    #[getset(get = "pub")]
    payload: Payload,
}

impl Message {
    /// Creates new [`Message`].
    pub fn new(start_string: [u8; 4], command: Command, payload: Payload) -> Self {
        Self {
            start_string,
            command,
            payload,
        }
    }
}

impl BitcoinSerialize for Message {
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError> {
        let mut payload = self.payload.to_bytes()?;
        let payload_len = payload.len();
        let payload_checksum = checksum(&payload);
        let mut buf = Vec::with_capacity(24 + payload.len());
        buf.write_all(&self.start_string)?;
        let mut command_bytes = self.command.to_bytes();
        let command_bytes_len = command_bytes.len();
        buf.append(&mut command_bytes);
        for _ in 0..(COMMAND_NAME_SIZE - command_bytes_len) {
            buf.write_u8(0x00)?;
        }
        buf.write_u32::<LittleEndian>(payload_len as u32)?;
        buf.write_all(&payload_checksum)?;
        buf.append(&mut payload);

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
        let command: Command = command_name.as_str().try_into()?;
        let payload_len = data.read_u32::<LittleEndian>()? as usize;
        if payload_len > MAX_SIZE {
            return Err(BitcoinMessageError::PayloadTooBig);
        }
        let mut checksum = vec![0u8; CHECKSUM_SIZE];
        data.read_exact(&mut checksum)?;
        let mut payload_bytes = vec![0u8; payload_len];
        data.read_exact(&mut payload_bytes)?;
        if checksum != utils::checksum(&payload_bytes) {
            return Err(BitcoinMessageError::ChecksumMismatch);
        }
        let payload = Payload::from_bytes(&mut Cursor::new(payload_bytes), &command)?;

        Ok(Self {
            start_string,
            command,
            payload,
        })
    }
}

#[derive(Debug, Clone)]
/// Bitcoin's Message payload.
pub enum Payload {
    /// An empty payload.
    Empty,

    /// Payload of `version` command
    Version(VersionData),
}

impl Payload {
    // special case as it needs to know the command name
    /// Deserializes [`Payload`] from buffer of bytes.
    pub fn from_bytes(
        data: &mut impl Read,
        command: &Command,
    ) -> Result<Self, BitcoinMessageError> {
        match command {
            Command::Version => Ok(Payload::Version(VersionData::from_bytes(data)?)),
            Command::VerAck => Ok(Payload::Empty),
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
/// `version` message payload.
pub struct VersionData {
    /// The highest protocol version understood by the transmitting node.
    #[getset(get = "pub")]
    version: i32,

    /// The services supported by the transmitting node encoded as a bitfield. See the list of service codes below.
    #[getset(get = "pub")]
    services: ServiceIdentifier,

    /// The current Unix epoch time according to the transmitting node’s clock.
    #[getset(get = "pub")]
    timestamp: i64,

    /// The services supported by the receiving node as perceived by the transmitting node. Same format as the ‘services’ field above.
    #[getset(get = "pub")]
    addr_recv_services: ServiceIdentifier,

    /// The IPv6 address of the receiving node as perceived by the transmitting node.
    #[getset(get = "pub")]
    addr_recv_socket_address: SocketAddr,

    /// The services supported by the transmitting node. Should be identical to the ‘services’ field above.
    #[getset(get = "pub")]
    addr_trans_services: ServiceIdentifier,

    /// The IPv6 address of the transmitting node in big endian byte order.
    #[getset(get = "pub")]
    addr_trans_socket_address: SocketAddr,

    /// A random nonce which can help a node detect a connection to itself.
    #[getset(get = "pub")]
    nonce: u64,

    /// User agent as defined by BIP14.
    #[getset(get = "pub")]
    user_agent: String,

    /// The height of the transmitting node’s best block chain or, in the case of an SPV client, best block header chain.
    #[getset(get = "pub")]
    start_height: i32,

    /// Transaction relay flag.
    #[getset(get = "pub")]
    relay: bool,
}

impl VersionData {
    #[allow(clippy::too_many_arguments)]
    /// Creates new [`VersionData`].
    ///
    /// # Panics
    ///
    /// This method will panic if `user_agent.len()` is more than [`MAX_USER_AGENT_LEN`].
    pub fn new(
        services: ServiceIdentifier,
        timestamp: i64,
        addr_recv_services: ServiceIdentifier,
        addr_recv_socket_address: SocketAddr,
        addr_trans_services: ServiceIdentifier,
        addr_trans_socket_address: SocketAddr,
        user_agent: String,
        start_height: i32,
        relay: bool,
    ) -> Self {
        if user_agent.len() > MAX_USER_AGENT_LEN {
            panic!(
                "user_agent length has to be {} bytes max",
                MAX_USER_AGENT_LEN
            );
        }
        Self {
            version: PROTOCOL_VERSION, // any data created by this lib, not deserialized from wire, will always be PROTCOL_VERSION
            services,
            timestamp,
            addr_recv_services,
            addr_recv_socket_address,
            addr_trans_services,
            addr_trans_socket_address,
            nonce: rand::random(),
            user_agent,
            start_height,
            relay,
        }
    }
}

impl BitcoinSerialize for VersionData {
    fn to_bytes(&self) -> Result<Vec<u8>, BitcoinMessageError> {
        let mut buf = Vec::with_capacity(86 + self.user_agent().len());
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
        buf.write_u8(self.user_agent().len() as u8)?;
        buf.write_all(self.user_agent().as_bytes())?;
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
        tracing::trace!("Deserialing version `{}`", version);
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use quickcheck::{Arbitrary, TestResult};
    use quickcheck_macros::quickcheck;
    use std::{
        io::Cursor,
        net::{IpAddr, Ipv4Addr},
        time::SystemTime,
    };

    impl Arbitrary for VersionData {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let user_agent = loop {
                let gen = String::arbitrary(g);
                if gen.len() <= MAX_USER_AGENT_LEN {
                    break gen;
                }
            };
            let services = ServiceIdentifier::arbitrary(g);
            Self::new(
                services,
                i64::arbitrary(g),
                services,
                SocketAddr::arbitrary(g),
                services,
                SocketAddr::arbitrary(g),
                user_agent,
                i32::arbitrary(g),
                bool::arbitrary(g),
            )
        }
    }

    impl Arbitrary for Message {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let command = Command::arbitrary(g);
            let payload = match command {
                Command::Version => Payload::Version(VersionData::arbitrary(g)),
                Command::VerAck => Payload::Empty,
            };

            Self::new(
                [
                    u8::arbitrary(g),
                    u8::arbitrary(g),
                    u8::arbitrary(g),
                    u8::arbitrary(g),
                ],
                command,
                payload,
            )
        }
    }

    #[quickcheck]
    fn bytes_to_message_fuzz(data: Vec<u8>) {
        let mut c = Cursor::new(data);
        let _ = Message::from_bytes(&mut c);
    }

    #[quickcheck]
    fn message_to_bytes_fuzz(x: Message) {
        let _ = x.to_bytes().unwrap();
    }

    #[quickcheck]
    fn version_data_has_correct_protocol_version(x: VersionData) -> bool {
        *x.version() == PROTOCOL_VERSION
    }

    #[quickcheck]
    fn version_data_to_bytes_fuzz(x: VersionData) {
        let _ = x.to_bytes().unwrap();
    }

    #[test]
    fn deserialization_checks_checksum() {
        // varack with invalid checksum:
        let mut data = Cursor::new(hex!("f9beb4d976657261636b000000000000000000005df6e0e1"));

        let result = Message::from_bytes(&mut data);

        assert!(matches!(result, Err(BitcoinMessageError::ChecksumMismatch)));
    }

    #[test]
    fn verack_deserialization() {
        // varack:
        let mut data = Cursor::new(hex!("f9beb4d976657261636b000000000000000000005df6e0e2"));

        let result = Message::from_bytes(&mut data);

        assert!(matches!(result, Ok(_)));
    }

    #[quickcheck]
    fn empty_payload_has_correct_checksum(m: Message) -> TestResult {
        match m.payload() {
            Payload::Version(_) => TestResult::discard(),
            Payload::Empty => TestResult::from_bool(
                m.to_bytes()
                    .unwrap()
                    .iter()
                    .rev()
                    .take(4)
                    .rev()
                    .copied()
                    .collect::<Vec<_>>()
                    == hex!("5df6e0e2"),
            ),
        }
    }

    #[test]
    #[should_panic]
    fn version_data_new_with_user_agent_longer_than_max_length_panics() {
        let user_agent = (0..MAX_USER_AGENT_LEN + 1).map(|_| 'a').collect::<String>();
        let _ = VersionData::new(
            ServiceIdentifier::NODE_NETWORK,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            ServiceIdentifier::NODE_NETWORK,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            ServiceIdentifier::NODE_NETWORK,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            user_agent,
            0,
            false,
        );
    }
}
