use sha2::{Digest, Sha256};

const CHECKSUM_SIZE: usize = 4;

/// Computes Bitcoin checksum for given data
pub fn checksum(data: &[u8]) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut hasher = Sha256::new();
    hasher.update(hash);
    let hash = hasher.finalize();

    let mut buf = [0u8; CHECKSUM_SIZE];
    buf.clone_from_slice(&hash[..CHECKSUM_SIZE]);

    buf
}

#[cfg(test)]
mod tests {
    use quickcheck_macros::quickcheck;

    use super::*;

    #[quickcheck]
    fn checksum_fuzz(data: Vec<u8>) {
        let _ = checksum(&data);
    }

    #[test]
    fn checksum_of_empty_data() {
        let data = vec![];
        assert_eq!(checksum(&data), [0x5d, 0xf6, 0xe0, 0xe2]);
    }
}
