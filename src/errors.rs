use thiserror::Error;

#[derive(Error, Debug)]
pub enum BitcoinMessageError {
    #[error("command name too long")]
    CommandNameTooLong,

    #[error("start_string too short")]
    StartStringTooShort,

    #[error("command name has to be ASCII string")]
    CommandNameNonAscii,

    #[error("unknown service identifier: {0}")]
    ServiceIdentifierUnknown(u64),

    #[error("IO Error during (de)serialization: {0}")]
    SerializationError(#[from] std::io::Error),
}
