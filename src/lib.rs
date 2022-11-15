pub mod enums;
pub mod errors;
pub mod message;

use sha2::{Digest, Sha256};

const CHECKSUM_SIZE: usize = 4;

/// Computes Bitcoin checksum for gived data
pub fn checksum(data: &[u8]) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.update(data);
    let hash = hasher.finalize();

    let mut buf = [0u8; CHECKSUM_SIZE];
    buf.clone_from_slice(&hash[..CHECKSUM_SIZE]);

    buf
}

pub trait BitcoinSerialize {
    fn to_bytes(&self) -> Result<Vec<u8>, errors::BitcoinMessageError>;
    fn from_bytes() -> Self;
}
