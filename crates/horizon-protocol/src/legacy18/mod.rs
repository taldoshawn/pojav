//! Experimental protocol 47 transport (Minecraft 1.8–1.8.9).
//!
//! This is a wire layer, not a second gameplay backend. It deliberately does not
//! feed old registry IDs or packets into the modern game. Login enters Play
//! directly: there is no Configuration phase or Login Acknowledged packet.
pub mod actions;
pub mod bridge;
pub mod chunk;
pub mod metadata;
pub mod play;
pub mod slot;

use crate::{
    Error, Result,
    buf::{Reader, Writer},
    connection::Connection,
    crypto::{generate_secret, server_hash},
    packets::handshake,
    rsa::RsaPublicKey,
};
use std::time::{Duration, Instant};

pub const PROTOCOL: i32 = 47;

#[derive(Debug, PartialEq, Eq)]
pub struct Profile {
    pub uuid: u128,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoginPacket {
    Disconnect(String),
    Encryption {
        server_id: String,
        public_key: Vec<u8>,
        verify_token: Vec<u8>,
    },
    Success(Profile),
    Compression(i32),
}

impl LoginPacket {
    pub fn decode(id: i32, payload: &[u8]) -> Result<Self> {
        let mut r = Reader::new(payload);
        let packet = match id {
            0 => Self::Disconnect(r.string(32767)?),
            1 => Self::Encryption {
                server_id: r.string(20)?,
                public_key: r.byte_array(8192)?,
                verify_token: r.byte_array(1024)?,
            },
            2 => {
                let uuid = parse_uuid(&r.string(36)?)?;
                let name = r.string(16)?;
                check_name(&name)?;
                Self::Success(Profile { uuid, name })
            }
            3 => Self::Compression(r.varint()?),
            _ => return Err(Error::new(format!("unknown protocol 47 login packet {id}"))),
        };
        finish(&r)?;
        Ok(packet)
    }
}

fn parse_uuid(s: &str) -> Result<u128> {
    let compact = match s.len() {
        32 if s.bytes().all(|b| b.is_ascii_hexdigit()) => s.to_owned(),
        36 if s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        }) =>
        {
            s.replace('-', "")
        }
        _ => return Err(Error::new("invalid protocol 47 profile UUID")),
    };
    u128::from_str_radix(&compact, 16).map_err(|_| Error::new("invalid profile UUID"))
}

fn check_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 16
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(Error::new(
            "player name must contain 1–16 ASCII letters, digits or underscores",
        ));
    }
    Ok(())
}

pub(super) fn finish(r: &Reader<'_>) -> Result<()> {
    if r.remaining() != 0 {
        return Err(Error::new("trailing bytes in protocol 47 packet"));
    }
    Ok(())
}

/// Log in over an already connected transport. A caller supplies session-service
/// authentication, using its signed-in account. The callback must return an error
/// on authentication failure; an encryption request never falls back to offline.
/// On error the owned connection closes. The negotiation budget is checked
/// between packets and used as the remaining socket read timeout; callers must
/// also bound their authentication callback's HTTP requests.
pub fn login(
    mut connection: Connection,
    host: &str,
    port: u16,
    name: &str,
    timeout: Duration,
    mut authenticate: impl FnMut(&str) -> Result<()>,
) -> Result<(Connection, Profile)> {
    check_name(name)?;
    if host.is_empty() || host.encode_utf16().count() > 255 || host.contains('\0') {
        return Err(Error::new("invalid handshake host"));
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| Error::new("invalid login timeout"))?;
    if timeout.is_zero() {
        return Err(Error::timed_out());
    }
    connection.send(
        &handshake::Serverbound::Intention(handshake::Intention {
            protocol: PROTOCOL,
            host: host.to_owned(),
            port,
            intent: handshake::LOGIN,
        })
        .encode(),
    )?;
    let mut hello = Writer::new();
    hello.varint(0);
    hello.string(name);
    connection.send(&hello.data)?;
    let mut encrypted = false;
    let mut compressed = false;
    // Only encryption, compression and success are legal transitions. A peer
    // cannot keep a connection alive indefinitely by repeating negotiation.
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Error::timed_out());
        }
        connection.reader.set_read_timeout(Some(left))?;
        let (id, bytes) = connection.recv()?;
        match LoginPacket::decode(id, &bytes)? {
            LoginPacket::Disconnect(reason) => {
                return Err(Error::new(format!("server disconnected: {reason}")));
            }
            LoginPacket::Success(profile) => {
                connection
                    .reader
                    .set_read_timeout(Some(Duration::from_secs(30)))?;
                return Ok((connection, profile));
            }
            LoginPacket::Compression(threshold) => {
                if compressed {
                    return Err(Error::new("repeated compression negotiation"));
                }
                compressed = true;
                connection.set_compression(threshold);
            }
            LoginPacket::Encryption {
                server_id,
                public_key,
                verify_token,
            } => {
                if encrypted {
                    return Err(Error::new("repeated encryption request"));
                }
                let key = RsaPublicKey::from_der(&public_key)?;
                let secret = generate_secret()?;
                authenticate(&server_hash(&server_id, &secret, &public_key))?;
                if Instant::now() >= deadline {
                    return Err(Error::timed_out());
                }
                let mut response = Writer::new();
                response.varint(1);
                response.byte_array(&key.encrypt(&secret)?);
                response.byte_array(&key.encrypt(&verify_token)?);
                connection.send(&response.data)?;
                connection.enable_encryption(&secret);
                encrypted = true;
            }
        }
    }
}
