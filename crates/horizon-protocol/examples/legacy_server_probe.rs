//! Test-only probe for an isolated protocol 47 server. This is not a game client.
use horizon_protocol::{
    buf::Writer,
    connection::Connection,
    legacy18::{
        self,
        chunk::{Column, decode_bulk},
        play::{Packet, Pose},
    },
};
use std::{
    net::TcpStream,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let peer = std::env::args().any(|arg| arg == "--peer");
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:25566".into());
    let socket: std::net::SocketAddr = address.parse()?;
    if !socket.ip().is_loopback() {
        return Err("probe is restricted to a local test server".into());
    }
    let stream = TcpStream::connect_timeout(&socket, Duration::from_secs(10))?;
    let connection = Connection::from_stream(stream)?;
    let (mut connection, profile) = legacy18::login(
        connection,
        "localhost",
        socket.port(),
        if peer {
            "LegacyOpponent"
        } else {
            "LegacyProbe"
        },
        Duration::from_secs(10),
        |_| {
            Err(horizon_protocol::Error::new(
                "test server must use its own offline authentication",
            ))
        },
    )?;
    connection
        .reader
        .set_read_timeout(Some(Duration::from_secs(5)))?;
    if peer {
        return run_peer(connection);
    }
    let start = Instant::now();
    let mut pose = Pose::default();
    let (mut joined, mut positioned, mut columns, mut states) = (false, false, 0usize, 0usize);
    let mut sky = true;
    let mut bridge = legacy18::bridge::Outbound::default();
    let place = horizon_protocol::packets::play::Serverbound::UseItemOn(
        horizon_protocol::packets::play::UseItemOn {
            hand: 0,
            pos: [0, 65, 3],
            face: 2,
            cursor: [0.5, 0.5, 0.],
            inside: false,
            world_border: false,
            sequence: 0,
        },
    );
    while start.elapsed() < Duration::from_secs(20) {
        let (id, bytes) = connection.recv()?;
        if let Some(reply) = bridge.observe(id, &bytes)? {
            connection.send(&reply)?;
        }
        let p = Packet::decode(id, &bytes)?;
        match p {
            Packet::KeepAlive(id) => connection.send(&legacy18::play::keep_alive(id))?,
            Packet::Join { dimension, .. } => {
                sky = dimension == 0;
                joined = true;
                let mut w = Writer::new();
                w.varint(0x15);
                w.string("en_US");
                w.i8(3);
                w.u8(0);
                w.bool(true);
                w.u8(127);
                connection.send(&w.data)?;
            }
            Packet::Position { .. } => {
                pose = pose.corrected(&p)?;
                positioned = true;
                connection.send(&pose.encode(false)?)?;
            }
            Packet::Chunk(bytes) => {
                let column = Column::decode(bytes, sky)?;
                if !column.is_unload() {
                    columns += 1;
                    states += validate(&column)?;
                }
            }
            Packet::ChunkBulk(bytes) => {
                for column in decode_bulk(bytes)? {
                    columns += 1;
                    states += validate(&column)?;
                }
            }
            Packet::Disconnect(reason) => {
                return Err(format!("server disconnected: {reason}").into());
            }
            _ => {}
        }
        if joined && positioned && columns >= 4 {
            let encoded = bridge.encode(&place)?.ok_or("placement not encoded")?;
            let held = legacy18::slot::Slot::read(&mut horizon_protocol::buf::Reader::new(
                &encoded[10..],
            ))?;
            if held.is_some() {
                break;
            }
        }
    }
    if !joined || !positioned || columns < 4 || states == 0 {
        return Err("missing spawn/chunk evidence".into());
    }
    connection.send(&bridge.encode(&place)?.ok_or("placement not encoded")?)?;
    await_block(&mut connection, &mut bridge, [0, 65, 2], 16)?;
    connection.send(&legacy18::actions::digging(
        legacy18::actions::Digging::Start,
        0,
        65,
        2,
        2,
    )?)?;
    connection.send(&legacy18::actions::swing_arm())?;
    await_block(&mut connection, &mut bridge, [0, 65, 2], 0)?;
    let moved = Pose {
        x: pose.x + 1.0,
        ..pose
    };
    connection.send(&moved.encode(true)?)?;
    std::thread::sleep(Duration::from_millis(300));
    println!(
        "{}",
        serde_json::json!({"profile":profile.name,"joined":joined,"positioned":positioned,"columns":columns,"states_checked":states,"position":[pose.x,pose.y,pose.z],"placement_confirmed":true,"break_confirmed":true,"rendering_tested":false})
    );
    if std::env::args().any(|arg| arg == "--attack-peer") {
        use std::io::Write;
        std::io::stdout().flush()?;
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let (id, bytes) = connection.recv()?;
            match Packet::decode(id, &bytes)? {
                Packet::KeepAlive(id) => connection.send(&legacy18::play::keep_alive(id))?,
                Packet::Unknown { id: 0x0c, payload } => {
                    let mut r = horizon_protocol::buf::Reader::new(payload);
                    let entity = r.varint()?;
                    let _server_uuid = r.uuid()?;
                    // This isolated two-client fixture has exactly one other
                    // player. Its server-selected offline UUID is not Mojang's.
                    connection.send(&legacy18::actions::interact_entity(entity, true)?)?;
                    std::thread::sleep(Duration::from_millis(300));
                    return Ok(());
                }
                _ => {}
            }
        }
        return Err("second client did not spawn for attack probe".into());
    }
    Ok(())
}

/// An isolated, stationary second client for server-confirmed combat tests.
/// It acknowledges teleports and keepalives, but never attacks or changes blocks.
fn run_peer(mut connection: Connection) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let start = Instant::now();
    let mut pose = Pose::default();
    while start.elapsed() < Duration::from_secs(180) {
        let (id, bytes) = connection.recv()?;
        let packet = Packet::decode(id, &bytes)?;
        match packet {
            Packet::KeepAlive(id) => connection.send(&legacy18::play::keep_alive(id))?,
            Packet::Join { .. } => {
                let mut settings = Writer::new();
                settings.varint(0x15);
                settings.string("en_US");
                settings.i8(3);
                settings.u8(0);
                settings.bool(true);
                settings.u8(127);
                connection.send(&settings.data)?;
            }
            Packet::Position { .. } => {
                pose = pose.corrected(&packet)?;
                connection.send(&pose.encode(false)?)?;
                println!(
                    "{}",
                    serde_json::json!({"peer_position": [pose.x, pose.y, pose.z]})
                );
                std::io::stdout().flush()?;
            }
            Packet::Health { health, .. } => {
                println!("{}", serde_json::json!({"peer_health": health}));
                std::io::stdout().flush()?;
            }
            Packet::Disconnect(reason) => return Err(format!("peer disconnected: {reason}").into()),
            _ => {}
        }
    }
    Ok(())
}
fn validate(column: &Column<'_>) -> Result<usize, Box<dyn std::error::Error>> {
    let mut checked = 0;
    let mut seen = std::collections::HashSet::new();
    for sy in 0..16 {
        if let Some(section) = column.section(sy) {
            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let state = section.state(x, y, z).unwrap();
                        if seen.insert(state) {
                            horizon_world::datafix::legacy_block_state(state)?;
                        }
                        checked += 1;
                    }
                }
            }
        }
    }
    Ok(checked)
}

fn await_block(
    connection: &mut Connection,
    bridge: &mut legacy18::bridge::Outbound,
    target: [i32; 3],
    expected: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    use horizon_protocol::buf::Reader;
    let start = Instant::now();
    for _ in 0..4096 {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        let (id, bytes) = connection.recv()?;
        if let Some(reply) = bridge.observe(id, &bytes)? {
            connection.send(&reply)?;
        }
        match Packet::decode(id, &bytes)? {
            Packet::KeepAlive(id) => connection.send(&legacy18::play::keep_alive(id))?,
            Packet::BlockChange { x, y, z, state }
                if [x, i32::from(y), z] == target && state == expected =>
            {
                return Ok(());
            }
            Packet::Unknown { id: 0x22, payload } => {
                let mut r = Reader::new(payload);
                let x = r.i32()? * 16;
                let z = r.i32()? * 16;
                let count = r.count(65536)?;
                for _ in 0..count {
                    let position = r.u16()?;
                    let state = r.varint()?;
                    let pos = [
                        x + i32::from(position >> 12),
                        i32::from(position & 255),
                        z + i32::from((position >> 8) & 15),
                    ];
                    if pos == target && state == i32::from(expected) {
                        return Ok(());
                    }
                }
            }
            Packet::Disconnect(reason) => return Err(reason.into()),
            _ => {}
        }
    }
    Err(format!("server did not confirm block {target:?} as {expected}").into())
}
