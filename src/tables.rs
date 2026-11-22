//! Numeric tables read from the stock firmware data.

/// One entry of the dLive device-model table: a 40-byte record at `0x09b63600` in dLive
/// Surface V2.12 (`u8 code; char name[32]; u8 b21, b22, b23; u32 kind`).
/// `ProcessDiscoveryReply` (`083e5db0`) finds the entry whose `code` equals payload byte 6
/// of a Discovery Reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceModel {
    /// Model code on the wire.
    pub code: u8,
    /// Name in the record.
    pub name: &'static str,
    /// Record byte `0x21`. Inferred: card slots.
    pub slots: u8,
    /// Record byte `0x22`. Inferred: input channels (read by `FUN_083d52a0`).
    pub inputs: u8,
    /// Record byte `0x23`. Inferred: output channels (read by `FUN_083d5310`).
    pub outputs: u8,
    /// Record `u32` at `0x24`: internal device class (all values fit a byte). Uses seen:
    /// `FUN_083d5e30` tests 3 or 4 (then the device goes to hub slots 8/9 and is logged as
    /// "Hub %d"), `FUN_083d5ec0` tests 7 (then the host sends `LCD Brightness`),
    /// `ValidateUnitCardConfiguration` tests 9 (DX-88 card check).
    pub kind: u8,
}

const fn m(
    code: u8,
    name: &'static str,
    slots: u8,
    inputs: u8,
    outputs: u8,
    kind: u8,
) -> DeviceModel {
    DeviceModel {
        code,
        name,
        slots,
        inputs,
        outputs,
        kind,
    }
}

/// All 21 records, in table order. `A-1604WLCD` has two codes. The leading byte that the
/// old strings-only stub left out is the code.
pub const DLIVE_DEVICE_MODELS: &[DeviceModel] = &[
    m(0x00, "DX-32 Proto", 4, 32, 32, 0),
    m(0x01, "DX-32", 4, 32, 32, 0),
    m(0x02, "DX-32 sch 1.1", 4, 32, 32, 0),
    m(0x04, "DX-1608", 2, 16, 8, 1),
    m(0x2c, "DX-1608LE", 2, 16, 8, 1),
    m(0x08, "DX Hub Proto", 0, 128, 128, 3),
    m(0x09, "DX Hub", 0, 128, 128, 3),
    m(0x0c, "DX-1604W", 2, 16, 4, 2),
    m(0x24, "DX-1604WLCD", 2, 16, 4, 2),
    m(0x10, "A-1608", 2, 16, 8, 6),
    m(0x14, "A-1604W", 2, 16, 4, 7),
    m(0x20, "A-1604WLCD", 2, 16, 4, 7),
    m(0x21, "A-1604WLCD", 2, 16, 4, 7),
    m(0x18, "A-1604FLCD", 2, 16, 4, 7),
    m(0x1d, "AStar", 0, 128, 128, 8),
    m(0x28, "DX-012", 1, 0, 16, 5),
    m(0x30, "GX-4816", 4, 48, 16, 4),
    m(0x31, "GX-4816", 4, 48, 16, 4),
    m(0x54, "DX-88", 2, 16, 16, 9),
    m(0x68, "DX-USB", 1, 8, 8, 10),
    m(0x5c, "DX-84", 1, 8, 4, 11),
];

/// Looks up a model code.
pub fn device_model(code: u8) -> Option<&'static DeviceModel> {
    DLIVE_DEVICE_MODELS.iter().find(|d| d.code == code)
}

/// Card-type names, indexed by card code. Proven: `FUN_083d5cf0` (dLive Surface) returns
/// `"Not Fitted" + code * 32` for `code < 32`, else `"Invalid Card ID"`. The same table is in
/// MixRack and Avantis (Avantis spells index 27 `GX2412_DX1608R IO`). Index 26 and 30 are
/// both `DX84 IO` in dLive.
pub const CARD_TYPES: [&str; 32] = [
    "Not Fitted",
    "Mic In",
    "Line Out",
    "Digital In",
    "Digital Out",
    "Mic In DLX -Old",
    "Line Out DLX",
    "Mic In DLX",
    "Reserved",
    "DX1608 IO",
    "DX1604 IO",
    "GX4816 IO",
    "DX012",
    "DX1608 Type2 IO",
    "DX1608 Type2B IO",
    "DX012 Type2",
    "GX4816 Type2 IO",
    "DX1604 Type2 IO",
    "DX88 Mic In",
    "DX88 Line Out",
    "DX88 Mic/Line",
    "DX32 Emerick Prime Pre",
    "DX32 Emerick Prime Out",
    "DX32 Emerick Prime Pre variant",
    "DX32 Emerick Mic Pre",
    "DX32 Emerick Mic Pre variant",
    "DX84 IO",
    "GX2412 IO",
    "DX168 Mk2 IO",
    "DX-USB IO",
    "DX84 IO",
    "Invalid",
];

/// Qu-5 V1.2.1 card table (34-byte records at `0x210e0c`: `char name[32]; u8; u8`), with
/// the record index. Empty records (5, 8, 14, 21..=25) are left out. Inferred: the index is
/// the card code, because it lines up with [`CARD_TYPES`] (9 = DX1608 IO, 11 = GX4816 IO,
/// 26 = DX84 IO, ...). No Qu lookup code was read. SQ-5+ adds `DX-USB IO` (index not read).
pub const QU_CARD_TYPES: &[(u8, &str)] = &[
    (0, "Not Fitted"),
    (1, "Mic In"),
    (2, "Line Out"),
    (3, "Digital In"),
    (4, "Digital Out"),
    (6, "Line Out DLX"),
    (7, "Mic In DLX"),
    (9, "DX1608 IO"),
    (10, "DX164-W IO"),
    (11, "GX4816 IO"),
    (12, "DX012 Out"),
    (13, "DX1608 Type2 IO"),
    (15, "DX012 Type2 Out"),
    (16, "GX4816 Type2 IO"),
    (17, "DX164-W Type2IO"),
    (18, "DX88LE In"),
    (19, "DX88LE Out"),
    (20, "DX88LE IO"),
    (26, "DX84 IO"),
    (27, "GX2412/DX1608R IO"),
];

/// Disconnect reasons (`cTSnakeDeviceManager::eDisconnectReason`), indexed by value. Proven:
/// `FUN_083d5d50` returns `"Lost link" + reason * 64` for `reason < 13`, else
/// `"Unknown reason"`. `ProcessDiscoveryReply` passes 12 for a second device with the same MAC.
pub const DISCONNECT_REASONS: [&str; 13] = [
    "Lost link",
    "Lost redundant link",
    "Cascaded device",
    "Timed out",
    "Soft reset",
    "Update complete",
    "Redundant mode enabled",
    "Lost upstream DX Hub",
    "Lost link on DX Hub",
    "Update timed out",
    "Lost link to DX Hub",
    "Assign ID message timed out",
    "Discovery reply from same MAC address",
];

/// One record of a Qu-5 V1.2.1, SQ-5 V1.6.3 or SQ-5+ V2.0.3 model table (38 bytes:
/// `char name[32]; u8 b20, b21, b22; u8 fw[2]; u8 flags`). Proven: `FUN_00110e28` (Qu-5) prints
/// the name at `table + code * 0x26`, code = Discovery Reply payload byte 6, and compares `fw`
/// with the reply version. The SQ images do the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuModel {
    /// Model code on the wire (record index).
    pub code: u8,
    /// Name in the record.
    pub name: &'static str,
    /// Bytes `0x20..0x23`. Inferred: card slots, inputs, outputs (same values as
    /// [`DeviceModel`] where the models match).
    pub slots_in_out: [u8; 3],
    /// Bytes `0x23..0x25`: the firmware version the Qu expects; a different reply version
    /// starts an update ("Device will be updated to v.").
    pub fw: [u8; 2],
    /// Byte `0x25`. Bits 0, 1 and 3 select code paths in `FUN_00110e28`; meaning not named.
    pub flags: u8,
}

const fn q(code: u8, name: &'static str, s: [u8; 3], fw: [u8; 2], flags: u8) -> QuModel {
    QuModel {
        code,
        name,
        slots_in_out: s,
        fw,
        flags,
    }
}

/// Qu-5 tSnake (DX / GX) models, table at `0x6d0d20`, used when the reply version is ASCII
/// (byte >= `'0'`). Codes match [`DLIVE_DEVICE_MODELS`] except the last three: Qu has
/// `DX84` at `0x74`, dLive V2.12 has `DX-84` at `0x5c`.
pub const QU_TSNAKE_MODELS: &[QuModel] = &[
    q(0x00, "DX32 Proto", [4, 32, 32], *b"41", 0x02),
    q(0x01, "DX32", [4, 32, 32], *b"41", 0x02),
    q(0x02, "DX32", [4, 32, 32], *b"41", 0x02),
    q(0x04, "DX1608", [2, 16, 8], *b"46", 0x0a),
    q(0x08, "DX Hub Proto", [0, 128, 128], *b"37", 0x01),
    q(0x09, "DX Hub", [0, 128, 128], *b"37", 0x01),
    q(0x0c, "DX164-W", [2, 16, 4], *b"34", 0x0a),
    q(0x28, "DX012", [1, 0, 16], *b"30", 0x0a),
    q(0x2c, "DX1608LE", [2, 16, 8], *b"66", 0x0a),
    q(0x30, "GX4816", [4, 48, 16], *b"52", 0x03),
    q(0x31, "GX4816 Type 2", [4, 48, 16], *b"52", 0x03),
    q(0x54, "DX88LE", [2, 8, 8], *b"65", 0x0a),
    q(0x60, "GX2412", [3, 24, 12], *b"26", 0x0b),
    q(0x70, "DX168R", [2, 16, 8], *b"32", 0x0a),
    q(0x74, "DX84", [1, 8, 4], *b"32", 0x0a),
];

/// Qu-5 dSnake (GLD AudioRack) models, table at `0x2112d0`, used when the reply version
/// byte is below `'0'` (binary version, logged as `v.0x%02x 0x%02x`).
pub const QU_DSNAKE_MODELS: &[QuModel] = &[
    q(0x00, "GLD-AR2412", [3, 24, 12], [0x01, 0x29], 0),
    q(0x01, "GLD-AR84", [1, 8, 4], [0x01, 0x29], 0),
    q(0x02, "GLD-AB168", [2, 16, 8], [0x01, 0x29], 0),
    q(0x03, "GLD-AR Comms Unknown", [0, 0, 0], [0, 0], 0),
    q(0x04, "GLD-80", [0, 48, 48], [0, 0], 0),
    q(0x10, "GLD-AR2412 XC3S400A", [3, 24, 12], [0x02, 0x30], 0),
    q(0x11, "GLD-AR84 XC3S400A", [1, 8, 4], [0x02, 0x30], 0),
    q(0x12, "GLD-AB168 XC3S400A", [2, 16, 8], [0x02, 0x30], 0),
];

/// SQ-5 V1.6.3 tSnake models: table at `0x831f3c` (Ghidra address, image base `0x100000`),
/// same record layout, same selection rule (ASCII version byte) and the same handler shape as
/// the Qu-5 (`FUN_001141e0`, case 12 of the message switch). Differences from the Qu-5 table,
/// proven by reading both: the expected firmware versions of `GX4816`, `GX4816 Type 2`,
/// `GX2412`, `DX168R`, and `DX84`, which sits at `0x5c` (the code dLive V2.12 uses) instead of
/// `0x74`.
pub const SQ5_TSNAKE_MODELS: &[QuModel] = &[
    q(0x00, "DX32 Proto", [4, 32, 32], *b"41", 0x02),
    q(0x01, "DX32", [4, 32, 32], *b"41", 0x02),
    q(0x02, "DX32", [4, 32, 32], *b"41", 0x02),
    q(0x04, "DX1608", [2, 16, 8], *b"46", 0x0a),
    q(0x08, "DX Hub Proto", [0, 128, 128], *b"37", 0x01),
    q(0x09, "DX Hub", [0, 128, 128], *b"37", 0x01),
    q(0x0c, "DX164-W", [2, 16, 4], *b"34", 0x0a),
    q(0x28, "DX012", [1, 0, 16], *b"30", 0x0a),
    q(0x2c, "DX1608LE", [2, 16, 8], *b"66", 0x0a),
    q(0x30, "GX4816", [4, 48, 16], *b"50", 0x03),
    q(0x31, "GX4816 Type 2", [4, 48, 16], *b"50", 0x03),
    q(0x54, "DX88LE", [2, 8, 8], *b"65", 0x0a),
    q(0x5c, "DX84", [1, 8, 4], *b"20", 0x0a),
    q(0x60, "GX2412", [3, 24, 12], *b"13", 0x0b),
    q(0x70, "DX168R", [2, 16, 8], *b"20", 0x0a),
];

/// SQ-5+ V2.0.3 tSnake models: table at `0xca2440`. Same as the Qu-5 table except `DX-USB`
/// (`0x68`, new in this image) and the expected firmware versions of `GX2412`, `DX168R` and
/// `DX84`. Proven by reading the table in the image.
pub const SQ5P_TSNAKE_MODELS: &[QuModel] = &[
    q(0x00, "DX32 Proto", [4, 32, 32], *b"41", 0x02),
    q(0x01, "DX32", [4, 32, 32], *b"41", 0x02),
    q(0x02, "DX32", [4, 32, 32], *b"41", 0x02),
    q(0x04, "DX1608", [2, 16, 8], *b"46", 0x0a),
    q(0x08, "DX Hub Proto", [0, 128, 128], *b"37", 0x01),
    q(0x09, "DX Hub", [0, 128, 128], *b"37", 0x01),
    q(0x0c, "DX164-W", [2, 16, 4], *b"34", 0x0a),
    q(0x28, "DX012", [1, 0, 16], *b"30", 0x0a),
    q(0x2c, "DX1608LE", [2, 16, 8], *b"66", 0x0a),
    q(0x30, "GX4816", [4, 48, 16], *b"52", 0x03),
    q(0x31, "GX4816 Type 2", [4, 48, 16], *b"52", 0x03),
    q(0x54, "DX88LE", [2, 8, 8], *b"65", 0x0a),
    q(0x60, "GX2412", [3, 24, 12], *b"16", 0x0b),
    q(0x68, "DX-USB", [1, 8, 8], *b"13", 0x0a),
    q(0x70, "DX168R", [2, 16, 8], *b"26", 0x0a),
    q(0x74, "DX84", [1, 8, 4], *b"26", 0x0a),
];

// The dSnake (GLD AudioRack) table is byte for byte the same in the Qu-5 (`0x2112d0`), SQ-5
// (`0x8334cc`) and SQ-5+ (`0x877154`) images for every row (codes 0..=4 and 0x10..=0x12), so
// [`QU_DSNAKE_MODELS`] serves all three.
