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
            .and_then(|r| r.legacy_blocks.get(usize::from(state)))
            .copied()
            .flatten()
        {
            return Ok(value);
        }
        let descriptor = horizon_world::datafix::legacy_block_state(state)?;
        let value = self.pack.assets.registry.resolve(&descriptor)?.0;
        if let Some(remote) = self.remote.as_mut() {
            remote.legacy_blocks[usize::from(state)] = Some(value);
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
    fn legacy_metadata(
        &mut self,
        entity: i32,
        fields: Vec<(u8, horizon_protocol::legacy18::metadata::Value)>,
    ) -> Result<()> {
        use horizon_protocol::{
            entity_data::{Value as New, serializer as S},
            legacy18::metadata::Value as Old,
        };
        let mut values = Vec::new();
        for (index, value) in fields {
            match (index, value) {
                (0, Old::Byte(flags)) => {
                    values.push((0, S::BYTE, New::Byte(flags & 0x2f)));
                    values.push((6, S::POSE, New::Id(if flags & 2 != 0 { 5 } else { 0 })));
                    values.push((8, S::BYTE, New::Byte(if flags & 0x10 != 0 { 1 } else { 0 })));
                }
                (1, Old::Short(air)) => values.push((1, S::INT, New::Int(i32::from(air)))),
                (2, Old::String(name)) => values.push((
                    2,
                    S::OPTIONAL_COMPONENT,
                    New::OptText(
                        (!name.is_empty()).then(|| horizon_protocol::text::literal(&name)),
                    ),
                )),
                (3, Old::Byte(show)) => values.push((3, S::BOOLEAN, New::Bool(show != 0))),
                (6, Old::Float(health)) if health.is_finite() => {
                    values.push((9, S::FLOAT, New::Float(health)))
                }
                (10, Old::Item(item)) => {
                    let stack =
                        item.and_then(|s| horizon_items::item_nbt::item_from_nbt(&s.upgraded()));
                    values.push((
                        horizon_protocol::entity_data::ITEM_ENTITY_ITEM,
                        S::ITEM_STACK,
                        New::Item(self.to_network(&stack)),
                    ));
                }
                _ => {}
            }
        }
        self.handle_packet(C::SetEntityData(play::SetEntityData { id: entity, values }))
    }
    fn legacy_misc(&mut self, id: i32, bytes: &[u8]) -> Result<()> {
        use horizon_protocol::buf::Reader;
        let mut r = Reader::new(bytes);
        if id == 0x38 {
            let action = r.varint()?;
            let count = r.count(1024)?;
            if !(0..=4).contains(&action) {
                return Err("unknown legacy player-list action".into());
            }
            let mut entries = Vec::with_capacity(count);
            let mut removed = Vec::new();
            for _ in 0..count {
                let uuid = r.uuid()?;
                if action == 4 {
                    removed.push(uuid);
                    continue;
                }
                let mut info = play::PlayerInfo {
                    uuid,
                    ..Default::default()
                };
                if action == 0 {
                    info.name = Some(r.string(16)?);
                    let properties = r.count(64)?;
                    for _ in 0..properties {
                        let name = r.string(32767)?;
                        let value = r.string(32767)?;
                        let signature = if r.bool()? {
                            Some(r.string(32767)?)
                        } else {
                            None
                        };
                        info.properties
                            .push(horizon_protocol::packets::login::Property {
                                name,
                                value,
                                signature,
                            });
                    }
                    info.listed = Some(true);
                    info.show_hat = Some(true);
                }
                if action == 0 || action == 1 {
                    info.game_mode = Some(r.varint()?);
                }
                if action == 0 || action == 2 {
                    info.latency = Some(r.varint()?);
                }
                if action == 0 || action == 3 {
                    info.display_name = Some(if r.bool()? {
                        Some(horizon_client::scoreboard::json_to_tag(
                            &serde_json::from_str(&r.string(32767)?)?,
                        ))
                    } else {
                        None
                    });
                }
                entries.push(info);
            }
            if r.remaining() != 0 {
                return Err("trailing legacy player-list data".into());
            }
            return if action == 4 {
                self.handle_packet(C::PlayerInfoRemove(play::PlayerInfoRemove {
                    uuids: removed,
                }))
            } else {
                self.handle_packet(C::PlayerInfoUpdate(play::PlayerInfoUpdate {
                    actions: match action {
                        0 => 189,
                        1 => 4,
                        2 => 16,
                        _ => 32,
                    },
                    entries,
                }))
            };
        }
        if id == 0x22 {
            let x = r.i32()?.checked_mul(16).ok_or("legacy chunk X overflow")?;
            let z = r.i32()?.checked_mul(16).ok_or("legacy chunk Z overflow")?;
            let count = r.count(r.remaining() / 3)?;
            let mut changes = Vec::with_capacity(count);
            for _ in 0..count {
                let packed = r.u16()?;
                let state = r.varint()?;
                let state = u16::try_from(state).map_err(|_| "invalid legacy block state")?;
                changes.push((
                    [
                        x + i32::from(packed >> 12),
                        i32::from(packed & 255),
                        z + i32::from((packed >> 8) & 15),
                    ],
                    state,
                ));
            }
            if r.remaining() != 0 {
                return Err("trailing legacy block update data".into());
            }
            for (pos, state) in changes {
                let state = self.legacy_state(state)? as i32;
                self.handle_packet(C::BlockUpdate(play::BlockUpdate { pos, state }))?;
            }
            return Ok(());
        }
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
        let mut metadata = None;
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
                metadata = Some((entity, horizon_protocol::legacy18::metadata::read(&mut r)?));
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
            0x04 => {
                let entity = r.varint()?;
                let slot = r.i16()?;
                let item = horizon_protocol::legacy18::slot::Slot::read(&mut r)?
                    .and_then(|s| horizon_items::item_nbt::item_from_nbt(&s.upgraded()));
                let slot = match slot {
                    0 => 0,
                    1 => 2,
                    2 => 3,
                    3 => 4,
                    4 => 5,
                    _ => return Err("invalid legacy equipment slot".into()),
                };
                C::SetEquipment(play::SetEquipment {
                    id: entity,
                    slots: vec![(slot, self.to_network(&item))],
                })
            }
            0x0f => {
                let entity = r.varint()?;
                let legacy_kind = r.u8()?;
                let name = match legacy_kind {
                    50 => "creeper",
                    51 => "skeleton",
                    52 => "spider",
                    53 => "giant",
                    54 => "zombie",
                    55 => "slime",
                    56 => "ghast",
                    57 => "zombified_piglin",
                    58 => "enderman",
                    59 => "cave_spider",
                    60 => "silverfish",
                    61 => "blaze",
                    62 => "magma_cube",
                    63 => "ender_dragon",
                    64 => "wither",
                    65 => "bat",
                    66 => "witch",
                    67 => "endermite",
                    68 => "guardian",
                    90 => "pig",
                    91 => "sheep",
                    92 => "cow",
                    93 => "chicken",
                    94 => "squid",
                    95 => "wolf",
                    96 => "mooshroom",
                    97 => "snow_golem",
                    98 => "ocelot",
                    99 => "iron_golem",
                    100 => "horse",
                    101 => "rabbit",
                    120 => "villager",
                    _ => return Err("unknown legacy mob type".into()),
                };
                let kind = self
                    .remote
                    .as_ref()
                    .and_then(|remote| {
                        remote
                            .builtin
                            .get("minecraft:entity_type")?
                            .id(&format!("minecraft:{name}"))
                    })
                    .ok_or("legacy mob missing from renderer registry")?;
                let position = [
                    f64::from(r.i32()?) / 32.,
                    f64::from(r.i32()?) / 32.,
                    f64::from(r.i32()?) / 32.,
                ];
                let yaw = r.i8()?;
                let pitch = r.i8()?;
                let head_yaw = r.i8()?;
                let velocity = [
                    f64::from(r.i16()?) / 8000.,
                    f64::from(r.i16()?) / 8000.,
                    f64::from(r.i16()?) / 8000.,
                ];
                metadata = Some((entity, horizon_protocol::legacy18::metadata::read(&mut r)?));
                C::AddEntity(play::AddEntity {
                    id: entity,
                    uuid: horizon_protocol::crypto::offline_uuid(&format!("legacy-mob-{entity}")),
                    kind,
                    position,
                    velocity,
                    yaw,
                    pitch,
                    head_yaw,
                    data: 0,
                })
            }
            0x1c => {
                let entity = r.varint()?;
                let values = horizon_protocol::legacy18::metadata::read(&mut r)?;
                if r.remaining() != 0 {
                    return Err("trailing legacy metadata".into());
                }
                return self.legacy_metadata(entity, values);
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
            0x3b => {
                let name = r.string(16)?;
                let method = r.i8()?;
                let display = if method == 0 || method == 2 {
                    let title = horizon_protocol::text::literal(&r.string(32)?);
                    let kind = match r.string(16)?.as_str() {
                        "integer" => 0,
                        "hearts" => 1,
                        _ => return Err("unknown legacy objective type".into()),
                    };
                    Some((title, kind, None))
                } else if method == 1 {
                    None
                } else {
                    return Err("invalid legacy objective action".into());
                };
                C::SetObjective(play::SetObjective {
                    name,
                    method,
                    display,
                })
            }
            0x3c => {
                let owner = r.string(40)?;
                let action = r.u8()?;
                let objective = r.string(16)?;
                match action {
                    0 => C::SetScore(play::SetScore {
                        owner,
                        objective,
                        score: r.varint()?,
                        display: None,
                        number_format: None,
                    }),
                    1 => C::ResetScore(play::ResetScore {
                        owner,
                        objective: (!objective.is_empty()).then_some(objective),
                    }),
                    _ => return Err("invalid legacy score action".into()),
                }
            }
            0x3d => C::SetDisplayObjective(play::SetDisplayObjective {
                slot: i32::from(r.u8()?),
                objective: r.string(16)?,
            }),
            0x3e => {
                let name = r.string(16)?;
                let method = r.i8()?;
                if !(0..=4).contains(&method) {
                    return Err("invalid legacy team action".into());
                }
                let parameters = if method == 0 || method == 2 {
                    let display_name = horizon_protocol::text::literal(&r.string(32)?);
                    let prefix = horizon_protocol::text::literal(&r.string(16)?);
                    let suffix = horizon_protocol::text::literal(&r.string(16)?);
                    let options = r.i8()?;
                    let name_tag_visibility = match r.string(32)?.as_str() {
                        "always" => 0,
                        "never" => 1,
                        "hideForOtherTeams" => 2,
                        "hideForOwnTeam" => 3,
                        _ => return Err("invalid legacy team visibility".into()),
                    };
                    let color = r.i8()?;
                    Some(play::TeamParameters {
                        display_name,
                        prefix,
                        suffix,
                        options,
                        name_tag_visibility,
                        collision_rule: 0,
                        color: (0..=15).contains(&color).then_some(i32::from(color)),
                    })
                } else {
                    None
                };
                let mut players = Vec::new();
                if matches!(method, 0 | 3 | 4) {
                    let count = r.count(4096)?;
                    for _ in 0..count {
                        players.push(r.string(40)?);
                    }
                }
                C::SetPlayerTeam(play::SetPlayerTeam {
                    name,
                    method,
                    parameters,
                    players,
                })
            }
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
        self.handle_packet(packet)?;
        if let Some((entity, fields)) = metadata {
            self.legacy_metadata(entity, fields)?;
        }
        Ok(())
    }
}
