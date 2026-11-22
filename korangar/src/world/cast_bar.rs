//! Cast progress bar above the local player's head.
//!
//! The original client shows a small progress bar with the skill name while a
//! skill with a cast time is being cast. This module renders that bar in screen
//! space above the caster: outlined skill name plus remaining/total seconds,
//! with a filled progress bar below it.
//!
//! Only the local player gets a cast bar (population shells cast constantly and
//! must not fill the screen); the bar is driven by server-confirmed cast start
//! packets (`ZC_USESKILL_ACK`, cast time > 0), so a rejected cast never shows
//! one.

use cgmath::Vector3;
use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::{Color, ScreenPosition, ScreenSize};
use crate::loaders::{FontSize, Scaling};
use crate::renderer::{AlignHorizontal, GameInterfaceRenderer};
use crate::world::{Camera, Entity};

/// Font size of the skill name line, in unscaled pixels.
const CAST_BAR_FONT_SIZE: f32 = 13.0;
/// Height of the progress bar, in unscaled pixels.
const CAST_BAR_HEIGHT: f32 = 8.0;
/// Minimum width of the progress bar, in unscaled pixels.
const CAST_BAR_MINIMUM_WIDTH: f32 = 100.0;
/// Vertical gap between the skill name text and the progress bar.
const CAST_BAR_GAP: f32 = 2.0;
/// Head anchor height in world units (same anchor as the overhead chat text).
const CAST_BAR_HEAD_HEIGHT: f32 = 21.0;
/// Head anchor height of a sitting character.
const CAST_BAR_SITTING_HEAD_HEIGHT: f32 = 12.0;
/// Background color of the progress bar.
const CAST_BAR_BACKGROUND_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.55);
/// Color of the skill name text.
const CAST_BAR_TEXT_COLOR: Color = Color::rgba(1.0, 0.95, 0.82, 1.0);
/// Outline of the skill name / timer text.
const CAST_BAR_OUTLINE_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.9);

/// An active cast of the local player.
struct CastBar {
    entity_id: EntityId,
    skill_name: String,
    started_at: ClientTick,
    duration_ms: u32,
}

/// The local player's current cast bar.
#[derive(Default)]
pub struct CastBars {
    own: Option<CastBar>,
}

impl CastBars {
    /// Starts (or restarts) the cast bar for the local player.
    pub fn start(&mut self, entity_id: EntityId, skill_name: String, duration_ms: u32, client_tick: ClientTick) {
        if duration_ms == 0 || skill_name.is_empty() {
            return;
        }

        self.own = Some(CastBar {
            entity_id,
            skill_name,
            started_at: client_tick,
            duration_ms,
        });
    }

    /// Whether the local player is currently in a server-confirmed cast.
    ///
    /// Only casts with a cast time (`ZC_USESKILL_ACK` with `delay_time > 0`)
    /// ever start a bar, so instant skills can never be seen as "casting" here.
    /// The bar also ends after its duration elapsed even if the end packet is
    /// lost, so a stuck bar cannot block movement forever.
    pub fn is_casting(&self, client_tick: ClientTick) -> bool {
        self.own
            .as_ref()
            .is_some_and(|bar| client_tick.0.wrapping_sub(bar.started_at.0) < bar.duration_ms)
    }

    /// Removes the cast bar (cast finished, cancelled or failed).
    pub fn clear(&mut self) {
        self.own = None;
    }

    /// Removes the cast bar if it belongs to the entity that disappeared.
    pub fn remove_entity(&mut self, entity_id: EntityId) {
        if self.own.as_ref().is_some_and(|bar| bar.entity_id == entity_id) {
            self.own = None;
        }
    }

    /// Renders the cast bar; nothing is drawn when there is no active cast or
    /// the cast duration has already elapsed (the server end packet clears
    /// it too).
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        entities: &[Entity],
        screen_size: ScreenSize,
        scaling: Scaling,
        client_tick: ClientTick,
        fill_color: Color,
    ) {
        let Some(bar) = &self.own else {
            return;
        };

        let age = client_tick.0.wrapping_sub(bar.started_at.0);

        if age >= bar.duration_ms {
            return;
        }

        let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == bar.entity_id) else {
            return;
        };

        let scaling_factor = scaling.get_factor();
        let head_height = match entity.is_sitting() {
            true => CAST_BAR_SITTING_HEAD_HEIGHT,
            false => CAST_BAR_HEAD_HEIGHT,
        };
        let head_position = entity.get_position() + Vector3::new(0.0, head_height * entity.get_scale(), 0.0);
        let clip_space_position = camera.view_projection_matrix() * head_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);

        if !(0.0..=1.0).contains(&screen_position.x) || !(0.0..=1.0).contains(&screen_position.y) {
            return;
        }

        let progress = (age as f32 / bar.duration_ms as f32).clamp(0.0, 1.0);
        // Only the skill name is shown; the remaining/total seconds used to be
        // appended to this line and made it harder to read at a glance.
        let text = bar.skill_name.as_str();

        let text_size = renderer.measure_text(text, FontSize(CAST_BAR_FONT_SIZE));
        let bar_width = text_size.width.max(CAST_BAR_MINIMUM_WIDTH * scaling_factor);
        let bar_height = CAST_BAR_HEIGHT * scaling_factor;

        let center = ScreenPosition {
            left: screen_position.x * screen_size.width,
            top: screen_position.y * screen_size.height,
        };

        let text_position = ScreenPosition {
            left: center.left - text_size.width / 2.0,
            top: center.top - text_size.height - bar_height - CAST_BAR_GAP * scaling_factor,
        };
        let bar_position = ScreenPosition {
            left: center.left - bar_width / 2.0,
            top: text_position.top + text_size.height + CAST_BAR_GAP * scaling_factor,
        };

        renderer.render_rectangle(
            bar_position,
            ScreenSize {
                width: bar_width,
                height: bar_height,
            },
            CAST_BAR_BACKGROUND_COLOR,
        );
        if progress > 0.0 {
            renderer.render_rectangle(
                bar_position,
                ScreenSize {
                    width: bar_width * progress,
                    height: bar_height,
                },
                fill_color,
            );
        }

        renderer.render_outlined_text(
            text,
            text_position,
            CAST_BAR_TEXT_COLOR,
            CAST_BAR_OUTLINE_COLOR,
            FontSize(CAST_BAR_FONT_SIZE),
            AlignHorizontal::Left,
        );
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, EntityId};

    use super::CastBars;

    #[test]
    fn start_and_clear_manage_the_own_cast() {
        let mut bars = CastBars::default();
        bars.start(EntityId(1), "Kyrie Eleison".to_owned(), 2_000, ClientTick(100));
        assert!(bars.own.is_some());

        bars.clear();
        assert!(bars.own.is_none());
    }

    #[test]
    fn entity_removal_only_clears_the_matching_bar() {
        let mut bars = CastBars::default();
        bars.start(EntityId(1), "Kyrie Eleison".to_owned(), 2_000, ClientTick(100));

        bars.remove_entity(EntityId(2));
        assert!(bars.own.is_some());

        bars.remove_entity(EntityId(1));
        assert!(bars.own.is_none());
    }

    #[test]
    fn is_casting_only_while_the_timed_cast_is_running() {
        let mut bars = CastBars::default();
        assert!(!bars.is_casting(ClientTick(0)));

        bars.start(EntityId(1), "Heal".to_owned(), 2_000, ClientTick(100));
        assert!(bars.is_casting(ClientTick(100)));
        assert!(bars.is_casting(ClientTick(2_099)));
        // Elapsed casts do not count as casting even without an end packet.
        assert!(!bars.is_casting(ClientTick(2_100)));
        assert!(!bars.is_casting(ClientTick(2_101)));

        // Instant casts never start a bar at all.
        bars.clear();
        bars.start(EntityId(1), "Blessing".to_owned(), 0, ClientTick(5_000));
        assert!(!bars.is_casting(ClientTick(5_000)));
    }

    #[test]
    fn zero_duration_or_empty_name_does_not_start_a_bar() {
        let mut bars = CastBars::default();
        bars.start(EntityId(1), "Heal".to_owned(), 0, ClientTick(0));
        bars.start(EntityId(1), String::new(), 1_000, ClientTick(0));
        assert!(bars.own.is_none());
    }
}
