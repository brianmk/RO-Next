//! Diagnostics for recoverable asset-load failures.
//!
//! Asset failures are not fatal in most cases: a missing sprite falls back to
//! the placeholder sprite, a missing item icon leaves a blank slot, a missing
//! animation uses the fallback animation. They must still be visible in a
//! release client, because "the client silently shows placeholders" is exactly
//! the kind of bug that only gets reported weeks later.
//!
//! This module provides
//!
//! * [`asset_failure_log_key`], which groups repeated failures of the same
//!   resource kind (a hundred entities without a weapon animation produce one
//!   warning, not a hundred), and
//! * [`TransientAssetNoticeLimiter`] plus [`transient_asset_failure_message`],
//!   which drive the non-modal in-game notice (a chat line) for transient
//!   failures.

use crate::loaders::LoaderId;
use crate::loaders::error::LoadError;

/// Maximum number of non-modal notices shown per client session.
///
/// The warn log has no such limit (it is deduplicated in
/// [`crate::logging::warn_once`]); the chat notice is capped because it is
/// disruptive and almost always the same root cause.
pub const MAX_TRANSIENT_ASSET_NOTICES: usize = 3;

/// Key used to deduplicate warn logs for a failed load.
///
/// Entity-specific loads (animations of a specific entity) are grouped by
/// resource kind: a whole crowd of characters with the same missing animation
/// should not produce one log line per character. Item and skill loads keep the
/// numeric id, which is the actionable part for the game-file maintainer.
pub fn asset_failure_log_key(id: &LoaderId) -> String {
    match id {
        LoaderId::AnimationData(_) => "animation-data".to_owned(),
        LoaderId::CharacterPreview(_) => "character-preview".to_owned(),
        LoaderId::FalconAnimationData(_) => "falcon-animation-data".to_owned(),
        LoaderId::CartAnimationData(_) => "cart-animation-data".to_owned(),
        LoaderId::WeaponAnimationData(_) => "weapon-animation-data".to_owned(),
        LoaderId::ItemSprite(item_id) => format!("item-sprite:{}", item_id.0),
        LoaderId::SkillSprite(skill_id) => format!("skill-sprite:{}", skill_id.0),
        LoaderId::SkillActions(skill_id) => format!("skill-actions:{}", skill_id.0),
        LoaderId::Map(map_name) => format!("map:{map_name}"),
    }
}

/// Whether a failed load needs the player's attention (fatal for the current
/// screen) or is a transient, non-modal problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetFailureSeverity {
    /// The current screen cannot continue (e.g. the map itself failed).
    Fatal,
    /// The client keeps working with a placeholder or missing decoration.
    Transient,
}

/// Classify a failed load.
pub fn asset_failure_severity(id: &LoaderId) -> AssetFailureSeverity {
    match id {
        LoaderId::Map(_) => AssetFailureSeverity::Fatal,
        _ => AssetFailureSeverity::Transient,
    }
}

/// Non-modal, user-readable message for a transient asset failure.
///
/// The full error is logged (see [`asset_failure_log_key`]); the chat message
/// only names the affected feature so the player knows what looks wrong.
pub fn transient_asset_failure_message(id: &LoaderId, _error: &LoadError) -> String {
    let what = match id {
        LoaderId::AnimationData(_)
        | LoaderId::FalconAnimationData(_)
        | LoaderId::CartAnimationData(_)
        | LoaderId::WeaponAnimationData(_) => "Some character animations",
        LoaderId::CharacterPreview(_) => "The character preview",
        LoaderId::ItemSprite(_) => "Some item icons",
        LoaderId::SkillSprite(_) | LoaderId::SkillActions(_) => "Some skill icons",
        // Maps are fatal and reported through the error window instead.
        LoaderId::Map(_) => "Map data",
    };

    format!("{what} could not be loaded; affected objects show a placeholder. See the client log for details.")
}

/// Rate limiter for the non-modal transient-asset notices.
#[derive(Debug, Default)]
pub struct TransientAssetNoticeLimiter {
    emitted: usize,
}

impl TransientAssetNoticeLimiter {
    /// Returns `true` if another notice should be shown.
    pub fn should_notify(&mut self) -> bool {
        if self.emitted >= MAX_TRANSIENT_ASSET_NOTICES {
            return false;
        }

        self.emitted += 1;
        true
    }

    /// Number of notices shown so far.
    #[cfg(test)]
    pub fn emitted(&self) -> usize {
        self.emitted
    }
}

#[cfg(test)]
mod tests {
    use korangar_loaders::FileNotFoundError;
    use ragnarok_packets::{EntityId, ItemId, SkillId};

    use super::{
        AssetFailureSeverity, MAX_TRANSIENT_ASSET_NOTICES, TransientAssetNoticeLimiter, asset_failure_log_key, asset_failure_severity,
        transient_asset_failure_message,
    };
    use crate::loaders::LoaderId;
    use crate::loaders::error::LoadError;

    fn error() -> LoadError {
        LoadError::File(FileNotFoundError::new("data\\sprite\\missing.spr".to_owned()))
    }

    #[test]
    fn entity_animation_failures_are_grouped_by_kind() {
        let first = asset_failure_log_key(&LoaderId::AnimationData(EntityId(1)));
        let second = asset_failure_log_key(&LoaderId::AnimationData(EntityId(99)));

        assert_eq!(first, second);
        assert_eq!(first, "animation-data");
    }

    #[test]
    fn item_and_skill_failures_keep_the_actionable_id() {
        assert_eq!(asset_failure_log_key(&LoaderId::ItemSprite(ItemId(501))), "item-sprite:501");
        assert_eq!(asset_failure_log_key(&LoaderId::SkillSprite(SkillId(28))), "skill-sprite:28");
        assert_eq!(asset_failure_log_key(&LoaderId::Map("prontera".to_owned())), "map:prontera");
    }

    #[test]
    fn map_failures_are_fatal_and_everything_else_is_transient() {
        assert_eq!(
            asset_failure_severity(&LoaderId::Map("prontera".to_owned())),
            AssetFailureSeverity::Fatal
        );
        assert_eq!(
            asset_failure_severity(&LoaderId::ItemSprite(ItemId(501))),
            AssetFailureSeverity::Transient
        );
        assert_eq!(
            asset_failure_severity(&LoaderId::AnimationData(EntityId(1))),
            AssetFailureSeverity::Transient
        );
    }

    #[test]
    fn transient_messages_are_non_modal_and_kind_specific() {
        let item_message = transient_asset_failure_message(&LoaderId::ItemSprite(ItemId(501)), &error());
        assert!(item_message.contains("item icons"), "{item_message}");
        assert!(item_message.contains("placeholder"), "{item_message}");

        let animation_message = transient_asset_failure_message(&LoaderId::AnimationData(EntityId(1)), &error());
        assert!(animation_message.contains("character animations"), "{animation_message}");
    }

    #[test]
    fn transient_notices_are_capped() {
        let mut limiter = TransientAssetNoticeLimiter::default();

        for _ in 0..MAX_TRANSIENT_ASSET_NOTICES {
            assert!(limiter.should_notify());
        }

        assert!(!limiter.should_notify());
        assert!(!limiter.should_notify());
        assert_eq!(limiter.emitted(), MAX_TRANSIENT_ASSET_NOTICES);
    }
}
