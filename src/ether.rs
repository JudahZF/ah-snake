//! Ethernet header templates that the host writes into FPGA header RAM.
//!
//! The FPGA builds the frame from this template; software does not show the rest of the
//! frame. Header RAM takes 32-bit words with the bytes of each word reversed.
//! Evidence: `cGigaACEBase::SetEthernetHeaders` (`08365210`) writes
//! `55 55 55 55 d5 55 55 55 ff ff ff ff ...`; reversing each 4-byte group gives the
//! Ethernet preamble and SFD `55 55 55 55 55 55 55 d5` followed by the broadcast MAC.
//!
//! tSnake (`cTSnakeDriver::SetEthernetHeaders`, `083a94f0`): five 16-byte entries at
//! [`crate::fpga::TSNAKE_HEADER_RAM`]. Each entry, after the swap: destination (driver
//! field `0x520`, set to `ff:ff:ff:ff:ff:ff` by the constructor `FUN_083d9450`), source
//! (driver field `0x4f8 + 8 * i`), type/length `0x00dd`, two zero bytes. The constructor
//! sets the five sources to `base + i` (base = a MAC passed in) or leaves some of them 0,
//! depending on two flags.

use crate::error::Result;
use crate::fpga::{Access, TSNAKE_HEADER_RAM};
use crate::message::Mac;

/// Bytes 12..14 of every tSnake template. As an IEEE 802.3 field below `0x0600` this is
/// a length (221), not an EtherType; which one the FPGA means is not known.
pub const TSNAKE_TYPE_OR_LEN: u16 = 0x00dd;
/// Number of tSnake template entries.
pub const TSNAKE_ENTRIES: usize = 5;

/// Destination every stock template uses.
pub const BROADCAST: Mac = Mac([0xff; 6]);

/// Reverses the bytes of each 4-byte group (header RAM word order). `bytes.len()` must be a
/// multiple of 4; a short tail is left as is.
pub fn swap_words(bytes: &mut [u8]) {
    for w in bytes.as_chunks_mut::<4>().0 {
        w.reverse();
    }
}

/// One tSnake template entry: the 16 bytes as they go into header RAM.
pub fn tsnake_entry(dst: Mac, src: Mac) -> [u8; 16] {
    let mut e = [0; 16];
    e[..6].copy_from_slice(&dst.0);
    e[6..12].copy_from_slice(&src.0);
    e[12..14].copy_from_slice(&TSNAKE_TYPE_OR_LEN.to_be_bytes());
    swap_words(&mut e);
    e
}

/// The `SetEthernetHeaders` command: all five entries with the stock broadcast destination.
pub fn tsnake_set_headers(port: u8, sources: &[Mac; TSNAKE_ENTRIES]) -> Result<Vec<u8>> {
    let data = sources
        .iter()
        .flat_map(|&s| tsnake_entry(BROADCAST, s))
        .collect();
    Access::WriteBlock {
        addr: TSNAKE_HEADER_RAM,
        data,
    }
    .encode(port)
}

/// The five sources the `cTSnakeDriver` constructor sets in its default case: `base + i`,
/// counting the MAC as a 48-bit number. Returns `None` if `base + 4` overflows 48 bits.
pub fn tsnake_default_sources(base: Mac) -> Option<[Mac; TSNAKE_ENTRIES]> {
    let mut n = [0u8; 8];
    n[2..].copy_from_slice(&base.0);
    let b = u64::from_be_bytes(n);
    if b + 4 > 0xffff_ffff_ffff {
        return None;
    }
    let mac = |i: u64| {
        let v = (b + i).to_be_bytes();
        let mut m = [0; 6];
        m.copy_from_slice(&v[2..]);
        Mac(m)
    };
    Some([mac(0), mac(1), mac(2), mac(3), mac(4)])
}
