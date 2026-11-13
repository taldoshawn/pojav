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
        "LegacyProbe",
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
    let start = Instant::now();
    let mut pose = Pose::default();
    let (mut joined, mut positioned, mut columns, mut states) = (false, false, 0usize, 0usize);
    let mut sky = true;
    while start.elapsed() < Duration::from_secs(20) {
        let (id, bytes) = connection.recv()?;
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
            break;
        }
    }
    if !joined || !positioned || columns < 4 || states == 0 {
        return Err("missing spawn/chunk evidence".into());
    }
    println!(
        "{}",
        serde_json::json!({"profile":profile.name,"joined":joined,"positioned":positioned,"columns":columns,"states_checked":states,"position":[pose.x,pose.y,pose.z],"rendering_tested":false})
    );
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
