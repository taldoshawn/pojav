//! Independent byte fixtures and real loopback sockets; no Minecraft assets or
//! player accounts are required. These establish wire behavior, not gameplay.
use horizon_protocol::{
    Error,
    buf::{Reader, Writer},
    connection::Connection,
    crypto::server_hash,
    legacy18::{
        self, LoginPacket,
        chunk::Column,
        play::{self, Packet, Pose},
    },
    rsa::RsaPrivateKey,
};
use std::{
    net::{TcpListener, TcpStream},
    time::Duration,
};

fn loopback() -> (Connection, Connection) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    let client = Connection::from_stream(client).unwrap();
    let server = Connection::from_stream(server).unwrap();
    server
        .reader
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    (client, server)
}

fn success() -> Vec<u8> {
    let mut bytes = vec![2, 36];
    bytes.extend_from_slice(b"00112233-4455-6677-8899-aabbccddeeff");
    bytes.extend_from_slice(b"\x04Alex");
    bytes
}

fn assert_hello(server: &mut Connection) {
    let (id, body) = server.recv().unwrap();
    assert_eq!(id, 0);
    assert_eq!(body, b"\x2f\x09localhost\x63\xdd\x02");
    assert_eq!(server.recv().unwrap(), (0, b"\x04Alex".to_vec()));
}

#[test]
fn offline_login_enters_play_without_modern_ack_and_answers_keepalive() {
    for threshold in [None, Some(0), Some(256)] {
        let (client, mut server) = loopback();
        let worker = std::thread::spawn(move || {
            assert_hello(&mut server);
            if let Some(threshold) = threshold {
                let bytes = if threshold == 0 {
                    vec![3, 0]
                } else {
                    vec![3, 0x80, 2]
                };
                server.send(&bytes).unwrap();
                server.set_compression(threshold);
            }
            server.send(&success()).unwrap();
            // A signed VarInt keepalive, not the modern eight-byte long.
            server.send(&[0, 0xff, 0xff, 0xff, 0xff, 0x0f]).unwrap();
            assert_eq!(
                server.recv().unwrap(),
                (0, vec![0xff, 0xff, 0xff, 0xff, 0x0f])
            );
            server.send(b"\x40\x0e{\"text\":\"bye\"}").unwrap();
        });
        let (mut client, profile) = legacy18::login(
            client,
            "localhost",
            25565,
            "Alex",
            Duration::from_secs(5),
            |_| panic!("offline server requested no authentication"),
        )
        .unwrap();
        assert_eq!(profile.name, "Alex");
        assert_eq!(profile.uuid, 0x00112233445566778899aabbccddeeff);
        let (id, bytes) = client.recv().unwrap();
        assert_eq!(Packet::decode(id, &bytes).unwrap(), Packet::KeepAlive(-1));
        client.send(&play::keep_alive(-1)).unwrap();
        let (id, bytes) = client.recv().unwrap();
        assert_eq!(
            Packet::decode(id, &bytes).unwrap(),
            Packet::Disconnect("{\"text\":\"bye\"}".into())
        );
        worker.join().unwrap();
    }
}

#[test]
fn online_login_authenticates_before_encrypted_compressed_play() {
    let key = RsaPrivateKey::generate(1024).unwrap();
    let public = key.public_der();
    let (client, mut server) = loopback();
    let (send_hash, receive_hash) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        assert_hello(&mut server);
        let mut request = Writer::new();
        request.varint(1);
        request.string("");
        request.byte_array(&public);
        request.byte_array(&[0xa1, 0xb2, 0xc3, 0xd4]);
        // Protocol 47 has no should_authenticate boolean after the token.
        server.send(&request.data).unwrap();
        let (id, bytes) = server.recv().unwrap();
        assert_eq!(id, 1);
        let mut r = Reader::new(&bytes);
        let secret: [u8; 16] = key
            .decrypt(&r.byte_array(256).unwrap())
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(
            key.decrypt(&r.byte_array(256).unwrap()).unwrap(),
            [0xa1, 0xb2, 0xc3, 0xd4]
        );
        assert_eq!(r.remaining(), 0);
        assert_eq!(
            receive_hash.recv_timeout(Duration::from_secs(5)).unwrap(),
            server_hash("", &secret, &public)
        );
        server.enable_encryption(&secret);
        server.send(&[3, 0]).unwrap();
        server.set_compression(0);
        server.send(&success()).unwrap();
        server.send(&[0, 0xac, 2]).unwrap();
        assert_eq!(server.recv().unwrap(), (0, vec![0xac, 2]));
    });
    let (mut client, _) = legacy18::login(
        client,
        "localhost",
        25565,
        "Alex",
        Duration::from_secs(5),
        |hash| {
            send_hash.send(hash.to_owned()).unwrap();
            Ok(())
        },
    )
    .unwrap();
    let (id, bytes) = client.recv().unwrap();
    assert_eq!(Packet::decode(id, &bytes).unwrap(), Packet::KeepAlive(300));
    client.send(&play::keep_alive(300)).unwrap();
    worker.join().unwrap();
}

#[test]
fn refused_authentication_closes_instead_of_falling_back_offline() {
    let key = RsaPrivateKey::generate(1024).unwrap();
    let (client, mut server) = loopback();
    let worker = std::thread::spawn(move || {
        assert_hello(&mut server);
        let mut request = Writer::new();
        request.varint(1);
        request.string("");
        request.byte_array(&key.public_der());
        request.byte_array(&[1, 2, 3, 4]);
        server.send(&request.data).unwrap();
        assert!(server.recv().unwrap_err().is_end_of_stream());
    });
    let result = legacy18::login(
        client,
        "localhost",
        25565,
        "Alex",
        Duration::from_secs(5),
        |_| Err(Error::new("session refused")),
    );
    assert_eq!(result.err().unwrap().to_string(), "session refused");
    worker.join().unwrap();
}

#[test]
fn malformed_login_and_duplicate_negotiation_are_rejected() {
    let valid = success();
    for end in 0..valid.len() - 1 {
        assert!(LoginPacket::decode(2, &valid[1..=end]).is_err());
    }
    let mut trailing = valid[1..].to_vec();
    trailing.push(0);
    assert!(LoginPacket::decode(2, &trailing).is_err());
    let mut bad_uuid = valid[1..].to_vec();
    bad_uuid[9] = b'0'; // Replace the first required hyphen.
    assert!(LoginPacket::decode(2, &bad_uuid).is_err());
    assert!(LoginPacket::decode(4, &[]).is_err());
    let (client, mut server) = loopback();
    let worker = std::thread::spawn(move || {
        assert_hello(&mut server);
        server.send(&[3, 0]).unwrap();
        server.set_compression(0);
        server.send(&[3, 0]).unwrap();
    });
    let result = legacy18::login(
        client,
        "localhost",
        25565,
        "Alex",
        Duration::from_secs(5),
        |_| Ok(()),
    );
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("repeated compression")
    );
    worker.join().unwrap();
}

#[test]
fn silent_peer_times_out_and_invalid_inputs_send_nothing() {
    let (client, _server) = loopback();
    let result = legacy18::login(
        client,
        "localhost",
        25565,
        "Alex",
        Duration::from_millis(20),
        |_| Ok(()),
    );
    assert!(result.err().unwrap().is_timed_out());
    for (host, name) in [
        ("localhost", ""),
        ("localhost", "bad name"),
        ("bad\0host", "Alex"),
    ] {
        let (client, mut server) = loopback();
        assert!(
            legacy18::login(
                client,
                host,
                25565,
                name,
                Duration::from_secs(1),
                |_| Ok(())
            )
            .is_err()
        );
        assert!(server.recv().unwrap_err().is_end_of_stream());
    }
}

fn column_bytes(mask: u16, ground_up: bool, data: &[u8]) -> Vec<u8> {
    let mut w = Writer::new();
    w.i32(-2);
    w.i32(3);
    w.bool(ground_up);
    w.u16(mask);
    w.byte_array(data);
    w.data
}

#[test]
fn sparse_sections_keep_little_endian_states_and_separate_light_arrays() {
    // Sections y=0 and y=15, with distinct patterns across each array.
    let mut data = vec![0; 2 * (8192 + 2048 + 2048) + 256];
    data[0..4].copy_from_slice(&[0x31, 0x12, 0x42, 0x23]);
    data[8192..8194].copy_from_slice(&[0x54, 0x03]);
    data[16384] = 0xa3;
    data[16384 + 2048] = 0xb4;
    data[20480] = 0xc5;
    data[20480 + 2048] = 0xd6;
    data[24576] = 7;
    let bytes = column_bytes(0x8001, true, &data);
    let column = Column::decode(&bytes, true).unwrap();
    assert_eq!((column.x, column.z), (-2, 3));
    assert!(!column.is_unload());
    let bottom = column.section(0).unwrap();
    assert_eq!(bottom.state(0, 0, 0), Some(0x1231));
    assert_eq!(bottom.state(1, 0, 0), Some(0x2342));
    assert_eq!(bottom.block_light(0, 0, 0), Some(3));
    assert_eq!(bottom.block_light(1, 0, 0), Some(10));
    assert_eq!(bottom.sky_light(1, 0, 0), Some(12));
    let top = column.section(15).unwrap();
    assert_eq!(top.state(0, 0, 0), Some(0x0354));
    assert_eq!(top.block_light(1, 0, 0), Some(11));
    assert_eq!(top.sky_light(0, 0, 0), Some(6));
    assert_eq!(column.biomes().unwrap()[0], 7);
    assert!(column.section(1).is_none());
    assert!(column.section(16).is_none());
    assert!(top.state(16, 0, 0).is_none());
    assert!(top.block_light(0, 16, 0).is_none());
}

#[test]
fn chunk_dimensions_partial_updates_unloads_and_length_limits() {
    let partial = column_bytes(2, false, &vec![0; 8192 + 2048]);
    let nether = Column::decode(&partial, false).unwrap();
    assert!(nether.biomes().is_none());
    assert!(nether.section(1).unwrap().sky_light(0, 0, 0).is_none());
    assert!(Column::decode(&partial, true).is_err());
    for data in [vec![], vec![0; 256]] {
        let bytes = column_bytes(0, true, &data);
        assert!(Column::decode(&bytes, true).unwrap().is_unload());
    }
    let no_change = column_bytes(0, false, &[]);
    assert!(!Column::decode(&no_change, false).unwrap().is_unload());
    for end in [0, 8, 10, 11, partial.len() - 1] {
        assert!(Column::decode(&partial[..end], false).is_err());
    }
    let mut trailing = partial.clone();
    trailing.push(0);
    assert!(Column::decode(&trailing, false).is_err());
    let too_large = column_bytes(1, false, &vec![0; 8192 + 2048 + 1]);
    assert!(Column::decode(&too_large, false).is_err());
    let mut bad_length = no_change[..11].to_vec();
    bad_length.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x07]);
    assert!(Column::decode(&bad_length, false).is_err());
}

#[test]
fn bulk_columns_follow_headers_and_validate_all_lengths_before_use() {
    let mut w = Writer::new();
    w.bool(false);
    w.varint(2);
    for (x, z, mask) in [(10, -20, 1), (-30, 40, 0x8000)] {
        w.i32(x);
        w.i32(z);
        w.u16(mask);
    }
    let mut first = vec![0; 8192 + 2048 + 256];
    first[..2].copy_from_slice(&[0x23, 0x01]);
    // Nonzero x/y/z establish the block-index order independently.
    let offset = ((3 * 16 + 4) * 16 + 5) * 2;
    first[offset..offset + 2].copy_from_slice(&[0x54, 0x03]);
    let mut second = vec![0; first.len()];
    second[..2].copy_from_slice(&[0x67, 0x04]);
    w.bytes(&first);
    w.bytes(&second);
    let columns = legacy18::chunk::decode_bulk(&w.data).unwrap();
    assert_eq!(columns.len(), 2);
    assert_eq!((columns[0].x, columns[0].z), (10, -20));
    assert_eq!((columns[1].x, columns[1].z), (-30, 40));
    assert_eq!(columns[0].section(0).unwrap().state(0, 0, 0), Some(0x123));
    assert_eq!(columns[0].section(0).unwrap().state(5, 3, 4), Some(0x354));
    assert_eq!(columns[1].section(15).unwrap().state(0, 0, 0), Some(0x467));
    assert!(legacy18::chunk::decode_bulk(&w.data[..w.data.len() - 1]).is_err());
    w.data.push(0);
    assert!(legacy18::chunk::decode_bulk(&w.data).is_err());
    assert!(legacy18::chunk::decode_bulk(&[0, 0xff, 0xff, 0xff, 0xff, 0x07]).is_err());
    assert!(legacy18::chunk::decode_bulk(&[0, 0]).unwrap().is_empty());

    // Bulk zero-mask columns load an empty column; only the individual chunk
    // packet uses a zero mask as the unload signal.
    let mut empty = vec![0, 1];
    empty.extend_from_slice(&[0; 10]);
    empty.extend_from_slice(&[7; 256]);
    let columns = legacy18::chunk::decode_bulk(&empty).unwrap();
    assert!(!columns[0].is_unload());
    assert_eq!(columns[0].biomes().unwrap(), &[7; 256]);
}

#[test]
fn relative_teleports_and_position_ack_have_legacy_layout() {
    // Fixed wire vector: dx=1, y=64, z=-2, dyaw=90, pitch=0, relative X/yaw.
    let bytes = [
        0x3f, 0xf0, 0, 0, 0, 0, 0, 0, 0x40, 0x50, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0,
        0x42, 0xb4, 0, 0, 0, 0, 0, 0, 9,
    ];
    let packet = Packet::decode(8, &bytes).unwrap();
    let pose = Pose {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        yaw: 45.0,
        pitch: 5.0,
    }
    .corrected(&packet)
    .unwrap();
    assert_eq!(
        pose,
        Pose {
            x: 11.0,
            y: 64.0,
            z: -2.0,
            yaw: 135.0,
            pitch: 0.0
        }
    );
    let expected = [
        6, 0x40, 0x26, 0, 0, 0, 0, 0, 0, 0x40, 0x50, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0,
        0x43, 0x07, 0, 0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(pose.encode(false).unwrap(), expected);
    for end in 0..bytes.len() {
        assert!(Packet::decode(8, &bytes[..end]).is_err());
    }
    let mut bad_flags = bytes;
    bad_flags[32] = 0x20;
    assert!(Packet::decode(8, &bad_flags).is_err());
    assert!(
        Pose {
            x: f64::INFINITY,
            ..pose
        }
        .encode(false)
        .is_err()
    );
    assert!(pose.corrected(&Packet::KeepAlive(0)).is_err());
}

#[test]
fn play_packets_do_not_use_modern_registry_or_string_encodings() {
    assert_eq!(
        Packet::decode(1, b"\x00\x00\x00\x2a\x01\xff\x02\x14\x07default\x00").unwrap(),
        Packet::Join {
            entity: 42,
            gamemode: 1,
            dimension: -1,
            difficulty: 2,
            max_players: 20,
            level_type: "default".into(),
            reduced_debug: false
        }
    );
    assert_eq!(
        Packet::decode(7, b"\xff\xff\xff\xff\x02\x00\x07default").unwrap(),
        Packet::Respawn {
            dimension: -1,
            difficulty: 2,
            gamemode: 0,
            level_type: "default".into()
        }
    );
    assert_eq!(
        Packet::decode(2, b"\x0e{\"text\":\"hey\"}\x01").unwrap(),
        Packet::Chat {
            json: "{\"text\":\"hey\"}".into(),
            position: 1
        }
    );
    assert_eq!(play::chat("hello").unwrap(), b"\x01\x05hello");
    assert!(play::chat(&"a".repeat(101)).is_err());
    assert!(play::chat(&"😀".repeat(51)).is_err());
    assert!(play::chat("hello\nworld").is_err());
    assert!(play::chat("§ahello").is_err());
    assert_eq!(
        Packet::decode(0x55, &[1, 2]).unwrap(),
        Packet::Unknown {
            id: 0x55,
            payload: &[1, 2]
        }
    );
    assert!(Packet::decode(0, &[1, 2]).is_err());
}

#[test]
fn compressed_legacy_play_carries_actions_and_world_updates() {
    use horizon_protocol::legacy18::actions::{self, Digging};
    let (client, mut server) = loopback();
    let worker = std::thread::spawn(move || {
        assert_hello(&mut server);
        server.send(&[3, 0]).unwrap();
        server.set_compression(0);
        server.send(&success()).unwrap();
        // A block at x=1, y=64, z=2 becomes stone (ID 1, metadata 0).
        server.send(&[0x23, 0, 0, 0, 65, 0, 0, 0, 2, 16]).unwrap();
        assert_eq!(server.recv().unwrap(), (9, vec![0, 2]));
        assert_eq!(server.recv().unwrap(), (2, vec![0xac, 2, 1]));
        assert_eq!(server.recv().unwrap(), (10, vec![]));
        assert_eq!(
            server.recv().unwrap(),
            (7, vec![0, 0, 0, 0, 65, 0, 0, 0, 2, 1])
        );
        server.send(&[0x23, 0, 0, 0, 65, 0, 0, 0, 2, 0]).unwrap();
    });
    let (mut client, _) = legacy18::login(
        client,
        "localhost",
        25565,
        "Alex",
        Duration::from_secs(5),
        |_| panic!("unexpected authentication"),
    )
    .unwrap();
    let (id, body) = client.recv().unwrap();
    assert_eq!(
        Packet::decode(id, &body).unwrap(),
        Packet::BlockChange {
            x: 1,
            y: 64,
            z: 2,
            state: 16
        }
    );
    client.send(&actions::held_slot(2).unwrap()).unwrap();
    client
        .send(&actions::interact_entity(300, true).unwrap())
        .unwrap();
    client.send(&actions::swing_arm()).unwrap();
    client
        .send(&actions::digging(Digging::Start, 1, 64, 2, 1).unwrap())
        .unwrap();
    let (id, body) = client.recv().unwrap();
    assert_eq!(
        Packet::decode(id, &body).unwrap(),
        Packet::BlockChange {
            x: 1,
            y: 64,
            z: 2,
            state: 0
        }
    );
    worker.join().unwrap();
}
