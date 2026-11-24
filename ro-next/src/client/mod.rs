//! Client subsystems split out of `lib.rs`.
//!
//! `lib.rs` keeps the `Client` struct, its lifecycle (`init`/`run`) and the two
//! event dispatchers. Everything that is a self-contained helper lives here as
//! an `impl Client` block in its own module.

pub(crate) mod chat;
pub(crate) mod effects;
pub(crate) mod frame;
pub(crate) mod hud;
pub(crate) mod items;
pub(crate) mod navigation;
pub(crate) mod network_events;
pub(crate) mod render;
pub(crate) mod user_events;
