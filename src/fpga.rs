//! Host-to-FPGA commands. The host `fwrite`s these to the FPGA device file of a
//! `cUSBManager` driver and `fread`s the reply. The same framing is used by the tSnake
//! driver (`TSnakeDriverBase.cpp`), `cGigaACEBase` and `cPortBPlusGigaACE`.
//!
//! Command layout (every writer in dLive Surface V2.12 and MixRack V2.12):
//!
//! | Byte | Field |
//! |---|---|
//! | 0..2 | total length, big-endian (`ReadTSnakeMessage` writes byte 0 = `len >> 8`) |
//! | 2 | `0x02` |
//! | 3..7 | `total - 10`; bytes 3..6 are 0 in every writer, so inferred big-endian u32 |
//! | 7 | FPGA / port index (`cTSnakeDriverBase` field `0x4e8`) |
//! | 8 | [`Endpoint`] |
//! | 9 | `0x00` |
//! | 10.. | payload, see [`Access`] for the register FPGAs |
//!
//! The reply to a read is 4 bytes, then the data. The meaning of the 4 bytes is not
//! known, except that a [`Access::ReadByte`] reply carries the value in byte 3.

use crate::error::{Error, Result};
use crate::message::Message;

/// Bytes 0..10.
pub const HEADER_LEN: usize = 10;
/// Bytes before the data in a read reply.
pub const REPLY_PREFIX_LEN: usize = 4;

/// Byte 8 of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Endpoint {
    /// `0x1d`: every write without a reply (block writes, register writes, SPI WREN/WRDI/WRITE).
    Write = 0x1d,
    /// `0x1c`: every command whose reply the host reads.
    Read = 0x1c,
    /// `0x15`: only `cPortBPlusGigaACE::ProgRegSPITransaction` (`08397c20`). Meaning not known.
    ProgReg = 0x15,
}

/// Frames a payload. Fails if the command would be longer than `0xffff` bytes.
pub fn command(port: u8, endpoint: Endpoint, payload: &[u8]) -> Result<Vec<u8>> {
    let total = HEADER_LEN + payload.len();
    let len = u16::try_from(total).map_err(|_| Error::OutOfRange("command length"))?;
    let rest = u32::from(len) - HEADER_LEN as u32;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&len.to_be_bytes());
    out.push(0x02);
    out.extend_from_slice(&rest.to_be_bytes());
    out.extend_from_slice(&[port, endpoint as u8, 0x00]);
    out.extend_from_slice(payload);
    Ok(out)
}

/// Payload of a command to a register FPGA (tSnake, gigaACE): byte 10 is the mode.
/// Addresses are little-endian.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// Mode 0: `[0, addr lo, addr hi, 0]`. Only `CheckForNewMessage` (`083d6680`).
    ReadByte {
        /// Register address.
        addr: u16,
    },
    /// Mode 1: `[1, addr lo, addr hi, value]` (`ResetTSnakeRx`, `cGigaACEBase::SetTxFormat`, ...).
    WriteByte {
        /// Register address.
        addr: u16,
        /// Value.
        value: u8,
    },
    /// Mode 2: `[2, len - 1, addr lo, addr hi]` then `len` zero bytes.
    ReadBlock {
        /// First address.
        addr: u16,
        /// Bytes to read, 1..=256.
        len: u16,
    },
    /// Mode 3: `[3, len - 1, addr lo, addr hi]` then the data (1..=256 bytes).
    WriteBlock {
        /// First address.
        addr: u16,
        /// Data.
        data: Vec<u8>,
    },
}

impl Access {
    /// Endpoint the stock firmware uses for this access.
    pub const fn endpoint(&self) -> Endpoint {
        match self {
            Self::ReadByte { .. } | Self::ReadBlock { .. } => Endpoint::Read,
            Self::WriteByte { .. } | Self::WriteBlock { .. } => Endpoint::Write,
        }
    }

    /// Length of the reply the host reads, or `None` for writes.
    pub fn reply_len(&self) -> Option<usize> {
        match self {
            Self::ReadByte { .. } => Some(REPLY_PREFIX_LEN),
            Self::ReadBlock { len, .. } => Some(REPLY_PREFIX_LEN + usize::from(*len)),
            Self::WriteByte { .. } | Self::WriteBlock { .. } => None,
        }
    }

    /// The full command for FPGA `port`.
    pub fn encode(&self, port: u8) -> Result<Vec<u8>> {
        let count_minus_one = |n: usize| match n {
            1..=256 => Ok((n - 1) as u8),
            _ => Err(Error::OutOfRange("block length")),
        };
        let payload = match self {
            Self::ReadByte { addr } => {
                let a = addr.to_le_bytes();
                vec![0, a[0], a[1], 0]
            }
            Self::WriteByte { addr, value } => {
                let a = addr.to_le_bytes();
                vec![1, a[0], a[1], *value]
            }
            Self::ReadBlock { addr, len } => {
                let n = usize::from(*len);
                let a = addr.to_le_bytes();
                let mut p = vec![2, count_minus_one(n)?, a[0], a[1]];
                p.resize(4 + n, 0);
                p
            }
            Self::WriteBlock { addr, data } => {
                let a = addr.to_le_bytes();
                let mut p = vec![3, count_minus_one(data.len())?, a[0], a[1]];
                p.extend_from_slice(data);
                p
            }
        };
        command(port, self.endpoint(), &payload)
    }
}

/// Value of a [`Access::ReadByte`] reply.
pub fn read_byte_reply(reply: &[u8]) -> Result<u8> {
    reply.get(3).copied().ok_or(Error::TooShort {
        need: REPLY_PREFIX_LEN,
        got: reply.len(),
    })
}

/// Data of a [`Access::ReadBlock`] reply of `len` bytes.
pub fn read_block_reply(reply: &[u8], len: usize) -> Result<&[u8]> {
    let need = REPLY_PREFIX_LEN + len;
    reply.get(REPLY_PREFIX_LEN..need).ok_or(Error::TooShort {
        need,
        got: reply.len(),
    })
}

/// tSnake FPGA transmit mailbox (`FUN_083d92d0` stores `0x700` in driver field `0x4dc`).
pub const TSNAKE_TX_MAILBOX: u16 = 0x0700;
/// tSnake FPGA receive mailbox (`0x800` in driver field `0x4de`). The byte at this
/// address is the receive status, the message starts at the next address.
pub const TSNAKE_RX_MAILBOX: u16 = 0x0800;
/// tSnake Ethernet header RAM ([`crate::ether`]).
pub const TSNAKE_HEADER_RAM: u16 = 0x0300;

/// `ResetTSnakeRx` (`083d6290`).
pub const TSNAKE_RESET_RX: Access = Access::WriteByte {
    addr: 0x0203,
    value: 1,
};
/// `ResetTSnakeTx` (`083d6380`).
pub const TSNAKE_RESET_TX: Access = Access::WriteByte {
    addr: 0x0106,
    value: 1,
};

/// `SendTSnakeMessage` (`083d6530`): write the message into the transmit mailbox.
pub fn send_message(msg: &Message) -> Access {
    Access::WriteBlock {
        addr: TSNAKE_TX_MAILBOX,
        data: msg.encode(),
    }
}

/// `CheckForNewMessage` (`083d6680`): read the receive status byte.
pub const CHECK_FOR_MESSAGE: Access = Access::ReadByte {
    addr: TSNAKE_RX_MAILBOX,
};

/// What the receive status byte says (`FUN_083e8560` when not reading a message).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxStatus {
    /// `0`: nothing waiting.
    Empty,
    /// `0xff`: the host sets a flag (field `0x364`) and reads nothing. Meaning not known.
    Flag,
    /// Otherwise: a message of `n + 1` bytes is waiting. Read it with [`read_message`].
    Pending(u8),
}

impl RxStatus {
    /// Classifies the status byte.
    pub const fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Empty,
            0xff => Self::Flag,
            n => Self::Pending(n),
        }
    }
}

/// `ReadTSnakeMessage(n)` (`083d6760`): read the `n + 1` message bytes after the status byte.
/// The message is `read_block_reply(reply, n + 1)`.
pub fn read_message(n: u8) -> Access {
    Access::ReadBlock {
        addr: TSNAKE_RX_MAILBOX + 1,
        len: u16::from(n) + 1,
    }
}
