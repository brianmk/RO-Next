//! State of the small content windows added for the 2019 feature gaps:
//!
//! * the chat room window (`CZ_CREATE_CHATROOM` / `ZC_ENTER_ROOM` family),
//! * the optional character-server pincode prompt (`HC_ACK_PINCODE`, 0x08B9),
//! * the menu-side state of the packet-stream recorder (replay recording).
//!
//! Keeping all three in one root keeps the [`ClientState`] change to a single
//! field. The book window has no mutable state of its own: it is rebuilt from
//! the client archive whenever a page changes.
//!
//! [`ClientState`]: crate::state::ClientState

use korangar_interface::element::StateElement;
use ragnarok_packets::AccountId;
use rust_state::RustState;

/// The 2019 server caps a chat room at this many users; used as the default
/// room limit of the create form.
pub const DEFAULT_CHAT_ROOM_LIMIT: u16 = 20;

/// Maximum number of messages kept in the chat-room window.
pub const MAXIMUM_CHAT_ROOM_MESSAGES: usize = 200;

/// One member of the open chat room.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct ChatRoomMember {
    pub name: String,
    /// The owner gets the room menu in the official client; only used as a
    /// marker `(owner)` here.
    pub owner: bool,
}

/// One line of the chat-room message list.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct ChatRoomMessage {
    pub text: String,
    /// Join/leave/error lines are drawn with the server color.
    pub system: bool,
}

/// A chat room that was announced with `ZC_ROOM_NEWENTRY` (0x00D7).
#[derive(Clone, Debug, RustState, StateElement)]
pub struct ChatRoomSummary {
    pub owner_id: AccountId,
    pub chat_id: u32,
    pub title: String,
    pub users: u16,
    pub limit: u16,
    /// `0` = private (password), `1` = public, `2` = NPC arena,
    /// `3` = PK zone.
    pub room_type: u8,
}

impl Default for ChatRoomSummary {
    fn default() -> Self {
        Self {
            owner_id: AccountId(0),
            chat_id: 0,
            title: String::new(),
            users: 0,
            limit: 0,
            room_type: 0,
        }
    }
}

impl ChatRoomSummary {
    /// Human readable line for the "nearby rooms" list.
    pub fn display(&self) -> String {
        let visibility = match self.room_type {
            0 => " [private]",
            1 => "",
            2 => " [arena]",
            _ => " [pk]",
        };

        format!(
            "{}{} ({}/{})",
            self.title.trim_end_matches('\0'),
            visibility,
            self.users,
            self.limit
        )
    }
}

/// State of the chat-room window.
#[derive(RustState, StateElement)]
pub struct ChatRoom {
    #[hidden_element]
    pub in_room: bool,
    /// Title of the open room, or the last title the server sent.
    #[hidden_element]
    pub title: String,
    /// Id of the joined room (`0` while the create ack did not provide one).
    #[hidden_element]
    pub chat_id: u32,
    #[hidden_element]
    pub limit: u16,
    #[hidden_element]
    pub members: Vec<ChatRoomMember>,
    #[hidden_element]
    pub messages: Vec<ChatRoomMessage>,
    /// Rooms announced by `ZC_ROOM_NEWENTRY` on the current map.
    #[hidden_element]
    pub nearby: Vec<ChatRoomSummary>,
    /// Title the player clicked in the nearby list; used for `ZC_ENTER_ROOM`
    /// because that packet does not carry the room title.
    #[hidden_element]
    joining_title: String,
    /// Text box of the create form.
    pub create_title: String,
    /// Password text box of the join form (private rooms).
    pub join_password: String,
    /// Text box of the room chat line.
    pub message_text: String,
    /// Last error shown in the window (create/join failure).
    #[hidden_element]
    pub error: String,
}

impl Default for ChatRoom {
    fn default() -> Self {
        Self {
            in_room: false,
            title: String::new(),
            chat_id: 0,
            limit: DEFAULT_CHAT_ROOM_LIMIT,
            members: Vec::new(),
            messages: Vec::new(),
            nearby: Vec::new(),
            joining_title: String::new(),
            create_title: String::new(),
            join_password: String::new(),
            message_text: String::new(),
            error: String::new(),
        }
    }
}

impl ChatRoom {
    pub fn is_in_room(&self) -> bool {
        self.in_room
    }

    /// Window title selector text.
    pub fn display_title(&self) -> String {
        if self.in_room && !self.title.is_empty() {
            format!("Chat room: {}", self.title.trim_end_matches('\0'))
        } else {
            "Chat room".to_owned()
        }
    }

    /// A create request was accepted (`ZC_ACK_CREATE_CHATROOM` flag 0).
    ///
    /// rAthena does not send the chat id or a member list back to the creator,
    /// so the room starts with the local player as its only member and leave
    /// uses `CZ_EXIT_ROOM` (which has no id).
    pub fn created(&mut self, local_player_name: &str) {
        self.in_room = true;
        self.title = self.create_title.trim().to_owned();
        self.chat_id = 0;
        self.limit = DEFAULT_CHAT_ROOM_LIMIT;
        self.members.clear();
        if !local_player_name.is_empty() {
            self.members.push(ChatRoomMember {
                name: local_player_name.to_owned(),
                owner: true,
            });
        }
        self.messages.clear();
        self.error.clear();
        self.join_password.clear();
    }

    /// `ZC_ENTER_ROOM` (0x00DB): the local player joined a room.
    pub fn entered(&mut self, chat_id: u32, members: &[(String, bool)]) {
        self.in_room = true;
        self.chat_id = chat_id;
        self.title = self.joining_title.trim_end_matches('\0').to_owned();
        self.members = members
            .iter()
            .map(|(name, owner)| ChatRoomMember {
                name: name.trim_end_matches('\0').to_owned(),
                owner: *owner,
            })
            .collect();
        self.limit = self.limit.max(self.members.len() as u16);
        self.messages.clear();
        self.error.clear();
        self.joining_title.clear();
        self.join_password.clear();
    }

    /// `ZC_MEMBER_NEWENTRY` (0x00DC).
    pub fn member_joined(&mut self, name: &str) {
        let name = name.trim_end_matches('\0');

        if name.is_empty() || self.members.iter().any(|member| member.name == name) {
            return;
        }

        self.members.push(ChatRoomMember {
            name: name.to_owned(),
            owner: false,
        });
        self.push_message(format!("{name} entered the chat room."), true);
    }

    /// `ZC_MEMBER_EXIT` (0x00DD). Returns `true` when the local player left.
    pub fn member_left(&mut self, name: &str, kicked: bool, local_player_name: &str) -> bool {
        let name = name.trim_end_matches('\0');
        let was_owner = self
            .members
            .iter()
            .find(|member| member.name == name)
            .is_some_and(|member| member.owner);

        self.members.retain(|member| member.name != name);

        let action = match kicked {
            true => "was kicked from",
            false => "left",
        };
        self.push_message(format!("{name} {action} the chat room."), true);

        if was_owner && !self.members.is_empty() {
            // The server promotes the next user; the `ZC_ROLE_CHANGE` that
            // follows names the new owner. Until then, keep a member marked so
            // the list never shows a room without an owner.
            self.members[0].owner = true;
        }

        name == local_player_name
    }

    /// `ZC_ROLE_CHANGE` (0x00E1).
    pub fn role_changed(&mut self, flag: u32, name: &str) {
        let name = name.trim_end_matches('\0');
        let is_owner = flag == 0;

        if is_owner {
            for member in &mut self.members {
                member.owner = false;
            }
        }

        if let Some(member) = self.members.iter_mut().find(|member| member.name == name) {
            member.owner = is_owner;
        }
    }

    /// `ZC_CHANGE_CHATROOM` (0x00DF).
    pub fn changed(&mut self, chat_id: u32, title: String, limit: u16, room_type: u8, owner_id: AccountId, users: u16) {
        let title = title.trim_end_matches('\0').to_owned();

        if self.in_room && (self.chat_id == chat_id || self.chat_id == 0) {
            self.chat_id = chat_id;
            self.title = title;
            self.limit = limit;
            return;
        }

        self.upsert_nearby(ChatRoomSummary {
            owner_id,
            chat_id,
            title,
            users,
            limit,
            room_type,
        });
    }

    /// `ZC_ROOM_NEWENTRY` (0x00D7).
    pub fn announced(&mut self, summary: ChatRoomSummary) {
        // Our own room is reported through the create ack; ignore it in the
        // nearby list to avoid offering a join to a room we are already in.
        if self.in_room && summary.chat_id == self.chat_id {
            return;
        }

        self.upsert_nearby(summary);
    }

    pub fn destroyed(&mut self, chat_id: u32) {
        self.nearby.retain(|room| room.chat_id != chat_id);

        if self.in_room && self.chat_id == chat_id {
            self.leave_locally();
        }
    }

    fn upsert_nearby(&mut self, summary: ChatRoomSummary) {
        if let Some(existing) = self.nearby.iter_mut().find(|room| room.chat_id == summary.chat_id) {
            *existing = summary;
        } else {
            self.nearby.push(summary);
            self.nearby.sort_by_key(|room| (room.room_type, room.title.to_ascii_lowercase()));
        }
    }

    pub fn set_joining_title(&mut self, title: String) {
        self.joining_title = title;
    }

    pub fn push_message(&mut self, text: String, system: bool) {
        if self.messages.len() >= MAXIMUM_CHAT_ROOM_MESSAGES {
            self.messages.remove(0);
        }

        self.messages.push(ChatRoomMessage {
            text: text.trim_end_matches('\0').to_owned(),
            system,
        });
    }

    pub fn set_error(&mut self, error: String) {
        self.error = error;
    }

    /// Leave locally without waiting for the server, so the window closes even
    /// when `ZC_MEMBER_EXIT` is lost.
    pub fn leave_locally(&mut self) {
        self.in_room = false;
        self.title.clear();
        self.chat_id = 0;
        self.members.clear();
        self.messages.clear();
        self.message_text.clear();
        self.error.clear();
    }
}

/// Result of feeding an `HC_ACK_PINCODE` state into [`PincodePrompt`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PincodePromptOutcome {
    /// Show (or keep showing) the prompt.
    Open,
    /// The server said the PIN check passed; no prompt is needed.
    Passed,
    /// The server asks for a flow this client does not implement.
    Unsupported(&'static str),
}

/// State of the optional character-server pincode prompt.
#[derive(RustState, StateElement)]
pub struct PincodePrompt {
    #[hidden_element]
    pub visible: bool,
    #[hidden_element]
    pub seed: u32,
    #[hidden_element]
    pub account_id: AccountId,
    /// `true` for `PINCODE_NEW`/`PINCODE_EXPIRED`-style "set a first PIN"
    /// states, `false` for the normal check.
    #[hidden_element]
    pub is_new: bool,
    /// Text box of the prompt.
    pub code: String,
    /// Error line shown below the prompt.
    pub error: String,
}

impl Default for PincodePrompt {
    fn default() -> Self {
        Self {
            visible: false,
            seed: 0,
            account_id: AccountId(0),
            is_new: false,
            code: String::new(),
            error: String::new(),
        }
    }
}

impl PincodePrompt {
    /// Feed one `HC_ACK_PINCODE` (0x08B9) state.
    ///
    /// The state values are the rAthena `pincode_state` enum
    /// (`char.hpp:113-131`): 0 = ok, 1 = ask, 2 = not set/force new,
    /// 3 = expired, 4 = new, 5/6 = unsupported KSSN messages, 7 = passed,
    /// 8 = wrong PIN.
    pub fn handle_state(&mut self, seed: u32, account_id: AccountId, state: u16) -> PincodePromptOutcome {
        self.seed = seed;
        self.account_id = account_id;

        match state {
            0 | 7 => {
                self.visible = false;
                self.code.clear();
                self.error.clear();
                PincodePromptOutcome::Passed
            }
            1 => {
                self.visible = true;
                self.is_new = false;
                self.code.clear();
                self.error.clear();
                PincodePromptOutcome::Open
            }
            2 | 4 => {
                self.visible = true;
                self.is_new = true;
                self.code.clear();
                self.error.clear();
                PincodePromptOutcome::Open
            }
            8 => {
                self.visible = true;
                self.code.clear();
                self.error = "The PIN was incorrect. Try again.".to_owned();
                PincodePromptOutcome::Open
            }
            3 => {
                self.visible = false;
                PincodePromptOutcome::Unsupported("The server requires a PIN change, which this client does not support yet.")
            }
            5 => {
                self.visible = false;
                PincodePromptOutcome::Unsupported("The server reported an invalid PIN request (PINCODE_ILLEGAL).")
            }
            6 => {
                self.visible = false;
                PincodePromptOutcome::Unsupported("The server requires a KSSN based PIN, which is not supported.")
            }
            _ => {
                self.visible = false;
                PincodePromptOutcome::Unsupported("The server sent an unknown pincode state.")
            }
        }
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
        self.code.clear();
        self.error.clear();
    }
}

/// State of the menu-side replay recording toggle.
#[derive(Default, RustState, StateElement)]
pub struct ReplayControl {
    #[hidden_element]
    pub recording: bool,
    /// Path of the replay that was recorded last, shown in the menu.
    #[hidden_element]
    pub last_path: String,
}

/// Root of the window state added for this feature set.
#[derive(Default, RustState, StateElement)]
pub struct ContentWindows {
    #[hidden_element]
    pub chat_room: ChatRoom,
    #[hidden_element]
    pub pincode: PincodePrompt,
    #[hidden_element]
    pub replay: ReplayControl,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(chat_id: u32, title: &str, room_type: u8) -> ChatRoomSummary {
        ChatRoomSummary {
            owner_id: AccountId(1),
            chat_id,
            title: title.to_owned(),
            users: 1,
            limit: 20,
            room_type,
        }
    }

    #[test]
    fn nearby_rooms_are_replaced_and_removed_by_id() {
        let mut room = ChatRoom::default();
        room.announced(summary(7, "First", 1));
        room.announced(summary(8, "Second", 1));
        room.announced(summary(7, "First renamed", 0));

        assert_eq!(room.nearby.len(), 2);
        assert_eq!(room.nearby[0].title, "First renamed");

        room.destroyed(7);
        assert_eq!(room.nearby.len(), 1);
        assert_eq!(room.nearby[0].chat_id, 8);
    }

    #[test]
    fn joining_uses_the_clicked_title_and_member_flags() {
        let mut room = ChatRoom::default();
        room.set_joining_title("Party".to_owned());
        room.entered(12, &[("Owner".to_owned(), true), ("Guest".to_owned(), false)]);

        assert!(room.is_in_room());
        assert_eq!(room.chat_id, 12);
        assert_eq!(room.title, "Party");
        assert_eq!(room.members.len(), 2);
        assert!(room.members[0].owner);
        assert!(!room.members[1].owner);
    }

    #[test]
    fn member_events_keep_the_list_in_sync() {
        let mut room = ChatRoom::default();
        room.entered(1, &[("Me".to_owned(), true)]);

        room.member_joined("Other");
        room.member_joined("Other");
        assert_eq!(room.members.len(), 2);

        room.role_changed(0, "Other");
        assert!(!room.members[0].owner);
        assert!(room.members[1].owner);

        assert!(!room.member_left("Other", false, "Me"));
        assert_eq!(room.members.len(), 1);
        assert!(room.members[0].owner);

        assert!(room.member_left("Me", false, "Me"));
        assert!(room.members.is_empty());
    }

    #[test]
    fn messages_are_bounded() {
        let mut room = ChatRoom::default();
        for index in 0..(MAXIMUM_CHAT_ROOM_MESSAGES + 5) {
            room.push_message(format!("line {index}"), false);
        }

        assert_eq!(room.messages.len(), MAXIMUM_CHAT_ROOM_MESSAGES);
        assert_eq!(room.messages.last().unwrap().text, "line 204");
    }

    #[test]
    fn pincode_states_map_to_the_expected_prompt() {
        let mut prompt = PincodePrompt::default();

        assert_eq!(prompt.handle_state(1, AccountId(5), 0), PincodePromptOutcome::Passed);
        assert!(!prompt.visible);

        assert_eq!(prompt.handle_state(2, AccountId(5), 1), PincodePromptOutcome::Open);
        assert!(prompt.visible);
        assert!(!prompt.is_new);

        assert_eq!(prompt.handle_state(3, AccountId(5), 2), PincodePromptOutcome::Open);
        assert!(prompt.is_new);

        assert_eq!(prompt.handle_state(4, AccountId(5), 8), PincodePromptOutcome::Open);
        assert!(!prompt.error.is_empty());

        assert!(matches!(
            prompt.handle_state(5, AccountId(5), 3),
            PincodePromptOutcome::Unsupported(_)
        ));
    }
}
