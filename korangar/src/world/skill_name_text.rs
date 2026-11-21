//! Floating skill name labels above the caster's head.
//!
//! Classic Ragnarok Online shows the name of a skill above the caster when a
//! skill is used (`Blessing !!`, `Bowling Bash !!`). This module renders those
//! labels with the same world-text path and anchor height as the overhead chat
//! bubbles, but as a short-lived floating label with a fade-out.
//!
//! Every label is created once per cast (not per animation frame), refreshed
//! instead of stacked when the same entity casts again, and the global count is
//! capped so a plaza full of ambient population shells casting cannot produce
//! unbounded work.

use std::sync::LazyLock;

use cgmath::Vector3;
use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::{Color, ScreenPosition, ScreenSize};
use crate::loaders::{FontSize, Scaling};
use crate::renderer::{AlignHorizontal, GameInterfaceRenderer};
use crate::world::{Camera, Entity};

/// How long a skill name stays on screen.
pub const SKILL_NAME_LIFETIME_MS: u32 = 1_300;
/// Time at the end of the lifetime during which the label fades out.
const SKILL_NAME_FADE_OUT_MS: u32 = 450;
/// Global cap on concurrent skill name labels.
const MAX_SKILL_NAME_LABELS: usize = 24;
/// Font size in unscaled pixels; the UI scaling factor is applied by the
/// renderer.
const SKILL_NAME_FONT_SIZE: f32 = 14.0;
/// Text color of a skill name (warm white, like the official client).
const SKILL_NAME_COLOR: Color = Color::rgba(1.0, 0.95, 0.82, 1.0);
/// Outline color (keeps the label readable on bright terrain in daylight).
const SKILL_NAME_OUTLINE_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.9);
/// Height of a character in world units, measured from the entity's feet to
/// just above the head. Matches the overhead chat bubble anchor: character
/// sprites are roughly 1.7 tiles tall and one tile is 10 world units, so 21
/// units puts the text above the head instead of on the torso.
const SKILL_NAME_HEAD_HEIGHT: f32 = 21.0;
/// Height of a sitting character in world units (the sit pose is much lower).
const SKILL_NAME_SITTING_HEAD_HEIGHT: f32 = 12.0;

struct SkillNameLabel {
    entity_id: EntityId,
    text: String,
    created_at: ClientTick,
}

/// All currently visible skill name labels.
#[derive(Default)]
pub struct SkillNameTexts {
    labels: Vec<SkillNameLabel>,
}

impl SkillNameTexts {
    /// Shows `text` above the entity that cast a skill.
    ///
    /// A new cast of the same entity replaces its previous label (the official
    /// client never stacks skill names), and the global label list is bounded.
    pub fn push(&mut self, entity_id: EntityId, text: String, client_tick: ClientTick) {
        if text.is_empty() {
            return;
        }

        self.retain_alive(client_tick);
        self.labels.retain(|label| label.entity_id != entity_id);

        // If the cap is reached, drop the oldest label (the newest cast is the
        // one the player is looking at).
        while self.labels.len() >= MAX_SKILL_NAME_LABELS {
            self.labels.remove(0);
        }

        self.labels.push(SkillNameLabel {
            entity_id,
            text,
            created_at: client_tick,
        });
    }

    /// Remove all labels (e.g. on a map change, where every entity is reset).
    pub fn clear(&mut self) {
        self.labels.clear();
    }

    /// Number of labels currently alive (diagnostics/testing).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Remove the label of a single entity (e.g. when it disappears).
    pub fn remove_entity(&mut self, entity_id: EntityId) {
        self.labels.retain(|label| label.entity_id != entity_id);
    }

    /// Drop labels whose lifetime has expired.
    pub fn retain_alive(&mut self, client_tick: ClientTick) {
        self.labels
            .retain(|label| label_age(client_tick, label.created_at) < SKILL_NAME_LIFETIME_MS);
    }

    /// Render the labels above the heads of the casting player characters.
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        entities: &[Entity],
        screen_size: ScreenSize,
        scaling: Scaling,
        client_tick: ClientTick,
    ) {
        let scaling_factor = scaling.get_factor();

        for label in &self.labels {
            let age = label_age(client_tick, label.created_at);

            if age >= SKILL_NAME_LIFETIME_MS {
                continue;
            }

            let alpha = match age + SKILL_NAME_FADE_OUT_MS >= SKILL_NAME_LIFETIME_MS {
                true => (SKILL_NAME_LIFETIME_MS - age) as f32 / SKILL_NAME_FADE_OUT_MS as f32,
                false => 1.0,
            };

            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == label.entity_id) else {
                continue;
            };

            // Only characters get the label (same rule as overhead chat bubbles):
            // NPC/monster skill use is not shown, so plaza population shells and
            // other players work but ambient monsters do not spam the screen.
            if !is_skill_name_entity(entity) {
                continue;
            }

            let head_height = match entity.is_sitting() {
                true => SKILL_NAME_SITTING_HEAD_HEIGHT,
                false => SKILL_NAME_HEAD_HEIGHT,
            };
            let head_position = entity.get_position() + Vector3::new(0.0, head_height * entity.get_scale(), 0.0);
            let clip_space_position = camera.view_projection_matrix() * head_position.to_homogeneous();
            let screen_position = camera.clip_to_screen_space(clip_space_position);

            if !(0.0..=1.0).contains(&screen_position.x) || !(0.0..=1.0).contains(&screen_position.y) {
                continue;
            }

            let text_size = renderer.measure_text(&label.text, FontSize(SKILL_NAME_FONT_SIZE));

            let center = ScreenPosition {
                left: screen_position.x * screen_size.width,
                top: screen_position.y * screen_size.height,
            };
            let text_position = ScreenPosition {
                left: center.left - text_size.width / 2.0,
                top: center.top - text_size.height - 2.0 * scaling_factor,
            };

            let color = Color {
                red: SKILL_NAME_COLOR.red,
                green: SKILL_NAME_COLOR.green,
                blue: SKILL_NAME_COLOR.blue,
                alpha: SKILL_NAME_COLOR.alpha * alpha,
            };
            let outline_color = Color {
                alpha: SKILL_NAME_OUTLINE_COLOR.alpha * alpha,
                ..SKILL_NAME_OUTLINE_COLOR
            };

            // Outlined text only (no dark box): matches the official client and does
            // not cover the world the way the chat subtitle background does.
            renderer.render_outlined_text(
                &label.text,
                text_position,
                color,
                outline_color,
                FontSize(SKILL_NAME_FONT_SIZE),
                AlignHorizontal::Left,
            );
        }
    }
}

fn label_age(client_tick: ClientTick, created_at: ClientTick) -> u32 {
    client_tick.0.wrapping_sub(created_at.0)
}

/// Whether an entity shows overhead skill names: player characters, which
/// includes the local player and the population engine shells.
pub fn is_skill_name_entity(entity: &Entity) -> bool {
    entity.get_entity_type() == crate::world::EntityType::Player
}

/// Headless test override: `KORANGAR_SKILL_NAMES=0|1` disables/enables the
/// overhead skill name labels without touching (and persisting) the interface
/// settings.
pub fn skill_names_enabled_by_environment() -> bool {
    static ENABLED: LazyLock<bool> = LazyLock::new(|| match std::env::var("KORANGAR_SKILL_NAMES") {
        Ok(value) => value != "0",
        Err(_) => true,
    });

    *ENABLED
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, EntityId};

    use super::{MAX_SKILL_NAME_LABELS, SKILL_NAME_LIFETIME_MS, SkillNameTexts};

    #[test]
    fn labels_expire_after_their_lifetime() {
        let mut labels = SkillNameTexts::default();
        labels.push(EntityId(1), "Blessing !!".to_owned(), ClientTick(100));

        labels.retain_alive(ClientTick(100 + SKILL_NAME_LIFETIME_MS - 1));
        assert_eq!(labels.labels.len(), 1);

        labels.retain_alive(ClientTick(100 + SKILL_NAME_LIFETIME_MS));
        assert!(labels.labels.is_empty());
    }

    #[test]
    fn a_new_cast_replaces_the_label_of_the_same_entity() {
        let mut labels = SkillNameTexts::default();
        labels.push(EntityId(1), "Blessing !!".to_owned(), ClientTick(0));
        labels.push(EntityId(1), "Bowling Bash !!".to_owned(), ClientTick(10));

        assert_eq!(labels.labels.len(), 1);
        assert_eq!(labels.labels[0].text, "Bowling Bash !!");
    }

    #[test]
    fn the_global_label_count_is_capped() {
        let mut labels = SkillNameTexts::default();

        for index in 0..MAX_SKILL_NAME_LABELS + 10 {
            labels.push(EntityId(index as u32 + 1), "Blessing !!".to_owned(), ClientTick(0));
        }

        assert_eq!(labels.labels.len(), MAX_SKILL_NAME_LABELS);
        // The oldest labels were dropped first.
        assert_eq!(labels.labels[0].entity_id, EntityId(11));
    }

    #[test]
    fn removing_an_entity_drops_its_label() {
        let mut labels = SkillNameTexts::default();
        labels.push(EntityId(1), "Blessing !!".to_owned(), ClientTick(0));
        labels.push(EntityId(2), "Heal !!".to_owned(), ClientTick(0));

        labels.remove_entity(EntityId(1));
        assert_eq!(labels.labels.len(), 1);
        assert_eq!(labels.labels[0].entity_id, EntityId(2));
    }
}
