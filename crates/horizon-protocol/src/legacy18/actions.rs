//! Protocol 47 player actions. These are legacy wire encodings, not modern
//! packets with a changed handshake. Inventory stacks need a separate codec.
use crate::{Error, Result, buf::Writer};

#[derive(Clone, Copy, Debug)]
pub enum Digging {
    Start = 0,
    Cancel = 1,
    Finish = 2,
    DropStack = 3,
    DropOne = 4,
    ReleaseUse = 5,
}
#[derive(Clone, Copy, Debug)]
pub enum EntityAction {
    StartSneaking = 0,
    StopSneaking = 1,
    LeaveBed = 2,
    StartSprinting = 3,
    StopSprinting = 4,
    HorseJump = 5,
    OpenHorseInventory = 6,
}

/// Legacy Position packs X:26, Y:12, Z:26 (modern Position uses X/Z/Y).
pub fn block_position(x: i32, y: i32, z: i32) -> Result<u64> {
    if !(-33_554_432..33_554_432).contains(&x)
        || !(0..4096).contains(&y)
        || !(-33_554_432..33_554_432).contains(&z)
    {
        return Err(Error::new("position outside protocol 47 coordinate range"));
    }
    Ok(((x as u64 & 0x3ff_ffff) << 38) | ((y as u64) << 26) | (z as u64 & 0x3ff_ffff))
}

pub fn digging(action: Digging, x: i32, y: i32, z: i32, face: u8) -> Result<Vec<u8>> {
    if face > 5 {
        return Err(Error::new("invalid protocol 47 block face"));
    }
    let mut w = Writer::new();
    w.varint(0x07);
    w.u8(action as u8);
    w.i64(block_position(x, y, z)? as i64);
    w.u8(face);
    Ok(w.data)
}

/// Attack and ordinary interaction have no hand field in 1.8.9.
pub fn interact_entity(entity: i32, attack: bool) -> Result<Vec<u8>> {
    if entity < 0 {
        return Err(Error::new("negative protocol 47 entity ID"));
    }
    let mut w = Writer::new();
    w.varint(0x02);
    w.varint(entity);
    w.varint(i32::from(attack));
    Ok(w.data)
}

pub fn entity_action(entity: i32, action: EntityAction, jump_boost: i32) -> Result<Vec<u8>> {
    if entity < 0
        || !(0..=100).contains(&jump_boost)
        || (!matches!(action, EntityAction::HorseJump) && jump_boost != 0)
    {
        return Err(Error::new("invalid protocol 47 entity action"));
    }
    let mut w = Writer::new();
    w.varint(0x0b);
    w.varint(entity);
    w.varint(action as i32);
    w.varint(jump_boost);
    Ok(w.data)
}

pub fn held_slot(slot: u8) -> Result<Vec<u8>> {
    if slot > 8 {
        return Err(Error::new("hotbar slot must be 0 through 8"));
    }
    let mut w = Writer::new();
    w.varint(0x09);
    w.i16(i16::from(slot));
    Ok(w.data)
}

pub fn swing_arm() -> Vec<u8> {
    vec![0x0a] // No hand field.
}
pub fn respawn() -> Vec<u8> {
    vec![0x16, 0x00]
}
pub fn ground(on_ground: bool) -> Vec<u8> {
    vec![0x03, u8::from(on_ground)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_position_places_y_between_x_and_z() {
        // x=1, y=64, z=2: 1<<38 | 64<<26 | 2.
        assert_eq!(block_position(1, 64, 2).unwrap(), 0x0000_0041_0000_0002);
        assert_eq!(block_position(-1, 255, -1).unwrap(), 0xffff_ffc3_ffff_ffff);
        assert!(block_position(33_554_432, 0, 0).is_err());
        assert!(block_position(0, -1, 0).is_err());
    }

    #[test]
    fn actions_have_legacy_packet_ids_and_field_widths() {
        assert_eq!(
            digging(Digging::Start, 1, 64, 2, 1).unwrap(),
            [7, 0, 0, 0, 0, 65, 0, 0, 0, 2, 1]
        );
        assert_eq!(interact_entity(300, true).unwrap(), [2, 0xac, 2, 1]);
        assert_eq!(interact_entity(1, false).unwrap(), [2, 1, 0]);
        assert_eq!(
            entity_action(300, EntityAction::StartSprinting, 0).unwrap(),
            [11, 0xac, 2, 3, 0]
        );
        assert_eq!(held_slot(8).unwrap(), [9, 0, 8]);
        assert_eq!(swing_arm(), [10]);
        assert_eq!(respawn(), [22, 0]);
        assert_eq!(ground(false), [3, 0]);
    }

    #[test]
    fn invalid_actions_are_rejected_before_sending() {
        assert!(digging(Digging::Finish, 0, 64, 0, 6).is_err());
        assert!(held_slot(9).is_err());
        assert!(interact_entity(-1, true).is_err());
        assert!(entity_action(1, EntityAction::HorseJump, 101).is_err());
        assert!(entity_action(1, EntityAction::StartSneaking, 1).is_err());
    }
}
