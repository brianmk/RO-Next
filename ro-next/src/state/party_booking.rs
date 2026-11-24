//! Party booking ("party search" / LFG) state for the 2019 rAthena server.
//!
//! The 2019 server implements the KRO party-booking board:
//!
//! * `CZ_PARTY_BOOKING_REQ_REGISTER` (0x0802) advertises the own party,
//! * `CZ_PARTY_BOOKING_REQ_SEARCH` (0x0804) queries the board,
//! * `CZ_PARTY_BOOKING_REQ_UPDATE` (0x0808) changes the desired jobs,
//! * `CZ_PARTY_BOOKING_REQ_DELETE` (0x0806) removes the own advertisement,
//! * `ZC_PARTY_BOOKING_ACK_*` / `ZC_PARTY_BOOKING_NOTIFY_*` report the result
//!   and keep every connected board in sync.
//!
//! The 2019 search result row contains no account/character id (only the
//! leader's name, level, map index and desired jobs), and the `0x0AE6` join
//! request is only parsed by servers with `PACKETVER >= 20191204`. The UI
//! therefore shows the board on the pinned CondaRO 20190605 server and offers
//! a whisper action as the 2019 join workaround; the join request/reply state
//! is still implemented for newer servers.

use ragnarok_packets::{AccountId, CharacterId, JobId, PartyBookingEntry2019, TilePosition};
use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// Maximum number of rows the server returns per search
/// (`MAX_PARTY_BOOKING_RESULTS` in rAthena `party.hpp`).
pub const MAXIMUM_SEARCH_RESULTS: u16 = 10;

/// Six job slots per advertisement; `u16::MAX` means "any job".
pub const PARTY_BOOKING_JOB_SLOTS: usize = 6;

/// One party-booking advertisement.
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct PartyBookingEntry {
    pub index: u32,
    pub character_name: String,
    pub level: u16,
    /// rAthena map index, `0` = any map. The client has no map-index table, so
    /// the UI shows the numeric index.
    pub map_id: u16,
    pub jobs: [u16; PARTY_BOOKING_JOB_SLOTS],
}

impl PartyBookingEntry {
    fn from_packet(entry: PartyBookingEntry2019) -> Self {
        Self {
            index: entry.index,
            character_name: entry.character_name.trim_end_matches('\0').to_owned(),
            level: entry.level,
            map_id: entry.map_id,
            jobs: entry.jobs,
        }
    }

    /// One row of the search result list.
    pub fn display_line(&self) -> String {
        let map = match self.map_id {
            0 => "any map".to_owned(),
            map_id => format!("map #{map_id}"),
        };

        format!(
            "{}  Lv {}  {}  {}",
            self.character_name,
            self.level,
            map,
            job_list_text(&self.jobs)
        )
    }
}

/// A join request another player sent to the local party leader
/// (`ZC_PARTY_REQ_MASTER_TO_JOIN`, 0x0AE7).
#[derive(Debug, Clone, RustState, StateElement)]
pub struct PartyBookingJoinRequest {
    pub character_id: CharacterId,
    pub account_id: AccountId,
    pub name: String,
    pub position: TilePosition,
}

impl Default for PartyBookingJoinRequest {
    fn default() -> Self {
        Self {
            character_id: CharacterId(0),
            account_id: AccountId(0),
            name: String::new(),
            position: TilePosition { x: 0, y: 0 },
        }
    }
}

/// The party leader's answer to a booking join request
/// (`ZC_PARTY_JOIN_REQ_ACK_FROM_MASTER`, 0x0AFA).
#[derive(Debug, Clone, RustState, StateElement)]
pub struct PartyBookingJoinResult {
    pub player_name: String,
    pub party_name: String,
    pub account_id: AccountId,
    pub refused: bool,
}

impl Default for PartyBookingJoinResult {
    fn default() -> Self {
        Self {
            player_name: String::new(),
            party_name: String::new(),
            account_id: AccountId(0),
            refused: false,
        }
    }
}

/// State of the party-booking board.
#[derive(Default, RustState, StateElement)]
pub struct PartyBooking {
    /// Whether the booking section is selected in the party window.
    pub visible: bool,
    /// Set after a successful `ZC_PARTY_BOOKING_ACK_REGISTER`.
    pub registered: bool,
    /// Text input of the register level field.
    pub register_level_input: String,
    /// Text input of the register map index field (`0` = any map).
    pub register_map_input: String,
    /// Six comma separated desired job ids (`-1`/`65535` = any job).
    pub register_jobs_input: String,
    /// Text input of the search level field (`0` = any level).
    pub search_level_input: String,
    /// Text input of the search map index field (`0` = any map).
    pub search_map_input: String,
    /// Search job id (`-1` = any job).
    pub search_job_input: String,
    /// Current search result page.
    pub results: Vec<PartyBookingEntry>,
    /// The server reported that more rows are available.
    pub more_results: bool,
    /// Index of the last row of the current page, used to page forward.
    pub last_index: u32,
    /// Short feedback line shown in the booking section.
    pub status_message: String,
    /// Pending join requests (only a `PACKETVER >= 20191204` server sends
    /// these; the list stays empty on CondaRO 20190605).
    pub join_requests: Vec<PartyBookingJoinRequest>,
    /// Last join result received from a party leader.
    pub join_result: Option<PartyBookingJoinResult>,
    /// Opt-in for sending the 0x0AE6 join request. rAthena only parses it with
    /// `PACKETVER >= 20191204`; the CondaRO 20190605 server does not, so the
    /// option defaults to off.
    pub join_requests_enabled: bool,
}

impl PartyBooking {
    /// Resets the transient state after leaving the map server.
    pub fn clear(&mut self) {
        self.registered = false;
        self.results.clear();
        self.more_results = false;
        self.last_index = 0;
        self.join_requests.clear();
        self.join_result = None;
        self.status_message.clear();
    }

    /// Applies a search result page.
    pub fn apply_search(&mut self, more_results: bool, entries: Vec<PartyBookingEntry2019>) {
        self.more_results = more_results;
        self.last_index = entries.last().map(|entry| entry.index).unwrap_or(self.last_index);
        self.results = entries.into_iter().map(PartyBookingEntry::from_packet).collect();

        self.status_message = match self.results.is_empty() {
            true => "No party advertisements found. The server may have party booking disabled, or there are no listings for these \
                     filters."
                .to_owned(),
            false => format!("{} party advertisement(s) found.", self.results.len()),
        };
    }

    /// Inserts a new advertisement announced by the server.
    pub fn apply_inserted(&mut self, entry: PartyBookingEntry2019) {
        let entry = PartyBookingEntry::from_packet(entry);

        match self.results.iter_mut().find(|known| known.index == entry.index) {
            Some(known) => *known = entry,
            None => self.results.push(entry),
        }
    }

    /// Applies a desired-job update announced by the server.
    pub fn apply_updated(&mut self, index: u32, jobs: [u16; PARTY_BOOKING_JOB_SLOTS]) {
        if let Some(entry) = self.results.iter_mut().find(|entry| entry.index == index) {
            entry.jobs = jobs;
        }
    }

    /// Removes an advertisement announced by the server.
    ///
    /// The board index of the own advertisement is not sent back to the client
    /// in the register acknowledgement, so a notify-delete cannot tell whether
    /// it removed the own ad; `registered` is only cleared by
    /// `ZC_PARTY_BOOKING_ACK_DELETE` (0x0807) or by a fresh register result.
    pub fn apply_deleted(&mut self, index: u32) {
        self.results.retain(|entry| entry.index != index);
    }

    /// Applies the register acknowledgement.
    pub fn apply_register_result(&mut self, result: u16) {
        self.status_message = match result {
            0 => {
                self.registered = true;
                "The party advertisement was registered.".to_owned()
            }
            1 => "The server rejected the party advertisement.".to_owned(),
            2 => {
                self.registered = true;
                "An advertisement is already registered for this character.".to_owned()
            }
            result => format!("Unexpected party booking register result {result}."),
        };
    }

    /// Applies the delete acknowledgement.
    pub fn apply_delete_result(&mut self, result: u16) {
        self.status_message = match result {
            0 | 1 => {
                self.registered = false;
                "The party advertisement was removed.".to_owned()
            }
            2 => "The server could not remove the party advertisement.".to_owned(),
            3 => {
                self.registered = false;
                "There was no registered party advertisement.".to_owned()
            }
            result => format!("Unexpected party booking delete result {result}."),
        };
    }

    /// Adds an incoming booking join request.
    pub fn apply_join_request(&mut self, character_id: CharacterId, account_id: AccountId, name: String, position: TilePosition) {
        let name = name.trim_end_matches('\0').to_owned();

        if self.join_requests.iter().any(|request| request.character_id == character_id) {
            return;
        }

        self.status_message = format!("{name} requested to join the party.");
        self.join_requests.push(PartyBookingJoinRequest {
            character_id,
            account_id,
            name,
            position,
        });
    }

    /// Removes a handled join request.
    pub fn remove_join_request(&mut self, character_id: CharacterId) {
        self.join_requests.retain(|request| request.character_id != character_id);
    }

    /// Applies the leader's answer to a join request.
    pub fn apply_join_result(&mut self, player_name: String, party_name: String, account_id: AccountId, refused: bool) {
        self.status_message = match refused {
            true => format!("{player_name} rejected your request to join {party_name}."),
            false => format!("{player_name} accepted your request to join {party_name}."),
        };

        self.join_result = Some(PartyBookingJoinResult {
            player_name,
            party_name,
            account_id,
            refused,
        });
    }

    /// Parse the six desired job ids of the register/update field.
    pub fn parse_register_jobs(&self) -> [u16; PARTY_BOOKING_JOB_SLOTS] {
        parse_jobs(&self.register_jobs_input)
    }

    /// Parse the register level field (`0` when empty/invalid).
    pub fn register_level(&self) -> u16 {
        self.register_level_input.trim().parse().unwrap_or(0)
    }

    /// Parse the register map index field (`0` = any map).
    pub fn register_map_id(&self) -> u16 {
        self.register_map_input.trim().parse().unwrap_or(0)
    }

    /// Parse the search level field (`0` = any level).
    pub fn search_level(&self) -> u16 {
        self.search_level_input.trim().parse().unwrap_or(0)
    }

    /// Parse the search map index field (`0` = any map).
    pub fn search_map_id(&self) -> u16 {
        self.search_map_input.trim().parse().unwrap_or(0)
    }

    /// Parse the search job field (`-1` = any job).
    pub fn search_job(&self) -> i16 {
        self.search_job_input.trim().parse().unwrap_or(-1)
    }

    /// Advertisement with the given board index.
    pub fn result(&self, index: u32) -> Option<&PartyBookingEntry> {
        self.results.iter().find(|entry| entry.index == index)
    }
}

/// Parses up to six job ids from a comma/space separated string.
///
/// Missing entries become `u16::MAX` ("any job"), which is the value the
/// rAthena register parser stores as `-1` (`party.cpp:1536`).
pub fn parse_jobs(input: &str) -> [u16; PARTY_BOOKING_JOB_SLOTS] {
    let mut jobs = [u16::MAX; PARTY_BOOKING_JOB_SLOTS];

    for (slot, value) in input
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|value| !value.is_empty())
        .take(PARTY_BOOKING_JOB_SLOTS)
        .enumerate()
    {
        jobs[slot] = match value.parse::<i32>() {
            Ok(job_id) if (0..=u16::MAX as i32).contains(&job_id) => job_id as u16,
            // `-1` (and anything invalid) means "any job".
            _ => u16::MAX,
        };
    }

    jobs
}

/// Human readable list of the desired jobs of an advertisement.
pub fn job_list_text(jobs: &[u16; PARTY_BOOKING_JOB_SLOTS]) -> String {
    let jobs: Vec<String> = jobs
        .iter()
        .filter(|job| **job != u16::MAX)
        .map(|job| crate::state::guild::job_name(JobId(*job)))
        .collect();

    match jobs.is_empty() {
        true => "any job".to_owned(),
        false => jobs.join(", "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet_entry(index: u32, name: &str) -> PartyBookingEntry2019 {
        PartyBookingEntry2019 {
            index,
            character_name: name.to_owned(),
            expire_time: 0,
            level: 99,
            map_id: 0,
            jobs: [7, 8, u16::MAX, u16::MAX, u16::MAX, u16::MAX],
        }
    }

    #[test]
    fn jobs_parse_from_comma_and_space_separated_input() {
        let jobs = parse_jobs("7, 8 -1 16");
        assert_eq!(jobs, [7, 8, u16::MAX, 16, u16::MAX, u16::MAX]);

        assert_eq!(parse_jobs(""), [u16::MAX; 6]);
        assert_eq!(parse_jobs("1,2,3,4,5,6,7"), [1, 2, 3, 4, 5, 6], "only six slots are sent");
    }

    #[test]
    fn search_results_replace_the_previous_page_and_track_paging() {
        let mut booking = PartyBooking::default();
        booking.apply_search(false, vec![packet_entry(3, "A"), packet_entry(4, "B")]);

        assert_eq!(booking.results.len(), 2);
        assert_eq!(booking.last_index, 4);
        assert_eq!(booking.results[0].display_line(), "A  Lv 99  any map  Knight, Priest");

        booking.apply_search(true, vec![packet_entry(5, "C")]);
        assert_eq!(booking.results.len(), 1);
        assert!(booking.more_results);
    }

    #[test]
    fn insert_update_and_delete_keep_the_board_in_sync() {
        let mut booking = PartyBooking::default();
        booking.apply_search(false, vec![packet_entry(3, "A")]);

        booking.apply_inserted(packet_entry(9, "New"));
        assert_eq!(booking.results.len(), 2);

        booking.apply_updated(9, [1, u16::MAX, u16::MAX, u16::MAX, u16::MAX, u16::MAX]);
        assert_eq!(booking.results[1].jobs[0], 1);

        booking.apply_deleted(9);
        assert_eq!(booking.results.len(), 1);
    }

    #[test]
    fn register_and_delete_results_set_the_status() {
        let mut booking = PartyBooking::default();

        booking.apply_register_result(0);
        assert!(booking.registered);

        booking.apply_register_result(2);
        assert!(booking.registered);

        booking.apply_delete_result(0);
        assert!(!booking.registered);

        booking.apply_delete_result(3);
        assert!(!booking.registered);
        assert!(booking.status_message.contains("no registered"));
    }

    #[test]
    fn join_requests_are_deduplicated_and_removed() {
        let mut booking = PartyBooking::default();
        let position = TilePosition { x: 10, y: 20 };

        booking.apply_join_request(CharacterId(100), AccountId(200), "Aspirant".to_owned(), position);
        booking.apply_join_request(CharacterId(100), AccountId(200), "Aspirant".to_owned(), position);
        assert_eq!(booking.join_requests.len(), 1);

        booking.remove_join_request(CharacterId(100));
        assert!(booking.join_requests.is_empty());
    }
}
