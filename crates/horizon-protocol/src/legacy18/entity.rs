//! Legacy object spawns: fixed-point coordinates and conditional velocity.
use crate::{Error, Result, buf::Reader};
#[derive(Debug, PartialEq)]
pub struct Object {
    pub id: i32,
    pub kind: u8,
    pub position: [f64; 3],
    pub pitch: i8,
    pub yaw: i8,
    pub data: i32,
    pub velocity: [f64; 3],
}
impl Object {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let id = r.varint()?;
        let kind = r.u8()?;
        let position = [
            f64::from(r.i32()?) / 32.,
            f64::from(r.i32()?) / 32.,
            f64::from(r.i32()?) / 32.,
        ];
        let pitch = r.i8()?;
        let yaw = r.i8()?;
        let data = r.i32()?;
        let velocity = if data != 0 {
            [
                f64::from(r.i16()?) / 8000.,
                f64::from(r.i16()?) / 8000.,
                f64::from(r.i16()?) / 8000.,
            ]
        } else {
            [0.; 3]
        };
        if r.remaining() != 0 {
            return Err(Error::new("trailing legacy object data"));
        }
        Ok(Self {
            id,
            kind,
            position,
            pitch,
            yaw,
            data,
            velocity,
        })
    }
    pub fn name(&self) -> Option<&'static str> {
        Some(match self.kind {
            1 => "oak_boat",
            2 => "item",
            10 => match self.data {
                0 => "minecart",
                1 => "chest_minecart",
                2 => "furnace_minecart",
                3 => "tnt_minecart",
                4 => "spawner_minecart",
                5 => "hopper_minecart",
                6 => "command_block_minecart",
                _ => return None,
            },
            50 => "tnt",
            51 => "end_crystal",
            60 => "arrow",
            61 => "snowball",
            62 => "egg",
            63 => "fireball",
            64 => "small_fireball",
            65 => "ender_pearl",
            66 => "wither_skull",
            70 => "falling_block",
            71 => "item_frame",
            72 => "eye_of_ender",
            73 => "splash_potion",
            75 => "experience_bottle",
            76 => "firework_rocket",
            77 => "leash_knot",
            78 => "armor_stand",
            90 => "fishing_bobber",
            _ => return None,
        })
    }
}

pub fn effect_name(id: u8) -> Option<&'static str> {
    Some(match id {
        1 => "speed",
        2 => "slowness",
        3 => "haste",
        4 => "mining_fatigue",
        5 => "strength",
        6 => "instant_health",
        7 => "instant_damage",
        8 => "jump_boost",
        9 => "nausea",
        10 => "regeneration",
        11 => "resistance",
        12 => "fire_resistance",
        13 => "water_breathing",
        14 => "invisibility",
        15 => "blindness",
        16 => "night_vision",
        17 => "hunger",
        18 => "weakness",
        19 => "poison",
        20 => "wither",
        21 => "health_boost",
        22 => "absorption",
        23 => "saturation",
        _ => return None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::Writer;
    #[test]
    fn object_uses_fixed_coordinates_pitch_first_and_optional_velocity() {
        let mut w = Writer::new();
        w.varint(300);
        w.u8(60);
        w.i32(-48);
        w.i32(2048);
        w.i32(80);
        w.i8(-32);
        w.i8(64);
        w.i32(0);
        let object = Object::read(&w.data).unwrap();
        assert_eq!(object.position, [-1.5, 64., 2.5]);
        assert_eq!((object.pitch, object.yaw), (-32, 64));
        assert_eq!(object.name(), Some("arrow"));
        assert_eq!(object.velocity, [0.; 3]);
        let data_offset = w.data.len() - 4;
        w.data[data_offset..].copy_from_slice(&42i32.to_be_bytes());
        assert!(Object::read(&w.data).is_err());
        w.i16(4000);
        w.i16(-8000);
        w.i16(0);
        assert_eq!(Object::read(&w.data).unwrap().velocity, [0.5, -1., 0.]);
        w.u8(0);
        assert!(Object::read(&w.data).is_err());
    }
}
