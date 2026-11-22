//! Message codes (byte 3 of every message).
//!
//! Proof that the code is the index into the `MessageName` table, dLive Surface V2.12:
//! - `FUN_083d5c30` returns `table[code * 32]` (`table` = `"Reserved, No Command"` at
//!   `0x09b62de0`) for `code <= 0x29`, else `"Invalid Command"`.
//! - The receive dispatcher (inside `FUN_083e8560`, jump table at `0x09b5f1b8`, index
//!   `code - 12`) sends code 12 to `cTSnakeDeviceManager::ProcessDiscoveryReply` (`083e5db0`)
//!   and code 14 to `cTSnakeDeviceManager::ProcessConfirmation` (`083e7f80`).
//!   The MixRack build has the same switch (`FUN_0010b808` in `TLDStageBox`).
//! - Senders write the code into byte 3: `Discovery` = 2 (`FUN_083e51f0`), `Assign ID` = 3
//!   (`FUN_083e53a0`), and so on (see the crate README). Qu-5 V1.2.1 senders
//!   (`FUN_00112c00` = 7, `FUN_00112c9c` = 8, `FUN_001117a4` = 38, ...) use the same codes.
//! - Avantis V2.02 `FUN_004d8fb0` does the same lookup (table `0x024512e0`) for `code < 0x2d`,
//!   which puts the three `ME ...` names at 41..=43 and `"Invalid command"` at 44.
//!
//! Not built or handled by any read build: 0, 4, 11, 13, 15, 18, 24, 25, 30, 40, 41, 42, 43.
//! The Surface and Avantis send a 1-byte message `00` (length byte 0, no code byte;
//! Surface `FUN_083e69c0`, Avantis `FUN_004ddad0`); it is not a code-0 message.

macro_rules! codes {
    ($( $(#[$m:meta])* $v:ident = $n:literal => $s:literal, )+) => {
        /// A tSnake message code. The doc of each variant is its firmware name.
        #[non_exhaustive]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum MessageCode {
            $( #[doc = concat!("`", $s, "`")] $(#[$m])* $v = $n, )+
        }

        impl MessageCode {
            /// Every code, in numeric order.
            pub const ALL: &'static [Self] = &[$(Self::$v,)+];

            /// The name in the firmware's message-name table.
            pub const fn name(self) -> &'static str {
                match self { $(Self::$v => $s,)+ }
            }

            /// The code for a byte, if the table has a name for it.
            pub const fn from_u8(v: u8) -> Option<Self> {
                match v { $($n => Some(Self::$v),)+ _ => None }
            }
        }
    };
}

codes! {
    ReservedNoCommand = 0 => "Reserved, No Command",
    /// Host to device. Payload: 1 byte that the Surface does not set (`FUN_083e5850`).
    SoftReset = 1 => "Soft Reset",
    /// Host broadcast. Payload `"DiscoveryV2\0"` (`FUN_083e51f0`).
    Discovery = 2 => "Discovery",
    /// Host broadcast. Payload: device MAC (LSB first), new device id (`FUN_083e53a0`).
    AssignId = 3 => "Assign ID",
    Unused = 4 => "Unused",
    /// Host to device, reply timeout 30000 (`FUN_083e54a0`).
    EnterUpdateMode = 5 => "Enter Update Mode",
    /// Host to device: 16 firmware bytes per message (`FUN_083e55f0`).
    UpdateData = 6 => "Update Data",
    /// Host to device, 2 payload bytes (`FUN_083e50e0`).
    SetDeviceConfig = 7 => "Set Device Config",
    /// Host to device, no payload; the reply is code 17 (`FUN_083e8c80`).
    GetDeviceStatus = 8 => "Get Device Status",
    /// Host to device (`AddMicPreMessage`, `083e6f80`).
    MicPreSettings = 9 => "Mic Pre Settings",
    /// Host broadcast, no reply expected (`FUN_0827a3b0`).
    ChannelNames = 10 => "Channel Names",
    RemoteMiimCommand = 11 => "Remote MIIM Command",
    /// Device to host, handled by `ProcessDiscoveryReply`.
    DiscoveryReply = 12 => "Discovery Reply",
    AssignIdReply = 13 => "Assign ID Reply",
    /// Both ways. Matched to the sent message by message id.
    ConfirmationMessage = 14 => "Confirmation Message",
    MiimReply = 15 => "MIIM Reply",
    /// Host broadcast, no payload (`FUN_083e5300`).
    DevicePoke = 16 => "Device Poke",
    /// Device to host, reply to code 8 (`FUN_083e8210`).
    DeviceStatusMessage = 17 => "Device Status Message",
    /// Not built or parsed by any read build, so no checksum algorithm is known.
    ChannelNamesChecksum = 18 => "Channel Names Checksum",
    /// Host broadcast, no reply expected (`FUN_083e6800`).
    SetMacAddress = 19 => "Set MAC Address",
    /// Host to device for Digital In cards: card index, one bit per channel pair
    /// (dLive `FUN_083e7740`, Qu `FUN_00112068`).
    SetSrcBypass = 20 => "Set SRC Bypass",
    /// Host to device for Digital Out cards: card index, 2-bit rate per channel pair, two
    /// one-hot bytes, a mask, a mode byte (dLive `FUN_083e7940`, Qu `FUN_001121f8`).
    SetDigitalOutputRate = 21 => "Set Digital Output rate",
    /// Host to device, payload: port (`FUN_083e4f40`).
    ConfigureLineOut = 22 => "Configure Line Out",
    /// Host to device (`AddMicPreMessage`).
    DeluxeMicPreSettings = 23 => "Deluxe Mic Pre Settings",
    ReadRegRequest = 24 => "Read Reg Request",
    ReadRegData = 25 => "Read Reg Data",
    /// Device to host (dLive `FUN_083d6b50`, Avantis `FUN_004d8d30`).
    DxHubStatusMessage = 26 => "DX Hub Status Message",
    /// Host to device, 20 payload bytes (`FUN_0821a090`).
    LcdData = 27 => "LCD Data",
    /// Host to device, 2 payload bytes (`FUN_083e6910`).
    LcdBrightness = 28 => "LCD Brightness",
    /// Device to host (dLive `FUN_083e5c50`, Avantis `FUN_004dd800`).
    GetGxStatus = 29 => "Get GX Status",
    WriteReg = 30 => "Write reg",
    /// Both ways: channel, gain (big-endian u16).
    GainShareMessage1 = 31 => "Gain share message1",
    /// Both ways: channel, gain (big-endian u16).
    GainShareMessage2 = 32 => "Gain share message2",
    /// Both ways: channel, flag byte.
    HasMicPre = 33 => "Has mic pre",
    /// Both ways: channel, flag byte.
    PadShareMessage1 = 34 => "Pad share message1",
    /// Both ways: channel, flag byte.
    PadShareMessage2 = 35 => "Pad share message2",
    /// Both ways: channel, flag byte.
    PpShareMessage1 = 36 => "PP share message1",
    /// Both ways: channel, flag byte.
    PpShareMessage2 = 37 => "PP share message2",
    /// Host to device, payload: 0 or 1 (`FUN_083e5040`).
    SetIoCardType = 38 => "Set IO Card type",
    /// Host to device instead of code 5 (Avantis `FUN_004dc6a0`, Qu `FUN_001130e8`).
    EnterUpdateMode48k = 39 => "Enter Update Mode 48k",
    SetDigitalOutputMode = 40 => "Set Digital Output Mode",
    /// Avantis V2.02 name-table index 41 (`FUN_004d8fb0`); dLive puts `"Invalid command"`
    /// here. No Avantis code builds or parses it: the receive switch (`FUN_004e0840`) drops
    /// codes above 29.
    MeDiscoveryReply = 41 => "ME Discovery Reply",
    /// Avantis V2.02 name-table index 42; not built or parsed.
    MeHubStatus = 42 => "ME Hub Status",
    /// Avantis V2.02 name-table index 43; not built or parsed.
    MeDeviceStatus = 43 => "ME Device Status",
}
