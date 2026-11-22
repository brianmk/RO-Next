//! Session/connection lifecycle state machine.
//!
//! [`SessionMachine`] owns the sequence of login, character selection, map
//! entry, warping, recovery and shutdown, together with every deadline that
//! belongs to that sequence. It is a plain data structure: the client calls
//! the transition methods below and executes the [`SessionAction`]s returned
//! by [`SessionMachine::tick`].
//!
//! Transition contract:
//!
//! * Legal transitions are listed in [`SESSION_TRANSITIONS`]; they are the
//!   single source of truth for the table-driven tests below.
//! * An illegal transition is ignored, counted and logged at debug level. The
//!   client must never panic because the server sent an unexpected event.
//!
//! Timers (all of them previously loose fields on `Client`):
//!
//! | timer | old field | policy |
//! |---|---|---|
//! | map entry watchdog | `map_entry_watchdog` | 20 s after character selection |
//! | map load retry | `pending_map_load` tuple | retry every 8 s, give up after 4 retries |
//! | map abort recovery | `map_entry_abort_deadline` | 2 s after a failed map load |
//! | character reconnect | `character_server_reconnect_at/attempts` | every 2 s, 30 attempts |
//! | shutdown disconnect | `shutdown_deadline` | 2 s after a quit request |

use std::time::{Duration, Instant};

use ragnarok_packets::TilePosition;

/// How long the client waits for the map server to send the map entry data
/// after a character was selected.
pub const MAP_ENTRY_WATCHDOG: Duration = Duration::from_secs(20);
/// Delay between two retries of a map asset load.
pub const MAP_LOAD_RETRY_DELAY: Duration = super::resource::ResourcePolicy::MAP_DATA.retry_delay;
/// Number of retries before a map load is given up (the initial request does
/// not count).
pub const MAP_LOAD_MAX_ATTEMPTS: u32 = super::resource::ResourcePolicy::MAP_DATA.maximum_attempts;
/// How long the client waits for the map server disconnect after aborting a map
/// entry before it recovers on its own.
pub const MAP_ABORT_RECOVERY_DELAY: Duration = Duration::from_millis(2000);
/// Delay between two character-server reconnect attempts.
pub const CHARACTER_RECONNECT_DELAY: Duration = Duration::from_secs(2);
/// Number of character-server reconnect attempts after a map-server restart.
pub const CHARACTER_RECONNECT_MAX_ATTEMPTS: u32 = 30;
/// How long the client waits for the map server to confirm a clean logout
/// before it closes the remaining sockets.
pub const SHUTDOWN_DISCONNECT_TIMEOUT: Duration = Duration::from_millis(2000);

/// The high-level state of the client session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// No connection at all (startup, or after a completed shutdown).
    Disconnected,
    /// Logging in to the login server.
    Login,
    /// Connected to the character server / selecting a character (also the
    /// state the client returns to after a failed map entry).
    CharSelect,
    /// A character was selected and the client is connecting to the map server
    /// and waiting for/loading the map data.
    MapLoading,
    /// The map is loaded and the local player exists.
    InGame,
    /// A map entry, map connection or map load failed and the client is
    /// recovering back to the character selection screen.
    Recovering,
    /// A quit was requested; the client is disconnecting cleanly.
    Shutdown,
}

/// Inputs that can move the session between states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionEvent {
    /// The player submitted the login form.
    LoginStarted,
    /// The login server accepted the credentials.
    LoginSucceeded,
    /// A character was selected; the map entry starts.
    CharacterSelected,
    /// The map server sent the map change packet (`ZC_NPCACK_MAPMOVE` /
    /// `ZC_LOADING_END`).
    MapChangeReceived,
    /// The map assets finished loading and the local player exists.
    MapAssetsLoaded,
    /// The map assets could not be loaded.
    MapEntryFailed,
    /// The map entry watchdog expired (no map change packet arrived).
    MapEntryWatchdogExpired,
    /// The map server connection was lost.
    MapServerDisconnected,
    /// The character-server reconnect succeeded.
    CharacterServerConnected,
    /// The recovery to the character selection screen finished.
    RecoveryFinished,
    /// The player (or SIGTERM/SIGHUP) requested a quit.
    ShutdownRequested,
    /// The clean disconnect finished.
    ShutdownFinished,
}

/// One edge of the session transition table.
pub type SessionTransition = (SessionState, SessionEvent, SessionState);

/// The complete transition table. Every entry is exercised by the
/// `transition_table` test; every other (state, event) pair must be illegal.
pub const SESSION_TRANSITIONS: &[SessionTransition] = &[
    // Login flow.
    (SessionState::Disconnected, SessionEvent::LoginStarted, SessionState::Login),
    (SessionState::Login, SessionEvent::LoginSucceeded, SessionState::CharSelect),
    (SessionState::Login, SessionEvent::ShutdownRequested, SessionState::Shutdown),
    // Character selection.
    (
        SessionState::CharSelect,
        SessionEvent::CharacterSelected,
        SessionState::MapLoading,
    ),
    (SessionState::CharSelect, SessionEvent::LoginStarted, SessionState::Login),
    (
        SessionState::CharSelect,
        SessionEvent::MapServerDisconnected,
        SessionState::CharSelect,
    ),
    (
        SessionState::CharSelect,
        SessionEvent::RecoveryFinished,
        SessionState::CharSelect,
    ),
    (
        SessionState::CharSelect,
        SessionEvent::ShutdownRequested,
        SessionState::Shutdown,
    ),
    // Map loading (login map load and in-game warps).
    (
        SessionState::MapLoading,
        SessionEvent::MapChangeReceived,
        SessionState::MapLoading,
    ),
    (SessionState::MapLoading, SessionEvent::MapAssetsLoaded, SessionState::InGame),
    (SessionState::MapLoading, SessionEvent::MapEntryFailed, SessionState::Recovering),
    (
        SessionState::MapLoading,
        SessionEvent::MapEntryWatchdogExpired,
        SessionState::Recovering,
    ),
    (
        SessionState::MapLoading,
        SessionEvent::MapServerDisconnected,
        SessionState::Recovering,
    ),
    (
        SessionState::MapLoading,
        SessionEvent::ShutdownRequested,
        SessionState::Shutdown,
    ),
    // In game.
    (SessionState::InGame, SessionEvent::MapChangeReceived, SessionState::MapLoading),
    (
        SessionState::InGame,
        SessionEvent::MapServerDisconnected,
        SessionState::Recovering,
    ),
    (SessionState::InGame, SessionEvent::ShutdownRequested, SessionState::Shutdown),
    // Recovery back to the character selection screen.
    (SessionState::Recovering, SessionEvent::MapEntryFailed, SessionState::Recovering),
    (
        SessionState::Recovering,
        SessionEvent::MapServerDisconnected,
        SessionState::Recovering,
    ),
    (
        SessionState::Recovering,
        SessionEvent::CharacterServerConnected,
        SessionState::Recovering,
    ),
    (
        SessionState::Recovering,
        SessionEvent::RecoveryFinished,
        SessionState::CharSelect,
    ),
    (
        SessionState::Recovering,
        SessionEvent::ShutdownRequested,
        SessionState::Shutdown,
    ),
    // Shutdown.
    (SessionState::Shutdown, SessionEvent::ShutdownRequested, SessionState::Shutdown),
    (
        SessionState::Shutdown,
        SessionEvent::ShutdownFinished,
        SessionState::Disconnected,
    ),
];

/// A pending map asset load (formerly the `pending_map_load` tuple).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingMapLoad {
    pub map_name: String,
    pub position: Option<TilePosition>,
    /// When the most recent load request was issued.
    pub requested_at: Instant,
    /// Number of retries that were already issued (the initial request is not
    /// counted).
    pub retries: u32,
}

/// Facts about the rest of the client that the session machine needs to make
/// decisions but does not own. Passed to [`SessionMachine::tick`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionObservation {
    /// `Client::map` is `Some` (we are on the menu map or in a loaded map).
    pub map_assets_loaded: bool,
    /// The local player entity exists (`this_player()`).
    pub player_present: bool,
    pub login_server_connected: bool,
    /// The login or server-selection window is open.
    pub login_surface_visible: bool,
}

/// Side effect the client has to execute after [`SessionMachine::tick`].
#[derive(Debug, Clone, PartialEq)]
pub enum SessionAction {
    /// Abort the current map entry (failed map load or watchdog expiry).
    AbortMapEntry { reason: String },
    /// The map-server disconnect that normally triggers the recovery did not
    /// arrive; recover directly.
    RecoverToCharacterSelection,
    /// Re-request a map asset load.
    RetryMapLoad { map_name: String, position: Option<TilePosition> },
    /// A map load failed all retries.
    MapLoadGaveUp { map_name: String },
    /// Reconnect to the character server (map-server restart recovery).
    ReconnectCharacterServer,
    /// The character-server reconnect attempts are exhausted.
    CharacterReconnectGaveUp,
}

/// Explicit session state machine.
#[derive(Debug, Clone)]
pub struct SessionMachine {
    state: SessionState,
    illegal_transitions: u64,

    /// Deadline until which the client expects the map server to send the map
    /// entry data after a character was selected.
    map_entry_watchdog: Option<Instant>,
    /// Deadline for the map server disconnect that aborts a failed map entry.
    map_abort_deadline: Option<Instant>,
    /// Pending retry of a map asset load.
    pending_map_load: Option<PendingMapLoad>,
    /// Set while the client recovers from an aborted map entry; suppresses the
    /// follow-up "Rejected from server" window.
    map_entry_aborted: bool,
    /// Map error that is shown at the character selection screen.
    map_load_error: Option<String>,

    /// Whether the character server connection completed its login handshake.
    ///
    /// `NetworkingSystem::is_character_server_connected` only says that a
    /// socket task exists; it is set optimistically when the connection command
    /// is queued. A reconnect that the server silently drops (for example while
    /// the old map session is still registered) must not stop the retry loop,
    /// therefore the retry loop is driven by this protocol-level flag instead.
    character_server_ready: bool,
    /// Next character-server reconnect attempt.
    character_reconnect_at: Option<Instant>,
    /// Remaining character-server reconnect attempts.
    character_reconnect_attempts: u32,

    /// Deadline for the clean disconnect when quitting.
    shutdown_deadline: Option<Instant>,
}

impl Default for SessionMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionMachine {
    pub fn new() -> Self {
        Self {
            state: SessionState::Disconnected,
            illegal_transitions: 0,
            map_entry_watchdog: None,
            map_abort_deadline: None,
            pending_map_load: None,
            map_entry_aborted: false,
            map_load_error: None,
            character_server_ready: false,
            character_reconnect_at: None,
            character_reconnect_attempts: 0,
            shutdown_deadline: None,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Number of illegal transition attempts that were ignored (debug helper).
    #[cfg(test)]
    pub fn illegal_transitions(&self) -> u64 {
        self.illegal_transitions
    }

    fn set_state(&mut self, next: SessionState) {
        let previous = self.state;
        self.state = next;
        crate::logging::log_debug!("lifecycle: session {:?} -> {:?}", previous, next);

        match next {
            SessionState::CharSelect => {
                self.map_entry_watchdog = None;
                self.map_abort_deadline = None;
                self.pending_map_load = None;
            }
            SessionState::InGame => {
                self.map_entry_watchdog = None;
                self.map_abort_deadline = None;
                self.pending_map_load = None;
                self.map_entry_aborted = false;
                self.map_load_error = None;
            }
            SessionState::Recovering => {
                self.map_entry_watchdog = None;
                self.map_entry_aborted = true;
            }
            SessionState::Shutdown => {
                self.character_reconnect_at = None;
                self.character_reconnect_attempts = 0;
            }
            SessionState::Disconnected | SessionState::Login | SessionState::MapLoading => {}
        }
    }

    /// Apply one event. Returns `true` when the transition was legal.
    pub fn transition(&mut self, event: SessionEvent) -> bool {
        let Some(&(_, _, next)) = SESSION_TRANSITIONS
            .iter()
            .find(|(from, table_event, _)| *from == self.state && *table_event == event)
        else {
            self.illegal_transitions += 1;
            crate::logging::log_debug!(
                "lifecycle: ignoring illegal session transition {:?} in state {:?}",
                event,
                self.state
            );
            return false;
        };

        self.set_state(next);
        true
    }

    /// Start a map entry after a character was selected: enter `MapLoading`,
    /// arm the map-entry watchdog and forget a previous map load.
    pub fn begin_map_entry(&mut self, now: Instant) {
        self.transition(SessionEvent::CharacterSelected);
        self.pending_map_load = None;
        self.map_abort_deadline = None;
        self.map_entry_watchdog = Some(now + MAP_ENTRY_WATCHDOG);
    }

    /// The map server sent the map change packet; record the asset load that
    /// the caller is about to request.
    pub fn map_change_received(&mut self, map_name: String, position: Option<TilePosition>, now: Instant) {
        self.transition(SessionEvent::MapChangeReceived);
        self.pending_map_load = Some(PendingMapLoad {
            map_name,
            position,
            requested_at: now,
            retries: 0,
        });
    }

    /// The map assets loaded (only call this once the local player exists; the
    /// menu map load is not a session transition).
    pub fn map_assets_loaded(&mut self) {
        self.transition(SessionEvent::MapAssetsLoaded);
    }

    /// A map asset load failed; the entry is aborted and the client recovers to
    /// the character selection screen.
    pub fn map_entry_failed(&mut self, message: String, now: Instant) {
        self.map_load_error = Some(message);
        self.map_entry_aborted = true;
        self.pending_map_load = None;
        self.transition(SessionEvent::MapEntryFailed);
        self.map_abort_deadline = Some(now + MAP_ABORT_RECOVERY_DELAY);
    }

    /// The map entry watchdog expired.
    pub fn map_entry_watchdog_expired(&mut self) {
        self.transition(SessionEvent::MapEntryWatchdogExpired);
    }

    /// The map server disconnected (or, while recovering, the recovery is
    /// allowed to proceed).
    pub fn map_server_disconnected(&mut self) {
        self.transition(SessionEvent::MapServerDisconnected);
    }

    /// The client returned to the character selection screen.
    pub fn recovery_finished(&mut self) {
        self.transition(SessionEvent::RecoveryFinished);
    }

    /// A clean quit was requested. Returns `true` if this is the first frame of
    /// the shutdown (the caller then starts the disconnect sequence).
    pub fn request_shutdown(&mut self) -> bool {
        if self.state == SessionState::Shutdown {
            return false;
        }

        self.transition(SessionEvent::ShutdownRequested);
        self.shutdown_deadline = None;
        true
    }

    /// Finish the clean shutdown; the client is about to exit.
    pub fn finish_shutdown(&mut self) {
        self.transition(SessionEvent::ShutdownFinished);
    }

    pub fn shutdown_deadline(&self) -> Option<Instant> {
        self.shutdown_deadline
    }

    pub fn set_shutdown_deadline(&mut self, deadline: Option<Instant>) {
        self.shutdown_deadline = deadline;
    }

    /// Map load error that has to be shown at the character selection screen.
    pub fn set_map_load_error(&mut self, message: String) {
        self.map_load_error = Some(message);
    }

    pub fn take_map_load_error(&mut self) -> Option<String> {
        self.map_load_error.take()
    }

    /// Whether a map load error is pending (used to report only the first
    /// failure of a map entry).
    pub fn has_map_load_error(&self) -> bool {
        self.map_load_error.is_some()
    }

    pub fn map_entry_aborted(&self) -> bool {
        self.map_entry_aborted
    }

    /// Schedule character-server reconnects (used after a map-server restart or
    /// a character-server disconnect).
    pub fn schedule_character_reconnect(&mut self, now: Instant) {
        self.character_reconnect_attempts = CHARACTER_RECONNECT_MAX_ATTEMPTS;
        self.character_reconnect_at = Some(now);
    }

    pub fn clear_character_reconnect(&mut self) {
        self.character_reconnect_at = None;
        self.character_reconnect_attempts = 0;
    }

    /// The character server connection completed its login handshake.
    pub fn character_server_connected(&mut self) {
        self.character_server_ready = true;
        self.clear_character_reconnect();
    }

    /// The character server connection was lost (or never authenticated).
    pub fn character_server_disconnected(&mut self) {
        self.character_server_ready = false;
    }

    /// A pending map asset load (used by the tests; the client only reads the
    /// retry actions).
    #[cfg(test)]
    pub fn pending_map_load(&self) -> Option<&PendingMapLoad> {
        self.pending_map_load.as_ref()
    }

    /// Advance all timers by one frame and return the side effects the client
    /// has to execute.
    pub fn tick(&mut self, now: Instant, observation: &SessionObservation) -> Vec<SessionAction> {
        let mut actions = Vec::new();

        if self.state == SessionState::Shutdown {
            return actions;
        }

        self.tick_character_reconnect(now, observation, &mut actions);

        if self.state == SessionState::MapLoading {
            self.tick_map_entry(now, observation, &mut actions);
        }

        if self.state == SessionState::Recovering
            && let Some(deadline) = self.map_abort_deadline
            && now >= deadline
        {
            crate::logging::log_debug!("lifecycle: map abort recovery deadline expired, recovering directly");
            self.map_abort_deadline = None;
            actions.push(SessionAction::RecoverToCharacterSelection);
        }

        actions
    }

    fn tick_character_reconnect(&mut self, now: Instant, observation: &SessionObservation, actions: &mut Vec<SessionAction>) {
        let in_game = observation.map_assets_loaded && observation.player_present;

        // Parity with the old `update_character_server_reconnect`: stop the
        // retry loop as soon as the player is in game again, is back on the
        // login/server-selection screen, or a server connection is live.
        if self.state == SessionState::Shutdown
            || in_game
            || observation.login_surface_visible
            || observation.login_server_connected
            || self.character_server_ready
        {
            if self.character_reconnect_at.take().is_some() {
                crate::logging::log_debug!("lifecycle: character reconnect cancelled");
            }
            return;
        }

        let Some(next_attempt) = self.character_reconnect_at else {
            return;
        };

        if now < next_attempt {
            return;
        }

        if self.character_reconnect_attempts == 0 {
            self.character_reconnect_at = None;
            crate::logging::log_warn!("lifecycle: character server reconnect gave up");
            actions.push(SessionAction::CharacterReconnectGaveUp);
            return;
        }

        self.character_reconnect_attempts -= 1;
        self.character_reconnect_at = Some(now + CHARACTER_RECONNECT_DELAY);
        crate::logging::log_debug!(
            "lifecycle: reconnecting to character server ({} attempts left)",
            self.character_reconnect_attempts
        );
        actions.push(SessionAction::ReconnectCharacterServer);
    }

    fn tick_map_entry(&mut self, now: Instant, observation: &SessionObservation, actions: &mut Vec<SessionAction>) {
        if let Some(deadline) = self.map_entry_watchdog {
            if observation.map_assets_loaded {
                self.map_entry_watchdog = None;
            } else if self.pending_map_load.is_none() && now >= deadline {
                self.map_entry_watchdog = None;
                crate::logging::log_warn!("lifecycle: map entry watchdog expired, aborting the map entry");
                self.map_entry_watchdog_expired();
                actions.push(SessionAction::AbortMapEntry {
                    reason: "The server did not send the map entry data. Please try again.".to_owned(),
                });
            }
        }

        let Some(pending) = self.pending_map_load.as_mut() else {
            return;
        };

        if now.duration_since(pending.requested_at) < MAP_LOAD_RETRY_DELAY {
            return;
        }

        if pending.retries >= MAP_LOAD_MAX_ATTEMPTS {
            let map_name = std::mem::take(&mut pending.map_name);
            self.pending_map_load = None;
            crate::logging::log_warn!(
                "lifecycle: map load of '{}' gave up after {} retries",
                map_name,
                MAP_LOAD_MAX_ATTEMPTS
            );
            actions.push(SessionAction::MapLoadGaveUp { map_name });
            return;
        }

        pending.retries += 1;
        pending.requested_at = now;
        crate::logging::log_debug!(
            "lifecycle: retrying map load of '{}' (retry {}/{})",
            pending.map_name,
            pending.retries,
            MAP_LOAD_MAX_ATTEMPTS
        );
        actions.push(SessionAction::RetryMapLoad {
            map_name: pending.map_name.clone(),
            position: pending.position,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn base_observation() -> SessionObservation {
        SessionObservation {
            map_assets_loaded: false,
            player_present: false,
            login_server_connected: false,
            login_surface_visible: false,
        }
    }

    fn enter_char_select(machine: &mut SessionMachine) {
        assert!(machine.transition(SessionEvent::LoginStarted));
        assert!(machine.transition(SessionEvent::LoginSucceeded));
        assert_eq!(machine.state(), SessionState::CharSelect);
    }

    fn enter_map_loading(machine: &mut SessionMachine, now: Instant) -> Instant {
        enter_char_select(machine);
        machine.begin_map_entry(now);
        assert_eq!(machine.state(), SessionState::MapLoading);
        now
    }

    #[test]
    fn transition_table_contains_only_legal_transitions() {
        for &(from, event, to) in SESSION_TRANSITIONS {
            let mut machine = SessionMachine::new();
            machine.state = from;
            assert!(
                machine.transition(event),
                "legal transition {from:?} --{event:?}--> {to:?} was rejected"
            );
            assert_eq!(machine.state(), to);
            assert_eq!(machine.illegal_transitions(), 0);
        }
    }

    #[test]
    fn every_transition_has_exactly_one_target() {
        let mut seen = HashSet::new();
        for &(from, event, to) in SESSION_TRANSITIONS {
            assert!(
                seen.insert((from, event)),
                "duplicate transition entry for {from:?} --{event:?}--> {to:?}"
            );
        }
    }

    #[test]
    fn illegal_transitions_are_ignored_and_do_not_change_the_state() {
        let all_states = [
            SessionState::Disconnected,
            SessionState::Login,
            SessionState::CharSelect,
            SessionState::MapLoading,
            SessionState::InGame,
            SessionState::Recovering,
            SessionState::Shutdown,
        ];
        let all_events = [
            SessionEvent::LoginStarted,
            SessionEvent::LoginSucceeded,
            SessionEvent::CharacterSelected,
            SessionEvent::MapChangeReceived,
            SessionEvent::MapAssetsLoaded,
            SessionEvent::MapEntryFailed,
            SessionEvent::MapEntryWatchdogExpired,
            SessionEvent::MapServerDisconnected,
            SessionEvent::CharacterServerConnected,
            SessionEvent::RecoveryFinished,
            SessionEvent::ShutdownRequested,
            SessionEvent::ShutdownFinished,
        ];

        let legal: HashSet<_> = SESSION_TRANSITIONS.iter().map(|&(from, event, _)| (from, event)).collect();

        let mut checked_illegal = 0;
        for state in all_states {
            for event in all_events {
                if legal.contains(&(state, event)) {
                    continue;
                }

                let mut machine = SessionMachine::new();
                machine.state = state;
                assert!(
                    !machine.transition(event),
                    "illegal transition {state:?} --{event:?}--> was accepted"
                );
                assert_eq!(machine.state(), state);
                assert_eq!(machine.illegal_transitions(), 1);
                checked_illegal += 1;
            }
        }

        assert_eq!(
            checked_illegal,
            all_states.len() * all_events.len() - SESSION_TRANSITIONS.len(),
            "the transition table must stay exhaustive for the test matrix"
        );
    }

    #[test]
    fn login_flow_reaches_in_game() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        assert_eq!(machine.state(), SessionState::Disconnected);
        assert!(machine.transition(SessionEvent::LoginStarted));
        assert_eq!(machine.state(), SessionState::Login);
        assert!(machine.transition(SessionEvent::LoginSucceeded));
        assert_eq!(machine.state(), SessionState::CharSelect);
        machine.begin_map_entry(now);
        assert_eq!(machine.state(), SessionState::MapLoading);
        machine.map_change_received("prontera".to_owned(), Some(TilePosition::new(156, 191)), now);
        assert_eq!(machine.state(), SessionState::MapLoading);
        machine.map_assets_loaded();
        assert_eq!(machine.state(), SessionState::InGame);
        assert!(machine.pending_map_load().is_none());
    }

    #[test]
    fn map_entry_watchdog_aborts_when_no_map_change_arrives() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);

        // Nothing happens before the deadline.
        assert!(machine.tick(now + MAP_ENTRY_WATCHDOG / 2, &base_observation()).is_empty());
        assert_eq!(machine.state(), SessionState::MapLoading);

        let actions = machine.tick(now + MAP_ENTRY_WATCHDOG, &base_observation());
        assert_eq!(actions, vec![SessionAction::AbortMapEntry {
            reason: "The server did not send the map entry data. Please try again.".to_owned()
        }]);
        assert_eq!(machine.state(), SessionState::Recovering);
        assert!(machine.map_entry_aborted());
    }

    #[test]
    fn map_entry_watchdog_does_not_fire_when_the_map_load_is_in_flight() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_change_received("prontera".to_owned(), Some(TilePosition::new(156, 191)), now);

        let actions = machine.tick(now + MAP_ENTRY_WATCHDOG, &base_observation());
        assert!(
            !actions.iter().any(|action| matches!(action, SessionAction::AbortMapEntry { .. })),
            "pending map load must not trigger the map-entry watchdog"
        );
        assert_eq!(machine.state(), SessionState::MapLoading);
    }

    #[test]
    fn map_load_is_retried_and_gives_up() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_change_received("prontera".to_owned(), Some(TilePosition::new(1, 2)), now);

        let mut current = now;
        for retry in 1..=MAP_LOAD_MAX_ATTEMPTS {
            current += MAP_LOAD_RETRY_DELAY;
            let actions = machine.tick(current, &base_observation());
            assert_eq!(
                actions,
                vec![SessionAction::RetryMapLoad {
                    map_name: "prontera".to_owned(),
                    position: Some(TilePosition::new(1, 2)),
                }],
                "retry {retry} must be requested"
            );
        }

        current += MAP_LOAD_RETRY_DELAY;
        let actions = machine.tick(current, &base_observation());
        assert_eq!(actions, vec![SessionAction::MapLoadGaveUp {
            map_name: "prontera".to_owned()
        }]);
        assert!(machine.pending_map_load().is_none());
    }

    #[test]
    fn map_load_retry_does_not_fire_before_the_delay() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_change_received("prontera".to_owned(), None, now);

        assert!(machine.tick(now + MAP_LOAD_RETRY_DELAY / 2, &base_observation()).is_empty());
    }

    #[test]
    fn failed_map_entry_recovers_when_the_map_disconnect_arrives() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_entry_failed("map data missing".to_owned(), now);
        assert_eq!(machine.state(), SessionState::Recovering);

        machine.map_server_disconnected();
        assert_eq!(machine.state(), SessionState::Recovering);
        machine.recovery_finished();
        assert_eq!(machine.state(), SessionState::CharSelect);
        assert_eq!(machine.take_map_load_error().as_deref(), Some("map data missing"));
        assert!(
            machine.map_entry_aborted(),
            "the flag stays set until the next map entry succeeds"
        );
    }

    #[test]
    fn failed_map_entry_recovers_without_the_map_disconnect_after_the_deadline() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_entry_failed("map data missing".to_owned(), now);

        assert!(machine.tick(now, &base_observation()).is_empty());
        let actions = machine.tick(now + MAP_ABORT_RECOVERY_DELAY, &base_observation());
        assert_eq!(actions, vec![SessionAction::RecoverToCharacterSelection]);
        // The machine stays in `Recovering`; the client calls
        // `recovery_finished` while handling the action.
        assert_eq!(machine.state(), SessionState::Recovering);
        machine.recovery_finished();
        assert_eq!(machine.state(), SessionState::CharSelect);
    }

    #[test]
    fn reaching_in_game_clears_the_map_entry_timers() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_change_received("prontera".to_owned(), None, now);
        machine.map_assets_loaded();
        assert_eq!(machine.state(), SessionState::InGame);
        assert!(machine.tick(now + MAP_ENTRY_WATCHDOG * 4, &base_observation()).is_empty());
    }

    #[test]
    fn warp_goes_back_to_map_loading_and_into_in_game() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_assets_loaded();
        assert_eq!(machine.state(), SessionState::InGame);

        machine.map_change_received("geffen".to_owned(), Some(TilePosition::new(120, 60)), now);
        assert_eq!(machine.state(), SessionState::MapLoading);
        machine.map_assets_loaded();
        assert_eq!(machine.state(), SessionState::InGame);
    }

    #[test]
    fn character_reconnect_backoff_counts_down_and_gives_up() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_char_select(&mut machine);
        machine.map_server_disconnected();
        machine.recovery_finished();
        machine.schedule_character_reconnect(now);
        assert_eq!(machine.character_reconnect_at, Some(now));

        // The character server is still down while the reconnects are running.
        let observation = base_observation();

        let mut current = now;
        for remaining in (0..CHARACTER_RECONNECT_MAX_ATTEMPTS).rev() {
            let actions = machine.tick(current, &observation);
            assert_eq!(actions, vec![SessionAction::ReconnectCharacterServer]);
            assert_eq!(machine.character_reconnect_at, Some(current + CHARACTER_RECONNECT_DELAY));
            assert_eq!(machine.character_reconnect_attempts, remaining);

            current += CHARACTER_RECONNECT_DELAY;
            assert!(machine.tick(current - CHARACTER_RECONNECT_DELAY / 2, &observation).is_empty());
        }

        let actions = machine.tick(current, &observation);
        assert_eq!(actions, vec![SessionAction::CharacterReconnectGaveUp]);
        assert!(machine.character_reconnect_at.is_none());
    }

    #[test]
    fn character_reconnect_stops_when_the_player_is_in_game_again() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_assets_loaded();
        machine.schedule_character_reconnect(now);

        let observation = SessionObservation {
            map_assets_loaded: true,
            player_present: true,
            ..base_observation()
        };

        assert!(machine.tick(now, &observation).is_empty());
        assert!(machine.character_reconnect_at.is_none());
    }

    #[test]
    fn character_server_connection_stops_the_reconnect_loop() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_char_select(&mut machine);
        machine.schedule_character_reconnect(now);
        machine.character_server_connected();
        assert!(machine.character_reconnect_at.is_none());
        assert_eq!(machine.character_reconnect_attempts, 0);
    }

    #[test]
    fn reconnect_keeps_retrying_until_the_character_server_authenticates() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_char_select(&mut machine);
        machine.schedule_character_reconnect(now);

        let observation = base_observation();
        let mut current = now;

        // The socket task exists after the first attempt, but the server did
        // not complete the login handshake yet: keep retrying instead of
        // stopping the loop on the optimistic connection flag.
        for _ in 0..3 {
            assert_eq!(machine.tick(current, &observation), vec![
                SessionAction::ReconnectCharacterServer
            ]);
            current += CHARACTER_RECONNECT_DELAY;
        }

        machine.character_server_connected();
        assert!(machine.tick(current, &observation).is_empty());
        assert!(machine.character_reconnect_at.is_none());
    }

    #[test]
    fn map_server_disconnect_recovers_and_a_new_map_entry_is_possible() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        enter_map_loading(&mut machine, now);
        machine.map_change_received("prontera".to_owned(), None, now);
        machine.map_assets_loaded();
        assert_eq!(machine.state(), SessionState::InGame);

        // The map connection drops (map-server restart / kick).
        machine.map_server_disconnected();
        assert_eq!(machine.state(), SessionState::Recovering);
        machine.recovery_finished();
        assert_eq!(machine.state(), SessionState::CharSelect);

        // The character-server connection is re-established and the player
        // selects the character again.
        machine.schedule_character_reconnect(now);
        assert_eq!(machine.tick(now, &base_observation()), vec![
            SessionAction::ReconnectCharacterServer
        ]);
        machine.character_server_connected();
        assert!(machine.tick(now + CHARACTER_RECONNECT_DELAY, &base_observation()).is_empty());

        machine.begin_map_entry(now + CHARACTER_RECONNECT_DELAY);
        assert_eq!(machine.state(), SessionState::MapLoading);
        machine.map_change_received("prontera".to_owned(), None, now + CHARACTER_RECONNECT_DELAY);
        machine.map_assets_loaded();
        assert_eq!(machine.state(), SessionState::InGame);
    }

    #[test]
    fn shutdown_is_reached_from_every_live_state() {
        let now = Instant::now();

        let states = [
            SessionState::Login,
            SessionState::CharSelect,
            SessionState::MapLoading,
            SessionState::InGame,
            SessionState::Recovering,
        ];

        for state in states {
            let mut machine = SessionMachine::new();
            machine.state = state;
            assert!(machine.request_shutdown(), "shutdown from {state:?} must be accepted");
            assert_eq!(machine.state(), SessionState::Shutdown);
            assert!(
                !machine.request_shutdown(),
                "a repeated shutdown request must not restart the sequence"
            );
            assert!(machine.tick(now + Duration::from_secs(120), &base_observation()).is_empty());
            machine.finish_shutdown();
            assert_eq!(machine.state(), SessionState::Disconnected);
        }
    }

    #[test]
    fn shutdown_deadline_is_owned_by_the_machine() {
        let now = Instant::now();
        let mut machine = SessionMachine::new();
        machine.transition(SessionEvent::LoginStarted);
        machine.request_shutdown();
        assert!(machine.shutdown_deadline().is_none());
        machine.set_shutdown_deadline(Some(now + SHUTDOWN_DISCONNECT_TIMEOUT));
        assert_eq!(machine.shutdown_deadline(), Some(now + SHUTDOWN_DISCONNECT_TIMEOUT));
        machine.finish_shutdown();
        assert_eq!(machine.state(), SessionState::Disconnected);
    }

    #[test]
    fn map_errors_are_consumed_once() {
        let mut machine = SessionMachine::new();
        machine.set_map_load_error("boom".to_owned());
        assert_eq!(machine.take_map_load_error().as_deref(), Some("boom"));
        assert!(machine.take_map_load_error().is_none());
    }
}
