//! Explicit lifecycle state machines (P1-3).
//!
//! Before this module the client encoded its login/map-entry/reload/shutdown
//! sequencing in a set of loose `Option<Instant>` fields on [`crate::Client`]
//! (`map_entry_watchdog`, `map_entry_abort_deadline`, `shutdown_deadline`,
//! `character_server_reconnect_at`, `pending_map_load`, …). Those timers
//! overlapped, were impossible to unit test and every new failure mode added
//! another flag.
//!
//! This module replaces them with three explicit, side-effect free machines:
//!
//! * [`SessionMachine`] — the connection/session lifecycle (`Disconnected` →
//!   `Login` → `CharSelect` → `MapLoading` → `InGame` → `Recovering` →
//!   `Shutdown`) including map-entry/abort deadlines, map load retries,
//!   character-server reconnect backoff and the clean-shutdown timer.
//! * [`ResourceState`] — one lazily loaded resource (entity details, animation
//!   data, map data, …) with a request deadline, bounded retries and give-up
//!   logging. Replaces the never-retried `ResourceState::Requested` variant in
//!   `world::entity`.
//! * [`EntityLifecycle`] — per-entity spawn/despawn/respawn/job-change state,
//!   so duplicate `AddEntity` packets (job change, respawn) and warp cleanup
//!   are handled explicitly instead of with ad-hoc `Vec::retain` calls.
//!
//! The machines never touch the client state directly. The client drives them
//! and executes the actions they return, which keeps the transition tables
//! unit-testable without a window, a network or a game server.

mod entity;
mod resource;
mod session;

pub use self::entity::{AddEntityDecision, EntityLifecycle, RemoveEntityOutcome};
pub use self::resource::{ResourcePolicy, ResourceRetryQueue, ResourceState};
pub use self::session::{SHUTDOWN_DISCONNECT_TIMEOUT, SessionAction, SessionEvent, SessionMachine, SessionObservation, SessionState};
