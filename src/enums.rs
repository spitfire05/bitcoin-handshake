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
