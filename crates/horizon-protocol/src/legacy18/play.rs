//! Initial play-state codecs. Unknown packets remain explicit borrowed bytes;
//! callers must implement them before claiming a playable legacy backend.
use crate::{
    Error, Result,
    buf::{Reader, Writer},
};

#[derive(Debug, PartialEq)]
pub enum Packet<'a> {
    KeepAlive(i32),
    Join {
        entity: i32,
        gamemode: u8,
        dimension: i8,
        difficulty: u8,
        max_players: u8,
        level_type: String,
        reduced_debug: bool,
    },
    Chat {
        json: String,
        position: u8,
    },
    Position {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        flags: u8,
    },
    Respawn {
        dimension: i32,
        difficulty: u8,
        gamemode: u8,
        level_type: String,
    },
    Chunk(&'a [u8]),
    ChunkBulk(&'a [u8]),
    Disconnect(String),
    Unknown {
        id: i32,
        payload: &'a [u8],
    },
}

impl<'a> Packet<'a> {
    pub fn decode(id: i32, payload: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(payload);
        let packet = match id {
            0x00 => Self::KeepAlive(r.varint()?),
            0x01 => Self::Join {
                entity: r.i32()?,
                gamemode: r.u8()?,
                dimension: r.i8()?,
                difficulty: r.u8()?,
                max_players: r.u8()?,
                level_type: r.string(16)?,
                reduced_debug: r.bool()?,
            },
            0x02 => Self::Chat {
                json: r.string(32767)?,
                position: r.u8()?,
            },
            0x07 => Self::Respawn {
                dimension: r.i32()?,
                difficulty: r.u8()?,
                gamemode: r.u8()?,
                level_type: r.string(16)?,
            },
            0x08 => {
                let (x, y, z, yaw, pitch, flags) =
                    (r.f64()?, r.f64()?, r.f64()?, r.f32()?, r.f32()?, r.u8()?);
                if ![x, y, z, f64::from(yaw), f64::from(pitch)]
                    .iter()
                    .all(|v| v.is_finite())
                    || flags & !0x1f != 0
                {
                    return Err(Error::new("invalid protocol 47 position"));
                }
                Self::Position {
                    x,
                    y,
                    z,
                    yaw,
                    pitch,
                    flags,
                }
            }
            0x21 => Self::Chunk(r.rest()),
            0x26 => Self::ChunkBulk(r.rest()),
            0x40 => Self::Disconnect(r.string(32767)?),
            _ => Self::Unknown {
                id,
                payload: r.rest(),
            },
        };
        super::finish(&r)?;
        Ok(packet)
    }
}

pub fn keep_alive(id: i32) -> Vec<u8> {
    let mut w = Writer::new();
    w.varint(0);
    w.varint(id);
    w.data
}

/// 1.8 chat is an unsigned string of at most 100 UTF-16 code units.
pub fn chat(message: &str) -> Result<Vec<u8>> {
    if message.encode_utf16().count() > 100
        || message
            .chars()
            .any(|c| c < ' ' || c == '\u{7f}' || c == '§')
    {
        return Err(Error::new("invalid or overlong protocol 47 chat message"));
    }
    let mut w = Writer::new();
    w.varint(1);
    w.string(message);
    Ok(w.data)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

impl Pose {
    /// Apply the server's relative flags before acknowledging. 1.8 uses a
    /// position/look response, not the modern teleport-ID acknowledgement.
    pub fn corrected(self, packet: &Packet<'_>) -> Result<Self> {
        let Packet::Position {
            x,
            y,
            z,
            yaw,
            pitch,
            flags,
        } = packet
        else {
            return Err(Error::new("expected position packet"));
        };
        if flags & !0x1f != 0 {
            return Err(Error::new("invalid relative flags"));
        }
        let pose = Self {
            x: x + if flags & 1 != 0 { self.x } else { 0.0 },
            y: y + if flags & 2 != 0 { self.y } else { 0.0 },
            z: z + if flags & 4 != 0 { self.z } else { 0.0 },
            yaw: yaw + if flags & 8 != 0 { self.yaw } else { 0.0 },
            pitch: pitch + if flags & 16 != 0 { self.pitch } else { 0.0 },
        };
        pose.validate()?;
        Ok(pose)
    }

    fn validate(&self) -> Result<()> {
        if ![
            self.x,
            self.y,
            self.z,
            f64::from(self.yaw),
            f64::from(self.pitch),
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Err(Error::new("non-finite player position"));
        }
        Ok(())
    }

    pub fn encode(self, on_ground: bool) -> Result<Vec<u8>> {
        self.validate()?;
        let mut w = Writer::new();
        w.varint(0x06);
        w.f64(self.x);
        w.f64(self.y);
        w.f64(self.z);
        w.f32(self.yaw);
        w.f32(self.pitch);
        w.bool(on_ground);
        Ok(w.data)
    }
}
