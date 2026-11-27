//! Monster-only pick assist for the pixel-exact GPU picker.
//!
//! The picker render pass draws every entity sprite at its real size and the
//! pixel under the cursor identifies the entity. That is exact, but small or
//! fast monsters are hard to hit. This module resolves a click against the
//! projected sprite bounds of monsters with a padding/minimum-size assist,
//! while leaving the raw GPU pick in charge whenever it already found an
//! entity (so a human under the cursor can never be swallowed by a padded
//! monster next to it).

use cgmath::Vector2;
use ragnarok_packets::{ClientTick, EntityId};

use crate::graphics::{PickerTarget, ScreenSize};
use crate::world::{Camera, Entity, EntityType};

/// Extra clickable margin around a monster sprite, in physical pixels (one
/// side). 12 px is large enough to make a sprite that is a few pixels wide
/// practical to hit without turning the whole neighbouring floor into a
/// monster.
pub const MONSTER_PICK_PADDING: f32 = 12.0;

/// Minimum width/height of a monster's assisted pick rectangle, in pixels.
/// Mirrors the official client's `s_minPickSize` (40 px at 100 % UI scale),
/// which is what makes tiny monsters targetable there.
pub const MONSTER_MIN_PICK_SIZE: f32 = 40.0;

/// Axis-aligned rectangle in physical screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickScreenBounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl PickScreenBounds {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    pub fn width(self) -> f32 {
        self.right - self.left
    }

    pub fn height(self) -> f32 {
        self.bottom - self.top
    }

    /// Whether the point is inside the rectangle. Uses inclusive edges so a
    /// click exactly on the sprite border still counts.
    pub fn contains(self, cursor: Vector2<f32>) -> bool {
        cursor.x >= self.left && cursor.x <= self.right && cursor.y >= self.top && cursor.y <= self.bottom
    }

    /// Grow the rectangle by `padding` on every side.
    pub fn padded(self, padding: f32) -> Self {
        let padding = padding.max(0.0);

        Self {
            left: self.left - padding,
            top: self.top - padding,
            right: self.right + padding,
            bottom: self.bottom + padding,
        }
    }

    /// Grow the rectangle around its center until it is at least
    /// `minimum_size` wide and high. Smaller rectangles stay unchanged.
    pub fn grown_to_minimum_size(self, minimum_size: f32) -> Self {
        let minimum_size = minimum_size.max(0.0);

        let mut result = self;

        let width = self.width();
        if width < minimum_size {
            let grow = (minimum_size - width) / 2.0;
            result.left -= grow;
            result.right += grow;
        }

        let height = self.height();
        if height < minimum_size {
            let grow = (minimum_size - height) / 2.0;
            result.top -= grow;
            result.bottom += grow;
        }

        result
    }

    /// The assisted (monster-only) hit rectangle used by the pick assist.
    pub fn assisted(self, padding: f32, minimum_size: f32) -> Self {
        self.padded(padding).grown_to_minimum_size(minimum_size)
    }

    fn center_distance_squared(self, cursor: Vector2<f32>) -> f32 {
        let center = Vector2::new((self.left + self.right) / 2.0, (self.top + self.bottom) / 2.0);
        let offset = center - cursor;

        offset.x * offset.x + offset.y * offset.y
    }
}

/// One entity that can win the pick assist, with its exact projected sprite
/// bounds and camera distance.
#[derive(Clone, Copy, Debug)]
pub struct MonsterPickCandidate {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub bounds: PickScreenBounds,
    /// Distance from the camera, used to pick the nearest entity when several
    /// assisted rectangles overlap.
    pub depth: f32,
}

/// Projected screen rectangle of an entity's real sprite bounds.
///
/// The entity enum itself does not need a picker-specific accessor; the pick
/// assist reads the shared `Common` state through the public accessors of the
/// two actor variants.
pub fn entity_pick_screen_bounds(
    entity: &Entity,
    camera: &dyn Camera,
    screen_size: ScreenSize,
    client_tick: ClientTick,
) -> Option<PickScreenBounds> {
    match entity {
        Entity::Player(player) => player.get_common().pick_screen_bounds(camera, screen_size, client_tick),
        Entity::Npc(npc) => npc.get_common().pick_screen_bounds(camera, screen_size, client_tick),
    }
}

/// Resolve the entity under the cursor, allowing a wider area for monsters.
///
/// The raw picker target always wins when it is an entity: it is the exact
/// sprite surface under the cursor and already resolves overlapping entities by
/// depth. Only `Nothing`/`Tile` targets (the cursor is not on any sprite) are
/// tested against the padded monster rectangles.
///
/// Among the overlapping padded monsters the one nearest to the camera wins
/// (the same preference the depth-tested picker has); the distance to the
/// cursor is only a tie-breaker for entities at the same depth.
pub fn resolve_monster_pick(
    raw_target: PickerTarget,
    cursor: Vector2<f32>,
    candidates: &[MonsterPickCandidate],
    padding: f32,
    minimum_size: f32,
) -> PickerTarget {
    if !matches!(raw_target, PickerTarget::Nothing | PickerTarget::Tile { .. }) {
        return raw_target;
    }

    let mut best: Option<&MonsterPickCandidate> = None;

    for candidate in candidates {
        if candidate.entity_type != EntityType::Monster {
            continue;
        }

        if !candidate.bounds.assisted(padding, minimum_size).contains(cursor) {
            continue;
        }

        let replace = match best {
            None => true,
            Some(current) => {
                let depth_order = candidate.depth.total_cmp(&current.depth);
                if depth_order != std::cmp::Ordering::Equal {
                    depth_order == std::cmp::Ordering::Less
                } else {
                    candidate.bounds.center_distance_squared(cursor) < current.bounds.center_distance_squared(cursor)
                }
            }
        };

        if replace {
            best = Some(candidate);
        }
    }

    best.map_or(raw_target, |candidate| PickerTarget::Entity(candidate.entity_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(entity_id: u32, entity_type: EntityType, bounds: PickScreenBounds, depth: f32) -> MonsterPickCandidate {
        MonsterPickCandidate {
            entity_id: EntityId(entity_id),
            entity_type,
            bounds,
            depth,
        }
    }

    fn monster_bounds() -> PickScreenBounds {
        // Left 100, top 100, 40 wide, 80 high.
        PickScreenBounds::new(100.0, 100.0, 140.0, 180.0)
    }

    #[test]
    fn click_just_outside_monster_bounds_selects_it_when_the_assist_is_on() {
        let candidates = [candidate(42, EntityType::Monster, monster_bounds(), 100.0)];
        let just_outside = Vector2::new(94.0, 140.0);

        assert_eq!(
            resolve_monster_pick(
                PickerTarget::Nothing,
                just_outside,
                &candidates,
                MONSTER_PICK_PADDING,
                MONSTER_MIN_PICK_SIZE
            ),
            PickerTarget::Entity(EntityId(42))
        );
        assert_eq!(
            resolve_monster_pick(PickerTarget::Nothing, just_outside, &candidates, 0.0, 0.0),
            PickerTarget::Nothing
        );
    }

    #[test]
    fn minimum_size_makes_tiny_monsters_clickable() {
        // A 10 x 12 sprite: the real bounds do not contain the click, but the
        // 40 px minimum pick rectangle does.
        let tiny = PickScreenBounds::new(105.0, 114.0, 115.0, 126.0);
        let candidates = [candidate(7, EntityType::Monster, tiny, 50.0)];
        let click = Vector2::new(96.0, 120.0);

        assert!(!tiny.contains(click));
        assert_eq!(
            resolve_monster_pick(PickerTarget::Nothing, click, &candidates, 0.0, MONSTER_MIN_PICK_SIZE),
            PickerTarget::Entity(EntityId(7))
        );
        assert_eq!(
            resolve_monster_pick(PickerTarget::Nothing, click, &candidates, 0.0, 0.0),
            PickerTarget::Nothing
        );
    }

    #[test]
    fn raw_entity_wins_over_an_overlapping_padded_monster() {
        // The human's exact sprite is under the cursor, so the raw picker
        // target is the human. A monster rectangle overlaps it and is even
        // nearer to the camera; it must not swallow the human.
        let candidates = [candidate(
            99,
            EntityType::Monster,
            PickScreenBounds::new(0.0, 0.0, 500.0, 500.0),
            1.0,
        )];
        let cursor = Vector2::new(120.0, 140.0);

        assert_eq!(
            resolve_monster_pick(
                PickerTarget::Entity(EntityId(5)),
                cursor,
                &candidates,
                MONSTER_PICK_PADDING,
                MONSTER_MIN_PICK_SIZE
            ),
            PickerTarget::Entity(EntityId(5))
        );
    }

    #[test]
    fn nearest_padded_monster_wins_when_several_overlap() {
        let bounds = monster_bounds();
        let near = candidate(1, EntityType::Monster, bounds, 10.0);
        let far = candidate(2, EntityType::Monster, bounds, 90.0);
        let cursor = Vector2::new(130.0, 140.0);

        // Both iteration orders have to return the nearest entity.
        assert_eq!(
            resolve_monster_pick(
                PickerTarget::Tile { x: 100, y: 100 },
                cursor,
                &[far, near],
                MONSTER_PICK_PADDING,
                MONSTER_MIN_PICK_SIZE
            ),
            PickerTarget::Entity(EntityId(1))
        );
        assert_eq!(
            resolve_monster_pick(
                PickerTarget::Tile { x: 100, y: 100 },
                cursor,
                &[near, far],
                MONSTER_PICK_PADDING,
                MONSTER_MIN_PICK_SIZE
            ),
            PickerTarget::Entity(EntityId(1))
        );
    }

    #[test]
    fn only_monster_entities_get_the_assist() {
        let bounds = PickScreenBounds::new(0.0, 0.0, 500.0, 500.0);
        let cursor = Vector2::new(250.0, 250.0);

        for entity_type in [
            EntityType::Player,
            EntityType::Npc,
            EntityType::Pet,
            EntityType::Homunculus,
            EntityType::Mercenary,
            EntityType::Warp,
            EntityType::Hidden,
        ] {
            let candidates = [candidate(10, entity_type, bounds, 1.0)];

            assert_eq!(
                resolve_monster_pick(
                    PickerTarget::Nothing,
                    cursor,
                    &candidates,
                    MONSTER_PICK_PADDING,
                    MONSTER_MIN_PICK_SIZE
                ),
                PickerTarget::Nothing,
                "{entity_type:?} must keep its exact pick bounds"
            );
        }
    }
}
