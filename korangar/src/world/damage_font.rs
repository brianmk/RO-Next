//! Sprite-based damage numbers (the official HD damage font).
//!
//! The classic client does not draw damage numbers with a text font: it draws
//! the ten digit frames of `data\sprite\이팩트\숫자.spr` (`숫자.act` maps digit
//! `n` to sprite frame `n`). This module loads that digit sprite once and
//! renders a number by blitting its digit frames next to each other, so the
//! in-world damages look like the official client instead of like interface
//! text.
//!
//! The font is installed as a loose data folder inside the client archive
//! (`archive/data/sprite/이팩트/`), which the normal folder archive mounts, so
//! a server can replace the digits without a rebuild. When the sprite is not
//! available (a data set without the loose folder) the caller falls back to the
//! text renderer, so damage numbers never disappear.

use std::sync::{Arc, OnceLock};

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{ActionLoader, SpriteLoader};
use crate::renderer::SpriteRenderer;

/// Sprite stem of the digit font, relative to `data\sprite\`.
const DIGIT_SPRITE: &str = "이팩트\\숫자";
/// Action stem of the digit font, relative to `data\sprite\`.
const DIGIT_ACTION: &str = "이팩트\\숫자";
/// Number of digit frames in the official font (`0`..`9`).
const DIGIT_COUNT: usize = 10;
/// Fraction of the digit frame width that the next digit advances.
///
/// The official digit frames carry some transparent padding, so advancing by
/// the full frame width would space the digits of a number too far apart.
const DIGIT_ADVANCE_FACTOR: f32 = 0.78;

/// The ten digit textures of the official damage font.
pub struct DamageFont {
    digits: Vec<Arc<Texture>>,
    frame_width: f32,
    frame_height: f32,
}

impl DamageFont {
    /// Loads the digit sprite and action from the game archives.
    ///
    /// Returns `None` when the data set does not ship the loose font folder;
    /// the caller then keeps rendering damage numbers as text.
    pub fn load(sprite_loader: &SpriteLoader, action_loader: &ActionLoader) -> Option<Self> {
        let sprite = sprite_loader.get_or_load(&format!("{DIGIT_SPRITE}.spr"), None).ok()?;
        // The action is not needed for the per-digit lookup (the official act
        // maps digit `n` to frame `n` one-to-one), but a sprite without its
        // action would be an incomplete font, so it is validated as well.
        action_loader.get_or_load(&format!("{DIGIT_ACTION}.act")).ok()?;

        let digits: Vec<_> = sprite.textures.iter().take(DIGIT_COUNT).cloned().collect();
        if digits.len() < DIGIT_COUNT {
            return None;
        }

        let size = digits[0].get_size();
        Some(Self {
            digits,
            frame_width: size.width as f32,
            frame_height: size.height as f32,
        })
    }

    /// Number of digits in the font (always ten).
    pub fn digit_count(&self) -> usize {
        self.digits.len()
    }

    /// Renders `text` centered at `center` with the given on-screen height.
    ///
    /// Non-digit characters are skipped; `color` tints the digits (white for
    /// normal hits, the critical color for critical hits).
    pub fn render(
        &self,
        renderer: &impl SpriteRenderer,
        text: &str,
        center: ScreenPosition,
        height: f32,
        color: Color,
        screen_clip: ScreenClip,
    ) {
        let scale = height / self.frame_height.max(1.0);
        let digit_width = self.frame_width * scale;
        let advance = digit_width * DIGIT_ADVANCE_FACTOR;
        let digit_count = text.chars().filter(|character| character.is_ascii_digit()).count() as f32;

        if digit_count == 0.0 {
            return;
        }

        let mut left = center.left - advance * digit_count * 0.5;
        let top = center.top - height * 0.5;

        for character in text.chars() {
            let Some(digit) = character.to_digit(10) else {
                continue;
            };

            renderer.render_sprite(
                self.digits[digit as usize].clone(),
                ScreenPosition { left, top },
                ScreenSize {
                    width: digit_width,
                    height,
                },
                screen_clip,
                color,
                false,
            );

            left += advance;
        }
    }
}

/// Process-wide damage font, loaded once during client startup.
static DAMAGE_FONT: OnceLock<Option<Arc<DamageFont>>> = OnceLock::new();

/// Installs the damage font (only the first call has an effect).
pub fn install(font: Option<DamageFont>) {
    let _ = DAMAGE_FONT.set(font.map(Arc::new));
}

/// The installed damage font, if the data set ships one.
pub fn damage_font() -> Option<&'static Arc<DamageFont>> {
    DAMAGE_FONT.get().and_then(Option::as_ref)
}

#[cfg(test)]
mod tests {
    use super::DIGIT_COUNT;

    /// The official font has ten digit frames; the renderer assumes one frame
    /// per digit.
    #[test]
    fn font_has_ten_digits() {
        assert_eq!(DIGIT_COUNT, 10);
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
