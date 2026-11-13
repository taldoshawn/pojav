//! Translate engine actions to protocol 47 without ever encoding a modern
//! packet on the legacy socket. Unsupported gameplay actions remain errors.
use super::{
    actions::{self, Digging, EntityAction},
    play::{self, Pose},
};
use crate::{
    Error, Result,
    buf::{Reader, Writer},
    packets::play::Serverbound as S,
};

#[derive(Default)]
pub struct Outbound {
    pub entity: i32,
    sneaking: bool,
    selected: u8,
    slots: std::collections::HashMap<(i32, i16), Option<super::slot::Slot>>,
    transaction: i16,
    speeds: Option<(f32, f32)>,
}
impl Outbound {
    /// Observe server inventory before exposing it to the game thread.
    pub fn observe(&mut self, id: i32, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        let mut r = Reader::new(bytes);
        match id {
            0x09 => {
                let slot = r.u8()?;
                if slot > 8 {
                    return Err(Error::new("invalid held slot"));
                }
                self.selected = slot;
            }
            0x2f => {
                let window = i32::from(r.i8()?);
                let slot = r.i16()?;
                let item = super::slot::Slot::read(&mut r)?;
                super::finish(&r)?;
                if window == -2 && (0..36).contains(&slot) {
                    let mapped = if slot < 9 { slot + 36 } else { slot };
                    self.slots.insert((0, mapped), item);
                } else {
                    self.slots.insert((window, slot), item);
                }
            }
            0x30 => {
                let window = i32::from(r.u8()?);
                let count = r.i16()?;
                if !(0..=4096).contains(&count) {
                    return Err(Error::new("invalid legacy inventory size"));
                }
                let mut incoming = Vec::with_capacity(count as usize);
                for slot in 0..count {
                    incoming.push((slot, super::slot::Slot::read(&mut r)?));
                }
                super::finish(&r)?;
                self.slots.retain(|(w, _), _| *w != window);
                for (slot, item) in incoming {
                    self.slots.insert((window, slot), item);
                }
            }
            0x32 => {
                let window = r.u8()?;
                let action = r.i16()?;
                let accepted = r.bool()?;
                super::finish(&r)?;
                if !accepted {
                    let mut w = Writer::new();
                    w.varint(0x0f);
                    w.u8(window);
                    w.i16(action);
                    w.bool(true);
                    return Ok(Some(w.data));
                }
            }
            0x39 => {
                r.u8()?;
                let fly = r.f32()?;
                let walk = r.f32()?;
                if !fly.is_finite() || !walk.is_finite() || fly < 0. || walk < 0. {
                    return Err(Error::new("invalid legacy ability speeds"));
                }
                self.speeds = Some((fly, walk));
            }
            _ => return Ok(None),
        }
        super::finish(&r)?;
        Ok(None)
    }
    fn held(&self) -> Option<&super::slot::Slot> {
        self.slots
            .get(&(0, 36 + i16::from(self.selected)))
            .and_then(Option::as_ref)
    }
    pub fn encode(&mut self, packet: &S) -> Result<Option<Vec<u8>>> {
        let bytes = match packet {
            S::AcceptTeleportation(p) => Pose {
                x: p.x,
                y: p.y,
                z: p.z,
                yaw: p.yaw,
                pitch: p.pitch,
            }
            .encode(false)?,
            S::MovePlayerPosRot(p) => Pose {
                x: p.x,
                y: p.y,
                z: p.z,
                yaw: p.yaw,
                pitch: p.pitch,
            }
            .encode(p.flags & 1 != 0)?,
            S::MovePlayerPos(p) => {
                Pose {
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    ..Pose::default()
                }
                .encode(false)?;
                let mut w = Writer::new();
                w.varint(4);
                w.f64(p.x);
                w.f64(p.y);
                w.f64(p.z);
                w.bool(p.flags & 1 != 0);
                w.data
            }
            S::MovePlayerRot(p) => {
                Pose {
                    yaw: p.yaw,
                    pitch: p.pitch,
                    ..Pose::default()
                }
                .encode(false)?;
                let mut w = Writer::new();
                w.varint(5);
                w.f32(p.yaw);
                w.f32(p.pitch);
                w.bool(p.flags & 1 != 0);
                w.data
            }
            S::MovePlayerStatus(p) => actions::ground(p.flags & 1 != 0),
            S::Attack(p) => actions::interact_entity(p.id, true)?,
            S::Interact(p) if p.hand == 0 => actions::interact_entity(p.id, false)?,
            S::Punch(_) => actions::swing_arm(),
            S::SetCarriedItem(p) => {
                let slot = u8::try_from(p.slot).map_err(|_| Error::new("invalid hotbar slot"))?;
                let bytes = actions::held_slot(slot)?;
                self.selected = slot;
                bytes
            }
            S::PlayerAbilities(p) => {
                let (fly, walk) = self.speeds.unwrap_or((0.05, 0.1));
                let mut w = Writer::new();
                w.varint(0x13);
                w.i8(p.flags);
                w.f32(fly);
                w.f32(walk);
                w.data
            }
            S::UseItem(p) if p.hand == 0 => {
                let mut w = Writer::new();
                w.varint(8);
                w.i64(-1);
                w.u8(255);
                super::slot::Slot::write(self.held(), &mut w)?;
                w.bytes(&[0, 0, 0]);
                w.data
            }
            S::UseItemOn(p) if p.hand == 0 => {
                if !(0..=5).contains(&p.face) || p.cursor.iter().any(|v| !v.is_finite()) {
                    return Err(Error::new("invalid legacy placement"));
                }
                let mut w = Writer::new();
                w.varint(8);
                w.i64(actions::block_position(p.pos[0], p.pos[1], p.pos[2])? as i64);
                w.u8(p.face as u8);
                super::slot::Slot::write(self.held(), &mut w)?;
                for c in p.cursor {
                    w.u8((c.clamp(0., 1.) * 16.) as u8);
                }
                w.data
            }
            S::ContainerClose(p) => {
                let window =
                    u8::try_from(p.container).map_err(|_| Error::new("invalid legacy window"))?;
                let mut w = Writer::new();
                w.varint(0x0d);
                w.u8(window);
                w.data
            }
            S::ContainerClick(p) if (0..=6).contains(&p.input) => {
                let window =
                    u8::try_from(p.container).map_err(|_| Error::new("invalid legacy window"))?;
                let action = self.transaction.wrapping_add(1);
                let mut w = Writer::new();
                w.varint(0x0e);
                w.u8(window);
                w.i16(p.slot);
                w.i8(p.button);
                w.i16(action);
                w.u8(p.input as u8);
                let clicked = self
                    .slots
                    .get(&(p.container, p.slot))
                    .and_then(Option::as_ref);
                super::slot::Slot::write(clicked, &mut w)?;
                self.transaction = action;
                w.data
            }
            S::Chat(p) => play::chat(&p.message)?,
            S::ChatCommand(p) => play::chat(&format!("/{}", p.command))?,
            S::ClientCommand(p) if p.action == 0 => actions::respawn(),
            S::PlayerAction(p) => {
                let action = match p.action {
                    0 => Digging::Start,
                    1 => return Ok(None), // 26.3 CHANGE_DESTROY_DIRECTION has no 1.8 equivalent.
                    2 => Digging::Cancel,
                    3 => Digging::Finish,
                    4 => Digging::DropStack,
                    5 => Digging::DropOne,
                    6 => Digging::ReleaseUse,
                    _ => return Err(Error::new("unsupported legacy block action")),
                };
                actions::digging(action, p.pos[0], p.pos[1], p.pos[2], p.direction)?
            }
            S::PlayerCommand(p) => {
                let action = match p.action {
                    0 => EntityAction::LeaveBed,
                    1 => EntityAction::StartSprinting,
                    2 => EntityAction::StopSprinting,
                    3 => EntityAction::HorseJump,
                    4 => return Ok(None),
                    5 => EntityAction::OpenHorseInventory,
                    _ => return Err(Error::new("unsupported legacy player command")),
                };
                actions::entity_action(p.id, action, p.data)?
            }
            S::PlayerInput(p) => {
                let sneaking = p.flags & 32 != 0;
                if sneaking == self.sneaking {
                    return Ok(None);
                }
                let encoded = actions::entity_action(
                    self.entity,
                    if sneaking {
                        EntityAction::StartSneaking
                    } else {
                        EntityAction::StopSneaking
                    },
                    0,
                )?;
                self.sneaking = sneaking;
                encoded
            }
            S::ClientInformation(p) => {
                if p.language.encode_utf16().count() > 7 || !(0..=2).contains(&p.chat_visibility) {
                    return Err(Error::new("invalid legacy client information"));
                }
                let mut w = Writer::new();
                w.varint(0x15);
                w.string(&p.language);
                w.i8(p.view_distance);
                w.u8(p.chat_visibility as u8);
                w.bool(p.chat_colors);
                w.u8(p.model_customisation);
                w.data
            }
            // These are modern protocol bookkeeping, not gameplay actions.
            S::ClientTickEnd(_)
            | S::PlayerLoaded(_)
            | S::ChatAck(_)
            | S::ChatSessionUpdate(_)
            | S::ChunkBatchReceived(_) => return Ok(None),
            _ => {
                return Err(Error::new(
                    "engine action is not implemented for protocol 47",
                ));
            }
        };
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::play as modern;
    #[test]
    fn modern_digging_ids_are_translated_not_forwarded() {
        let mut bridge = Outbound::default();
        let p = |action| {
            S::PlayerAction(modern::PlayerAction {
                action,
                pos: [1, 64, 2],
                direction: 1,
                sequence: 123,
            })
        };
        assert_eq!(
            bridge.encode(&p(3)).unwrap().unwrap(),
            [7, 2, 0, 0, 0, 65, 0, 0, 0, 2, 1]
        );
        assert!(bridge.encode(&p(1)).unwrap().is_none());
        assert!(bridge.encode(&p(7)).is_err());
    }
    #[test]
    fn teleport_ack_is_a_position_packet_and_has_no_teleport_id() {
        let p = S::AcceptTeleportation(modern::AcceptTeleportation {
            id: 99,
            x: 0.,
            y: 0.,
            z: 0.,
            yaw: 0.,
            pitch: 0.,
        });
        let bytes = Outbound::default().encode(&p).unwrap().unwrap();
        assert_eq!(bytes.len(), 34);
        assert_eq!(bytes[0], 6);
        assert!(bytes[1..].iter().all(|b| *b == 0));
    }
    #[test]
    fn input_sends_only_sneaking_transitions() {
        let mut b = Outbound {
            entity: 300,
            ..Default::default()
        };
        let p = |flags| S::PlayerInput(modern::PlayerInput { flags });
        assert!(b.encode(&p(1)).unwrap().is_none());
        assert_eq!(b.encode(&p(33)).unwrap().unwrap(), [11, 0xac, 2, 0, 0]);
        assert!(b.encode(&p(32)).unwrap().is_none());
        assert_eq!(b.encode(&p(0)).unwrap().unwrap(), [11, 0xac, 2, 1, 0]);
    }
    #[test]
    fn placement_uses_server_held_stack_and_legacy_cursor_bytes() {
        let mut b = Outbound::default();
        b.observe(0x2f, &[0, 0, 36, 0, 1, 64, 0, 0, 0]).unwrap();
        let packet = S::UseItemOn(modern::UseItemOn {
            hand: 0,
            pos: [1, 64, 2],
            face: 1,
            cursor: [0.5, 1., 0.],
            inside: false,
            world_border: false,
            sequence: 999,
        });
        assert_eq!(
            b.encode(&packet).unwrap().unwrap(),
            [8, 0, 0, 0, 65, 0, 0, 0, 2, 1, 0, 1, 64, 0, 0, 0, 8, 16, 0]
        );
    }
    #[test]
    fn rejected_inventory_transaction_is_acknowledged_in_legacy_format() {
        let mut b = Outbound::default();
        assert_eq!(
            b.observe(0x32, &[0, 0, 9, 0]).unwrap().unwrap(),
            [15, 0, 0, 9, 1]
        );
        assert!(b.observe(0x32, &[0, 0, 9, 1]).unwrap().is_none());
    }
}
