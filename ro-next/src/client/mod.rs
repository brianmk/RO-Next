//! The `Client` behaviour that is not part of its lifecycle.
//!
//! `lib.rs` keeps the `Client` struct and `init`/`run`; this module holds the
//! event dispatchers (`network/`, `user_events.rs` + `input_*.rs`) and the
//! self-contained helpers (`effects`, `render`, `frame`, `items`, `hud`,
//! `navigation`, `chat`), each as its own `impl Client` file.

pub(crate) mod chat;
pub(crate) mod effects;
pub(crate) mod frame;
pub(crate) mod hud;
pub(crate) mod items;
pub(crate) mod navigation;
pub(crate) mod network;
pub(crate) mod render;
pub(crate) mod user_events;

pub(crate) mod input_character;
pub(crate) mod input_chat;
pub(crate) mod input_friends;
pub(crate) mod input_guild;
pub(crate) mod input_inventory;
pub(crate) mod input_mail;
pub(crate) mod input_map;
pub(crate) mod input_party;
pub(crate) mod input_session;
pub(crate) mod input_skills;
pub(crate) mod input_trade;
pub(crate) mod input_ui;
pub(crate) mod input_windows;
