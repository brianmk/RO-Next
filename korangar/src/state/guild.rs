//! Guild state for the 2019 rAthena guild system.
//!
//! The 2019 server splits the guild window into several packets:
//!
//! * `ZC_GUILD_INFO` (0x0A84) for the basic information,
//! * `ZC_MEMBERMGR_INFO` (0x0AA5) for the member list,
//! * `ZC_POSITION_ID_NAME_INFO` (0x0166) for the position names,
//! * `ZC_POSITION_INFO` (0x0160) for the position rights,
//! * `ZC_GUILD_NOTICE` (0x016F) for the notice,
//! * `ZC_MYGUILD_BASIC_INFO` (0x014C) for the alliance list.
//!
//! At PACKETVER 20190605 the member list has **no name field** (rAthena only
//! added it for `PACKETVER >= 20200902`). The client requests every member name
//! by character id with `CZ_REQNAME_BYGID` (0x094C) and merges the
//! `ZC_ACK_REQNAME_BYGID` (0x0AF7) answers here. Until a name arrives the row
//! shows an explicit `Unknown (#<char id>)` placeholder, never a blank name.

use korangar_interface::element::StateElement;
use ragnarok_packets::{
    AccountId, CharacterId, GuildAllianceInfo, GuildInfoPacket, GuildMemberInfo, GuildMemberPositionChangedEntry,
    GuildPositionChangedEntry, GuildPositionInfoEntry, GuildPositionNameEntry, JobId,
};
use rust_state::RustState;

/// Position id of the guild master (`guild.member[0].position`).
pub const GUILD_MASTER_POSITION: i32 = 0;

/// `GUILD_PERM_INVITE` (`src/common/mmo.hpp:887`).
pub const GUILD_PERMISSION_INVITE: u32 = 0x001;
/// `GUILD_PERM_EXPEL` (`src/common/mmo.hpp:888`).
pub const GUILD_PERMISSION_EXPEL: u32 = 0x010;
/// `GUILD_PERM_STORAGE` (`src/common/mmo.hpp:889`).
pub const GUILD_PERMISSION_STORAGE: u32 = 0x100;

/// `GMENUFLAG_MASTER` in the `ZC_ACK_GUILD_MENUINTERFACE` mask.
///
/// rAthena sends `0xD7` to the guild master and `0x57` to a normal member
/// (`clif_guild_masterormember`, clif.cpp:8715), so the `0x80` bit is the
/// master/notice-edit flag.
pub const GUILD_MENU_FLAG_MASTER: u32 = 0x80;

/// One guild member merged from `ZC_MEMBERMGR_INFO` and its name response.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct GuildMember {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    /// Resolved character name; empty until `ZC_ACK_REQNAME_BYGID` arrived.
    pub name: String,
    pub job_id: JobId,
    pub level: u16,
    pub position_id: i32,
    pub online: bool,
    pub contribution_exp: u32,
    pub last_login: u32,
}

impl Default for GuildMember {
    fn default() -> Self {
        Self {
            account_id: AccountId(0),
            character_id: CharacterId(0),
            name: String::new(),
            job_id: JobId(0),
            level: 0,
            position_id: 0,
            online: false,
            contribution_exp: 0,
            last_login: 0,
        }
    }
}

impl GuildMember {
    /// Display name of the member, an explicit placeholder while unknown.
    pub fn display_name(&self) -> String {
        if self.name.trim().is_empty() {
            format!("Unknown (#{})", self.character_id.0)
        } else {
            self.name.clone()
        }
    }
}

/// One guild position (grade) merged from 0x0166 and 0x0160.
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct GuildPosition {
    pub position_id: u32,
    pub name: String,
    /// `enum e_guild_permission` bits.
    pub mode: u32,
    pub ranking: u32,
    pub pay_rate: u32,
}

/// One guild ally/antagonist row of `ZC_MYGUILD_BASIC_INFO`.
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct GuildAlliance {
    pub guild_id: u32,
    pub name: String,
    /// `false` = ally, `true` = antagonist (`guild_alliance.opposition`).
    pub antagonist: bool,
}

/// State of the guild the local player is in.
#[derive(RustState, StateElement)]
pub struct Guild {
    /// Guild id from `ZC_GUILD_INFO`, `0` when unknown.
    pub guild_id: u32,
    pub name: String,
    pub level: u32,
    /// Currently connected members (`guild.connect_member`).
    pub online_members: u32,
    pub max_members: u32,
    pub average_level: u32,
    pub experience: u32,
    pub max_experience: u32,
    pub points: u32,
    pub honor: u32,
    pub virtue: u32,
    pub emblem_id: u32,
    pub manage_land: String,
    pub zeny: u32,
    pub master_character_id: CharacterId,
    /// Resolved guild master name (empty until the member list/name arrives).
    pub master_name: String,
    /// `ZC_ACK_GUILD_MENUINTERFACE` mask.
    pub menu_flags: u32,
    pub notice_subject: String,
    pub notice: String,
    pub members: Vec<GuildMember>,
    pub positions: Vec<GuildPosition>,
    pub alliances: Vec<GuildAlliance>,
    /// Whether `ZC_GUILD_INFO` was received for this map session.
    pub has_info: bool,
    /// Local account id, set when the character is selected.
    pub local_account_id: Option<AccountId>,
    /// Local character id, set when the character is selected.
    pub local_character_id: Option<CharacterId>,
    /// Character id -> name requests that were already sent this session.
    pub requested_names: Vec<CharacterId>,
    /// Text input of the "invite character" field.
    pub invite_name_input: String,
    /// Text input of the "expel character" field.
    pub expel_name_input: String,
    /// Text input of the notice subject field.
    pub notice_subject_input: String,
    /// Text input of the notice body field.
    pub notice_input: String,
    /// Short feedback line of the last action, shown in the window.
    pub status_message: String,
}

impl Default for Guild {
    fn default() -> Self {
        Self {
            guild_id: 0,
            name: String::new(),
            level: 0,
            online_members: 0,
            max_members: 0,
            average_level: 0,
            experience: 0,
            max_experience: 0,
            points: 0,
            honor: 0,
            virtue: 0,
            emblem_id: 0,
            manage_land: String::new(),
            zeny: 0,
            master_character_id: CharacterId(0),
            master_name: String::new(),
            menu_flags: 0,
            notice_subject: String::new(),
            notice: String::new(),
            members: Vec::new(),
            positions: Vec::new(),
            alliances: Vec::new(),
            has_info: false,
            local_account_id: None,
            local_character_id: None,
            requested_names: Vec::new(),
            invite_name_input: String::new(),
            expel_name_input: String::new(),
            notice_subject_input: String::new(),
            notice_input: String::new(),
            status_message: String::new(),
        }
    }
}

impl Guild {
    /// Resets all guild data, including the local ids and text inputs.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Resets the guild data of the previous map session but remembers the
    /// local account/character id so the next map can act on its own guild.
    pub fn set_local_player(&mut self, account_id: AccountId, character_id: CharacterId) {
        self.reset();
        self.local_account_id = Some(account_id);
        self.local_character_id = Some(character_id);
    }

    pub fn is_in_guild(&self) -> bool {
        self.has_info && self.guild_id != 0
    }

    /// Whether the local player is the guild master.
    ///
    /// The menu mask is authoritative even before the member list arrives; once
    /// the member list is known, the master character id is checked as well.
    pub fn is_master(&self) -> bool {
        if self.menu_flags & GUILD_MENU_FLAG_MASTER != 0 {
            return true;
        }

        match (self.local_character_id, self.master_character_id) {
            (Some(character_id), CharacterId(master_id)) => master_id != 0 && character_id == CharacterId(master_id),
            _ => false,
        }
    }

    /// The local member, if the member list contains it.
    pub fn local_member(&self) -> Option<&GuildMember> {
        self.members
            .iter()
            .find(|member| Some(member.account_id) == self.local_account_id)
            .or_else(|| {
                self.members
                    .iter()
                    .find(|member| Some(member.character_id) == self.local_character_id)
            })
    }

    /// Permission bits of the local player's position, if known.
    pub fn local_position_mode(&self) -> Option<u32> {
        let position_id = self.local_member()?.position_id;
        self.positions
            .iter()
            .find(|position| position.position_id == position_id as u32)
            .map(|position| position.mode)
    }

    /// Whether the local player may invite (`GUILD_PERM_INVITE`).
    pub fn can_invite(&self) -> bool {
        self.is_in_guild() && (self.is_master() || self.local_position_mode().is_some_and(|mode| mode & GUILD_PERMISSION_INVITE != 0))
    }

    /// Whether the local player may expel (`GUILD_PERM_EXPEL`).
    pub fn can_expel(&self) -> bool {
        self.is_in_guild() && (self.is_master() || self.local_position_mode().is_some_and(|mode| mode & GUILD_PERMISSION_EXPEL != 0))
    }

    /// Only the guild master may edit the notice (`0x80` menu flag).
    pub fn can_edit_notice(&self) -> bool {
        self.is_in_guild() && self.is_master()
    }

    /// Name of the guild master, resolved from the member list when needed.
    pub fn master_display_name(&self) -> String {
        if !self.master_name.trim().is_empty() {
            return self.master_name.clone();
        }

        if self.master_character_id.0 != 0
            && let Some(member) = self.members.iter().find(|member| member.character_id == self.master_character_id)
        {
            return member.display_name();
        }

        if let Some(member) = self.members.iter().find(|member| member.position_id == GUILD_MASTER_POSITION) {
            return member.display_name();
        }

        match self.master_character_id.0 {
            0 => "Unknown".to_owned(),
            character_id => format!("Unknown (#{character_id})"),
        }
    }

    /// Name of a position id, `Position #<id>` while the names are unknown.
    pub fn position_name(&self, position_id: i32) -> String {
        self.positions
            .iter()
            .find(|position| position.position_id == position_id as u32)
            .map(|position| position.name.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Position #{position_id}"))
    }

    /// Replaces the basic information from `ZC_GUILD_INFO`.
    pub fn apply_info(&mut self, information: &GuildInfoPacket) {
        self.guild_id = information.guild_id;
        self.name = information.guild_name.trim().to_owned();
        self.level = information.level;
        self.online_members = information.member_count;
        self.max_members = information.max_member_count;
        self.average_level = information.average_level;
        self.experience = information.experience;
        self.max_experience = information.max_experience;
        self.points = information.points;
        self.honor = information.honor;
        self.virtue = information.virtue;
        self.emblem_id = information.emblem_id;
        self.manage_land = information.manage_land.trim().to_owned();
        self.zeny = information.zeny;
        self.master_character_id = information.master_character_id;
        self.has_info = true;
        self.refresh_master_name();
    }

    /// Replaces the member list from `ZC_MEMBERMGR_INFO`, keeping the names
    /// that already arrived through `ZC_ACK_REQNAME_BYGID`.
    pub fn apply_members(&mut self, members: &[GuildMemberInfo]) {
        let previous = std::mem::take(&mut self.members);

        self.members = members
            .iter()
            .map(|information| {
                let name = previous
                    .iter()
                    .find(|known| known.character_id == information.character_id)
                    .map(|known| known.name.clone())
                    .unwrap_or_default();

                GuildMember {
                    account_id: information.account_id,
                    character_id: information.character_id,
                    name,
                    job_id: information.job_id,
                    level: information.level,
                    position_id: information.position_id,
                    online: information.online != 0,
                    contribution_exp: information.contribution_exp,
                    last_login: information.last_login,
                }
            })
            .collect();

        self.refresh_master_name();
    }

    /// Applies one `ZC_ACK_REQNAME_BYGID` answer.
    pub fn apply_member_name(&mut self, character_id: CharacterId, name: String) {
        let name = name.trim().to_owned();

        if name.is_empty() {
            return;
        }

        if let Some(member) = self.members.iter_mut().find(|member| member.character_id == character_id) {
            member.name = name.clone();
        }

        if character_id == self.master_character_id {
            self.master_name = name;
        }
    }

    /// Applies `ZC_POSITION_ID_NAME_INFO`.
    pub fn apply_position_names(&mut self, names: &[GuildPositionNameEntry]) {
        let previous = std::mem::take(&mut self.positions);

        self.positions = names
            .iter()
            .map(|entry| {
                let known = previous.iter().find(|position| position.position_id == entry.position_id);

                GuildPosition {
                    position_id: entry.position_id,
                    name: entry.name.trim().to_owned(),
                    mode: known.map(|position| position.mode).unwrap_or_default(),
                    ranking: known.map(|position| position.ranking).unwrap_or_default(),
                    pay_rate: known.map(|position| position.pay_rate).unwrap_or_default(),
                }
            })
            .collect();
    }

    /// Applies `ZC_POSITION_INFO` (rights/pay rates).
    pub fn apply_position_info(&mut self, information: &[GuildPositionInfoEntry]) {
        for entry in information {
            match self.positions.iter_mut().find(|position| position.position_id == entry.position_id) {
                Some(position) => {
                    position.mode = entry.mode;
                    position.ranking = entry.ranking;
                    position.pay_rate = entry.pay_rate;
                }
                None => self.positions.push(GuildPosition {
                    position_id: entry.position_id,
                    name: String::new(),
                    mode: entry.mode,
                    ranking: entry.ranking,
                    pay_rate: entry.pay_rate,
                }),
            }
        }
    }

    /// Applies one or more `ZC_ACK_CHANGE_GUILD_POSITIONINFO` rows.
    pub fn apply_position_changes(&mut self, changes: &[GuildPositionChangedEntry]) {
        for change in changes {
            let name = change.name.trim().to_owned();

            match self
                .positions
                .iter_mut()
                .find(|position| position.position_id == change.position_id)
            {
                Some(position) => {
                    position.mode = change.mode;
                    position.ranking = change.ranking;
                    position.pay_rate = change.pay_rate;
                    if !name.is_empty() {
                        position.name = name;
                    }
                }
                None => self.positions.push(GuildPosition {
                    position_id: change.position_id,
                    name,
                    mode: change.mode,
                    ranking: change.ranking,
                    pay_rate: change.pay_rate,
                }),
            }
        }
    }

    /// Applies `ZC_ACK_REQ_CHANGE_MEMBERS` rows.
    pub fn apply_member_position_changes(&mut self, changes: &[GuildMemberPositionChangedEntry]) {
        for change in changes {
            if let Some(member) = self
                .members
                .iter_mut()
                .find(|member| member.account_id == change.account_id || member.character_id == change.character_id)
            {
                member.position_id = change.position_id;
            }
        }
    }

    /// Applies `ZC_GUILD_NOTICE` and fills the edit fields.
    pub fn apply_notice(&mut self, subject: String, notice: String) {
        self.notice_subject = subject;
        self.notice = notice;
        self.notice_subject_input = self.notice_subject.clone();
        self.notice_input = self.notice.clone();
    }

    /// Whether `guild_id` is the local guild or one of its **allied** guilds.
    ///
    /// This mirrors `GuildInfo::HasRelatedGuild`
    /// (`rodecomp/Source/Contents/Guild/GuildInfo.cpp`): only alliances with
    /// `relation == 0` (allies, not antagonists) count. The own guild is
    /// included so callers can use the same predicate for the guild id of a
    /// target entity; the local player itself is excluded separately.
    pub fn is_related_guild(&self, guild_id: u32) -> bool {
        guild_id != 0
            && (guild_id == self.guild_id
                || self
                    .alliances
                    .iter()
                    .any(|alliance| !alliance.antagonist && alliance.guild_id == guild_id))
    }

    /// Applies `ZC_MYGUILD_BASIC_INFO`.
    pub fn apply_alliances(&mut self, alliances: &[GuildAllianceInfo]) {
        self.alliances = alliances
            .iter()
            .map(|alliance| GuildAlliance {
                guild_id: alliance.guild_id,
                name: alliance.name.trim().to_owned(),
                antagonist: alliance.relation != 0,
            })
            .collect();
    }

    fn refresh_master_name(&mut self) {
        if self.master_character_id.0 != 0
            && let Some(member) = self.members.iter().find(|member| member.character_id == self.master_character_id)
            && !member.name.trim().is_empty()
        {
            self.master_name = member.name.clone();
        }
    }

    /// One row per member shown by the guild window.
    pub fn member_lines(&self) -> Vec<String> {
        self.members
            .iter()
            .map(|member| {
                let marker = if member.online { "●" } else { "○" };
                let position = self.position_name(member.position_id);

                format!(
                    "{marker} {}  {}  Lv {}  {}  {} contribution",
                    member.display_name(),
                    job_name(member.job_id),
                    member.level,
                    position,
                    member.contribution_exp
                )
            })
            .collect()
    }

    /// Multi-line text of the member list for the window.
    pub fn member_list_text(&self) -> String {
        let mut text = format!(
            "Members: {} online / {} / {} max",
            self.members.iter().filter(|member| member.online).count(),
            self.members.len(),
            self.max_members
        );

        if self.members.is_empty() {
            text.push_str("\nNo members loaded.");
            return text;
        }

        for line in self.member_lines() {
            text.push('\n');
            text.push_str(&line);
        }

        text
    }

    /// Multi-line text of the positions (grades) for the window.
    pub fn position_list_text(&self) -> String {
        if self.positions.is_empty() {
            return "No position information loaded.".to_owned();
        }

        let mut text = String::from("Positions:");
        for position in &self.positions {
            let name = match position.name.trim() {
                "" => format!("Position #{}", position.position_id),
                name => name.to_owned(),
            };
            let rights = permission_text(position.mode);

            text.push_str(&format!("\n{}: {}  [{}]", position.position_id, name, rights));
        }
        text
    }

    /// Multi-line text of the alliance/opposition list.
    pub fn alliance_list_text(&self) -> String {
        if self.alliances.is_empty() {
            return String::new();
        }

        self.alliances
            .iter()
            .map(|alliance| {
                let relation = if alliance.antagonist { "Antagonist" } else { "Ally" };
                format!("{relation}: {}", alliance.name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Member whose name matches the expel input, if any.
    pub fn member_by_name(&self, name: &str) -> Option<&GuildMember> {
        let name = name.trim();

        if name.is_empty() {
            return None;
        }

        self.members
            .iter()
            .find(|member| member.name == name || member.display_name() == name)
    }

    /// Removes a member that left or was expelled.
    ///
    /// Returns `true` when the member was part of the list (so the online
    /// counter could be adjusted).
    pub fn remove_member(&mut self, character_id: CharacterId) -> bool {
        let index = self.members.iter().position(|member| member.character_id == character_id);

        let Some(index) = index else {
            return false;
        };

        let was_online = self.members.remove(index).online;

        if was_online {
            self.online_members = self.online_members.saturating_sub(1);
        }

        true
    }

    /// Clears the guild after the local player left or was expelled but keeps
    /// the local account/character id for the next guild.
    pub fn leave_local_guild(&mut self) {
        let account_id = self.local_account_id;
        let character_id = self.local_character_id;

        self.reset();

        self.local_account_id = account_id;
        self.local_character_id = character_id;
    }
}

/// Name of a player job id (classic rAthena `e_job` ids 0..=25).
///
/// The 2019 member list only carries the numeric job id; this table lets the
/// member window show the classic class names. Unknown/custom jobs fall back to
/// an explicit `Job #<id>` so the row never shows a wrong class.
pub fn job_name(job_id: JobId) -> String {
    match job_id.0 {
        0 => "Novice",
        1 => "Swordman",
        2 => "Mage",
        3 => "Archer",
        4 => "Acolyte",
        5 => "Merchant",
        6 => "Thief",
        7 => "Knight",
        8 => "Priest",
        9 => "Wizard",
        10 => "Blacksmith",
        11 => "Hunter",
        12 => "Assassin",
        13 => "Knight",
        14 => "Crusader",
        15 => "Monk",
        16 => "Sage",
        17 => "Rogue",
        18 => "Alchemist",
        19 => "Bard",
        20 => "Dancer",
        21 => "Crusader",
        22 => "Wedding",
        23 => "Super Novice",
        24 => "Gunslinger",
        25 => "Ninja",
        job_id => return format!("Job #{job_id}"),
    }
    .to_owned()
}

/// Human readable permission bits of a guild position.
pub fn permission_text(mode: u32) -> String {
    let mut permissions = Vec::new();

    if mode & GUILD_PERMISSION_INVITE != 0 {
        permissions.push("invite");
    }
    if mode & GUILD_PERMISSION_EXPEL != 0 {
        permissions.push("expel");
    }
    if mode & GUILD_PERMISSION_STORAGE != 0 {
        permissions.push("storage");
    }

    if permissions.is_empty() {
        "no rights".to_owned()
    } else {
        permissions.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{
        CharacterId, GuildAllianceInfo, GuildInfoPacket, GuildMemberInfo, GuildMemberPositionChangedEntry, GuildPositionChangedEntry,
        GuildPositionInfoEntry, GuildPositionNameEntry, JobId,
    };

    use super::*;

    fn info() -> GuildInfoPacket {
        GuildInfoPacket {
            guild_id: 7,
            level: 5,
            member_count: 2,
            max_member_count: 20,
            average_level: 90,
            experience: 1_000,
            max_experience: 2_000,
            points: 10,
            honor: 1,
            virtue: 2,
            emblem_id: 42,
            guild_name: "Domination".to_owned(),
            manage_land: "Prontera".to_owned(),
            zeny: 0,
            master_character_id: CharacterId(150_009),
        }
    }

    fn member(account_id: u32, character_id: u32, position_id: i32, online: i32) -> GuildMemberInfo {
        GuildMemberInfo {
            account_id: AccountId(account_id),
            character_id: CharacterId(character_id),
            hair: 1,
            hair_color: 1,
            sex: 1,
            job_id: JobId(7),
            level: 99,
            contribution_exp: 500,
            online,
            position_id,
            last_login: 0,
        }
    }

    fn positions() -> Vec<GuildPositionNameEntry> {
        (0..5)
            .map(|index| GuildPositionNameEntry {
                position_id: index,
                name: format!("Grade {index}"),
            })
            .collect()
    }

    #[test]
    fn applies_info_members_names_and_positions() {
        let mut guild = Guild::default();
        guild.set_local_player(AccountId(150_009), CharacterId(150_009));
        guild.apply_info(&info());
        guild.apply_members(&[member(150_009, 150_009, 0, 1), member(150_010, 150_010, 1, 0)]);
        guild.apply_position_names(&positions());
        guild.apply_member_name(CharacterId(150_009), "Master".to_owned());
        guild.apply_member_name(CharacterId(150_010), "Alt".to_owned());

        assert!(guild.is_in_guild());
        assert_eq!(guild.name, "Domination");
        assert_eq!(guild.master_display_name(), "Master");
        assert_eq!(guild.members.len(), 2);
        assert_eq!(guild.members[0].name, "Master");
        assert_eq!(guild.members[1].name, "Alt");
        assert!(guild.members[0].online);
        assert!(!guild.members[1].online);
        assert_eq!(guild.position_name(1), "Grade 1");

        let lines = guild.member_lines();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Master"));
        assert!(lines[0].contains("Knight"));
        assert!(lines[0].contains("Grade 0"));
        assert!(lines[1].contains("○"));
    }

    #[test]
    fn unknown_member_names_use_an_explicit_placeholder() {
        let mut guild = Guild::default();
        guild.apply_info(&info());
        guild.apply_members(&[member(150_009, 150_009, 0, 1)]);

        assert_eq!(guild.members[0].display_name(), "Unknown (#150009)");
    }

    #[test]
    fn master_and_permissions_come_from_the_menu_mask_and_position() {
        let mut guild = Guild::default();
        guild.set_local_player(AccountId(150_010), CharacterId(150_010));
        guild.apply_info(&info());
        guild.apply_members(&[member(150_009, 150_009, 0, 1), member(150_010, 150_010, 1, 1)]);
        guild.apply_position_names(&positions());
        guild.apply_position_info(&[GuildPositionInfoEntry {
            position_id: 1,
            mode: GUILD_PERMISSION_INVITE,
            ranking: 1,
            pay_rate: 0,
        }]);

        // Normal member with the invite permission only.
        guild.menu_flags = 0x57;
        assert!(!guild.is_master());
        assert!(guild.can_invite());
        assert!(!guild.can_expel());
        assert!(!guild.can_edit_notice());

        // Guild master mask ("0xD7") unlocks everything.
        guild.menu_flags = 0xD7;
        assert!(guild.is_master());
        assert!(guild.can_expel());
        assert!(guild.can_edit_notice());

        // The master character id is also enough without the mask.
        guild.menu_flags = 0;
        guild.local_character_id = Some(CharacterId(150_009));
        assert!(guild.is_master());
    }

    #[test]
    fn position_and_member_updates_are_merged() {
        let mut guild = Guild::default();
        guild.set_local_player(AccountId(150_009), CharacterId(150_009));
        guild.apply_info(&info());
        guild.apply_members(&[member(150_009, 150_009, 0, 1)]);
        guild.apply_position_names(&positions());

        guild.apply_position_changes(&[GuildPositionChangedEntry {
            position_id: 2,
            mode: GUILD_PERMISSION_EXPEL,
            ranking: 2,
            pay_rate: 10,
            name: "Officer".to_owned(),
        }]);
        guild.apply_member_position_changes(&[GuildMemberPositionChangedEntry {
            account_id: AccountId(150_009),
            character_id: CharacterId(150_009),
            position_id: 2,
        }]);

        assert_eq!(guild.position_name(2), "Officer");
        assert_eq!(guild.members[0].position_id, 2);
        assert_eq!(guild.local_position_mode(), Some(GUILD_PERMISSION_EXPEL));
    }

    #[test]
    fn notice_and_alliances_are_applied() {
        let mut guild = Guild::default();
        guild.apply_info(&info());
        guild.apply_notice("Hello".to_owned(), "Be nice.".to_owned());
        guild.apply_alliances(&[
            GuildAllianceInfo {
                relation: 0,
                guild_id: 8,
                name: "Friends".to_owned(),
            },
            GuildAllianceInfo {
                relation: 1,
                guild_id: 9,
                name: "Enemies".to_owned(),
            },
        ]);

        assert_eq!(guild.notice_subject, "Hello");
        assert_eq!(guild.notice, "Be nice.");
        assert_eq!(guild.notice_input, "Be nice.");
        assert_eq!(guild.alliance_list_text(), "Ally: Friends\nAntagonist: Enemies");
    }

    #[test]
    fn refreshing_the_member_list_keeps_resolved_names() {
        let mut guild = Guild::default();
        guild.apply_info(&info());
        guild.apply_members(&[member(150_009, 150_009, 0, 1)]);
        guild.apply_member_name(CharacterId(150_009), "Master".to_owned());

        guild.apply_members(&[member(150_009, 150_009, 0, 1), member(150_010, 150_010, 1, 1)]);

        assert_eq!(guild.members[0].name, "Master");
        assert!(guild.members[1].name.is_empty());
    }

    #[test]
    fn member_removal_updates_the_online_counter() {
        let mut guild = Guild::default();
        guild.apply_info(&info());
        guild.apply_members(&[member(150_009, 150_009, 0, 1), member(150_010, 150_010, 1, 0)]);

        assert!(guild.remove_member(CharacterId(150_009)));
        assert_eq!(guild.members.len(), 1);
        assert_eq!(
            guild.online_members, 1,
            "an online member leaving must decrement the connected count"
        );
        assert!(
            !guild.remove_member(CharacterId(150_009)),
            "removing an unknown member is a no-op"
        );
    }

    #[test]
    fn leaving_the_guild_keeps_the_local_ids() {
        let mut guild = Guild::default();
        guild.set_local_player(AccountId(150_009), CharacterId(150_009));
        guild.apply_info(&info());

        guild.leave_local_guild();

        assert!(!guild.is_in_guild());
        assert_eq!(guild.guild_id, 0);
        assert_eq!(guild.local_account_id, Some(AccountId(150_009)));
        assert_eq!(guild.local_character_id, Some(CharacterId(150_009)));
    }

    #[test]
    fn reset_clears_guild_data_and_ids() {
        let mut guild = Guild::default();
        guild.set_local_player(AccountId(1), CharacterId(2));
        guild.apply_info(&info());

        guild.reset();

        assert!(!guild.is_in_guild());
        assert_eq!(guild.local_account_id, None);
        assert_eq!(guild.local_character_id, None);
    }

    #[test]
    fn permission_text_lists_the_classic_bits() {
        assert_eq!(permission_text(0), "no rights");
        assert_eq!(
            permission_text(GUILD_PERMISSION_INVITE | GUILD_PERMISSION_EXPEL),
            "invite, expel"
        );
        assert_eq!(permission_text(0x1FF), "invite, expel, storage");
    }
}
