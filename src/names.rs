//! Names only (provenance: strings). Nothing here has a wire value.

/// tSnake driver and device-manager C++ classes (RTTI / pretty-function names) in dLive
/// Surface, dLive MixRack and Avantis.
pub const SNAKE_CLASSES: &[&str] = &[
    "cTSnakeDeviceManager",
    "cTSnakeDriverBase",
    "cTSnakeDriver",
    "cTSnakeNetObject",
    "cTSnakeStatusManager",
    "cTSnakeStatusManagerParent",
    "cPortBPlusTSnakeDriver",
    "cPortBPlusTSnakeDriverBase",
    "cStageBoxTSnakeDeviceManager",
    "cStageBoxTSnakeAnalogueInputDriver",
    "cStageBoxTSnakeDigitalInputDriver",
    "cStageBoxTSnakeDigitalOutputDriver",
    "cStageBoxTSnakeAESOutModeShowHelper",
    "cSurfaceTSnakeAnalogueInputDriver",
    "cSurfaceTSnakeDigitalInputDriver",
    "cSurfaceTSnakeDigitalOutputDriver",
    "cSurfacePortBPlusTSnakeDigitalOutputDriver",
    "cSurfaceTSnakeAESOutModeShowHelper",
    "cTSnakeAESOutModeArchiverRestorer",
];

/// Avantis V2.02 device-model names. Their codes were not read.
pub const AVANTIS_DEVICE_MODELS: &[&str] = &[
    "DX-32 Proto",
    "DX-32 sch 1.1",
    "DX-1608",
    "DX-1608LE",
    "DX-1604",
    "DX-012",
    "DX-88",
    "DX Hub Proto",
    "DX Hub",
    "GLD-AR2412",
    "GLD-AR84",
    "GLD-AB168",
    "GLD-AR2412 XC3S400A",
    "GLD-AR84 XC3S400A",
    "GLD-AB168 XC3S400A",
    "GX4816",
    "GX-DualMaster",
    "DX-DualMaster",
    "dSnake-DualMaster",
];

/// tSnake / dSnake log text in Qu-5 V1.2.1 (`0x781444..0x7815d0`), also in SQ-5 and SQ-5+.
pub const QU_SQ_LOG: &[&str] = &[
    "tS: Q[%u] Full. Len %lu",
    "TSnakeTxRx too much data!",
    "tS: Discovery reply for %s",
    "Discovery reply from unknown device type 0x%2X",
    "dS: Discovery reply for %s ",
    "tS: MAC %02X:%02X:%02X:%02X:%02X:%02X",
    "tS: Unrecognised card type: %u",
    "tS: Current FW v.%c %c",
    "tS: Device will be updated to v.%c %c",
    "tS: Current FW v.0x%02x 0x%02x",
    "tS: Device will be updated to v.0x%02x 0x%02x",
];

/// Port-type label in the SQ-5 and SQ-5+ UI, next to `gigaACE/GX`.
pub const SQ_DSNAKE_LABEL: &str = "dSNAKE";

/// Avantis log text. A member named `mNewSnakeProtocol` exists; its meaning is not known.
pub const NEW_SNAKE_PROTOCOL_LOG: &str = "Unhandled mNewSnakeProtocol %d";
