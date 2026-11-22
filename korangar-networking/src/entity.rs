use ragnarok_packets::*;

#[derive(Debug)]
pub struct EntityData {
    pub entity_id: EntityId,
    pub movement_speed: u16,
    pub job_id: JobId,
    pub head: u16,
    pub position: WorldPosition,
    pub destination: Option<WorldPosition>,
    pub health_points: i32,
    pub maximum_health_points: i32,
    pub head_direction: usize,
    pub sex: Sex,
    /// Hair color palette index (`head_palette` in the packets).
    pub head_palette: u16,
    /// Clothes color palette index (`body_palette` in the packets).
    pub body_palette: u16,
    /// Id of the guild the entity is in, `0` if it is not in a guild.
    ///
    /// The entity-appear packets only carry the guild id; the guild *name* is
    /// sent later as part of the player details response (`0x0A30`,
    /// `RequestPlayerDetailsSuccessPacket`) and is therefore empty here.
    pub guild_id: u32,
    /// The name of the guild the entity is in, if known.
    ///
    /// This is empty for entities that spawn through the entity-appear
    /// packets; the name is filled in once the server answers a name request.
    pub guild_name: String,
    /// Lower headgear view id (`accessory` in the packets).
    pub accessory: u16,
    /// Upper headgear view id (`accessory2` in the packets).
    pub accessory2: u16,
    /// Middle headgear view id (`accessory3` in the packets).
    pub accessory3: u16,
    /// Weapon view id (`weapon` in the entity-appear packets).
    ///
    /// Carried through to the world layer so the weapon sprite can be composed
    /// once the weapon asset mapping is available (task-105).
    pub weapon: u32,
    /// Shield view id (`shield` in the entity-appear packets).
    pub shield: u32,
    /// Robe view id (`robe` in the entity-appear packets).
    pub robe: u16,
    /// Option bit mask (`effectState`): falcon, cart, riding, hide/cloak, costumes, ...
    pub effect_state: u32,
    /// `bodyState` (`opt1`): freeze/stone/stun/sleep/burning/... visual state.
    pub body_state: u16,
    /// `healthState` (`opt2`): poison/curse/silence/confusion/blind/... visual state.
    pub health_state: u16,
    /// Dead/sit state of the idle-unit packet (`state`: 0 standing, 1 dead, 2 sitting).
    pub dead_sit: u8,
}

impl EntityData {
    pub fn from_character(account_id: AccountId, character_information: &CharacterInformation, position: WorldPosition) -> Self {
        Self {
            entity_id: EntityId(account_id.0),
            movement_speed: character_information.movement_speed as u16,
            job_id: character_information.job_id,
            head: character_information.head as u16,
            position,
            destination: None,
            health_points: character_information.health_points as i32,
            maximum_health_points: character_information.maximum_health_points as i32,
            head_direction: 0, // TODO: get correct rotation
            sex: character_information.sex,
            head_palette: character_information.head_palette as u16,
            body_palette: character_information.body_palette as u16,
            // `CharacterInformation` (0x006B) does not contain any guild
            // information, the guild name of a character is only sent in the
            // player details response (`0x0A30`).
            guild_id: 0,
            guild_name: String::new(),
            accessory: character_information.accessory as u16,
            accessory2: character_information.accessory2 as u16,
            accessory3: character_information.accessory3 as u16,
            weapon: character_information.weapon as u32,
            shield: character_information.shield as u32,
            // The 2019 character list row has no robe view id.
            robe: 0,
            effect_state: character_information.effect_state as u32,
            body_state: character_information.body_state as u16,
            health_state: character_information.health_state as u16,
            dead_sit: 0,
        }
    }
}

impl From<EntityAppearPacket> for EntityData {
    fn from(packet: EntityAppearPacket) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            position: packet.position,
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
            head_palette: packet.head_palette,
            body_palette: packet.body_palette,
            guild_id: packet.guild_id,
            guild_name: String::new(),
            accessory: packet.accessory,
            accessory2: packet.accessory2,
            accessory3: packet.accessory3,
            weapon: packet.weapon,
            shield: packet.shield,
            robe: packet.robe,
            effect_state: packet.effect_state,
            body_state: packet.body_state,
            health_state: packet.health_state,
            dead_sit: 0,
        }
    }
}

impl From<EntityAppear2Packet> for EntityData {
    fn from(packet: EntityAppear2Packet) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            position: packet.position,
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
            head_palette: packet.head_palette,
            body_palette: packet.body_palette,
            guild_id: packet.guild_id,
            guild_name: String::new(),
            accessory: packet.accessory,
            accessory2: packet.accessory2,
            accessory3: packet.accessory3,
            weapon: packet.weapon,
            shield: packet.shield,
            robe: packet.robe,
            effect_state: packet.effect_state,
            body_state: packet.body_state,
            health_state: packet.health_state,
            // The idle-unit packet is the only appear packet that carries the
            // dead/sit state (0 = standing, 1 = dead, 2 = sitting).
            dead_sit: packet.state,
        }
    }
}

impl From<MovingEntityAppearPacket> for EntityData {
    fn from(packet: MovingEntityAppearPacket) -> Self {
        let (origin, destination) = packet.position.to_origin_destination();

        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            position: origin,
            destination: Some(destination),
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
            head_palette: packet.head_palette,
            body_palette: packet.body_palette,
            guild_id: packet.guild_id,
            guild_name: String::new(),
            accessory: packet.accessory,
            accessory2: packet.accessory2,
            accessory3: packet.accessory3,
            weapon: packet.weapon,
            shield: packet.shield,
            robe: packet.robe,
            effect_state: packet.effect_state,
            body_state: packet.body_state,
            health_state: packet.health_state,
            dead_sit: 0,
        }
    }
}
