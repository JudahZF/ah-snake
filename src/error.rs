/// Errors from the decoders and constructors in this crate.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The input is shorter than the layout needs.
    #[error("too short: need {need} bytes, got {got}")]
    TooShort {
        /// Bytes the layout needs.
        need: usize,
        /// Bytes given.
        got: usize,
    },
    /// A message is longer than the 25 bytes `SendTSnakeMessage` sends.
    #[error("message length {0} is more than 25")]
    TooLong(usize),
    /// Byte 0 of a message (length - 1) does not match the input length.
    #[error("length byte says {stated} bytes, input has {actual}")]
    LengthMismatch {
        /// `byte0 + 1`.
        stated: usize,
        /// Input length.
        actual: usize,
    },
    /// The message code is not in the message-name table.
    #[error("unknown message code {0:#04x}")]
    UnknownCode(u8),
    /// The code is known, but this crate has no typed body for it. Use the raw [`crate::Message`].
    #[error("no typed body for message code {0:#04x}")]
    UntypedCode(u8),
    /// A fixed byte of a typed body has another value.
    #[error("byte {offset} should be {expected:#04x}, is {actual:#04x}")]
    BadConstant {
        /// Offset in the payload.
        offset: usize,
        /// Value the stock firmware writes.
        expected: u8,
        /// Value found.
        actual: u8,
    },
    /// A value does not fit its field.
    #[error("{0} out of range")]
    OutOfRange(&'static str),
}

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, Error>;
