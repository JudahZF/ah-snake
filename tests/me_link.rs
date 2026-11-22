//! The ME-1 control block (`0xD1..0xE1`, nibble-swapped on the wire) and the tSnake
//! `Channel Names` message carry the same bytes after byte 0: byte `0xD1` is the ME-1
//! control-present flag (`0x40`) where tSnake carries its length byte (`len - 1 = 15`).

use ah_snake::{Body, Message};

#[test]
fn me_control_block_is_a_tsnake_channel_names_message() {
    // Un-swapped ME-1 control block for channel 3, name "Kick", stereo=true, flags (7, 5).
    let me_block: [u8; 16] = [
        0x40, 0xff, 0x00, 0x0a, 0x03, b'K', b'i', b'c', b'k', 0x00, 0x00, 0x00, 0x00, 0x01, 0x07,
        0x05,
    ];

    let message = Message::from_body(
        0xff,
        0,
        &Body::ChannelName {
            channel: 3,
            name: *b"Kick\0\0\0",
            stereo_link: 1,
            source_kind: 7,
            source_index: 5,
        },
    )
    .encode();

    assert_eq!(me_block[0], 0x40, "ME-1 presence byte");
    assert_eq!(message[0], 15, "tSnake length byte (message is 16 bytes)");
    assert_eq!(me_block.len(), message.len());
    assert_eq!(me_block[1..], message[1..]);
}
