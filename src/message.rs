//! The tSnake message (`cTSnakeDeviceManager::sTSnakeMessage`) and typed bodies.
//!
//! Wire layout, the same in both directions:
//!
//! | Byte | Field |
//! |---|---|
//! | 0 | total length - 1 |
//! | 1 | destination device id, `0xff` = all |
//! | 2 | message id, echoed by the reply |
//! | 3 | [`MessageCode`] |
//! | 4.. | payload, at most 21 bytes |
//!
//! These are the bytes the host writes to the FPGA transmit mailbox
//! (`cTSnakeDriverBase::SendTSnakeMessage`, `083d6530`) and reads back from the receive
//! mailbox. How the FPGA places them in an Ethernet frame is not visible in software.

use crate::code::MessageCode;
use crate::error::{Error, Result};
use crate::micpre::{DeluxeMicPre, MicPre};

/// Bytes 0..4.
pub const HEADER_LEN: usize = 4;
/// `SendTSnakeMessage` sends at most 0x19 bytes.
pub const MAX_LEN: usize = 25;
/// Largest payload.
pub const MAX_PAYLOAD: usize = MAX_LEN - HEADER_LEN;
/// Destination id for all devices.
pub const BROADCAST: u8 = 0xff;
/// Message id in a queued message that asks the host to take one from its pool
/// (`cTSnakeDeviceManager::TakeMessageID`, `083d53c0`). The pool holds ids 0..=254.
pub const ID_FROM_POOL: u8 = 0xff;

/// Payload of [`MessageCode::Discovery`].
pub const DISCOVERY_PAYLOAD: &[u8; 12] = b"DiscoveryV2\0";

/// One message as it goes through the FPGA mailbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Byte 1.
    pub dest: u8,
    /// Byte 2.
    pub id: u8,
    /// Byte 3.
    pub code: MessageCode,
    payload: Vec<u8>,
}

impl Message {
    /// Builds a message. Fails if the payload is longer than [`MAX_PAYLOAD`].
    pub fn new(dest: u8, id: u8, code: MessageCode, payload: &[u8]) -> Result<Self> {
        if payload.len() > MAX_PAYLOAD {
            return Err(Error::TooLong(HEADER_LEN + payload.len()));
        }
        Ok(Self {
            dest,
            id,
            code,
            payload: payload.to_vec(),
        })
    }

    /// Builds the message for a typed body.
    pub fn from_body(dest: u8, id: u8, body: &Body) -> Self {
        // Every typed body fits MAX_PAYLOAD; see Body::payload.
        Self {
            dest,
            id,
            code: body.code(),
            payload: body.payload(),
        }
    }

    /// Bytes 4.. .
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Wire bytes.
    pub fn encode(&self) -> Vec<u8> {
        let len = HEADER_LEN + self.payload.len();
        let mut out = Vec::with_capacity(len);
        // len <= MAX_LEN, so len - 1 fits a byte.
        out.extend_from_slice(&[(len - 1) as u8, self.dest, self.id, self.code as u8]);
        out.extend_from_slice(&self.payload);
        out
    }

    /// Parses one message. The input must be exactly `byte0 + 1` bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::TooShort {
                need: HEADER_LEN,
                got: bytes.len(),
            });
        }
        if bytes.len() > MAX_LEN {
            return Err(Error::TooLong(bytes.len()));
        }
        let stated = usize::from(bytes[0]) + 1;
        if stated != bytes.len() {
            return Err(Error::LengthMismatch {
                stated,
                actual: bytes.len(),
            });
        }
        let code = MessageCode::from_u8(bytes[3]).ok_or(Error::UnknownCode(bytes[3]))?;
        Ok(Self {
            dest: bytes[1],
            id: bytes[2],
            code,
            payload: bytes[HEADER_LEN..].to_vec(),
        })
    }
}

/// A MAC address in normal (first octet first) order.
///
/// tSnake payloads carry MACs last octet first: `ProcessDiscoveryReply` builds a u48 from
/// bytes 4..10 little-endian and the log (`FUN_083d5900`) prints its top byte first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mac(pub [u8; 6]);

impl Mac {
    fn to_payload(self) -> [u8; 6] {
        let mut b = self.0;
        b.reverse();
        b
    }

    fn from_payload(b: &[u8]) -> Self {
        let mut m = [0; 6];
        m.copy_from_slice(&b[..6]);
        m.reverse();
        Self(m)
    }

    /// The stock firmware treats a MAC whose last three octets are all 0 or all `0xff`
    /// as unset ("Discovered DX device MAC address is not set", `ProcessDiscoveryReply`).
    pub fn is_unset(self) -> bool {
        let tail = &self.0[3..];
        tail.iter().all(|&b| b == 0) || tail.iter().all(|&b| b == 0xff)
    }
}

/// Payload of [`MessageCode::DiscoveryReply`], read by `ProcessDiscoveryReply` (`083e5db0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryReply {
    /// Payload 0..6.
    pub mac: Mac,
    /// Payload 6: the code in [`crate::tables::DLIVE_DEVICE_MODELS`].
    pub model: u8,
    /// Payload 7, 8: firmware version, two ASCII characters ("Current FW v.%c %c" in Qu).
    /// The Surface ignores replies with `version[0] < b'0'` and compares both bytes with
    /// the update file.
    pub version: [u8; 2],
    /// Payload 9. Used as the link / port index when the host picks a device id.
    pub port: u8,
    /// Payload 10..14: [`crate::tables::CARD_TYPES`] code of each card slot.
    pub cards: [u8; 4],
}

/// Device family, told apart by the version bytes of a Discovery Reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// `version[0] >= b'0'`: ASCII version, DX / GX boxes. Qu logs `tS:`; the dLive Surface
    /// accepts only these (`ProcessDiscoveryReply` ignores `version[0] <= 0x2f`).
    TSnake,
    /// `version[0] < b'0'`: binary version, GLD AudioRacks. Qu logs `dS:` and uses the
    /// dSnake model table (`FUN_00110e28`).
    DSnake,
}

impl DiscoveryReply {
    /// Which model table the code belongs to.
    pub const fn family(&self) -> Family {
        if self.version[0] < b'0' {
            Family::DSnake
        } else {
            Family::TSnake
        }
    }
}

/// Payload of [`MessageCode::UpdateData`] (`FUN_083e55f0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateData {
    /// Payload 0: 1 on the last block.
    pub last: bool,
    /// Payload 2..5, big-endian 24 bit: byte address of `data` in the image. The
    /// Surface computes it as `base + size - remaining`. Only the low 24 bits are sent.
    pub address: u32,
    /// Payload 5..21.
    pub data: [u8; 16],
}

/// Payload of [`MessageCode::DeviceStatusMessage`] as `FUN_083e8210` reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceStatus {
    /// Payload 0: id of the device that sends it.
    pub device: u8,
    /// Payload 1: not read.
    pub byte5: u8,
    /// Payload 2. Bits 0..4: one flag per port; bits 4, 5, 6: three device flags.
    /// Their meaning is not named in the code.
    pub flags: u8,
    /// Payload 3..7: card type per slot.
    pub cards: [u8; 4],
    /// Payload 7: bit n set and card n fitted makes the host configure card n again.
    pub reconfigure: u8,
    /// Payload 8: 0 makes the host disconnect the next device with reason 6
    /// ("Redundant mode enabled") for some device types.
    pub byte12: u8,
    /// Payload 9: stored, meaning not known.
    pub byte13: u8,
}

/// Payload of [`MessageCode::DxHubStatusMessage`], device to host. Read by dLive Surface
/// `FUN_083d6b50`, Avantis `FUN_004d8d30` and Qu-5 `FUN_00110e28` (case 26). No stage-box
/// firmware was read, so the length the device sends is not known; the decoder needs the
/// 9 bytes the hosts read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HubStatus {
    /// Payload 0: id of the device that sends it.
    pub device: u8,
    /// Payload 1: not read.
    pub byte5: u8,
    /// Payload 2. Surface and Avantis store bit 0, Qu stores bit 4, in the same per-device
    /// flag that bit 4 of [`DeviceStatus::flags`] sets. Meaning not named.
    pub flags: u8,
    /// Payload 3..8: not read.
    pub bytes7_11: [u8; 5],
    /// Payload 8, bits 0..4: one bit per hub port. All hosts store the inverted bit; the Qu
    /// drops the devices behind a port whose bit is set. Inferred: set = link down.
    pub ports: u8,
}

/// Payload of [`MessageCode::GetGxStatus`], device to host. Read by dLive Surface
/// `FUN_083e5c50`, Avantis `FUN_004dd800` and Qu-5 `FUN_00110e28` (case 29). Device-side
/// length not known; the decoder needs the 14 bytes the hosts read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GxStatus {
    /// Payload 0: id of the device that sends it.
    pub device: u8,
    /// Payload 1: not read.
    pub byte5: u8,
    /// Payload 2: bit 4 goes to the same flag as bit 4 of [`DeviceStatus::flags`], bit 7 to
    /// a second per-device flag. Meanings not named.
    pub flags: u8,
    /// Payload 3..13: not read.
    pub bytes7_16: [u8; 10],
    /// Payload 13: one bit per port, stored inverted. Surface and Avantis read bits 2 and 3
    /// and disconnect devices `2n`, `2n + 1` when bit `n` becomes set; the Qu reads bits 0..4.
    /// Inferred: set = link down.
    pub ports: u8,
}

/// Payload of [`MessageCode::SetDigitalOutputRate`], built for `Digital Out` cards (type 4)
/// by dLive Surface `FUN_083e7940`, MixRack `FUN_00108ccc` and Qu-5 `FUN_001121f8`.
/// A "pair" is a channel pair of the card, counted from the card's first pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalOutputRate {
    /// Payload 0: card slot on the device.
    pub card: u8,
    /// Payload 1: a 2-bit code per pair, pair `k` at bits `2k`, for the pairs the card has.
    /// The Surface maps its setting 0, 1, 2 to code 3, 2, 1 (table `0x09b5f292`, else 0); the
    /// Qu writes its setting as the code (table `0x210e08` = 0, 1, 2, 3). What the codes mean
    /// is not named.
    pub rates: u8,
    /// Payload 2, 3: `1 << code(pair 2) | 1 << (4 + code(pair 3))`, then the same for pairs
    /// 0 and 1. Always built from four pairs.
    pub one_hot: [u8; 2],
    /// Payload 4: bit `k` set when the host's setting of pair `k` is 3 (the Surface and Qu
    /// number settings differently, see [`Self::rates`]).
    pub mask: u8,
    /// Payload 5, only in the dLive form (Surface and MixRack send 6 payload bytes, the Qu 5):
    /// 4 for output mode 0, 5 for mode 1, else 0 (Surface `FUN_083d5dd0`, mode array at driver
    /// field `0x3dc`).
    pub mode: Option<u8>,
}

/// Payload of [`MessageCode::LcdData`] (dLive Surface `FUN_0821a090`, MixRack `FUN_002c2500`).
/// Only the byte positions are known: the builder copies two 8-byte buffers between caller
/// bytes. Its caller is a virtual method that was not traced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LcdData {
    /// Payload 0, 1: builder arguments 3 and 4.
    pub head: [u8; 2],
    /// Payload 2..18: two 8-byte buffers. Inferred: two text lines.
    pub lines: [[u8; 8]; 2],
    /// Payload 18, 19: builder arguments 7 and 8.
    pub tail: [u8; 2],
}

/// Kinds of the one-flag share messages (codes 33..=37).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlagShare {
    /// Code 33.
    HasMicPre,
    /// Code 34.
    Pad1,
    /// Code 35.
    Pad2,
    /// Code 36.
    Pp1,
    /// Code 37.
    Pp2,
}

impl FlagShare {
    const fn code(self) -> MessageCode {
        match self {
            Self::HasMicPre => MessageCode::HasMicPre,
            Self::Pad1 => MessageCode::PadShareMessage1,
            Self::Pad2 => MessageCode::PadShareMessage2,
            Self::Pp1 => MessageCode::PpShareMessage1,
            Self::Pp2 => MessageCode::PpShareMessage2,
        }
    }
}

/// Typed payloads for the codes whose layout the stock firmware shows. Every code that any
/// read build (dLive Surface and MixRack V2.12, Avantis V2.02, Qu-5 V1.2.1) sends or parses
/// has a variant. The rest (0, 4, 11, 13, 15, 18, 24, 25, 30, 40, 41, 42, 43) are neither
/// built nor handled there and decode to [`Error::UntypedCode`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// Code 2, payload [`DISCOVERY_PAYLOAD`].
    Discovery,
    /// Code 12.
    DiscoveryReply(DiscoveryReply),
    /// Code 3: device MAC, then the id the host gives it.
    AssignId {
        /// Payload 0..6.
        mac: Mac,
        /// Payload 6.
        device: u8,
    },
    /// Code 14. Byte 2 of the message is the id of the confirmed message.
    Confirmation {
        /// Payload 0. The host writes `0xff`. Inferred: the sender id.
        source: u8,
        /// Payload 1: code of the confirmed message (raw: may be any byte).
        /// Payload 2 is not set by `FUN_083e48a0` (length byte 6 = 7 bytes); encoded as 0,
        /// ignored on decode.
        confirmed: u8,
    },
    /// Code 1. The 1-byte payload is not set by the Surface; encoded as 0, ignored on decode.
    SoftReset,
    /// Code 5. Same payload as [`Body::SoftReset`].
    EnterUpdateMode,
    /// Code 6.
    UpdateData(UpdateData),
    /// Code 8, no payload.
    GetDeviceStatus,
    /// Code 16, no payload.
    DevicePoke,
    /// Code 17.
    DeviceStatus(DeviceStatus),
    /// Code 19: payload `00 02 07 f0 00` then the MAC. Inferred: the 5 bytes follow the
    /// [`UpdateData`] layout (not last, 2, address `0x07f000`).
    SetMacAddress(Mac),
    /// Code 10, broadcast, no reply. The Surface (`FUN_0827a8f0` every 4th call) and the Qu
    /// (`FUN_00110e28`, once a second) send one channel per message and cycle 0..40.
    ChannelName {
        /// Payload 0.
        channel: u8,
        /// Payload 1..8: name, NUL padded (Surface `FUN_0827a5e0`). Payload 8 is 0 from the
        /// Surface and the 8th name byte from the Qu; encoded as 0, ignored on decode.
        name: [u8; 7],
        /// Payload 9: `cTSnakeDriverBase::GetStereoLink(channel >> 1)` (Surface); the
        /// source record's stereo byte (Qu `FUN_001108ec`, record byte 6).
        stereo_link: u8,
        /// Payload 10: source category. Surface `FUN_0827a8b0`: table `0x09232de0` indexed by
        /// the source type (`0x10` for types >= `0x22`); Qu record byte 4. Values not named.
        source_kind: u8,
        /// Payload 11: source number. Surface `FUN_0827a8e0` (+`0x20` for source type 6); Qu
        /// record byte 5.
        source_index: u8,
    },
    /// Code 7 (Surface `FUN_083e50e0`, MixRack `FUN_00109c80`, Avantis `FUN_004dc3f0`, Qu
    /// `FUN_00112c00`). A new one replaces a queued code-7 message with the same device and
    /// key (Surface, MixRack), so payload 0 names the setting. The Qu sends keys 4, 5, `0x0c`,
    /// `0x24`, `0x27`, `0x30` to dSnake devices; their meanings are not named.
    SetDeviceConfig {
        /// Payload 0.
        key: u8,
        /// Payload 1.
        value: u8,
    },
    /// Code 9, tSnake forms. See [`crate::micpre`].
    MicPreSettings(MicPre),
    /// Code 23. See [`crate::micpre`].
    DeluxeMicPreSettings(DeluxeMicPre),
    /// Code 20, for `Digital In` cards (type 3): Surface `FUN_083e7740`, MixRack
    /// `ConfigureNewCard` (`00109e94`), Qu `FUN_00112068`.
    SetSrcBypass {
        /// Payload 0: card slot on the device.
        card: u8,
        /// Payload 1: bit `k` = the host's flag for pair `k` of the card (Surface driver field
        /// `0x29a`). Inferred from the message name: set = bypass the sample-rate converter.
        /// The Surface sends 4 more payload bytes it never sets (length byte 9); the Qu sends
        /// none (length byte 5). Encoded in the Qu form, extra bytes ignored on decode.
        pairs: u8,
    },
    /// Code 21.
    SetDigitalOutputRate(DigitalOutputRate),
    /// Code 22, for line-out cards (types 2, 6, 19 and 20): Surface `FUN_083e4f40`, MixRack
    /// `FUN_00109dcc`, Avantis `FUN_004dc280`, Qu `FUN_001124bc`.
    ConfigureLineOut {
        /// Payload 0: card slot on the device.
        card: u8,
    },
    /// Code 26, device to host.
    DxHubStatus(HubStatus),
    /// Code 27.
    LcdData(LcdData),
    /// Code 28 (Surface `FUN_083e6910`, MixRack `FUN_00109600`). Both hosts send only
    /// `ff 60`, to devices of model class 7, after the Assign ID is confirmed. Byte meanings
    /// not named.
    LcdBrightness([u8; 2]),
    /// Code 29, device to host.
    GxStatus(GxStatus),
    /// Code 38 (Surface `FUN_083e5040`, MixRack `FUN_00109d5c`, Avantis `FUN_004dc370`, Qu
    /// `FUN_001117a4`). The value is 1 when the first card is type 13, 15, 16 or 17 (the
    /// "Type2" cards), else 0; the Surface sends nothing for card types 0..=4, 6, 7, 26, 29.
    SetIoCardType(u8),
    /// Code 39. Built instead of code 5 by Avantis `FUN_004dc6a0` (when a per-device flag is
    /// set, then followed by code 5) and Qu `FUN_001130e8` (third argument non-zero). The Qu
    /// treats its confirmation like that of code 5. Same 1-byte unset payload as code 5.
    EnterUpdateMode48k,
    /// Codes 31 (`second == false`) and 32.
    GainShare {
        /// Code 32 when true.
        second: bool,
        /// Payload 0.
        channel: u8,
        /// Payload 1..3, big-endian.
        gain: u16,
    },
    /// Codes 33..=37.
    FlagShare {
        /// Which message.
        kind: FlagShare,
        /// Payload 0.
        channel: u8,
        /// Payload 1.
        value: u8,
    },
}

const SET_MAC_PREFIX: [u8; 5] = [0x00, 0x02, 0x07, 0xf0, 0x00];
const UPDATE_DATA_CONST: u8 = 0x02;

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

fn constant(p: &[u8], offset: usize, expected: u8) -> Result<()> {
    if p[offset] == expected {
        Ok(())
    } else {
        Err(Error::BadConstant {
            offset,
            expected,
            actual: p[offset],
        })
    }
}

impl Body {
    /// The message code of this body.
    pub const fn code(&self) -> MessageCode {
        match self {
            Self::Discovery => MessageCode::Discovery,
            Self::DiscoveryReply(_) => MessageCode::DiscoveryReply,
            Self::AssignId { .. } => MessageCode::AssignId,
            Self::Confirmation { .. } => MessageCode::ConfirmationMessage,
            Self::SoftReset => MessageCode::SoftReset,
            Self::EnterUpdateMode => MessageCode::EnterUpdateMode,
            Self::UpdateData(_) => MessageCode::UpdateData,
            Self::GetDeviceStatus => MessageCode::GetDeviceStatus,
            Self::DevicePoke => MessageCode::DevicePoke,
            Self::DeviceStatus(_) => MessageCode::DeviceStatusMessage,
            Self::SetMacAddress(_) => MessageCode::SetMacAddress,
            Self::ChannelName { .. } => MessageCode::ChannelNames,
            Self::SetDeviceConfig { .. } => MessageCode::SetDeviceConfig,
            Self::MicPreSettings(_) => MessageCode::MicPreSettings,
            Self::DeluxeMicPreSettings(_) => MessageCode::DeluxeMicPreSettings,
            Self::SetSrcBypass { .. } => MessageCode::SetSrcBypass,
            Self::SetDigitalOutputRate(_) => MessageCode::SetDigitalOutputRate,
            Self::ConfigureLineOut { .. } => MessageCode::ConfigureLineOut,
            Self::DxHubStatus(_) => MessageCode::DxHubStatusMessage,
            Self::LcdData(_) => MessageCode::LcdData,
            Self::LcdBrightness(_) => MessageCode::LcdBrightness,
            Self::GxStatus(_) => MessageCode::GetGxStatus,
            Self::SetIoCardType(_) => MessageCode::SetIoCardType,
            Self::EnterUpdateMode48k => MessageCode::EnterUpdateMode48k,
            Self::GainShare { second: false, .. } => MessageCode::GainShareMessage1,
            Self::GainShare { second: true, .. } => MessageCode::GainShareMessage2,
            Self::FlagShare { kind, .. } => kind.code(),
        }
    }

    /// Payload bytes. The longest body is [`Body::UpdateData`] with 21 bytes.
    pub fn payload(&self) -> Vec<u8> {
        match self {
            Self::Discovery => DISCOVERY_PAYLOAD.to_vec(),
            Self::DiscoveryReply(r) => {
                let mut p = r.mac.to_payload().to_vec();
                p.extend_from_slice(&[r.model, r.version[0], r.version[1], r.port]);
                p.extend_from_slice(&r.cards);
                p
            }
            Self::AssignId { mac, device } => {
                let mut p = mac.to_payload().to_vec();
                p.push(*device);
                p
            }
            Self::Confirmation { source, confirmed } => vec![*source, *confirmed, 0],
            Self::SoftReset | Self::EnterUpdateMode | Self::EnterUpdateMode48k => vec![0],
            Self::UpdateData(u) => {
                let a = (u.address & 0xff_ffff).to_be_bytes();
                let mut p = vec![u8::from(u.last), UPDATE_DATA_CONST, a[1], a[2], a[3]];
                p.extend_from_slice(&u.data);
                p
            }
            Self::GetDeviceStatus | Self::DevicePoke => Vec::new(),
            Self::DeviceStatus(s) => {
                let mut p = vec![s.device, s.byte5, s.flags];
                p.extend_from_slice(&s.cards);
                p.extend_from_slice(&[s.reconfigure, s.byte12, s.byte13]);
                p
            }
            Self::SetMacAddress(mac) => {
                let mut p = SET_MAC_PREFIX.to_vec();
                p.extend_from_slice(&mac.to_payload());
                p
            }
            Self::ChannelName {
                channel,
                name,
                stereo_link,
                source_kind,
                source_index,
            } => {
                let mut p = vec![*channel];
                p.extend_from_slice(name);
                p.extend_from_slice(&[0, *stereo_link, *source_kind, *source_index]);
                p
            }
            Self::SetDeviceConfig { key, value } => vec![*key, *value],
            Self::MicPreSettings(m) => m.payload(),
            Self::DeluxeMicPreSettings(m) => m.payload(),
            Self::SetSrcBypass { card, pairs } => vec![*card, *pairs],
            Self::SetDigitalOutputRate(r) => {
                let mut p = vec![r.card, r.rates, r.one_hot[0], r.one_hot[1], r.mask];
                p.extend(r.mode);
                p
            }
            Self::ConfigureLineOut { card } => vec![*card],
            Self::DxHubStatus(s) => {
                let mut p = vec![s.device, s.byte5, s.flags];
                p.extend_from_slice(&s.bytes7_11);
                p.push(s.ports);
                p
            }
            Self::LcdData(d) => {
                let mut p = d.head.to_vec();
                p.extend(d.lines.iter().flatten());
                p.extend_from_slice(&d.tail);
                p
            }
            Self::LcdBrightness(b) => b.to_vec(),
            Self::GxStatus(s) => {
                let mut p = vec![s.device, s.byte5, s.flags];
                p.extend_from_slice(&s.bytes7_16);
                p.push(s.ports);
                p
            }
            Self::SetIoCardType(v) => vec![*v],
            Self::GainShare { channel, gain, .. } => {
                let g = gain.to_be_bytes();
                vec![*channel, g[0], g[1]]
            }
            Self::FlagShare { channel, value, .. } => vec![*channel, *value],
        }
    }

    /// Reads the typed body of a message. Extra payload bytes are ignored, as the stock
    /// handlers ignore them.
    pub fn from_message(m: &Message) -> Result<Self> {
        let p = m.payload();
        Ok(match m.code {
            MessageCode::Discovery => {
                need(p, DISCOVERY_PAYLOAD.len())?;
                for (i, &b) in DISCOVERY_PAYLOAD.iter().enumerate() {
                    constant(p, i, b)?;
                }
                Self::Discovery
            }
            MessageCode::DiscoveryReply => {
                need(p, 14)?;
                let mut cards = [0; 4];
                cards.copy_from_slice(&p[10..14]);
                Self::DiscoveryReply(DiscoveryReply {
                    mac: Mac::from_payload(p),
                    model: p[6],
                    version: [p[7], p[8]],
                    port: p[9],
                    cards,
                })
            }
            MessageCode::AssignId => {
                need(p, 7)?;
                Self::AssignId {
                    mac: Mac::from_payload(p),
                    device: p[6],
                }
            }
            MessageCode::ConfirmationMessage => {
                need(p, 3)?;
                Self::Confirmation {
                    source: p[0],
                    confirmed: p[1],
                }
            }
            MessageCode::SoftReset => Self::SoftReset,
            MessageCode::EnterUpdateMode => Self::EnterUpdateMode,
            MessageCode::EnterUpdateMode48k => Self::EnterUpdateMode48k,
            MessageCode::SetDeviceConfig => {
                need(p, 2)?;
                Self::SetDeviceConfig {
                    key: p[0],
                    value: p[1],
                }
            }
            MessageCode::MicPreSettings => Self::MicPreSettings(MicPre::from_payload(p)?),
            MessageCode::DeluxeMicPreSettings => {
                Self::DeluxeMicPreSettings(DeluxeMicPre::from_payload(p)?)
            }
            MessageCode::SetSrcBypass => {
                need(p, 2)?;
                Self::SetSrcBypass {
                    card: p[0],
                    pairs: p[1],
                }
            }
            MessageCode::SetDigitalOutputRate => {
                need(p, 5)?;
                Self::SetDigitalOutputRate(DigitalOutputRate {
                    card: p[0],
                    rates: p[1],
                    one_hot: [p[2], p[3]],
                    mask: p[4],
                    mode: p.get(5).copied(),
                })
            }
            MessageCode::ConfigureLineOut => {
                need(p, 1)?;
                Self::ConfigureLineOut { card: p[0] }
            }
            MessageCode::DxHubStatusMessage => {
                need(p, 9)?;
                let mut bytes7_11 = [0; 5];
                bytes7_11.copy_from_slice(&p[3..8]);
                Self::DxHubStatus(HubStatus {
                    device: p[0],
                    byte5: p[1],
                    flags: p[2],
                    bytes7_11,
                    ports: p[8],
                })
            }
            MessageCode::LcdData => {
                need(p, 20)?;
                let mut lines = [[0; 8]; 2];
                lines[0].copy_from_slice(&p[2..10]);
                lines[1].copy_from_slice(&p[10..18]);
                Self::LcdData(LcdData {
                    head: [p[0], p[1]],
                    lines,
                    tail: [p[18], p[19]],
                })
            }
            MessageCode::LcdBrightness => {
                need(p, 2)?;
                Self::LcdBrightness([p[0], p[1]])
            }
            MessageCode::GetGxStatus => {
                need(p, 14)?;
                let mut bytes7_16 = [0; 10];
                bytes7_16.copy_from_slice(&p[3..13]);
                Self::GxStatus(GxStatus {
                    device: p[0],
                    byte5: p[1],
                    flags: p[2],
                    bytes7_16,
                    ports: p[13],
                })
            }
            MessageCode::SetIoCardType => {
                need(p, 1)?;
                Self::SetIoCardType(p[0])
            }
            MessageCode::UpdateData => {
                need(p, 21)?;
                constant(p, 1, UPDATE_DATA_CONST)?;
                let mut data = [0; 16];
                data.copy_from_slice(&p[5..21]);
                Self::UpdateData(UpdateData {
                    last: p[0] != 0,
                    address: u32::from_be_bytes([0, p[2], p[3], p[4]]),
                    data,
                })
            }
            MessageCode::GetDeviceStatus => Self::GetDeviceStatus,
            MessageCode::DevicePoke => Self::DevicePoke,
            MessageCode::DeviceStatusMessage => {
                need(p, 10)?;
                let mut cards = [0; 4];
                cards.copy_from_slice(&p[3..7]);
                Self::DeviceStatus(DeviceStatus {
                    device: p[0],
                    byte5: p[1],
                    flags: p[2],
                    cards,
                    reconfigure: p[7],
                    byte12: p[8],
                    byte13: p[9],
                })
            }
            MessageCode::SetMacAddress => {
                need(p, 11)?;
                for (i, &b) in SET_MAC_PREFIX.iter().enumerate() {
                    constant(p, i, b)?;
                }
                Self::SetMacAddress(Mac::from_payload(&p[5..]))
            }
            MessageCode::ChannelNames => {
                need(p, 12)?;
                let mut name = [0; 7];
                name.copy_from_slice(&p[1..8]);
                Self::ChannelName {
                    channel: p[0],
                    name,
                    stereo_link: p[9],
                    source_kind: p[10],
                    source_index: p[11],
                }
            }
            MessageCode::GainShareMessage1 | MessageCode::GainShareMessage2 => {
                need(p, 3)?;
                Self::GainShare {
                    second: m.code == MessageCode::GainShareMessage2,
                    channel: p[0],
                    gain: u16::from_be_bytes([p[1], p[2]]),
                }
            }
            MessageCode::HasMicPre
            | MessageCode::PadShareMessage1
            | MessageCode::PadShareMessage2
            | MessageCode::PpShareMessage1
            | MessageCode::PpShareMessage2 => {
                need(p, 2)?;
                let kind = match m.code {
                    MessageCode::HasMicPre => FlagShare::HasMicPre,
                    MessageCode::PadShareMessage1 => FlagShare::Pad1,
                    MessageCode::PadShareMessage2 => FlagShare::Pad2,
                    MessageCode::PpShareMessage1 => FlagShare::Pp1,
                    _ => FlagShare::Pp2,
                };
                Self::FlagShare {
                    kind,
                    channel: p[0],
                    value: p[1],
                }
            }
            other => return Err(Error::UntypedCode(other as u8)),
        })
    }
}
