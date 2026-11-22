//! Party state for the 2019 rAthena party system.
//!
//! The 2019 server sends the party roster as `ZC_GROUP_LIST` (0x0AE5) with one
//! entry per member and keeps it up to date with the lighter
//! `ZC_ADD_MEMBER_TO_GROUP` (0x0AE4), HP (0x080E), position (0x0107), job/level
//! (0x0ABD) and death (0x0AB2) packets. This state merges all of them so the
//! party window can show the classic 12 slot roster with HP bars, and
//! `korangar/src/lib.rs` can show party invitations, results and chat messages.

use korangar_interface::element::StateElement;
use korangar_networking::PartyMemberDetails;
use ragnarok_packets::{AccountId, CharacterId, JobId, PartyId, PartyListMember, TilePosition};
use rust_state::RustState;

/// The 2019 server allows at most 12 party members.
pub const MAXIMUM_PARTY_MEMBERS: usize = 12;

/// A single member of the party.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct PartyMember {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub name: String,
    pub job_id: JobId,
    pub base_level: u16,
    pub map_name: String,
    /// Latest known position on the current map, if the server sent one.
    pub position: Option<TilePosition>,
    pub health_points: Option<i32>,
    pub maximum_health_points: Option<i32>,
    pub online: bool,
    pub leader: bool,
    pub dead: bool,
}

impl Default for PartyMember {
    fn default() -> Self {
        Self {
            account_id: AccountId(0),
            character_id: CharacterId(0),
            name: String::new(),
            job_id: JobId(0),
            base_level: 0,
            map_name: String::new(),
            position: None,
            health_points: None,
            maximum_health_points: None,
            online: false,
            leader: false,
            dead: false,
        }
    }
}

impl PartyMember {
    /// One line of the party roster, e.g. `(L) MrChamp Lv 99`.
    pub fn display_name(&self) -> String {
        let mut line = String::new();

        if self.leader {
            line.push_str("(L) ");
        }

        line.push_str(&self.name);

        if self.base_level > 0 {
            line.push_str(&format!(" Lv {}", self.base_level));
        }

        if !self.online {
            line.push_str(" [offline]");
        }

        if self.dead {
            line.push_str(" [dead]");
        }

        line
    }

    /// Health fraction for the party HP bar, [`None`] while the HP is unknown.
    pub fn health_fraction(&self) -> Option<f32> {
        match (self.health_points, self.maximum_health_points) {
            (Some(health_points), Some(maximum_health_points)) if maximum_health_points > 0 => {
                Some((health_points.max(0) as f32 / maximum_health_points as f32).clamp(0.0, 1.0))
            }
            _ => None,
        }
    }

    fn from_roster_entry(entry: &PartyListMember) -> Self {
        Self {
            account_id: entry.account_id,
            character_id: entry.character_id,
            name: entry.name.clone(),
            job_id: entry.job_id,
            base_level: entry.base_level,
            map_name: entry.map_name.clone(),
            position: None,
            health_points: None,
            maximum_health_points: None,
            online: entry.offline == 0,
            leader: entry.role == 0,
            dead: false,
        }
    }

    fn from_details(details: &PartyMemberDetails) -> Self {
        Self {
            account_id: details.account_id,
            character_id: details.character_id,
            name: details.name.clone(),
            job_id: details.job_id,
            base_level: details.base_level,
            map_name: details.map_name.clone(),
            position: Some(details.position),
            health_points: None,
            maximum_health_points: None,
            online: details.online,
            leader: details.leader,
            dead: false,
        }
    }
}

/// An open party invitation that is waiting for the player to accept or reject
/// it.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct PartyInvitePrompt {
    pub party_id: PartyId,
    pub party_name: String,
}

/// State of the party the local player is in.
#[derive(Default, RustState, StateElement)]
pub struct Party {
    /// Party id of the party the local player is in, [`None`] when not in a
    /// party.
    pub party_id: Option<PartyId>,
    /// Name of the party.
    pub name: String,
    /// Current members, sorted by the order the server sent them.
    pub members: Vec<PartyMember>,
    /// EXP sharing rule (`ZC_PARTY_OPTION`): 0 = disabled, 1 = enabled,
    /// 2 = cannot be changed.
    pub experience_option: u32,
    /// "Even share" item pickup rule.
    pub item_pickup: bool,
    /// Item sharing rule.
    pub item_share: bool,
    /// Whether the player accepts party invitations (`ZC_PARTY_CONFIG`).
    pub allow_invites: bool,
    /// Text input of the "create party" field.
    pub create_name_input: String,
    /// Text input of the "invite to party" field.
    pub invite_name_input: String,
    /// Pending invitation, shown in the invitation window.
    pub invite: Option<PartyInvitePrompt>,
}

impl Party {
    /// Whether the local player is currently in a party.
    pub fn is_in_party(&self) -> bool {
        self.party_id.is_some() || !self.members.is_empty()
    }

    /// One roster line per member without padding (used by tests and the chat).
    #[cfg(test)]
    pub fn roster_lines(&self) -> Vec<String> {
        self.members.iter().map(PartyMember::display_name).collect()
    }

    /// Replaces the roster with the one from `ZC_GROUP_LIST`.
    pub fn apply_roster(&mut self, party_name: String, members: &[PartyListMember]) {
        let previous = std::mem::take(&mut self.members);

        self.name = party_name;
        self.members = members
            .iter()
            .map(|entry| {
                // Keep the HP/death information that arrived through the lighter
                // packets for members that are already known.
                let mut member = PartyMember::from_roster_entry(entry);

                if let Some(known) = previous.iter().find(|known| known.account_id == entry.account_id) {
                    member.health_points = known.health_points;
                    member.maximum_health_points = known.maximum_health_points;
                    member.dead = known.dead;
                    member.position = known.position;
                }

                member
            })
            .collect();
    }

    /// Applies `ZC_ADD_MEMBER_TO_GROUP`, adding or updating a member.
    pub fn apply_member_details(&mut self, details: &PartyMemberDetails) {
        let mut member = PartyMember::from_details(details);

        if let Some(known) = self.members.iter().find(|known| known.account_id == details.account_id) {
            member.health_points = known.health_points;
            member.maximum_health_points = known.maximum_health_points;
            member.dead = known.dead;
        }

        match self.find_member_index(details.account_id) {
            Some(index) => self.members[index] = member,
            None => {
                if self.members.len() < MAXIMUM_PARTY_MEMBERS {
                    self.members.push(member);
                }
            }
        }

        if self.name.is_empty() {
            self.name = details.party_name.clone();
        }
    }

    /// Removes a member that left or was expelled.
    pub fn apply_member_removed(&mut self, account_id: AccountId) {
        self.members.retain(|member| member.account_id != account_id);
    }

    /// Clears all party state, e.g. after the player left the party.
    pub fn clear(&mut self) {
        self.party_id = None;
        self.name.clear();
        self.members.clear();
        self.experience_option = 0;
        self.item_pickup = false;
        self.item_share = false;
        self.invite = None;
    }

    pub fn find_member(&self, account_id: AccountId) -> Option<&PartyMember> {
        self.members.iter().find(|member| member.account_id == account_id)
    }

    pub fn find_member_mut(&mut self, account_id: AccountId) -> Option<&mut PartyMember> {
        self.members.iter_mut().find(|member| member.account_id == account_id)
    }

    fn find_member_index(&self, account_id: AccountId) -> Option<usize> {
        self.members.iter().position(|member| member.account_id == account_id)
    }

    /// Display name for a party member account id, used for party chat.
    pub fn member_name(&self, account_id: AccountId) -> Option<&str> {
        self.find_member(account_id).map(|member| member.name.as_str())
    }

    pub fn apply_health(&mut self, account_id: AccountId, health_points: i32, maximum_health_points: i32) {
        if let Some(member) = self.find_member_mut(account_id) {
            member.health_points = Some(health_points);
            member.maximum_health_points = Some(maximum_health_points);
        }
    }

    pub fn apply_position(&mut self, account_id: AccountId, position: TilePosition) {
        if let Some(member) = self.find_member_mut(account_id) {
            member.position = Some(position);
        }
    }

    pub fn apply_job_level(&mut self, account_id: AccountId, job_id: JobId, base_level: u16) {
        if let Some(member) = self.find_member_mut(account_id) {
            member.job_id = job_id;
            member.base_level = base_level;
        }
    }

    pub fn apply_alive(&mut self, account_id: AccountId, is_dead: bool) {
        if let Some(member) = self.find_member_mut(account_id) {
            member.dead = is_dead;
        }
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{AccountId, CharacterId, JobId};

    use super::*;

    fn roster_entry(account_id: u32, name: &str, role: u8, offline: u8) -> PartyListMember {
        PartyListMember {
            account_id: AccountId(account_id),
            character_id: CharacterId(account_id),
            name: name.to_owned(),
            map_name: "prontera".to_owned(),
            role,
            offline,
            job_id: JobId(7),
            base_level: 99,
        }
    }

    #[test]
    fn roster_is_applied_and_keeps_known_health() {
        let mut party = Party::default();
        party.party_id = Some(PartyId(13));
        party.apply_roster("feeders".to_owned(), &[roster_entry(150_009, "MrChamp", 0, 0)]);

        assert_eq!(party.name, "feeders");
        assert_eq!(party.members.len(), 1);
        assert_eq!(party.members[0].name, "MrChamp");
        assert!(party.members[0].leader);
        assert!(party.members[0].online);

        party.apply_health(AccountId(150_009), 7500, 10_000);
        assert_eq!(party.members[0].health_fraction(), Some(0.75));

        // A roster refresh must keep the HP that arrived through 0x080E.
        party.apply_roster("feeders".to_owned(), &[roster_entry(150_009, "MrChamp", 0, 0)]);
        assert_eq!(party.members[0].health_points, Some(7500));
    }

    #[test]
    fn member_details_update_an_existing_member_without_dropping_health() {
        let mut party = Party::default();
        party.party_id = Some(PartyId(13));
        party.apply_roster("feeders".to_owned(), &[roster_entry(150_009, "MrChamp", 0, 0)]);
        party.apply_health(AccountId(150_009), 5000, 10_000);

        party.apply_member_details(&PartyMemberDetails {
            account_id: AccountId(150_009),
            character_id: CharacterId(150_009),
            name: "MrChamp".to_owned(),
            party_name: "feeders".to_owned(),
            map_name: "geffen".to_owned(),
            job_id: JobId(7),
            base_level: 99,
            position: TilePosition { x: 120, y: 80 },
            online: false,
            leader: true,
        });

        assert_eq!(party.members.len(), 1);
        assert_eq!(party.members[0].map_name, "geffen");
        assert_eq!(party.members[0].position, Some(TilePosition { x: 120, y: 80 }));
        assert!(!party.members[0].online);
        assert_eq!(party.members[0].health_points, Some(5000));
    }

    #[test]
    fn removing_the_last_member_empties_the_party() {
        let mut party = Party::default();
        party.party_id = Some(PartyId(13));
        party.apply_roster("feeders".to_owned(), &[roster_entry(150_009, "MrChamp", 0, 0)]);

        party.apply_member_removed(AccountId(150_009));

        assert!(party.members.is_empty());
        assert!(party.is_in_party(), "the party id is only cleared explicitly on leave");
        party.clear();
        assert!(!party.is_in_party());
    }

    #[test]
    fn roster_line_shows_leader_offline_and_level() {
        let mut party = Party::default();
        party.apply_roster("feeders".to_owned(), &[roster_entry(150_009, "MrChamp", 0, 1)]);

        assert_eq!(party.roster_lines(), vec!["(L) MrChamp Lv 99 [offline]".to_owned()]);
    }
}
