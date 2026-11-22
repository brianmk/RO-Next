//! Anchors of skill effects that depend on the entity (head/body height).
//!
//! The skill effect **data** (layers, anchors, assets, timings) lives in
//! [`crate::world::library::skill_effects`]; this module only provides the
//! entity-dependent height anchors those layers use:
//!
//! * the overhead anchor (above the head) of buff/support effects, which
//!   follows the entity scale and the sitting height,
//! * the body anchor (torso) of body-attached layers,
//! * the shared cast circle radius.
//!
//! The official client's effect table marks these effects with `head: true`;
//! the data file records that flag per layer and the spawner calls
//! [`overhead_effect_height`] for those layers. There is no hard-coded
//! per-skill overhead list any more.
//!
//! `.str` effects are screen space sprites in the original client format, so
//! they cannot be rotated into the ground plane. The cast circle is therefore
//! drawn procedurally by [`crate::world::CastCircle`] (flat ground quads) and
//! only *completion* effects that are meant to be screen sprites use `.str`
//! files.

/// Head anchor height in world units for a standing character (the same anchor
/// the skill-name label and the cast bar use). Multiplied by the entity scale.
const HEAD_HEIGHT: f32 = 21.0;
/// Head anchor height in world units for a sitting character.
const SITTING_HEAD_HEIGHT: f32 = 12.0;
/// Extra clearance above the head so an overhead buff effect clears the
/// head/hair instead of overlapping it. Multiplied by the entity scale.
const OVERHEAD_CLEARANCE: f32 = 5.0;
/// Torso height in world units for a standing character (body-attached layers
/// such as the AGI Up / Blessing texture glows). Multiplied by the entity
/// scale.
const BODY_HEIGHT: f32 = 13.0;

/// Radius of the ground cast circle in world units. The official cast aura is
/// roughly one tile wide around the caster.
pub const CAST_CIRCLE_RADIUS: f32 = 7.0;

/// Vertical anchor (above the feet, in world units) of an overhead buff or
/// completion effect for an entity.
///
/// This is the single overhead anchor used by every buff/support effect: the
/// skill-name/cast-bar head height (21 standing, 12 sitting) plus a fixed
/// clearance, scaled by the entity's sprite scale. It therefore works for
/// tall/small entities and for sitting characters, and for other players as
/// well as the local player.
pub fn overhead_effect_height(entity: &crate::world::Entity) -> f32 {
    overhead_effect_height_for(entity.is_sitting(), entity.get_scale())
}

/// Pure version of [`overhead_effect_height`] (no entity needed,
/// unit-testable).
pub fn overhead_effect_height_for(is_sitting: bool, scale: f32) -> f32 {
    let head_height = match is_sitting {
        true => SITTING_HEAD_HEIGHT,
        false => HEAD_HEIGHT,
    };

    (head_height + OVERHEAD_CLEARANCE) * scale
}

/// Vertical anchor (above the feet, in world units) of a body-attached layer
/// (torso height) for an entity. Scaled by the entity sprite scale and lowered
/// for sitting characters.
pub fn body_effect_height(entity: &crate::world::Entity) -> f32 {
    body_effect_height_for(entity.is_sitting(), entity.get_scale())
}

/// Pure version of [`body_effect_height`] (no entity needed, unit-testable).
pub fn body_effect_height_for(is_sitting: bool, scale: f32) -> f32 {
    let body_height = match is_sitting {
        // Keep the same standing/sitting ratio as the head anchor so a body
        // glow on a sitting character does not float above the lowered torso.
        true => BODY_HEIGHT * (SITTING_HEAD_HEIGHT / HEAD_HEIGHT),
        false => BODY_HEIGHT,
    };

    body_height * scale
}

#[cfg(test)]
mod tests {
    use super::{
        BODY_HEIGHT, CAST_CIRCLE_RADIUS, HEAD_HEIGHT, OVERHEAD_CLEARANCE, SITTING_HEAD_HEIGHT, body_effect_height_for,
        overhead_effect_height_for,
    };

    #[test]
    fn overhead_effect_height_is_above_the_head_and_scales() {
        // Standing characters: head anchor plus clearance, above the 21-unit head.
        let standing = overhead_effect_height_for(false, 1.0);
        assert!(standing > HEAD_HEIGHT, "overhead anchor must be above the head");
        assert!((standing - (HEAD_HEIGHT + OVERHEAD_CLEARANCE)).abs() < f32::EPSILON);

        // Sitting characters use the lower sitting head anchor.
        let sitting = overhead_effect_height_for(true, 1.0);
        assert!((sitting - (SITTING_HEAD_HEIGHT + OVERHEAD_CLEARANCE)).abs() < f32::EPSILON);
        assert!(sitting < standing);

        // Entity scale (large/small characters, status size changes) scales it.
        let scaled = overhead_effect_height_for(false, 2.0);
        assert!((scaled - 2.0 * standing).abs() < f32::EPSILON);
    }

    #[test]
    fn body_effect_height_is_at_the_torso_and_scales() {
        let standing = body_effect_height_for(false, 1.0);
        assert!((standing - BODY_HEIGHT).abs() < f32::EPSILON);
        // The torso anchor is below the overhead anchor and above the feet.
        assert!(standing > 0.0 && standing < overhead_effect_height_for(false, 1.0));

        let sitting = body_effect_height_for(true, 1.0);
        assert!(sitting < standing, "a sitting torso is lower");
        assert!((sitting - BODY_HEIGHT * (SITTING_HEAD_HEIGHT / HEAD_HEIGHT)).abs() < f32::EPSILON);

        assert!((body_effect_height_for(false, 2.0) - 2.0 * standing).abs() < f32::EPSILON);
    }

    #[test]
    fn cast_circle_radius_stays_the_official_one_tile_size() {
        assert!((CAST_CIRCLE_RADIUS - 7.0).abs() < f32::EPSILON);
    }
}
