//! Clan state for the 2019 rAthena server.
//!
//! At PACKETVER 20190605 the clan feature only sends two packets:
//!
//! * `ZC_CLANINFO` (0x098A) — clan id/name, master name, territory map and the
//!   ally/antagonist name lists (clif.cpp:20618), and
//! * `ZC_NOTIFY_CLAN_CONNECTINFO` (0x0988) — online and maximum member count
//!   (clif.cpp:20657).
//!
//! Both are sent on map login when the character is in a clan
//! (`clan.cpp:113`, `clan.cpp:133`). **There is no clan member list packet** in
//! the 2019 set: the client cannot show who is in the clan, only how many
//! members are online. The window states this explicitly instead of pretending
//! the list is merely still loading.

use ragnarok_packets::ClanInfoPacket;
use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// One ally or antagonist clan of `ZC_CLANINFO`.
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct ClanRelation {
    pub name: String,
    /// `false` = ally, `true` = antagonist (`clan_alliance.opposition`).
    pub antagonist: bool,
}

/// Basic clan information from `ZC_CLANINFO` (0x098A).
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct ClanInformation {
    pub clan_id: u32,
    pub name: String,
    pub master_name: String,
    /// Territory map name (`mapindex_getmapname_ext`).
    pub map_name: String,
    pub allies: Vec<ClanRelation>,
    pub antagonists: Vec<ClanRelation>,
}

/// State of the clan the local player is in.
#[derive(Default, RustState, StateElement)]
pub struct Clan {
    pub information: Option<ClanInformation>,
    pub online_members: u16,
    pub maximum_members: u16,
    /// Whether `ZC_CLANINFO` was received for this map session.
    pub has_info: bool,
}

impl Clan {
    /// Applies `ZC_CLANINFO`.
    pub fn apply_info(&mut self, packet: ClanInfoPacket) {
        let mut allies = Vec::with_capacity(packet.aliance_count as usize);
        let mut antagonists = Vec::with_capacity(packet.antagonist_count as usize);

        // `Aliance`/`Antagonist` are two separate packet structs in the client
        // model; rAthena sends the ally names first and the antagonist names
        // second (clif.cpp:20636).
        for alliance in packet.aliances.iter() {
            allies.push(ClanRelation {
                name: alliance.name.trim_end_matches('\0').to_owned(),
                antagonist: false,
            });
        }

        for antagonist in packet.antagonists.iter() {
            antagonists.push(ClanRelation {
                name: antagonist.name.trim_end_matches('\0').to_owned(),
                antagonist: true,
            });
        }

        self.has_info = true;
        self.information = Some(ClanInformation {
            clan_id: packet.clan_id,
            name: packet.clan_name.trim_end_matches('\0').to_owned(),
            master_name: packet.clan_master.trim_end_matches('\0').to_owned(),
            map_name: packet.clan_map.trim_end_matches('\0').to_owned(),
            allies,
            antagonists,
        });
    }

    /// Applies `ZC_NOTIFY_CLAN_CONNECTINFO`.
    pub fn apply_online_count(&mut self, online_members: u16, maximum_members: u16) {
        self.online_members = online_members;
        self.maximum_members = maximum_members;
    }

    /// Header line of the clan window.
    pub fn header_text(&self) -> String {
        let Some(information) = &self.information else {
            return "No clan information loaded. You are not in a clan, or the server has not answered yet.".to_owned();
        };

        let name = match information.name.trim() {
            "" => format!("Clan #{}", information.clan_id),
            name => name.to_owned(),
        };

        format!("{name}   Online {} / {}", self.online_members, self.maximum_members)
    }

    /// Master/territory line of the clan window.
    pub fn master_text(&self) -> String {
        let Some(information) = &self.information else {
            return String::new();
        };

        let mut text = format!("Clan master: {}", information.master_name);

        if !information.map_name.trim().is_empty() {
            text.push_str(&format!("   Territory: {}", information.map_name));
        }

        text
    }

    /// Ally/antagonist rows of the clan window.
    pub fn relation_text(&self) -> String {
        let Some(information) = &self.information else {
            return String::new();
        };

        let allies = match information.allies.is_empty() {
            true => "none".to_owned(),
            false => information
                .allies
                .iter()
                .map(|relation| relation.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        };
        let antagonists = match information.antagonists.is_empty() {
            true => "none".to_owned(),
            false => information
                .antagonists
                .iter()
                .map(|relation| relation.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        };

        format!("Allies: {allies}\nAntagonists: {antagonists}")
    }

    /// Member list text. The 2019 packet set has no member list, so this is an
    /// explicit statement instead of an empty list.
    pub fn member_text(&self) -> String {
        if !self.has_info {
            return String::new();
        }

        "Clan members: the 2019 server does not send a clan member list packet. Only the online/maximum member count is available. \
         (rAthena 20190605 has ZC_CLANINFO 0x098A and ZC_NOTIFY_CLAN_CONNECTINFO 0x0988, but no member roster.)"
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{Aliance, Antagonist};

    use super::*;

    fn info() -> ClanInfoPacket {
        ClanInfoPacket {
            clan_id: 7,
            clan_name: "Wolves".to_owned(),
            clan_master: "Leader".to_owned(),
            clan_map: "prontera".to_owned(),
            aliance_count: 1,
            antagonist_count: 1,
            aliances: vec![Aliance { name: "Bears".to_owned() }],
            antagonists: vec![Antagonist { name: "Rats".to_owned() }],
        }
    }

    #[test]
    fn info_and_online_count_are_applied() {
        let mut clan = Clan::default();
        clan.apply_info(info());
        clan.apply_online_count(3, 20);

        assert!(clan.header_text().contains("Wolves"));
        assert!(clan.header_text().contains("Online 3 / 20"));
        assert!(clan.master_text().contains("Leader"));
        assert!(clan.relation_text().contains("Bears"));
        assert!(clan.relation_text().contains("Rats"));
        assert!(clan.member_text().contains("does not send a clan member list"));
    }

    #[test]
    fn empty_clan_relations_are_rendered_as_none() {
        let mut clan = Clan::default();
        let mut packet = info();
        packet.aliance_count = 0;
        packet.antagonist_count = 0;
        packet.aliances.clear();
        packet.antagonists.clear();
        clan.apply_info(packet);

        assert!(clan.relation_text().contains("Allies: none"));
        assert!(clan.relation_text().contains("Antagonists: none"));
    }
}
