//! Status icons above entity heads.
//!
//! Classic Ragnarok Online draws a small icon above the head of an entity for
//! the non-stackable body states (`opt1`: stone/freeze/stun/sleep/burning/...)
//! and the stackable health states (`opt2`: poison/curse/silence/confusion/
//! blind/...). The client already stores both values per entity (`Common::
//! body_state` / `health_state`, updated by `ZC_STATE_CHANGE` 0x0229 and the
//! entity-appear packets) but they were never rendered.
//!
//! The icons are the official status textures (`data\texture\effect\*.tga`),
//! loaded once at startup and drawn with the same camera projection / head
//! height as the overhead chat bubbles and skill name labels.

use std::sync::Arc;

use cgmath::Vector3;
use korangar_interface::application::Clip;

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{ImageType, Scaling, TextureLoader};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Camera, Entity};

/// Icon edge length in unscaled pixels.
const HEAD_MARK_SIZE: f32 = 16.0;
/// Gap between two head mark icons.
const HEAD_MARK_GAP: f32 = 2.0;
/// Height of a character in world units (same anchor as the chat bubbles).
const HEAD_MARK_HEIGHT: f32 = 21.0;
/// Additional height above the chat bubble anchor, so a bubble and a status
/// icon do not overlap.
const HEAD_MARK_OFFSET: f32 = 7.0;
/// Height of a sitting character (matches the other overhead labels).
const HEAD_MARK_SITTING_HEIGHT: f32 = 12.0;

/// One status icon that can be rendered above a head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadMark {
    Stone,
    Freezing,
    Stun,
    Sleep,
    Petrifying,
    Burning,
    Poison,
    Curse,
    Silence,
    Confusion,
}

impl HeadMark {
    /// Client texture path (`data\texture\...`) of the official status icon.
    pub fn texture_path(self) -> &'static str {
        match self {
            HeadMark::Stone | HeadMark::Petrifying => "effect\\BD_StoneCurse.tga",
            HeadMark::Freezing => "effect\\BD_Freezing.tga",
            HeadMark::Stun => "effect\\BD_Stun.tga",
            HeadMark::Sleep => "effect\\BD_Sleep.tga",
            HeadMark::Burning => "effect\\BD_BURNNING.tga",
            HeadMark::Poison => "effect\\HL_Poison.tga",
            HeadMark::Curse => "effect\\HL_Curse.tga",
            HeadMark::Silence => "effect\\HL_Silence.tga",
            HeadMark::Confusion => "effect\\HL_Confusion.tga",
        }
    }

    fn index(self) -> usize {
        match self {
            HeadMark::Stone => 0,
            HeadMark::Freezing => 1,
            HeadMark::Stun => 2,
            HeadMark::Sleep => 3,
            HeadMark::Petrifying => 4,
            HeadMark::Burning => 5,
            HeadMark::Poison => 6,
            HeadMark::Curse => 7,
            HeadMark::Silence => 8,
            HeadMark::Confusion => 9,
        }
    }
}

const HEAD_MARK_COUNT: usize = 10;

/// Decode the rAthena `opt1` (`e_sc_opt1`, status.hpp) body state into a head
/// mark. `opt1` is non-stackable, so there is at most one mark.
pub fn body_state_mark(body_state: u16) -> Option<HeadMark> {
    match body_state {
        1 => Some(HeadMark::Stone),
        2 => Some(HeadMark::Freezing),
        3 => Some(HeadMark::Stun),
        4 => Some(HeadMark::Sleep),
        6 => Some(HeadMark::Petrifying),
        7 => Some(HeadMark::Burning),
        _ => None,
    }
}

/// Decode the rAthena `opt2` (`e_sc_opt2`) health-state bit mask. `opt2` is
/// stackable, so several marks can be shown at once.
pub fn health_state_marks(health_state: u16) -> Vec<HeadMark> {
    // `OPT2_ANGELUS` (0x20) is a buff and is deliberately not shown as a
    // debuff mark; `OPT2_DPOISON` (0x80) reuses the poison icon; `OPT2_BLIND`
    // (0x10) has no official icon in the client data and is skipped as well.
    const MARKS: [(u16, HeadMark); 4] = [
        (0x0001, HeadMark::Poison),
        (0x0002, HeadMark::Curse),
        (0x0004, HeadMark::Silence),
        (0x0008, HeadMark::Confusion),
    ];

    let mut marks = Vec::new();

    for (bit, mark) in MARKS {
        if health_state & bit != 0 {
            marks.push(mark);
        }
    }

    if health_state & 0x0080 != 0 && !marks.contains(&HeadMark::Poison) {
        marks.push(HeadMark::Poison);
    }

    marks
}

/// All head marks of an entity, body state first.
pub fn entity_marks(body_state: u16, health_state: u16) -> Vec<HeadMark> {
    let mut marks = Vec::new();

    if let Some(mark) = body_state_mark(body_state) {
        marks.push(mark);
    }

    marks.extend(health_state_marks(health_state));

    marks
}

/// The loaded head mark textures. Missing textures fall back to nothing (the
/// official client also simply does not draw an icon when the file is absent).
pub struct HeadMarks {
    textures: [Option<Arc<Texture>>; HEAD_MARK_COUNT],
}

impl HeadMarks {
    pub fn new(texture_loader: &TextureLoader) -> Self {
        let textures = std::array::from_fn(|index| {
            let mark = match index {
                0 => HeadMark::Stone,
                1 => HeadMark::Freezing,
                2 => HeadMark::Stun,
                3 => HeadMark::Sleep,
                4 => HeadMark::Petrifying,
                5 => HeadMark::Burning,
                6 => HeadMark::Poison,
                7 => HeadMark::Curse,
                8 => HeadMark::Silence,
                _ => HeadMark::Confusion,
            };
            let path = mark.texture_path();

            match texture_loader.get_or_load(path, ImageType::Color) {
                Ok(texture) => Some(texture),
                Err(_) => {
                    crate::logging::log_debug!("head marks: status icon '{}' is missing", path);
                    None
                }
            }
        });

        Self { textures }
    }

    fn texture(&self, mark: HeadMark) -> Option<&Arc<Texture>> {
        self.textures[mark.index()].as_ref()
    }

    /// Render the status icons above the heads of all entities in view.
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        entities: &[Entity],
        screen_size: ScreenSize,
        scaling: Scaling,
    ) {
        let scaling_factor = scaling.get_factor();
        let icon_size = HEAD_MARK_SIZE * scaling_factor;
        let icon_gap = HEAD_MARK_GAP * scaling_factor;

        for entity in entities {
            let body_state = entity.get_body_state();
            let health_state = entity.get_health_state();

            if body_state == 0 && health_state == 0 {
                continue;
            }

            let marks = entity_marks(body_state, health_state);

            if marks.is_empty() {
                continue;
            }

            let head_height = match entity.is_sitting() {
                true => HEAD_MARK_SITTING_HEIGHT,
                false => HEAD_MARK_HEIGHT,
            } + HEAD_MARK_OFFSET;
            let head_position = entity.get_position() + Vector3::new(0.0, head_height * entity.get_scale(), 0.0);
            let clip_space_position = camera.view_projection_matrix() * head_position.to_homogeneous();
            let screen_position = camera.clip_to_screen_space(clip_space_position);

            // Skip icons of entities that are off screen.
            if !(0.0..=1.0).contains(&screen_position.x) || !(0.0..=1.0).contains(&screen_position.y) {
                continue;
            }

            let center = ScreenPosition {
                left: screen_position.x * screen_size.width,
                top: screen_position.y * screen_size.height,
            };
            let total_width = marks.len() as f32 * icon_size + (marks.len().saturating_sub(1)) as f32 * icon_gap;
            let mut left = center.left - total_width / 2.0;

            for mark in marks {
                let Some(texture) = self.texture(mark) else {
                    left += icon_size + icon_gap;
                    continue;
                };

                let position = ScreenPosition {
                    left,
                    top: center.top - icon_size,
                };

                renderer.render_sprite(
                    texture.clone(),
                    position,
                    ScreenSize {
                        width: icon_size,
                        height: icon_size,
                    },
                    ScreenClip::unbound(),
                    Color::WHITE,
                    false,
                );

                left += icon_size + icon_gap;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HeadMark, body_state_mark, entity_marks, health_state_marks};

    #[test]
    fn opt1_body_states_map_to_the_classic_head_marks() {
        assert_eq!(body_state_mark(0), None);
        assert_eq!(body_state_mark(1), Some(HeadMark::Stone));
        assert_eq!(body_state_mark(2), Some(HeadMark::Freezing));
        assert_eq!(body_state_mark(3), Some(HeadMark::Stun));
        assert_eq!(body_state_mark(4), Some(HeadMark::Sleep));
        assert_eq!(body_state_mark(6), Some(HeadMark::Petrifying));
        assert_eq!(body_state_mark(7), Some(HeadMark::Burning));
        // Unknown/undead states are not rendered.
        assert_eq!(body_state_mark(5), None);
        assert_eq!(body_state_mark(8), None);
    }

    #[test]
    fn opt2_health_states_are_stackable() {
        assert!(health_state_marks(0).is_empty());
        assert_eq!(health_state_marks(0x0001), [HeadMark::Poison]);
        assert_eq!(health_state_marks(0x0004), [HeadMark::Silence]);

        let marks = health_state_marks(0x0002 | 0x0008 | 0x0010);
        assert_eq!(marks, [HeadMark::Curse, HeadMark::Confusion]);

        // Angelus is a buff and is not shown.
        assert!(health_state_marks(0x0020).is_empty());
    }

    #[test]
    fn the_body_state_comes_before_the_health_states() {
        let marks = entity_marks(3, 0x0002);

        assert_eq!(marks, [HeadMark::Stun, HeadMark::Curse]);
    }

    #[test]
    fn dead_poison_does_not_duplicate_the_poison_mark() {
        assert_eq!(health_state_marks(0x0001 | 0x0080), [HeadMark::Poison]);
        assert_eq!(health_state_marks(0x0080), [HeadMark::Poison]);
    }

    #[test]
    fn every_head_mark_has_a_unique_index() {
        let marks = [
            HeadMark::Stone,
            HeadMark::Freezing,
            HeadMark::Stun,
            HeadMark::Sleep,
            HeadMark::Petrifying,
            HeadMark::Burning,
            HeadMark::Poison,
            HeadMark::Curse,
            HeadMark::Silence,
            HeadMark::Confusion,
        ];
        let mut indices: Vec<_> = marks.iter().map(|mark| mark.index()).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), marks.len());
    }
}
