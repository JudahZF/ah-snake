//! Codec for the Allen & Heath tSnake stage-box control link (dLive, Avantis; dSnake in
//! Qu / SQ), recovered by disassembling stock firmware. Never tested on a real unit.
//!
//! Three layers, all from dLive Surface V2.12 (`TLDSurface`, x86) and checked against
//! dLive MixRack V2.12 (`TLDStageBox`, ARM):
//!
//! - [`message`]: the control message the host exchanges with stage boxes
//!   (`cTSnakeDeviceManager::sTSnakeMessage`): length, destination, message id, code, payload.
//!   [`micpre`] holds the mic pre payloads.
//! - [`fpga`]: the host-to-FPGA command that carries a message to the FPGA mailbox, and
//!   the register access format shared with gigaACE.
//! - [`ether`]: the Ethernet header template the host writes into FPGA header RAM.
//!
//! The FPGA builds the real frame from the template and the mailbox bytes. Where the
//! message sits in that frame, padding and any CRC are FPGA logic and not visible here.
//! [`tables`] holds the numeric device-model, card-type and disconnect-reason tables.
//! [`names`] holds strings without wire values.
//!
//! Plain codec: bytes in, typed values out, and back. No I/O.

mod code;
mod error;
pub mod ether;
pub mod fpga;
pub mod message;
pub mod micpre;
pub mod names;
pub mod tables;

pub use code::MessageCode;
pub use error::{Error, Result};
pub use message::{Body, Mac, Message};

/// Where the content of a crate comes from.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Recovered by disassembling stock firmware. Never tested against a real unit.
    Disasm,
}

/// Provenance of the codec in this crate. [`names`] is strings only.
pub const PROVENANCE: Provenance = Provenance::Disasm;
