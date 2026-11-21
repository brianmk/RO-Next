//! Overhead chat bubbles.
//!
//! Classic Ragnarok Online shows chat messages as a "subtitle" above the
//! speaking character in addition to the chat window. This module keeps track
//! of the recent messages per entity and renders them as screen space text (or
//! text with a dark background), moving together with the entity.
//!
//! Only `EntityType::Player` entities get bubbles: NPC/monster dialogue is
//! deliberately filtered out here, even though it arrives through the same
//! `ZC_NOTIFY_CHAT` packet.

use std::sync::LazyLock;

use cgmath::Vector3;
use korangar_networking::MessageColor;
use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::{Color, ScreenPosition, ScreenSize};
use crate::loaders::{FontSize, Scaling};
use crate::renderer::{AlignHorizontal, GameInterfaceRenderer};
use crate::world::{Camera, Entity, EntityType};

/// How long a single message stays on screen.
const BUBBLE_LIFETIME_MS: u32 = 5_000;
/// Time at the end of the lifetime during which the bubble fades out.
const BUBBLE_FADE_OUT_MS: u32 = 900;
/// How many messages are stacked above one entity at most.
const MAX_LINES_PER_ENTITY: usize = 3;
/// Global cap on the number of bubbles, to bound the work per frame.
const MAX_BUBBLES: usize = 48;
/// Cap on the length of a single message (the server normally limits this, but
/// a broken/garbage packet should not be able to allocate/render forever).
const MAX_MESSAGE_CHARS: usize = 160;
/// Font size in unscaled pixels; the UI scaling factor is applied by the
/// renderer.
const BUBBLE_FONT_SIZE: f32 = 14.0;
/// Horizontal/vertical padding of the dark background box, in unscaled pixels.
const BUBBLE_PADDING: f32 = 5.0;
/// Vertical gap between two stacked bubbles, in unscaled pixels.
const BUBBLE_LINE_GAP: f32 = 2.0;
/// Approximate height of a character in world units. Character sprites are
/// about 70-80 pixels tall, and one tile is `TILE_SIZE` (10) world units with
/// `SPRITE_SCALE` 1.4 (see `AnimationData::calculate_world_matrix`), so the
/// head is roughly nine to eleven units above the entity's feet.
const BUBBLE_HEAD_HEIGHT: f32 = 10.0;

struct ChatBubble {
    entity_id: EntityId,
    text: String,
    color: Color,
    created_at: ClientTick,
}

/// All currently visible overhead chat bubbles.
#[derive(Default)]
pub struct ChatBubbles {
    bubbles: Vec<ChatBubble>,
}

impl ChatBubbles {
    /// Add a new message for an entity. Older messages of the same entity are
    /// kept, but only up to [`MAX_LINES_PER_ENTITY`] are stacked above it.
    pub fn push(&mut self, entity_id: EntityId, text: String, color: MessageColor, client_tick: ClientTick) {
        let text = sanitize(&text);

        if text.is_empty() {
            return;
        }

        self.retain_alive(client_tick);
        self.bubbles.push(ChatBubble {
            entity_id,
            text,
            color: message_color(color),
            created_at: client_tick,
        });

        // Drop the oldest messages of this entity if it is speaking too much.
        let mut per_entity = self.bubbles.iter().filter(|bubble| bubble.entity_id == entity_id).count();

        let mut index = 0;
        while per_entity > MAX_LINES_PER_ENTITY && index < self.bubbles.len() {
            if self.bubbles[index].entity_id == entity_id {
                self.bubbles.remove(index);
                per_entity -= 1;
            } else {
                index += 1;
            }
        }

        // And keep the global list bounded as well.
        while self.bubbles.len() > MAX_BUBBLES {
            self.bubbles.remove(0);
        }
    }

    /// Remove all messages (e.g. on a map change, where every entity is reset).
    pub fn clear(&mut self) {
        self.bubbles.clear();
    }

    /// Remove the messages of a single entity (e.g. when it disappears).
    pub fn remove_entity(&mut self, entity_id: EntityId) {
        self.bubbles.retain(|bubble| bubble.entity_id != entity_id);
    }

    /// Drop messages whose lifetime has expired.
    pub fn retain_alive(&mut self, client_tick: ClientTick) {
        self.bubbles
            .retain(|bubble| message_age(client_tick, bubble.created_at) < BUBBLE_LIFETIME_MS);
    }

    /// Render all bubbles of player entities that are currently on screen.
    ///
    /// `show_background` controls the classic dark subtitle box. When it is
    /// disabled the text is drawn with an outline only, so it stays readable on
    /// bright terrain.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        entities: &[Entity],
        screen_size: ScreenSize,
        scaling: Scaling,
        show_background: bool,
        client_tick: ClientTick,
    ) {
        let scaling_factor = scaling.get_factor();
        let padding = BUBBLE_PADDING * scaling_factor;
        let line_gap = BUBBLE_LINE_GAP * scaling_factor;
        let font_size = BUBBLE_FONT_SIZE;

        // Newest message is drawn closest to the character, older ones are
        // stacked above it, so iterate from the back.
        let mut stacks: Vec<(EntityId, usize)> = Vec::new();

        for bubble in self.bubbles.iter().rev() {
            let age = message_age(client_tick, bubble.created_at);

            if age >= BUBBLE_LIFETIME_MS {
                continue;
            }

            let alpha = match age + BUBBLE_FADE_OUT_MS >= BUBBLE_LIFETIME_MS {
                true => (BUBBLE_LIFETIME_MS - age) as f32 / BUBBLE_FADE_OUT_MS as f32,
                false => 1.0,
            };

            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == bubble.entity_id) else {
                continue;
            };

            // Only players get overhead chat bubbles; NPCs and monsters use the
            // same packet for their dialogue but must not show a bubble.
            if !is_bubble_entity(entity) {
                continue;
            }

            let stack_index = match stacks.iter_mut().find(|(entity_id, _)| *entity_id == bubble.entity_id) {
                Some((_, index)) => {
                    *index += 1;
                    *index - 1
                }
                None => {
                    stacks.push((bubble.entity_id, 1));
                    0
                }
            };

            let head_position = entity.get_position() + Vector3::new(0.0, BUBBLE_HEAD_HEIGHT * entity.get_scale(), 0.0);
            let clip_space_position = camera.view_projection_matrix() * head_position.to_homogeneous();
            let screen_position = camera.clip_to_screen_space(clip_space_position);

            // Skip bubbles of entities that are off screen.
            if !(0.0..=1.0).contains(&screen_position.x) || !(0.0..=1.0).contains(&screen_position.y) {
                continue;
            }

            let text_size = renderer.measure_text(&bubble.text, FontSize(font_size));
            let box_size = ScreenSize {
                width: text_size.width + padding * 2.0,
                height: text_size.height + padding * 2.0,
            };

            let center = ScreenPosition {
                left: screen_position.x * screen_size.width,
                top: screen_position.y * screen_size.height,
            };
            let stack_offset = (box_size.height + line_gap) * stack_index as f32;

            let box_position = ScreenPosition {
                left: center.left - box_size.width / 2.0,
                top: center.top - box_size.height - stack_offset,
            };
            let text_position = ScreenPosition {
                left: box_position.left + padding,
                top: box_position.top + padding,
            };

            let color = Color {
                red: bubble.color.red,
                green: bubble.color.green,
                blue: bubble.color.blue,
                alpha,
            };

            if show_background {
                renderer.render_rectangle(box_position, box_size, Color::rgba(0.0, 0.0, 0.0, 0.55 * alpha));
            }

            renderer.render_outlined_text(
                &bubble.text,
                text_position,
                color,
                Color::rgba(0.0, 0.0, 0.0, 0.9 * alpha),
                FontSize(font_size),
                AlignHorizontal::Left,
            );
        }
    }
}

/// Headless test override: `KORANGAR_CHAT_BUBBLES=0|1` disables/enables the
/// bubbles without touching (and persisting) the interface settings.
pub fn chat_bubbles_enabled_by_environment() -> bool {
    static ENABLED: LazyLock<bool> = LazyLock::new(|| environment_flag("KORANGAR_CHAT_BUBBLES"));

    *ENABLED
}

/// Headless test override: `KORANGAR_CHAT_BUBBLE_BACKGROUND=0|1`
/// disables/enables the dark background box without touching the interface
/// settings.
pub fn chat_bubble_background_enabled_by_environment() -> bool {
    static ENABLED: LazyLock<bool> = LazyLock::new(|| environment_flag("KORANGAR_CHAT_BUBBLE_BACKGROUND"));

    *ENABLED
}

fn environment_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => value != "0",
        Err(_) => true,
    }
}

/// Whether an entity is allowed to show an overhead chat bubble.
///
/// Only player speech gets a bubble. NPCs and monsters talk through the same
/// `ZC_NOTIFY_CHAT` packet (quests, `npctalk`/`monster talk`), and that text is
/// only shown in the chat window.
pub fn is_bubble_entity(entity: &Entity) -> bool {
    is_bubble_entity_type(entity.get_entity_type())
}

fn is_bubble_entity_type(entity_type: EntityType) -> bool {
    entity_type == EntityType::Player
}

fn message_age(client_tick: ClientTick, created_at: ClientTick) -> u32 {
    client_tick.0.wrapping_sub(created_at.0)
}

fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|character| !matches!(character, '\r' | '\n' | '\0'))
        .take(MAX_MESSAGE_CHARS)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn message_color(color: MessageColor) -> Color {
    match color {
        MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
        MessageColor::Broadcast | MessageColor::Server | MessageColor::Error | MessageColor::Information => Color::WHITE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity_id(id: u32) -> EntityId {
        EntityId(id)
    }

    fn tick(value: u32) -> ClientTick {
        ClientTick(value)
    }

    /// Number of bubbles that are still alive and belong to the given entity.
    fn alive_for(bubbles: &ChatBubbles, entity: EntityId) -> usize {
        bubbles.bubbles.iter().filter(|bubble| bubble.entity_id == entity).count()
    }

    #[test]
    fn only_player_entities_get_overhead_bubbles() {
        assert!(is_bubble_entity_type(EntityType::Player));

        for entity_type in [EntityType::Npc, EntityType::Monster, EntityType::Warp, EntityType::Hidden] {
            assert!(!is_bubble_entity_type(entity_type), "{entity_type:?} must not get a bubble");
        }
    }

    #[test]
    fn messages_of_one_entity_are_capped() {
        let mut bubbles = ChatBubbles::default();
        let entity = entity_id(1);

        for index in 0..10 {
            bubbles.push(entity, format!("message {index}"), MessageColor::Broadcast, tick(index));
        }

        assert_eq!(alive_for(&bubbles, entity), MAX_LINES_PER_ENTITY);
        // The oldest messages were dropped, the newest one is kept.
        assert_eq!(bubbles.bubbles.last().unwrap().text, "message 9");
    }

    #[test]
    fn messages_expire_after_their_lifetime() {
        let mut bubbles = ChatBubbles::default();
        let entity = entity_id(1);

        bubbles.push(entity, "hello".to_owned(), MessageColor::Broadcast, tick(100));
        assert_eq!(alive_for(&bubbles, entity), 1);

        bubbles.retain_alive(tick(100 + BUBBLE_LIFETIME_MS - 1));
        assert_eq!(alive_for(&bubbles, entity), 1);

        bubbles.retain_alive(tick(100 + BUBBLE_LIFETIME_MS));
        assert_eq!(alive_for(&bubbles, entity), 0);
    }

    #[test]
    fn messages_can_be_removed_per_entity_and_globally() {
        let mut bubbles = ChatBubbles::default();

        bubbles.push(entity_id(1), "a".to_owned(), MessageColor::Broadcast, tick(0));
        bubbles.push(entity_id(2), "b".to_owned(), MessageColor::Broadcast, tick(0));

        bubbles.remove_entity(entity_id(1));
        assert_eq!(alive_for(&bubbles, entity_id(1)), 0);
        assert_eq!(alive_for(&bubbles, entity_id(2)), 1);

        bubbles.clear();
        assert!(bubbles.bubbles.is_empty());
    }

    #[test]
    fn messages_are_bounded_and_sanitized() {
        let mut bubbles = ChatBubbles::default();

        // Newlines/control characters are removed and crazy long messages are cut.
        let long = "a\n\r\0".repeat(MAX_MESSAGE_CHARS * 2);
        bubbles.push(entity_id(1), long, MessageColor::Broadcast, tick(0));

        let text = &bubbles.bubbles[0].text;
        assert!(text.chars().count() <= MAX_MESSAGE_CHARS);
        assert!(!text.contains('\n'));
        assert!(!text.contains('\r'));

        // An empty message does not create a bubble.
        bubbles.push(entity_id(2), "  ".to_owned(), MessageColor::Broadcast, tick(0));
        assert_eq!(bubbles.bubbles.len(), 1);

        // The global cap is enforced.
        for index in 0..(MAX_BUBBLES * 2) {
            bubbles.push(
                entity_id(1000 + index as u32),
                format!("m{index}"),
                MessageColor::Broadcast,
                tick(0),
            );
        }
        assert!(bubbles.bubbles.len() <= MAX_BUBBLES);
    }
}
