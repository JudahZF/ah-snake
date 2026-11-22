//! Byte vectors written by hand from the stock dLive Surface V2.12 functions named in
//! each test (constants and offsets read from the disassembly), not from this crate's encoder.

use ah_snake::fpga::{self, Access, RxStatus};
use ah_snake::message::{
    DeviceStatus, DigitalOutputRate, DiscoveryReply, Family, GxStatus, HubStatus, LcdData,
    UpdateData,
};
use ah_snake::micpre::{DSnakeMicPre, DeluxeChannel, DeluxeMicPre, MicPre, MicPreChannel};
use ah_snake::{Body, Error, Mac, Message, MessageCode, ether, tables};

const MAC: Mac = Mac([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

fn roundtrip(bytes: &[u8], body: &Body) {
    let m = Message::decode(bytes).expect("decode");
    assert_eq!(&Body::from_message(&m).expect("body"), body);
    assert_eq!(Message::from_body(m.dest, m.id, body).encode(), bytes);
}

/// FUN_083e51f0 (Discovery) sent by SendTSnakeMessage (083d6530) with message id 7, port 2.
#[test]
fn discovery_through_mailbox() {
    let mut msg = vec![0x0f, 0xff, 0x07, 0x02];
    msg.extend_from_slice(b"DiscoveryV2\0");
    roundtrip(&msg, &Body::Discovery);

    let mut cmd = vec![
        0x00, 30, 0x02, 0, 0, 0, 20, 0x02, 0x1d, 0x00, 0x03, 15, 0x00, 0x07,
    ];
    cmd.extend_from_slice(&msg);
    let m = Message::decode(&msg).expect("decode");
    assert_eq!(fpga::send_message(&m).encode(2).expect("cmd"), cmd);
}

/// FUN_083e53a0 (Assign ID): MAC bytes LSB first, then the id. FUN_083e48a0 (host
/// confirmation): 6, 0xff, echoed id, 14, 0xff, confirmed code, unset byte.
#[test]
fn handshake_messages() {
    roundtrip(
        &[
            0x0a, 0xff, 0x05, 0x03, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x02,
        ],
        &Body::AssignId {
            mac: MAC,
            device: 2,
        },
    );
    roundtrip(
        &[0x06, 0xff, 0x2a, 0x0e, 0xff, 0x1f, 0x00],
        &Body::Confirmation {
            source: 0xff,
            confirmed: 0x1f,
        },
    );
    // ProcessDiscoveryReply (083e5db0): MAC at 4..10, model 10, version 11..13, port 13, cards 14..18.
    roundtrip(
        &[
            0x11, 0x00, 0x09, 0x0c, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x04, b'1', b'2', 0x01,
            0x09, 0x00, 0x09, 0x00,
        ],
        &Body::DiscoveryReply(DiscoveryReply {
            mac: MAC,
            model: 0x04,
            version: *b"12",
            port: 1,
            cards: [9, 0, 9, 0],
        }),
    );
    assert_eq!(tables::device_model(0x04).map(|d| d.name), Some("DX-1608"));

    // Qu-5 FUN_00110e28: version byte below '0' means a dSnake (GLD) device.
    let gld = [
        0x11, 0x00, 0x09, 0x0c, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0x01, 0x29, 0x00, 0, 0,
        0, 0,
    ];
    let Ok(Body::DiscoveryReply(r)) = Body::from_message(&Message::decode(&gld).expect("decode"))
    else {
        panic!("not a discovery reply");
    };
    assert_eq!(r.family(), Family::DSnake);
    assert_eq!(
        tables::QU_DSNAKE_MODELS
            .iter()
            .find(|m| m.code == r.model)
            .map(|m| m.name),
        Some("GLD-AR2412")
    );
}

/// Mic pre forms, byte positions from AddMicPreMessage (083e6f80) and FUN_083d6470 (flag
/// pairs: bit 2k = field 0x19a, bit 2k+1 = field 0x21a). Qu FUN_00111834 builds the same bytes.
#[test]
fn mic_pre_bodies() {
    // 8-channel form, model class 1 (DX-1608): slot s = channel 7 - s, gain code 0x10 + ch.
    // Flag a on channel 0 (slot 7: byte 8, bits 0-1), flag b on channel 5 (slot 2: byte 3,
    // bits 2-3).
    let mut eight = [MicPreChannel::default(); 8];
    for (s, c) in eight.iter_mut().enumerate() {
        c.gain = 0x17 - s as u8;
    }
    eight[7].flag_a = true;
    eight[2].flag_b = true;
    roundtrip(
        &[
            0x0e, 0x02, 0x09, 0x09, 0x01, 0x17, 0x16, 0x08, 0x15, 0x14, 0x13, 0x12, 0x01, 0x11,
            0x10,
        ],
        &Body::MicPreSettings(MicPre::Eight {
            card: 1,
            channels: eight,
        }),
    );

    // 12-channel form, model class 4 (GX-4816): slot s = channel 11 - s. Flag a on channel 0
    // (slot 11) shows in bytes 1 and 9 (bits 4 and 6, F(..,p,p)) and byte 14 (bit 0); flag b
    // on channel 10 (slot 1) in byte 1 bit 1.
    let mut twelve = [MicPreChannel::default(); 12];
    for (s, c) in twelve.iter_mut().enumerate() {
        c.gain = 0x20 + s as u8;
    }
    twelve[11].flag_a = true;
    twelve[1].flag_b = true;
    roundtrip(
        &[
            0x14, 0x02, 0x0a, 0x09, 0x00, 0x52, 0x20, 0x21, 0x22, 0x23, 0x00, 0x24, 0x25, 0x50,
            0x26, 0x27, 0x28, 0x29, 0x01, 0x2a, 0x2b,
        ],
        &Body::MicPreSettings(MicPre::Twelve {
            card: 0,
            channels: twelve,
        }),
    );

    // Deluxe (code 23): gain/3, then 0x30 | a<<3 | b<<2 | step, step 2 for 37 dB (37 % 3 = 1),
    // 3 for 0 dB, 3 for 60 dB.
    let mut dlx = [DeluxeChannel {
        gain_db: 60,
        flag_a: false,
        flag_b: false,
    }; 8];
    dlx[0] = DeluxeChannel {
        gain_db: 37,
        flag_a: true,
        flag_b: false,
    };
    dlx[1] = DeluxeChannel {
        gain_db: 0,
        flag_a: false,
        flag_b: true,
    };
    let mut bytes = vec![0x14, 0x02, 0x0b, 0x17, 0x00, 0x0c, 0x3a, 0x00, 0x37];
    for _ in 0..6 {
        bytes.extend_from_slice(&[0x14, 0x33]);
    }
    roundtrip(
        &bytes,
        &Body::DeluxeMicPreSettings(DeluxeMicPre {
            card: 0,
            channels: dlx,
        }),
    );
    let mut bad = bytes.clone();
    bad[6] = 0x3c; // step 0
    assert_eq!(
        Body::from_message(&Message::decode(&bad).expect("decode")),
        Err(Error::OutOfRange("deluxe mic pre step"))
    );

    // dSnake form, Qu FUN_00112a84 / Avantis FUN_004de090: card, 8 x BE16.
    let ds = DSnakeMicPre {
        card: 1,
        gains: [0x1234, 0, 0, 0, 0, 0, 0, 0x3c00],
    };
    let mut p = vec![0x01, 0x12, 0x34];
    p.extend_from_slice(&[0; 12]);
    p.extend_from_slice(&[0x3c, 0x00]);
    assert_eq!(ds.payload(), p);
    assert_eq!(DSnakeMicPre::from_payload(&p), Ok(ds));
}

/// Card configuration and housekeeping builders.
#[test]
fn config_bodies() {
    // Qu FUN_00112c00(dev 1, key 4, value 3): length 5, code 7.
    roundtrip(
        &[0x05, 0x01, 0x07, 0x07, 0x04, 0x03],
        &Body::SetDeviceConfig { key: 4, value: 3 },
    );
    // Qu FUN_00112068: length 5, card, pair mask. The Surface (FUN_083e7740) sends length 9
    // with 4 unset bytes; they decode to the same body.
    let src = Body::SetSrcBypass {
        card: 1,
        pairs: 0x05,
    };
    roundtrip(&[0x05, 0x03, 0x01, 0x14, 0x01, 0x05], &src);
    let s_form = [0x09, 0x03, 0x01, 0x14, 0x01, 0x05, 0xa5, 0xa5, 0xa5, 0xa5];
    assert_eq!(
        Body::from_message(&Message::decode(&s_form).expect("decode")),
        Ok(src)
    );

    // FUN_083e7940 with settings [0, 1, 2, 0] (codes 3, 2, 1, 3 via table 0x09b5f292) and
    // output mode 1 (FUN_083d5dd0 -> 5): rates 0xdb, one-hot 0x82 / 0x48, mask 0.
    let rate = DigitalOutputRate {
        card: 0,
        rates: 0xdb,
        one_hot: [0x82, 0x48],
        mask: 0,
        mode: Some(5),
    };
    roundtrip(
        &[0x09, 0x03, 0x02, 0x15, 0x00, 0xdb, 0x82, 0x48, 0x00, 0x05],
        &Body::SetDigitalOutputRate(rate),
    );
    // Qu FUN_001121f8: length 8, no mode byte.
    roundtrip(
        &[0x08, 0x03, 0x02, 0x15, 0x00, 0xdb, 0x82, 0x48, 0x00],
        &Body::SetDigitalOutputRate(DigitalOutputRate { mode: None, ..rate }),
    );

    // FUN_083e4f40 (length 4, card), FUN_083e5040 (length 4, 0/1), FUN_083e7d90 calling
    // FUN_083e6910 with ff 60, Avantis FUN_004dc6a0 (code 0x27, length 4).
    roundtrip(
        &[0x04, 0x01, 0x03, 0x16, 0x01],
        &Body::ConfigureLineOut { card: 1 },
    );
    roundtrip(&[0x04, 0x01, 0x04, 0x26, 0x01], &Body::SetIoCardType(1));
    roundtrip(
        &[0x05, 0x01, 0x05, 0x1c, 0xff, 0x60],
        &Body::LcdBrightness([0xff, 0x60]),
    );
    roundtrip(&[0x04, 0x01, 0x06, 0x27, 0x00], &Body::EnterUpdateMode48k);

    // FUN_0821a090: length 0x17, two caller bytes, 8 + 8 copied bytes, two caller bytes.
    let mut lcd = vec![0x17, 0x01, 0x07, 0x1b, 0x01, 0x02];
    lcd.extend_from_slice(b"Line 1  Line 2  ");
    lcd.extend_from_slice(&[0x03, 0x04]);
    roundtrip(
        &lcd,
        &Body::LcdData(LcdData {
            head: [1, 2],
            lines: [*b"Line 1  ", *b"Line 2  "],
            tail: [3, 4],
        }),
    );

    // FUN_0827a8f0 -> FUN_0827a3b0 for channel 5 fed by source type 5, number 2:
    // FUN_0827a5e0 names it "DX1-3", FUN_0827a8b0 reads 0x01 from table 0x09232de0[5],
    // FUN_0827a8e0 returns the number unchanged. Broadcast, length 0x0f.
    roundtrip(
        &[
            0x0f, 0xff, 0x0c, 0x0a, 0x05, b'D', b'X', b'1', b'-', b'3', 0, 0, 0, 0x01, 0x01, 0x02,
        ],
        &Body::ChannelName {
            channel: 5,
            name: *b"DX1-3\0\0",
            stereo_link: 1,
            source_kind: 0x01,
            source_index: 2,
        },
    );
}

/// Device-to-host status bodies at the offsets FUN_083d6b50 (code 26: [4], [6], [12]) and
/// FUN_083e5c50 (code 29: [4], [6], [17]) read.
#[test]
fn status_bodies() {
    roundtrip(
        &[
            0x0c, 0x00, 0x0d, 0x1a, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0x0e,
        ],
        &Body::DxHubStatus(HubStatus {
            device: 8,
            byte5: 0,
            flags: 0x01,
            bytes7_11: [0; 5],
            ports: 0x0e,
        }),
    );
    let mut gx = vec![0x11, 0x00, 0x0e, 0x1d, 0x02, 0x00, 0x90];
    gx.extend_from_slice(&[0; 10]);
    gx.push(0x04);
    roundtrip(
        &gx,
        &Body::GxStatus(GxStatus {
            device: 2,
            byte5: 0,
            flags: 0x90,
            bytes7_16: [0; 10],
            ports: 0x04,
        }),
    );
}

/// FUN_083e55f0 (Update Data), FUN_083e6800 (Set MAC Address), FUN_083e8210 (Device Status
/// reply), FUN_083e8d20 / FUN_083e8dd0 (gain and pad share).
#[test]
fn other_bodies() {
    let mut upd = vec![0x18, 0x03, 0x11, 0x06, 0x01, 0x02, 0x01, 0x23, 0x40];
    upd.extend(0u8..16);
    let mut data = [0; 16];
    data.iter_mut().zip(0u8..).for_each(|(d, i)| *d = i);
    roundtrip(
        &upd,
        &Body::UpdateData(UpdateData {
            last: true,
            address: 0x012340,
            data,
        }),
    );

    roundtrip(
        &[
            0x0e, 0xff, 0x03, 0x13, 0x00, 0x02, 0x07, 0xf0, 0x00, 0x66, 0x55, 0x44, 0x33, 0x22,
            0x11,
        ],
        &Body::SetMacAddress(MAC),
    );
    roundtrip(
        &[
            0x0d, 0xff, 0x08, 0x11, 0x03, 0x00, 0x13, 0x09, 0x00, 0x01, 0x00, 0x01, 0x01, 0x00,
        ],
        &Body::DeviceStatus(DeviceStatus {
            device: 3,
            byte5: 0,
            flags: 0x13,
            cards: [9, 0, 1, 0],
            reconfigure: 1,
            byte12: 1,
            byte13: 0,
        }),
    );
    roundtrip(
        &[0x06, 0x00, 0x04, 0x1f, 0x05, 0x01, 0x80],
        &Body::GainShare {
            second: false,
            channel: 5,
            gain: 0x0180,
        },
    );
    let pad = Message::decode(&[0x05, 0x00, 0x04, 0x22, 0x05, 0x01]).expect("decode");
    assert!(matches!(
        Body::from_message(&pad),
        Ok(Body::FlagShare {
            channel: 5,
            value: 1,
            ..
        })
    ));
}

/// ResetTSnakeRx (083d6290), ResetTSnakeTx (083d6380), CheckForNewMessage (083d6680),
/// ReadTSnakeMessage (083d6760), each for port 2.
#[test]
fn fpga_register_commands() {
    let h = |len: u8, ep: u8| vec![0x00, len, 0x02, 0, 0, 0, len - 10, 0x02, ep, 0x00];
    let cat = |mut a: Vec<u8>, b: &[u8]| {
        a.extend_from_slice(b);
        a
    };
    assert_eq!(
        fpga::TSNAKE_RESET_RX.encode(2).expect("cmd"),
        cat(h(14, 0x1d), &[1, 3, 2, 1])
    );
    assert_eq!(
        fpga::TSNAKE_RESET_TX.encode(2).expect("cmd"),
        cat(h(14, 0x1d), &[1, 6, 1, 1])
    );
    assert_eq!(
        fpga::CHECK_FOR_MESSAGE.encode(2).expect("cmd"),
        cat(h(14, 0x1c), &[0, 0, 8, 0])
    );
    let read = fpga::read_message(5);
    assert_eq!(
        read.encode(2).expect("cmd"),
        cat(h(20, 0x1c), &[2, 5, 1, 8, 0, 0, 0, 0, 0, 0])
    );
    assert_eq!(read.reply_len(), Some(10));

    // FUN_083e8560: status byte is reply byte 3; message is the reply after 4 bytes.
    assert_eq!(
        fpga::read_byte_reply(&[0, 0, 0, 5]).map(RxStatus::from_byte),
        Ok(RxStatus::Pending(5))
    );
    let reply = [9, 9, 9, 9, 0x05, 0xff, 0x01, 0x10, 0xaa, 0xbb];
    let msg = fpga::read_block_reply(&reply, 6).expect("block");
    assert_eq!(
        Message::decode(msg).map(|m| m.code),
        Ok(MessageCode::DevicePoke)
    );
}

/// cTSnakeDriver::SetEthernetHeaders (083a94f0): entry bytes in the order the function
/// stores them, with the constructor's broadcast destination.
#[test]
fn tsnake_header_template() {
    let entry = [
        0xff, 0xff, 0xff, 0xff, 0x22, 0x11, 0xff, 0xff, 0x66, 0x55, 0x44, 0x33, 0, 0, 0xdd, 0,
    ];
    assert_eq!(ether::tsnake_entry(ether::BROADCAST, MAC), entry);

    let srcs = ether::tsnake_default_sources(MAC).expect("sources");
    assert_eq!(srcs[4], Mac([0x11, 0x22, 0x33, 0x44, 0x55, 0x6a]));
    let cmd = ether::tsnake_set_headers(1, &srcs).expect("cmd");
    assert_eq!(cmd.len(), 0x5e);
    assert_eq!(
        &cmd[..14],
        &[
            0x00, 0x5e, 0x02, 0, 0, 0, 0x54, 0x01, 0x1d, 0x00, 0x03, 0x4f, 0x00, 0x03
        ]
    );
    assert_eq!(&cmd[14..30], &entry);
}

/// The table index is the code: names checked against the strings at 0x09b62de0 + 32 * code.
#[test]
fn message_codes() {
    assert_eq!(
        MessageCode::from_u8(12).map(MessageCode::name),
        Some("Discovery Reply")
    );
    assert_eq!(
        MessageCode::from_u8(14).map(MessageCode::name),
        Some("Confirmation Message")
    );
    assert_eq!(
        MessageCode::from_u8(40).map(MessageCode::name),
        Some("Set Digital Output Mode")
    );
    assert_eq!(MessageCode::from_u8(44), None);
    for (i, c) in MessageCode::ALL.iter().enumerate() {
        assert_eq!(*c as usize, i);
    }
}

/// Decoders must reject bad input with an error and never panic.
#[test]
fn decode_errors() {
    assert_eq!(
        Message::decode(&[0x02, 0, 0]),
        Err(Error::TooShort { need: 4, got: 3 })
    );
    assert_eq!(
        Message::decode(&[0x04, 0, 0, 1]),
        Err(Error::LengthMismatch {
            stated: 5,
            actual: 4
        })
    );
    assert_eq!(
        Message::decode(&[0x03, 0, 0, 0x50]),
        Err(Error::UnknownCode(0x50))
    );
    assert_eq!(Message::decode(&[0u8; 26]), Err(Error::TooLong(26)));
    let short = Message::decode(&[0x04, 0, 0, 0x0c, 0]).expect("decode");
    assert!(matches!(
        Body::from_message(&short),
        Err(Error::TooShort { .. })
    ));
    for len in 0..=26usize {
        for code in 0..=45u8 {
            let mut b = vec![0xa5; len];
            if len > 3 {
                b[0] = (len - 1) as u8;
                b[3] = code;
            }
            if let Ok(m) = Message::decode(&b) {
                let _ = Body::from_message(&m);
            }
        }
    }
    assert_eq!(
        Access::ReadBlock { addr: 0, len: 0 }.encode(0),
        Err(Error::OutOfRange("block length"))
    );
}
