//! Persistent character-id → friend-name cache.
//!
//! Since PACKETVER >= 20180307 the 2019 server sends the friend list
//! (`ZC_FRIENDS_LIST`, 0x0201) without names, and `ZC_FRIENDS_STATE` (0x0206)
//! only carries names for friends that are currently **online**. Offline
//! friends would therefore show up blank forever. The names we have seen are
//! cached in `client/friend_names.ron` so later logins can still show them; an
//! unknown friend is rendered as `Unknown (#<char_id>)`, never as a blank row.

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

use ragnarok_packets::Friend;

const CACHE_PATH: &str = "client/friend_names.ron";

static CACHE: OnceLock<Mutex<HashMap<u32, String>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<u32, String>> {
    CACHE.get_or_init(|| Mutex::new(load()))
}

fn load() -> HashMap<u32, String> {
    fs::read_to_string(CACHE_PATH)
        .ok()
        .and_then(|contents| ron::from_str::<HashMap<u32, String>>(&contents).ok())
        .unwrap_or_default()
}

fn save(cache: &HashMap<u32, String>) {
    if let Ok(contents) = ron::ser::to_string_pretty(cache, ron::ser::PrettyConfig::default()) {
        let _ = fs::write(CACHE_PATH, contents);
    }
}

/// Returns the cached name of a friend character id, if it was seen before.
pub fn cached_name(character_id: u32) -> Option<String> {
    cache().lock().ok().and_then(|cache| cache.get(&character_id).cloned())
}

/// Remembers a friend name (no-op for empty names). Only rewrites the cache
/// file when the name actually changed.
pub fn remember(character_id: u32, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let Ok(mut cache) = cache().lock() else {
        return;
    };
    if cache.get(&character_id).is_some_and(|existing| existing == name) {
        return;
    }
    cache.insert(character_id, name.to_string());
    save(&cache);
}

/// Display name for a friend: the packet name if present, otherwise the cached
/// name, otherwise an explicit placeholder (never blank).
pub fn display_name(friend: &Friend) -> String {
    if !friend.name.trim().is_empty() {
        return friend.name.clone();
    }
    cached_name(friend.character_id.0).unwrap_or_else(|| format!("Unknown (#{})", friend.character_id.0))
}
