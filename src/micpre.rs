//! Mic pre payloads: codes 9 (`Mic Pre Settings`) and 23 (`Deluxe Mic Pre Settings`).
//!
//! All forms start with the card slot on the device (port offset in the device divided by
//! the channels per card). The rest depends on the card, and the message itself does not say
//! which form it is:
//!
//! | Form | Code | Payload | Sent for |
//! |---|---|---|---|
//! | [`MicPre::Eight`] | 9 | 11 bytes | model classes 0..=3, 5..=7, 9..=11 ([`crate::tables::DeviceModel::kind`]) |
//! | [`MicPre::Twelve`] | 9 | 17 bytes | model class 4 (GX-4816) |
//! | [`DeluxeMicPre`] | 23 | 17 bytes | card type 7 (`Mic In DLX`) |
//! | [`DSnakeMicPre`] | 9 | 17 bytes | dSnake (GLD AudioRack) devices |
//!
//! Evidence: dLive Surface `cTSnakeDeviceManager::AddMicPreMessage` (`083e6f80`), MixRack
//! `AddMicPreMessage` (`00108ee8`), Qu-5 `FUN_00111834` (tSnake forms) and `FUN_00112a84`
//! (dSnake form), Avantis `FUN_004de090` (dSnake form). The Qu builds every tSnake form byte
//! for byte like the Surface.
//!
//! Channels are listed in **wire slot** order. The slot-to-channel map is fixed per form:
//! slot `s` is channel `first + 7 - s` (eight, except model class 9 = DX-88, where slot `s`
//! is channel `first + s`), `first + 11 - s` (twelve), `(first | 7) - s` (deluxe, dSnake).
//!
//! Each tSnake channel has two flags. `flag_a` is the per-channel byte array at driver field
//! `0x19a` (Surface; Qu channel byte `+0xcd`), `flag_b` the array at `0x21a` (Qu `+0xcc`).
//! **Inferred, not proven:** `flag_a` is the pad and `flag_b` the 48 V phantom power. The
//! Surface stores gain, `flag_a`, `flag_b` as three 128-entry arrays in that order, and its
//! share manager (`FUN_0854bd90` / `FUN_0854c140`) stores gain, pad, PP in the same order.

use crate::error::{Error, Result};

/// One channel of a [`MicPre`] message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MicPreChannel {
    /// Gain code. The Surface takes it from a table indexed by whole dB (`08279f10`, table
    /// built at run time at `0x0aa96760`), so the code-to-dB map is not in the binary.
    pub gain: u8,
    /// Bit `2k` of the flag byte (see the module doc).
    pub flag_a: bool,
    /// Bit `2k + 1` of the flag byte.
    pub flag_b: bool,
}

/// Payload of a tSnake [`crate::MessageCode::MicPreSettings`] message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicPre {
    /// 11-byte payload: `card, g0, g1, F(3,2,1,0), g2, g3, g4, g5, F(7,6,5,4), g6, g7`.
    Eight {
        /// Payload 0: card slot on the device.
        card: u8,
        /// Wire slots 0..8.
        channels: [MicPreChannel; 8],
    },
    /// 17-byte payload: `card, F(1,0,11,11), g0..g3, F(5,4,3,2), g4, g5, F(7,6,11,11),
    /// g6..g9, F(11,10,9,8), g10, g11`. Slot 11 appears in three flag bytes; the decoder
    /// reads it from the last one.
    Twelve {
        /// Payload 0: card slot on the device.
        card: u8,
        /// Wire slots 0..12.
        channels: [MicPreChannel; 12],
    },
}

/// `F(w, x, y, z)`: slots whose flag pairs sit at bits 0-1, 2-3, 4-5 and 6-7 of one byte
/// (`FUN_083d6470`: bit `2k` from field `0x19a`, bit `2k + 1` from `0x21a`).
type FlagByte = (usize, [usize; 4]);

const EIGHT_GAINS: [usize; 8] = [1, 2, 4, 5, 6, 7, 9, 10];
const EIGHT_FLAGS: [FlagByte; 2] = [(3, [3, 2, 1, 0]), (8, [7, 6, 5, 4])];
const TWELVE_GAINS: [usize; 12] = [2, 3, 4, 5, 7, 8, 10, 11, 12, 13, 15, 16];
// Byte 14 last: it holds the one real copy of slot 11 at bits 0-1.
const TWELVE_FLAGS: [FlagByte; 4] = [
    (1, [1, 0, 11, 11]),
    (6, [5, 4, 3, 2]),
    (9, [7, 6, 11, 11]),
    (14, [11, 10, 9, 8]),
];

/// Payload length of [`MicPre::Eight`].
pub const EIGHT_LEN: usize = 11;
/// Payload length of [`MicPre::Twelve`], [`DeluxeMicPre`] and [`DSnakeMicPre`].
pub const LONG_LEN: usize = 17;

fn need(p: &[u8], n: usize) -> Result<()> {
    if p.len() < n {
        Err(Error::TooShort {
            need: n,
            got: p.len(),
        })
    } else {
        Ok(())
    }
}

fn encode_slots(card: u8, ch: &[MicPreChannel], gains: &[usize], flags: &[FlagByte]) -> Vec<u8> {
    let mut p = vec![0; gains.len() + flags.len() + 1];
    p[0] = card;
    for (&at, c) in gains.iter().zip(ch) {
        p[at] = c.gain;
    }
    for &(at, slots) in flags {
        p[at] = slots.iter().enumerate().fold(0, |b, (k, &s)| {
            b | (u8::from(ch[s].flag_a) | u8::from(ch[s].flag_b) << 1) << (2 * k)
        });
    }
    p
}

fn decode_slots<const N: usize>(
    p: &[u8],
    gains: &[usize; N],
    flags: &[FlagByte],
) -> [MicPreChannel; N] {
    let mut ch = [MicPreChannel::default(); N];
    for (c, &at) in ch.iter_mut().zip(gains) {
        c.gain = p[at];
    }
    for &(at, slots) in flags {
        for (k, &s) in slots.iter().enumerate() {
            ch[s].flag_a = p[at] >> (2 * k) & 1 != 0;
            ch[s].flag_b = p[at] >> (2 * k + 1) & 1 != 0;
        }
    }
    ch
}

impl MicPre {
    /// Payload bytes.
    pub fn payload(&self) -> Vec<u8> {
        match self {
            Self::Eight { card, channels } => {
                encode_slots(*card, channels, &EIGHT_GAINS, &EIGHT_FLAGS)
            }
            Self::Twelve { card, channels } => {
                encode_slots(*card, channels, &TWELVE_GAINS, &TWELVE_FLAGS)
            }
        }
    }

    /// Reads a payload: 17 or more bytes is [`MicPre::Twelve`], 11 to 16 is
    /// [`MicPre::Eight`] (the stock senders use exactly 11 or 17).
    pub fn from_payload(p: &[u8]) -> Result<Self> {
        need(p, EIGHT_LEN)?;
        Ok(if p.len() >= LONG_LEN {
            Self::Twelve {
                card: p[0],
                channels: decode_slots(p, &TWELVE_GAINS, &TWELVE_FLAGS),
            }
        } else {
            Self::Eight {
                card: p[0],
                channels: decode_slots(p, &EIGHT_GAINS, &EIGHT_FLAGS),
            }
        })
    }
}

/// One channel of a [`DeluxeMicPre`] message. On the wire: `gain_db / 3`, then
/// `0x30 | flag_a << 3 | flag_b << 2 | step` with `step = 3 - gain_db % 3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeluxeChannel {
    /// Whole dB: the high byte of the channel's 16-bit gain (driver field `0x9a`). The
    /// Surface clamps the gain to `0..=0x3c00`, so 0..=60 dB (`08279f10`).
    pub gain_db: u8,
    /// Bit 3 of the second byte (field `0x19a`, as [`MicPreChannel::flag_a`]).
    pub flag_a: bool,
    /// Bit 2 of the second byte (field `0x21a`).
    pub flag_b: bool,
}

/// Payload of [`crate::MessageCode::DeluxeMicPreSettings`]: card slot, then 8 two-byte
/// channels, slot `s` = channel `(first | 7) - s`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeluxeMicPre {
    /// Payload 0: card slot on the device.
    pub card: u8,
    /// Wire slots 0..8.
    pub channels: [DeluxeChannel; 8],
}

/// High nibble of the second byte of every deluxe channel (`AddMicPreMessage` ORs `0x30`).
const DELUXE_TAG: u8 = 0x30;

impl DeluxeMicPre {
    /// Payload bytes (17).
    pub fn payload(&self) -> Vec<u8> {
        let mut p = vec![self.card];
        for c in &self.channels {
            let step = 3 - c.gain_db % 3;
            p.push(c.gain_db / 3);
            p.push(DELUXE_TAG | u8::from(c.flag_a) << 3 | u8::from(c.flag_b) << 2 | step);
        }
        p
    }

    /// Reads a payload. Fails on a second byte without the `0x30` tag, a step of 0, or a
    /// gain above 255 dB.
    pub fn from_payload(p: &[u8]) -> Result<Self> {
        need(p, LONG_LEN)?;
        let mut channels = [DeluxeChannel::default(); 8];
        for (i, c) in channels.iter_mut().enumerate() {
            let (coarse, fine) = (p[1 + 2 * i], p[2 + 2 * i]);
            if fine & 0xf0 != DELUXE_TAG {
                return Err(Error::BadConstant {
                    offset: 2 + 2 * i,
                    expected: DELUXE_TAG,
                    actual: fine & 0xf0,
                });
            }
            let step = fine & 3;
            if step == 0 {
                return Err(Error::OutOfRange("deluxe mic pre step"));
            }
            c.gain_db = u8::try_from(u16::from(coarse) * 3 + 3 - u16::from(step))
                .map_err(|_| Error::OutOfRange("deluxe mic pre gain"))?;
            c.flag_a = fine & 8 != 0;
            c.flag_b = fine & 4 != 0;
        }
        Ok(Self {
            card: p[0],
            channels,
        })
    }
}

/// The dSnake (GLD AudioRack) form of code 9: card number, then 8 big-endian 16-bit gains,
/// slot `s` = channel `(first | 7) - s`. Qu-5 `FUN_00112a84` and Avantis `FUN_004de090`
/// (`GetDSnakeCardNumber`) build it the same way. It has the length of [`MicPre::Twelve`],
/// so [`crate::Body::from_message`] cannot tell them apart and decodes the tSnake form; a
/// host that talks to a dSnake device reads the payload with [`DSnakeMicPre::from_payload`].
/// The gain unit is not shown in these builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DSnakeMicPre {
    /// Payload 0.
    pub card: u8,
    /// Payload 1..17.
    pub gains: [u16; 8],
}

impl DSnakeMicPre {
    /// Payload bytes (17), for [`crate::Message::new`] with code 9.
    pub fn payload(&self) -> Vec<u8> {
        let mut p = vec![self.card];
        for g in self.gains {
            p.extend_from_slice(&g.to_be_bytes());
        }
        p
    }

    /// Reads a payload.
    pub fn from_payload(p: &[u8]) -> Result<Self> {
        need(p, LONG_LEN)?;
        let mut gains = [0; 8];
        for (i, g) in gains.iter_mut().enumerate() {
            *g = u16::from_be_bytes([p[1 + 2 * i], p[2 + 2 * i]]);
        }
        Ok(Self { card: p[0], gains })
    }
}
