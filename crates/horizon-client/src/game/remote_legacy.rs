//! Protocol 47 receive adapter. Chunks enter the real streaming renderer; old
//! numeric state IDs are upgraded before they reach modern block registries.
use super::Game;
use horizon_client::{
    Result,
    chunk::{Chunk, Light, Section},
};
use horizon_protocol::{
    legacy18::{
        chunk::{Column, decode_bulk},
        play::Packet,
    },
    packets::play::{self, Clientbound as C},
};
use std::sync::Arc;

fn spawn(dimension: i32, gamemode: u8, level_type: &str) -> play::SpawnInfo {
    let (dimension_type, name) = match dimension {
        -1 => (1, "minecraft:the_nether"),
        1 => (2, "minecraft:the_end"),
        _ => (0, "minecraft:overworld"),
    };
    play::SpawnInfo {
        dimension_type,
        dimension: name.into(),
        seed: 0,
        game_type: i32::from(gamemode & 7),
        previous_game_type: None,
        debug: false,
        flat: level_type == "flat",
        last_death: None,
        portal_cooldown: 0,
        sea_level: 63,
    }
}
impl Game<'_> {
    fn legacy_state(&mut self, state: u16) -> Result<u32> {
        if let Some(value) = self
            .remote
            .as_ref()
            .and_then(|r| r.legacy_blocks.get(&state))
        {
            return Ok(*value);
        }
        let descriptor = horizon_world::datafix::legacy_block_state(state)?;
        let value = self.pack.assets.registry.resolve(&descriptor)?.0;
        if let Some(remote) = self.remote.as_mut() {
            remote.legacy_blocks.insert(state, value);
        }
        Ok(value)
    }

    fn legacy_column(&mut self, column: &Column<'_>) -> Result<()> {
        if column.is_unload() {
            return self.handle_packet(C::ForgetLevelChunk(play::ForgetLevelChunk {
                pos: [column.x, column.z],
            }));
        }
        let mut chunk = if !column.ground_up {
            match self.world.chunks.get(&(column.x, column.z)) {
                Some(chunk) => (**chunk).clone(),
                None => return Ok(()), // A partial update cannot create a column.
            }
        } else {
            Chunk::with_layout(column.x, column.z, self.world.layout())
        };
        let offset = -self.world.layout().min_y().div_euclid(16);
        for sy in 0..16u8 {
            let Some(source) = column.section(sy) else {
                continue;
            };
            let target = i32::from(sy) + offset;
            if target < 0 || target as usize >= chunk.sections.len() {
                continue;
            }
            let mut states = [0u32; 4096];
            let mut light = Box::new([0u8; 4096]);
            for i in 0..4096 {
                let (x, y, z) = ((i & 15) as u8, (i >> 8) as u8, ((i >> 4) & 15) as u8);
                states[i] = self.legacy_state(source.state(x, y, z).expect("in-range block"))?;
                light[i] = source.block_light(x, y, z).unwrap_or(0)
                    | (source.sky_light(x, y, z).unwrap_or(0) << 4);
            }
            let mut section = Section {
                blocks: Section::from_blocks(&states),
                light: Light::Full(light),
                ..(*chunk.sections[target as usize]).clone()
            };
            section.compact();
            chunk.sections[target as usize] = Arc::new(section);
        }
        if let Some(biomes) = column.biomes() {
            let mut cells = [0u32; 64];
            for z in 0..4 {
                for x in 0..4 {
                    let name =
                        horizon_world::datafix::legacy_biome(biomes[(z * 4 + 2) * 16 + x * 4 + 2]);
                    let id = u32::from(self.pack.context.biomes.id(name));
                    for y in 0..4 {
                        cells[y * 16 + z * 4 + x] = id;
                    }
                }
            }
            let container = horizon_core::paletted::PalettedContainer::from_values(
                horizon_client::chunk::BIOME_STRATEGY,
                &cells,
            );
            for section in &mut chunk.sections {
                Arc::make_mut(section).biomes = container.clone();
            }
        }
        self.streaming.insert_remote(&mut self.world, chunk);
        Ok(())
    }

    pub(super) fn handle_legacy_packet(&mut self, id: i32, bytes: &[u8]) -> Result<()> {
        let packet = Packet::decode(id, bytes)?;
        match packet {
            Packet::Join {
                entity,
                gamemode,
                dimension,
                max_players,
                level_type,
                reduced_debug,
                ..
            } => {
                self.handle_packet(C::Login(play::Login {
                    player_id: entity,
                    hardcore: gamemode & 8 != 0,
                    levels: vec![
                        "minecraft:overworld".into(),
                        "minecraft:the_nether".into(),
                        "minecraft:the_end".into(),
                    ],
                    max_players: i32::from(max_players),
                    chunk_radius: self.settings.render_distance,
                    simulation_distance: self.settings.render_distance,
                    reduced_debug_info: reduced_debug,
                    show_death_screen: true,
                    limited_crafting: false,
                    spawn: spawn(i32::from(dimension), gamemode, &level_type),
                    online_mode: false,
                    enforces_secure_chat: false,
                }))?;
            }
            Packet::Respawn {
                dimension,
                gamemode,
                level_type,
                ..
            } => self.handle_packet(C::Respawn(play::Respawn {
                spawn: spawn(dimension, gamemode, &level_type),
                keep: 0,
            }))?,
            Packet::Position {
                x,
                y,
                z,
                yaw,
                pitch,
                flags,
            } => self.handle_packet(C::PlayerPosition(play::PlayerPosition {
                teleport: 0,
                position: [x, y, z],
                delta: [0.; 3],
                yaw,
                pitch,
                relatives: i32::from(flags),
            }))?,
            Packet::Health {
                health,
                food,
                saturation,
            } => self.handle_packet(C::SetHealth(play::SetHealth {
                health,
                food,
                saturation,
            }))?,
            Packet::HeldSlot(slot) => self.handle_packet(C::SetHeldSlot(play::SetHeldSlot {
                slot: i32::from(slot),
            }))?,
            Packet::Velocity { entity, x, y, z } => {
                self.handle_packet(C::SetEntityMotion(play::SetEntityMotion {
                    id: entity,
                    velocity: [
                        f64::from(x) / 8000.,
                        f64::from(y) / 8000.,
                        f64::from(z) / 8000.,
                    ],
                }))?
            }
            Packet::Chat { json, position } => {
                let value = serde_json::from_str(&json)?;
                self.handle_packet(C::SystemChat(play::SystemChat {
                    content: horizon_client::scoreboard::json_to_tag(&value),
                    overlay: position == 2,
                }))?;
            }
            Packet::BlockChange { x, y, z, state } => {
                let state = self.legacy_state(state)?;
                self.handle_packet(C::BlockUpdate(play::BlockUpdate {
                    pos: [x, i32::from(y), z],
                    state: state as i32,
                }))?;
            }
            Packet::Chunk(bytes) => {
                let sky = self
                    .remote
                    .as_ref()
                    .is_some_and(|r| r.dimension.has_skylight);
                self.legacy_column(&Column::decode(bytes, sky)?)?;
            }
            Packet::ChunkBulk(bytes) => {
                for column in decode_bulk(bytes)? {
                    self.legacy_column(&column)?;
                }
            }
            Packet::Unknown { id, payload } => {
                self.legacy_misc(id, payload)?;
            } // Remaining packet families are tracked as unfinished support.
            Packet::KeepAlive(_) | Packet::Disconnect(_) => {} // Network thread owns these.
        }
        Ok(())
    }
    fn legacy_misc(&mut self, id: i32, bytes: &[u8]) -> Result<()> {
        use horizon_protocol::buf::Reader;
        let mut r = Reader::new(bytes);
        if id == 0x2f || id == 0x30 {
            use horizon_protocol::legacy18::slot::Slot;
            let window = if id == 0x2f {
                i32::from(r.i8()?)
            } else {
                i32::from(r.u8()?)
            };
            let mut changes = Vec::new();
            if id == 0x2f {
                let slot = r.i16()?;
                changes.push((slot, Slot::read(&mut r)?));
            } else {
                let count = r.i16()?;
                if !(0..=4096).contains(&count) {
                    return Err("invalid legacy window size".into());
                }
                for slot in 0..count {
                    changes.push((slot, Slot::read(&mut r)?));
                }
            }
            if r.remaining() != 0 {
                return Err("trailing legacy inventory data".into());
            }
            for (slot, item) in changes {
                let stack =
                    item.and_then(|item| horizon_items::item_nbt::item_from_nbt(&item.upgraded()));
                if window == -1 && slot == -1 {
                    self.carried = stack;
                } else if window == -2 && (0..36).contains(&slot) {
                    self.set_menu_slot(
                        0,
                        i32::from(if slot < 9 { slot + 36 } else { slot }),
                        stack,
                    );
                } else {
                    self.set_menu_slot(window, i32::from(slot), stack);
                }
            }
            self.remote_synced();
            return Ok(());
        }
        let packet = match id {
            0x0b => C::Animate(play::Animate {
                id: r.varint()?,
                action: r.u8()?,
            }),
            0x0c => {
                let entity = r.varint()?;
                let uuid = r.uuid()?;
                let position = [
                    f64::from(r.i32()?) / 32.,
                    f64::from(r.i32()?) / 32.,
                    f64::from(r.i32()?) / 32.,
                ];
                let yaw = r.i8()?;
                let pitch = r.i8()?;
                let _held_item = r.i16()?;
                // Metadata follows; its separate codec is required for equipment/status.
                r.rest();
                let kind = self
                    .remote
                    .as_ref()
                    .and_then(|remote| {
                        remote
                            .builtin
                            .get("minecraft:entity_type")?
                            .id("minecraft:player")
                    })
                    .ok_or("player entity type missing from renderer registry")?;
                C::AddEntity(play::AddEntity {
                    id: entity,
                    uuid,
                    kind,
                    position,
                    velocity: [0.; 3],
                    yaw,
                    pitch,
                    head_yaw: yaw,
                    data: 0,
                })
            }
            0x13 => {
                let count = r.count(bytes.len())?;
                let ids = (0..count)
                    .map(|_| r.varint())
                    .collect::<horizon_protocol::Result<Vec<_>>>()?;
                C::RemoveEntities(play::RemoveEntities { ids })
            }
            0x15..=0x17 => {
                let entity = r.varint()?;
                let delta = if id != 0x16 {
                    Some(play::VecDelta::Linear([
                        i16::from(r.i8()?) * 128,
                        i16::from(r.i8()?) * 128,
                        i16::from(r.i8()?) * 128,
                    ]))
                } else {
                    None
                };
                let rotation = if id != 0x15 {
                    Some((r.i8()?, r.i8()?))
                } else {
                    None
                };
                let on_ground = r.bool()?;
                C::MoveEntityPosRot(play::MoveEntityPosRot(play::MoveEntity {
                    id: entity,
                    delta,
                    rotation,
                    on_ground,
                }))
            }
            0x18 => C::TeleportEntity(play::TeleportEntity {
                id: r.varint()?,
                position: [
                    f64::from(r.i32()?) / 32.,
                    f64::from(r.i32()?) / 32.,
                    f64::from(r.i32()?) / 32.,
                ],
                delta: [0.; 3],
                yaw: horizon_protocol::buf::unpack_degrees(r.i8()?),
                pitch: horizon_protocol::buf::unpack_degrees(r.i8()?),
                relatives: 0,
                on_ground: r.bool()?,
            }),
            0x19 => C::RotateHead(play::RotateHead {
                id: r.varint()?,
                head_yaw: r.i8()?,
            }),
            0x1a => C::EntityEvent(play::EntityEvent {
                id: r.i32()?,
                event: r.i8()?,
            }),
            0x1f => C::SetExperience(play::SetExperience {
                progress: r.f32()?,
                level: r.varint()?,
                total: r.varint()?,
            }),
            0x2d => {
                let container = i32::from(r.u8()?);
                let kind = r.string(32)?;
                let title = r.string(32767)?;
                let slots = r.u8()?;
                let name = match kind.as_str() {
                    "minecraft:chest" | "minecraft:container"
                        if slots > 0 && slots <= 54 && slots % 9 == 0 =>
                    {
                        format!("minecraft:generic_9x{}", slots / 9)
                    }
                    "minecraft:crafting_table" => "minecraft:crafting".into(),
                    "minecraft:enchanting_table" => "minecraft:enchantment".into(),
                    "minecraft:dispenser" | "minecraft:dropper" => "minecraft:generic_3x3".into(),
                    "minecraft:furnace"
                    | "minecraft:brewing_stand"
                    | "minecraft:anvil"
                    | "minecraft:beacon"
                    | "minecraft:hopper" => kind,
                    _ => return Err(format!("legacy container not implemented: {kind}").into()),
                };
                let menu = self
                    .remote
                    .as_ref()
                    .and_then(|remote| remote.builtin.get("minecraft:menu")?.id(&name))
                    .ok_or_else(|| format!("missing menu {name}"))?;
                C::OpenScreen(play::OpenScreen {
                    container,
                    menu,
                    title: horizon_client::scoreboard::json_to_tag(&serde_json::from_str(&title)?),
                })
            }
            0x2e => C::ContainerClose(play::ContainerClose {
                container: i32::from(r.u8()?),
            }),
            0x31 => C::ContainerSetData(play::ContainerSetData {
                container: i32::from(r.u8()?),
                id: r.i16()?,
                value: r.i16()?,
            }),
            0x2b => C::GameEvent(play::GameEvent {
                event: r.u8()?,
                param: r.f32()?,
            }),
            0x39 => C::PlayerAbilities(play::PlayerAbilities {
                flags: r.i8()?,
                flying_speed: r.f32()?,
                walking_speed: r.f32()?,
            }),
            _ => return Ok(()),
        };
        if r.remaining() != 0 {
            return Err(format!("trailing data in legacy packet {id:#x}").into());
        }
        self.handle_packet(packet)
    }
}
