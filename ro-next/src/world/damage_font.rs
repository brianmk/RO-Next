//! Sprite atlas of the official damage numbers.
//!
//! The classic client does not draw damage numbers with a text font: it draws
//! the ten digit frames of `data\sprite\이팩트\숫자.spr` (`숫자.act` maps digit
//! `n` to sprite image `n`) and the miss graphic of
//! `data\sprite\이팩트\miss.spr`. A critical number also gets the
//! `MET_CRITICAL_EFFECT` backing burst of `data\sprite\이팩트\msg.spr` image 3
//! (`msg.act` action 2). This module loads that atlas once into a process-wide
//! [`DamageFont`], which [`super::damage_number::DamageNumber`] then draws with
//! the official per-digit offsets, scale and alpha.
//!
//! The atlas paths, the per-digit image extents, the miss cell pose and the
//! backing pose come from `archive/data/client/damage_numbers.ron` (generated
//! by `tools/gen_damage_numbers.py`), so a server can replace the font without
//! a rebuild. A missing sprite is not fatal: the damage-number object falls
//! back to the interface text renderer, and a missing critical backing only
//! drops the backing quad (its exact asset name is logged).

use std::sync::{Arc, OnceLock};

use crate::graphics::Texture;
use crate::loaders::{ActionLoader, SpriteLoader};

/// The ten digit textures plus the two optional K/M marker images.
const DIGIT_SLOTS: usize = 12;

/// The loaded digit/miss atlas.
pub struct DamageFont {
    digits: Vec<Option<Arc<Texture>>>,
    cell_scale: (f32, f32),
    miss_texture: Option<Arc<Texture>>,
    critical_backing_texture: Option<Arc<Texture>>,
}

impl DamageFont {
    /// Loads the digit and miss sprites from the game archives.
    ///
    /// Returns `None` when the digit sprite is unavailable; the caller then
    /// keeps the text fallback. The miss sprite is optional (it lives in
    /// `data.grf`, which a data set may not ship), so a missing miss graphic
    /// only falls back to the text "miss".
    pub fn load(sprite_loader: &SpriteLoader, action_loader: &ActionLoader) -> Option<Self> {
        let table = crate::world::damage_number::data();

        let sprite = sprite_loader.get_or_load(&format!("{}.spr", table.atlas.sprite), None).ok()?;
        // The action is not needed for the per-digit lookup (the official act
        // maps digit `n` to image `n` one-to-one), but a sprite without its
        // action would be an incomplete font, so it is validated as well.
        action_loader.get_or_load(&format!("{}.act", table.atlas.action)).ok()?;

        let mut digits = vec![None; DIGIT_SLOTS];
        for row in &table.digits {
            let Some(texture) = sprite.textures.get(row.image as usize).cloned() else {
                if row.digit < 10 {
                    crate::logging::log_warn!(
                        "damage font: digit {} image {} missing from {}",
                        row.digit,
                        row.image,
                        table.atlas.sprite
                    );
                    return None;
                }
                continue;
            };
            digits[row.digit as usize] = Some(texture);
        }

        if digits[..10].iter().any(Option::is_none) {
            return None;
        }

        // The miss graphic lives in `data.grf`, which a data set may not ship;
        // require both files of the pair before exposing it.
        let miss_texture = if action_loader.get_or_load(&format!("{}.act", table.miss.action)).is_ok() {
            sprite_loader
                .get_or_load(&format!("{}.spr", table.miss.sprite), None)
                .ok()
                .and_then(|sprite| sprite.textures.get(table.miss.image as usize).cloned())
        } else {
            None
        };
        if miss_texture.is_none() {
            crate::logging::log_debug!(
                "damage font: miss sprite {:?} is not in the archive; the miss number falls back to text",
                table.miss.sprite
            );
        }

        let critical_backing_texture = load_critical_backing(sprite_loader, action_loader);

        Some(Self {
            digits,
            cell_scale: (table.atlas.cell_scale_x, table.atlas.cell_scale_y),
            miss_texture,
            critical_backing_texture,
        })
    }

    /// Number of digit images that loaded (ten, plus markers when present).
    pub fn digit_count(&self) -> usize {
        self.digits.iter().filter(|digit| digit.is_some()).count()
    }

    /// The texture of digit `digit`, if the data set ships it.
    pub fn digit_texture(&self, digit: u8) -> Option<&Arc<Texture>> {
        self.digits.get(usize::from(digit)).and_then(Option::as_ref)
    }

    /// The ACT cell scale shared by every digit (`숫자.act`, 0.3).
    pub fn cell_scale(&self) -> (f32, f32) {
        self.cell_scale
    }

    /// The miss graphic, if `data.grf` (or the archive) ships it.
    pub fn miss_texture(&self) -> Option<&Arc<Texture>> {
        self.miss_texture.as_ref()
    }

    /// The `MET_CRITICAL_EFFECT` backing sprite, if the data set ships
    /// `data\sprite\이팩트\msg.spr` image 3 with `msg.act` action 2.
    pub fn critical_backing_texture(&self) -> Option<&Arc<Texture>> {
        self.critical_backing_texture.as_ref()
    }
}

/// Loads the `MET_CRITICAL_EFFECT` backing texture (`이팩트\msg`, ACT action
/// 2).
///
/// The loaders substitute a fallback asset when a file is missing, but the
/// backing must never be replaced by a different texture: if the data set does
/// not ship it, the official client draws nothing at all
/// (`CActRes::Animation` returns the empty animation for an out-of-range
/// action, `ActionResource.cpp:278-289`). The missing name is logged once here
/// and the critical number keeps drawing only its digits.
fn load_critical_backing(sprite_loader: &SpriteLoader, action_loader: &ActionLoader) -> Option<Arc<Texture>> {
    let table = crate::world::damage_number::data();
    let backing = &table.critical_backing;
    let sprite_name = format!("{}.spr", backing.sprite);
    let action_name = format!("{}.act", backing.action);

    if !sprite_loader.file_exists(&sprite_name) {
        crate::logging::log_warn!(
            "damage font: critical backing sprite data\\sprite\\{} is missing from the archives; critical numbers draw without it",
            sprite_name
        );
        return None;
    }
    if !action_loader.file_exists(&action_name) {
        crate::logging::log_warn!(
            "damage font: critical backing action data\\sprite\\{} is missing from the archives; critical numbers draw without it",
            action_name
        );
        return None;
    }

    let Ok(sprite) = sprite_loader.get_or_load(&sprite_name, None) else {
        crate::logging::log_warn!("damage font: failed to load critical backing sprite data\\sprite\\{sprite_name}");
        return None;
    };
    let Ok(actions) = action_loader.get_or_load(&action_name) else {
        crate::logging::log_warn!("damage font: failed to load critical backing action data\\sprite\\{action_name}");
        return None;
    };

    let Some(action) = actions.actions.get(backing.action_index) else {
        crate::logging::log_warn!(
            "damage font: critical backing action data\\sprite\\{action_name} has no action {}; critical numbers draw without it",
            backing.action_index
        );
        return None;
    };
    let Some(cell) = action.motions.first().and_then(|motion| motion.sprite_clips.first()) else {
        crate::logging::log_warn!(
            "damage font: critical backing action data\\sprite\\{action_name} action {} is empty; critical numbers draw without it",
            backing.action_index
        );
        return None;
    };
    // The data table records the cell pose of the shipped file; a mismatch
    // means the archive is overriding `msg.act` with a different asset, which
    // must not be drawn with the recorded geometry.
    if action.motions.len() != backing.motion_count || cell.sprite_number != backing.image {
        crate::logging::log_warn!(
            "damage font: critical backing data\\sprite\\{action_name} action {} does not match the damage-number data; critical numbers \
             draw without it",
            backing.action_index
        );
        return None;
    }

    let Some(texture) = sprite.textures.get(backing.image as usize).cloned() else {
        crate::logging::log_warn!(
            "damage font: critical backing sprite data\\sprite\\{sprite_name} has no image {}; critical numbers draw without it",
            backing.image
        );
        return None;
    };

    crate::logging::log_info!(
        "damage font: critical backing sprite loaded (data\\sprite\\{sprite_name}, action {} image {})",
        backing.action_index,
        backing.image
    );
    Some(texture)
}

/// Process-wide damage atlas, loaded once during client startup.
static DAMAGE_FONT: OnceLock<Option<Arc<DamageFont>>> = OnceLock::new();

/// Installs the damage atlas (only the first call has an effect).
pub fn install(font: Option<DamageFont>) {
    match &font {
        Some(font) => crate::logging::log_info!(
            "damage font: official sprite digits loaded ({} frames, critical backing {})",
            font.digit_count(),
            if font.critical_backing_texture().is_some() {
                "loaded"
            } else {
                "missing"
            }
        ),
        None => crate::logging::log_warn!("damage font: sprite digits not found, falling back to text damage numbers"),
    }
    let _ = DAMAGE_FONT.set(font.map(Arc::new));
}

/// The installed damage atlas, if the data set ships one.
pub fn damage_font() -> Option<&'static Arc<DamageFont>> {
    DAMAGE_FONT.get().and_then(Option::as_ref)
}

#[cfg(test)]
mod tests {
    use super::DIGIT_SLOTS;

    /// The official atlas has ten digits and room for the two K/M markers.
    #[test]
    fn atlas_has_ten_digits_and_two_marker_slots() {
        assert_eq!(DIGIT_SLOTS, 12);
    }

    /// The loose font folder is mounted by the client archive, so the files
    /// must exist in the repository (this is a plain file system check, no
    /// wgpu device is needed).
    #[test]
    fn font_files_are_installed_in_the_archive() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("archive/data/sprite/이팩트");
        for name in ["숫자.spr", "숫자.act", "숫자_larger.act", "darkbreath.spr", "darkbreath.act"] {
            assert!(
                root.join(name).is_file(),
                "missing damage font file {name} in {}",
                root.display()
            );
        }
    }
}
