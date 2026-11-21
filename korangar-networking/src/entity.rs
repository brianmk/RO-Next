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
        }
    }
}
