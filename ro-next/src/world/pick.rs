//! Bounds-based pick assist for the pixel-exact GPU picker.
//!
//! The picker render pass draws every entity sprite at its real size and the
//! pixel under the cursor identifies the entity. That is exact, but small or
//! fast monsters are hard to hit and many ground-item sprites are only a dozen
//! pixels wide. The official client therefore picks through a separate,
//! inflated rectangle per actor: monsters are padded and grown to at least
//! 40 px (`RenderObject.cpp:891`, `:982-989`), items are grown to at least
//! 34 px and get a large depth bonus so an item wins over an actor at the same
//! point (`Item.cpp:53-86`, `ActorPickNode.cpp:110-123`).
//!
//! This module mirrors that split. The raw GPU pick stays in charge whenever
//! it already found an entity (so a human under the cursor can never be
//! swallowed by a padded monster next to it). Only `Nothing`/`Tile` clicks are
//! tested against the projected item and monster rectangles; an item whose
//! rectangle contains the cursor beats every padded monster, which is what
//! keeps a small drop next to a monster clickable.

use cgmath::{Matrix4, Vector2};
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

/// Minimum width/height of a ground item's pick rectangle, in pixels at
/// 100 % UI scale. The official `CItem::SetRenderInfo` uses
/// `UiScaleX() * 34.0` and only grows the rectangle to that size; it does not
/// add a padding ring (`Item.cpp:53-86`). Most ground-item sprites are much
/// smaller than this (a Jellopy is 13 x 14 px), so this is the item-specific
/// part that makes small drops clickable.
pub const ITEM_MIN_PICK_SIZE: f32 = 34.0;

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

    /// The padded hit rectangle used by the monster pick assist.
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

/// One ground item that can win the pick assist, with its projected sprite
/// frame bounds and camera distance.
#[derive(Clone, Copy, Debug)]
pub struct ItemPickCandidate {
    pub entity_id: EntityId,
    pub bounds: PickScreenBounds,
    /// Distance from the camera, used to pick the nearest drop when two items
    /// overlap (the official pick node resolves ties by depth,
    /// `ActorPickNode.cpp:110-123`).
    pub depth: f32,
}

/// Pick-rectangle parameters, in physical pixels. The frame scales every
/// value by the interface scaling factor before calling [`resolve_pick`].
#[derive(Clone, Copy, Debug)]
pub struct PickAssistConfig {
    /// Minimum pick rectangle of a ground item (`Item.cpp:53-86`).
    pub item_minimum_size: f32,
    /// Extra margin around a monster sprite (`RenderObject.cpp:891`).
    pub monster_padding: f32,
    /// Minimum pick rectangle of a monster (`RenderObject.cpp:982-989`).
    pub monster_minimum_size: f32,
}

impl Default for PickAssistConfig {
    fn default() -> Self {
        Self {
            item_minimum_size: ITEM_MIN_PICK_SIZE,
            monster_padding: MONSTER_PICK_PADDING,
            monster_minimum_size: MONSTER_MIN_PICK_SIZE,
        }
    }
}

/// Project the sprite frame rectangle through the camera into screen pixels.
///
/// `world_matrix` is the matrix that `AnimationData::calculate_world_matrix`
/// built for the current sprite frame; the billboard rectangle spans x in
/// -1..=1 and y in 0..=2 in that space. Sprites at or behind the camera have no
/// usable screen position.
pub fn projected_pick_bounds(
    camera: &dyn Camera,
    screen_size: ScreenSize,
    world_matrix: Matrix4<f32>,
    frame_size: Vector2<i32>,
) -> Option<PickScreenBounds> {
    if frame_size.x <= 0 || frame_size.y <= 0 || screen_size.width <= 0.0 || screen_size.height <= 0.0 {
        return None;
    }

    let view_projection = camera.view_projection_matrix();
    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;

    for (x, y) in [(-1.0f32, 0.0f32), (-1.0, 2.0), (1.0, 0.0), (1.0, 2.0)] {
        let local = cgmath::Vector4::new(x, y, 0.0, 1.0);
        let clip = view_projection * (world_matrix * local);

        // `clip.w` is the view-space depth of the reverse-z projection;
        // sprites at or behind the camera have no usable screen position.
        if clip.w <= f32::EPSILON {
            return None;
        }

        let screen = camera.clip_to_screen_space(clip);
        if !screen.x.is_finite() || !screen.y.is_finite() {
            return None;
        }

        let pixel_x = screen.x * screen_size.width;
        let pixel_y = screen.y * screen_size.height;

        left = left.min(pixel_x);
        top = top.min(pixel_y);
        right = right.max(pixel_x);
        bottom = bottom.max(pixel_y);
    }

    if !left.is_finite() || !top.is_finite() || !right.is_finite() || !bottom.is_finite() {
        return None;
    }

    Some(PickScreenBounds::new(left, top, right, bottom))
}

/// Projected screen rectangle of an actor's real sprite bounds.
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

/// Whether `candidate` should replace `current`: the nearest entity to the
/// camera wins, the distance to the cursor only breaks ties at equal depth.
fn precedes(depth: f32, center_distance_squared: f32, current_depth: f32, current_center_distance_squared: f32) -> bool {
    let depth_order = depth.total_cmp(&current_depth);

    if depth_order != std::cmp::Ordering::Equal {
        depth_order == std::cmp::Ordering::Less
    } else {
        center_distance_squared < current_center_distance_squared
    }
}

fn best_item(candidates: &[ItemPickCandidate], cursor: Vector2<f32>, minimum_size: f32) -> Option<&ItemPickCandidate> {
    let mut best: Option<&ItemPickCandidate> = None;

    for candidate in candidates {
        if !candidate.bounds.grown_to_minimum_size(minimum_size).contains(cursor) {
            continue;
        }

        let replace = match best {
            None => true,
            Some(current) => precedes(
                candidate.depth,
                candidate.bounds.center_distance_squared(cursor),
                current.depth,
                current.bounds.center_distance_squared(cursor),
            ),
        };

        if replace {
            best = Some(candidate);
        }
    }

    best
}

fn best_monster(candidates: &[MonsterPickCandidate], cursor: Vector2<f32>, config: PickAssistConfig) -> Option<&MonsterPickCandidate> {
    let mut best: Option<&MonsterPickCandidate> = None;

    for candidate in candidates {
        if candidate.entity_type != EntityType::Monster {
            continue;
        }

        if !candidate
            .bounds
            .assisted(config.monster_padding, config.monster_minimum_size)
            .contains(cursor)
        {
            continue;
        }

        let replace = match best {
            None => true,
            Some(current) => precedes(
                candidate.depth,
                candidate.bounds.center_distance_squared(cursor),
                current.depth,
                current.bounds.center_distance_squared(cursor),
            ),
        };

        if replace {
            best = Some(candidate);
        }
    }

    best
}

/// Resolve a floor click against the item and monster pick rectangles.
///
/// The raw picker target always wins when it is an entity: it is the exact
/// sprite surface under the cursor and already resolves overlapping entities by
/// depth. Only `Nothing`/`Tile` targets (the cursor is not on any sprite) are
/// tested here.
///
/// Ground items win over padded monsters: the official item pick rectangle gets
/// a 34 px minimum and a large depth bonus so an item under the cursor beats an
/// actor (`Item.cpp:53-86`, `ActorPickNode.cpp:110-123`). Among overlapping
/// items the one nearest to the camera wins. Only when no item rectangle
/// contains the cursor are the padded monster rectangles considered, with the
/// same nearest-to-camera rule as before.
pub fn resolve_pick(
    raw_target: PickerTarget,
    cursor: Vector2<f32>,
    item_candidates: &[ItemPickCandidate],
    monster_candidates: &[MonsterPickCandidate],
    config: PickAssistConfig,
) -> PickerTarget {
    if !matches!(raw_target, PickerTarget::Nothing | PickerTarget::Tile { .. }) {
        return raw_target;
    }

    if let Some(item) = best_item(item_candidates, cursor, config.item_minimum_size) {
        return PickerTarget::Entity(item.entity_id);
    }

    best_monster(monster_candidates, cursor, config).map_or(raw_target, |candidate| PickerTarget::Entity(candidate.entity_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monster_candidate(entity_id: u32, entity_type: EntityType, bounds: PickScreenBounds, depth: f32) -> MonsterPickCandidate {
        MonsterPickCandidate {
            entity_id: EntityId(entity_id),
            entity_type,
            bounds,
            depth,
        }
    }

    fn item_candidate(entity_id: u32, bounds: PickScreenBounds, depth: f32) -> ItemPickCandidate {
        ItemPickCandidate {
            entity_id: EntityId(entity_id),
            bounds,
            depth,
        }
    }

    fn monster_bounds() -> PickScreenBounds {
        // Left 100, top 100, 40 wide, 80 high.
        PickScreenBounds::new(100.0, 100.0, 140.0, 180.0)
    }

    /// The production parameters (100 % interface scale).
    fn config() -> PickAssistConfig {
        PickAssistConfig::default()
    }

    /// A config with no item and no monster inflation: this is the raw
    /// pixel-picker behaviour before any assist.
    fn raw_config() -> PickAssistConfig {
        PickAssistConfig {
            item_minimum_size: 0.0,
            monster_padding: 0.0,
            monster_minimum_size: 0.0,
        }
    }

    // ---------------------------------------------------------------------
    // Item matrix: the exact sprite sizes below are from the CondaRO archives.
    // ---------------------------------------------------------------------

    #[test]
    fn large_item_click_resolves_the_item_with_the_setting_on_and_off() {
        // A large item icon: `Decorative_Golden_Bell` is a 39 x 37 sprite, so
        // the pixel picker hits it almost everywhere. Both the on and off
        // configurations still resolve it.
        let item = item_candidate(9001, PickScreenBounds::new(100.0, 100.0, 139.0, 137.0), 50.0);
        let monster = monster_candidate(7, EntityType::Monster, PickScreenBounds::new(0.0, 0.0, 500.0, 500.0), 1.0);
        let cursor = Vector2::new(120.0, 120.0);

        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[item], &[monster], config()),
            PickerTarget::Entity(EntityId(9001)),
            "item assist on"
        );
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[item], &[], config()),
            PickerTarget::Entity(EntityId(9001)),
            "item assist still on when easy monster picking is off"
        );
    }

    #[test]
    fn small_item_that_the_pixel_picker_misses_still_resolves() {
        // `Jellopy` is only 13 x 14 px in the archives. Clicking next to its
        // opaque pixels yields a Tile from the GPU picker; the official 34 px
        // item pick rectangle has to catch it.
        let bounds = PickScreenBounds::new(100.0, 100.0, 113.0, 114.0);
        let item = item_candidate(9002, bounds, 50.0);
        let cursor = Vector2::new(95.0, 95.0);

        assert!(!bounds.contains(cursor));
        assert_eq!(
            resolve_pick(PickerTarget::Tile { x: 100, y: 100 }, cursor, &[item], &[], config()),
            PickerTarget::Entity(EntityId(9002))
        );
        assert_eq!(
            resolve_pick(PickerTarget::Tile { x: 100, y: 100 }, cursor, &[item], &[], raw_config()),
            PickerTarget::Tile { x: 100, y: 100 },
            "the pixel-exact picker alone misses the small icon"
        );
    }

    #[test]
    fn thin_item_next_to_a_padded_monster_still_resolves() {
        // `Black_Frame_Glasses` is 19 x 6 px, an extreme aspect ratio. The
        // click is outside the real bounds but inside the 34 px item box.
        let bounds = PickScreenBounds::new(100.0, 100.0, 119.0, 106.0);
        let item = item_candidate(9003, bounds, 50.0);
        let monster = monster_candidate(7, EntityType::Monster, PickScreenBounds::new(40.0, 40.0, 160.0, 160.0), 1.0);
        let cursor = Vector2::new(95.0, 100.0);

        assert!(!bounds.contains(cursor));
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[item], &[monster], config()),
            PickerTarget::Entity(EntityId(9003))
        );
        // The padded monster is nearer to the camera and its rectangle also
        // contains the click, but the item still wins.
    }

    #[test]
    fn padded_monster_next_to_an_item_does_not_steal_the_item_click() {
        let item = item_candidate(9004, PickScreenBounds::new(100.0, 100.0, 113.0, 114.0), 50.0);
        let monster = monster_candidate(7, EntityType::Monster, PickScreenBounds::new(40.0, 40.0, 160.0, 160.0), 1.0);
        let cursor = Vector2::new(95.0, 95.0);

        assert_eq!(
            resolve_pick(PickerTarget::Tile { x: 100, y: 100 }, cursor, &[item], &[monster], config()),
            PickerTarget::Entity(EntityId(9004))
        );
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[item], &[monster], config()),
            PickerTarget::Entity(EntityId(9004))
        );
    }

    #[test]
    fn two_overlapping_drops_pick_the_nearest_to_the_camera() {
        // Two drops on almost the same spot. The official pick node resolves
        // overlapping rectangles by depth (`ActorPickNode.cpp:110-123`), so the
        // nearer drop wins regardless of iteration order.
        let near = item_candidate(9005, PickScreenBounds::new(100.0, 100.0, 113.0, 114.0), 10.0);
        let far = item_candidate(9006, PickScreenBounds::new(104.0, 100.0, 117.0, 114.0), 90.0);
        let cursor = Vector2::new(110.0, 110.0);

        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[far, near], &[], config()),
            PickerTarget::Entity(EntityId(9005))
        );
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[near, far], &[], config()),
            PickerTarget::Entity(EntityId(9005))
        );
    }

    #[test]
    fn item_click_keeps_the_entity_id_the_raw_picker_would_return() {
        let item = item_candidate(9010, PickScreenBounds::new(100.0, 100.0, 120.0, 120.0), 50.0);
        let monster = monster_candidate(7, EntityType::Monster, PickScreenBounds::new(40.0, 40.0, 160.0, 160.0), 1.0);
        let cursor = Vector2::new(110.0, 110.0);

        // A raw item target is never replaced, so the pickup request uses the
        // exact id the GPU picker returned.
        assert_eq!(
            resolve_pick(PickerTarget::Entity(EntityId(9010)), cursor, &[item], &[monster], config()),
            PickerTarget::Entity(EntityId(9010))
        );
        // A floor click resolves to that same id.
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, cursor, &[item], &[monster], config()),
            PickerTarget::Entity(EntityId(9010))
        );
    }

    #[test]
    fn a_raw_monster_target_is_not_replaced_by_an_item_next_to_it() {
        let item = item_candidate(9011, PickScreenBounds::new(100.0, 100.0, 120.0, 120.0), 50.0);
        let cursor = Vector2::new(110.0, 110.0);

        assert_eq!(
            resolve_pick(PickerTarget::Entity(EntityId(7)), cursor, &[item], &[], config()),
            PickerTarget::Entity(EntityId(7)),
            "the exact sprite under the cursor stays the target"
        );
    }

    // ---------------------------------------------------------------------
    // Monster assist: behaviour kept from the easy_monster_picking feature.
    // ---------------------------------------------------------------------

    #[test]
    fn click_just_outside_monster_bounds_selects_it_when_the_assist_is_on() {
        let candidates = [monster_candidate(42, EntityType::Monster, monster_bounds(), 100.0)];
        let just_outside = Vector2::new(94.0, 140.0);
        let assisted = PickAssistConfig {
            item_minimum_size: 0.0,
            ..config()
        };

        assert_eq!(
            resolve_pick(PickerTarget::Nothing, just_outside, &[], &candidates, assisted),
            PickerTarget::Entity(EntityId(42))
        );
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, just_outside, &[], &candidates, raw_config()),
            PickerTarget::Nothing
        );
    }

    #[test]
    fn minimum_size_makes_tiny_monsters_clickable() {
        // A 10 x 12 sprite: the real bounds do not contain the click, but the
        // 40 px minimum pick rectangle does.
        let tiny = PickScreenBounds::new(105.0, 114.0, 115.0, 126.0);
        let candidates = [monster_candidate(7, EntityType::Monster, tiny, 50.0)];
        let click = Vector2::new(96.0, 120.0);
        let minimum_only = PickAssistConfig {
            item_minimum_size: 0.0,
            monster_padding: 0.0,
            monster_minimum_size: MONSTER_MIN_PICK_SIZE,
        };

        assert!(!tiny.contains(click));
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, click, &[], &candidates, minimum_only),
            PickerTarget::Entity(EntityId(7))
        );
        assert_eq!(
            resolve_pick(PickerTarget::Nothing, click, &[], &candidates, raw_config()),
            PickerTarget::Nothing
        );
    }

    #[test]
    fn raw_entity_wins_over_an_overlapping_padded_monster() {
        // The human's exact sprite is under the cursor, so the raw picker
        // target is the human. A monster rectangle overlaps it and is even
        // nearer to the camera; it must not swallow the human.
        let candidates = [monster_candidate(
            99,
            EntityType::Monster,
            PickScreenBounds::new(0.0, 0.0, 500.0, 500.0),
            1.0,
        )];
        let cursor = Vector2::new(120.0, 140.0);

        assert_eq!(
            resolve_pick(PickerTarget::Entity(EntityId(5)), cursor, &[], &candidates, config()),
            PickerTarget::Entity(EntityId(5))
        );
    }

    #[test]
    fn nearest_padded_monster_wins_when_several_overlap() {
        let bounds = monster_bounds();
        let near = monster_candidate(1, EntityType::Monster, bounds, 10.0);
        let far = monster_candidate(2, EntityType::Monster, bounds, 90.0);
        let cursor = Vector2::new(130.0, 140.0);

        // Both iteration orders have to return the nearest entity.
        assert_eq!(
            resolve_pick(PickerTarget::Tile { x: 100, y: 100 }, cursor, &[], &[far, near], config()),
            PickerTarget::Entity(EntityId(1))
        );
        assert_eq!(
            resolve_pick(PickerTarget::Tile { x: 100, y: 100 }, cursor, &[], &[near, far], config()),
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
            let candidates = [monster_candidate(10, entity_type, bounds, 1.0)];

            assert_eq!(
                resolve_pick(PickerTarget::Nothing, cursor, &[], &candidates, config()),
                PickerTarget::Nothing,
                "{entity_type:?} must keep its exact pick bounds"
            );
        }
    }
}
