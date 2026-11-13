//! Legacy connection path. Authentication still uses the selected account and
//! the official session service; a failed join never falls back to offline.
use super::*;
use horizon_protocol::legacy18::{self, bridge::Outbound, play::Packet};

pub(super) fn run(
    address: &str,
    name: &str,
    auth: Option<&AuthSession>,
    lang: &Language,
    send: &Sender<Event>,
    writer: &Mutex<Option<ConnWriter>>,
    configuring: &AtomicBool,
    counters: &Mutex<Option<Arc<Counters>>>,
    bridge: &Mutex<Outbound>,
) -> Result<Reason, Reason> {
    let (host, port) =
        parse_address(address).ok_or_else(|| Reason::plain("Invalid server address"))?;
    status(send, &tr(lang, "connect.connecting"));
    let (connection, host, port) =
        connect_tcp(&host, port, false, READ_TIMEOUT).map_err(Reason::plain)?;
    *counters.lock().unwrap() = Some(connection.reader.counters.clone());
    let (mut connection, profile) =
        legacy18::login(connection, &host, port, name, READ_TIMEOUT, |hash| {
            let account = auth.ok_or_else(|| {
                horizon_protocol::Error::new("This server requires a signed-in Minecraft account")
            })?;
            crate::auth::join_server(account, hash).map_err(|e| {
                horizon_protocol::Error::new(match e.reason() {
                    Ok(key) => tr(lang, key),
                    Err(message) => message.to_owned(),
                })
            })
        })
        .map_err(|e| Reason::plain(e.to_string()))?;
    let mut brand = horizon_protocol::buf::Writer::new();
    brand.varint(0x17);
    brand.string("MC|Brand");
    brand.string("PojavRust");
    connection
        .send(&brand.data)
        .map_err(|e| Reason::plain(e.to_string()))?;
    let (mut reader, outgoing) = connection.split();
    *writer.lock().unwrap() = Some(outgoing);
    configuring.store(false, Ordering::Relaxed);
    let _ = send.send(Event::Joined {
        registries: Arc::new(registries()),
        uuid: profile.uuid,
        name: profile.name,
    });
    loop {
        let (id, payload) = reader
            .recv()
            .map_err(|e| Reason::plain(connection_lost(e, lang)))?;
        let response = bridge
            .lock()
            .unwrap()
            .observe(id, &payload)
            .map_err(|e| Reason::plain(e.to_string()))?;
        if let Some(bytes) = response {
            if let Some(w) = writer.lock().unwrap().as_mut() {
                w.send(&bytes).map_err(|e| Reason::plain(e.to_string()))?;
            }
        }
        match Packet::decode(id, &payload).map_err(|e| Reason::plain(e.to_string()))? {
            Packet::KeepAlive(value) => {
                if let Some(w) = writer.lock().unwrap().as_mut() {
                    w.send(&legacy18::play::keep_alive(value))
                        .map_err(|e| Reason::plain(e.to_string()))?;
                }
                continue;
            }
            Packet::Disconnect(reason) => return Ok(Reason::component(&reason, lang)),
            Packet::Join { entity, .. } => bridge.lock().unwrap().entity = entity,
            _ => {}
        }
        if send.send(Event::LegacyPacket { id, payload }).is_err() {
            return Ok(Reason::plain("Game disconnected"));
        }
    }
}

/// 1.8.9 has fixed dimensions and no configuration registry phase.
fn registries() -> Dynamic {
    let mut dynamic = Dynamic::default();
    dynamic.registries.insert(
        "minecraft:dimension_type".into(),
        [
            ("minecraft:overworld", true),
            ("minecraft:the_nether", false),
            ("minecraft:the_end", false),
        ]
        .into_iter()
        .map(|(name, sky)| {
            (
                name.into(),
                Some(Tag::compound([
                    ("min_y", Tag::Int(0)),
                    ("height", Tag::Int(256)),
                    ("logical_height", Tag::Int(256)),
                    ("has_skylight", Tag::Byte(i8::from(sky))),
                ])),
            )
        })
        .collect(),
    );
    dynamic
}
