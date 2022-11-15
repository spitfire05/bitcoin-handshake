use crate::errors::BitcoinMessageError;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use paste::paste;

macro_rules! impl_try_from {
    ($name:ident, $type:ty, $err_expr:expr) => {
        impl TryFrom<$type> for $name {
            type Error = BitcoinMessageError;

            fn try_from(value: $type) -> Result<Self, Self::Error> {
                paste! {
                    FromPrimitive::[<from_ $type>](value).ok_or($err_expr(value))
                }
            }
        }
    };
}

#[derive(Debug, Clone, Copy, FromPrimitive)]
pub enum ServiceIdentifier {
    Unnamed = 0x00,
    NodeNetwork = 0x01,
    NodeGetutxo = 0x02,
    NodeBloom = 0x04,
    NodeWitness = 0x08,
    NodeXthin = 0x10,
    NodeNetworkLimited = 0x0400,
}

impl_try_from!(
    ServiceIdentifier,
    u64,
    BitcoinMessageError::ServiceIdentifierUnknown
);
