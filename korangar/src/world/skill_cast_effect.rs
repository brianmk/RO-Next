//! Visuals for skill casts of other entities.
//!
//! Other clients show a *ground cast circle* below the caster while a skill
//! with a cast time is being cast. Korangar ignored the skill packets
//! completely until the ambient-plaza work; this module now provides the
//! per-skill cast circle color, the hard-coded "cylinder" auras of the official
//! client and the optional completion effect.
//!
//! `.str` effects are screen space sprites in the original client format, so
//! they cannot be rotated into the ground plane. The cast circle is therefore
//! drawn procedurally by [`crate::world::CastCircle`] (flat ground quads) and
//! only *completion* effects that are meant to be screen sprites use `.str`
//! files. The yellow cone/ring effects of the classic Monk/Champion and
//! Star Emperor skill sets are drawn procedurally by
//! [`crate::world::SkillAura`] because the original client does not use `.str`
//! files for them either.
//!
//! The 2019 GRFs shipped with this server do not contain the classic skill
//! effects (there is no `blessing.str`, `incagi.str`, `hammerfall.str`,
//! `warp.str`, ...), so the completion effects use representative files that
//! exist in the data:
//!
//! * forging uses the ground dust effect at the feet,
//! * self-buffs use a small sparkle effect around the torso,
//! * other casts only get the generic element-colored cast circle.
//!
//! The cast circle itself is generic: its color follows the skill element from
//! the `ZC_USESKILL_ACK` packet.

use cgmath::Vector3;
use ragnarok_packets::SkillId;

use crate::graphics::Color;
use crate::world::effect::{AuraGrow, OFFICIAL_EFFECT_UNIT, SkillAuraSpec};

/// Official skill ids used by this table (stable across the 2019 server).
mod skill_ids {
    pub const BS_HAMMERFALL: u16 = 5;
    /// Skill id the ambient crafting loop uses
    /// (`population_engine_ambient_craft_skill_id`, 110 = the hammering skill
    /// on the CondaRO data).
    pub const BS_HAMMERFALL_AMBIENT: u16 = 110;
    pub const AL_WARP: u16 = 27;
    pub const AL_INCAGI: u16 = 29;
    pub const AL_ANGELUS: u16 = 33;
    pub const AL_BLESSING: u16 = 34;
    pub const AC_CONCENTRATION: u16 = 45;
    /// `MO_EXPLOSIONSPIRITS` (Fury): consumes five spirit spheres and starts
    /// the `EFST_EXPLOSIONSPIRITS` status.
    pub const MO_EXPLOSIONSPIRITS: u16 = 270;
    /// `CH_SOULCOLLECT` (Zen): long self cast that adds five spirit spheres.
    pub const CH_SOULCOLLECT: u16 = 401;
}

/// A short screen sprite effect that is played when a cast completes.
pub struct CompletionEffect {
    /// Effect file, relative to `data\texture\effect\`.
    pub path: &'static str,
    /// Offset from the caster's feet in world units.
    pub offset: Vector3<f32>,
    /// Size multiplier of the screen sprite (1.0 = original size from the
    /// file).
    pub scale: f32,
}

/// A temporary tint of the caster's sprite (the "body flash" of the official
/// hard-coded skill effects, for example `EF_PORTAL5` of Zen).
#[derive(Debug, Clone, Copy)]
pub struct BodyFlash {
    pub color: Color,
    /// How long the flash keeps its full weight in milliseconds.
    pub hold_ms: u32,
    /// How long the flash fades back to the normal colors in milliseconds.
    pub fade_ms: u32,
}

/// Everything the client needs to show and hear a skill cast.
pub struct SkillVisuals {
    /// Color of the ground cast circle (radius is shared, see
    /// [`CAST_CIRCLE_RADIUS`]).
    pub cast_circle_color: Color,
    /// Effect on cast completion, if there is one.
    pub completion: Option<CompletionEffect>,
    /// Hard-coded cylinder aura spawned when a *cast-time* skill starts (the
    /// yellow `EF_BEGINSPELL` cone of the official client), if there is one.
    pub cast_aura: Option<SkillAuraSpec>,
    /// Hard-coded cylinder auras spawned when an *instant* skill completes
    /// (used by Fury, which has no cast time).
    pub instant_auras: &'static [SkillAuraSpec],
    /// Flash of the caster's sprite when the skill starts.
    pub body_flash: Option<BodyFlash>,
    /// Point light spawned with the cast (color, intensity).
    pub light: (Color, f32),
}

/// Radius of the ground cast circle in world units. The official cast aura is
/// roughly one tile wide around the caster.
pub const CAST_CIRCLE_RADIUS: f32 = 7.0;

/// Hammer Fall: the hammer slam animation itself (effect texture `hammer.bmp`),
/// the missing "hammer layer" that the previous `dust.str` placeholder did not
/// show.
const HAMMERFALL_EFFECT: &str = "black_hammerfall.str";
const SPARKLE_EFFECT: &str = "strangelights.str";
/// Animated angel that flutters/rotates above the character. This is the asset
/// that reads closest to the classic Blessing angel; the classic
/// `blessing.str` is not part of the 2019 data set.
const ANGEL_EFFECT: &str = "efst_angel_fluttering\\angel_fluttering.str";
/// Ring of blessing angels, used for Angelus.
const BLESSING_ANGELS_EFFECT: &str = "efst_blessing_of_angels\\tensi3.str";
/// Blessing-of-souls sparkles, used for Incagi.
const BLESSING_OF_SOULS_EFFECT: &str = "efst_c_blessings_of_soul\\blessingsofsoul.str";

/// Height of the angel above the feet in world units (just above the head).
const ANGEL_HEIGHT: f32 = 18.0;
/// Height of the Angelus ring above the feet.
const BLESSING_ANGELS_HEIGHT: f32 = 16.0;
/// Height of the torso sparkle effects.
const TORSO_HEIGHT: f32 = 10.0;

/// Yellow tint of the Monk/Champion cast effects (`ring_yellow` based).
const YELLOW_AURA_COLOR: Color = Color::rgb_u8(255, 225, 90);
/// Flash color of the Zen body flash (effect `EF_PORTAL5`): the character
/// flashes yellow for 200 ms and fades back to normal over 600 ms.
const ZEN_FLASH: BodyFlash = BodyFlash {
    color: Color::rgb(1.0, 1.0, 0.0),
    hold_ms: 200,
    fade_ms: 600,
};

/// `EF_BEGINSPELL` (effect 12): yellow cone around the caster. The bottom
/// radius stays small while the top radius grows to five effect units. Additive
/// blending, fades in and out over one second.
const BEGIN_SPELL_AURA: SkillAuraSpec = SkillAuraSpec {
    texture_path: "effect\\ring_yellow.tga",
    duration_ms: 1_000,
    height: 4.0 * OFFICIAL_EFFECT_UNIT,
    bottom_radius: 1.0 * OFFICIAL_EFFECT_UNIT,
    top_radius: 5.0 * OFFICIAL_EFFECT_UNIT,
    alpha_max: 0.8,
    additive: true,
    rotation_speed: 0.0,
    grow: AuraGrow::TopRadius,
    duplicates: 1,
    duplicate_delay_ms: 0,
};

/// `EF_GUMGANG2` (effect 261, "Fury Cast Animation"): four expanding yellow
/// rings, 100 ms apart, alpha blended.
const FURY_RINGS_AURA: SkillAuraSpec = SkillAuraSpec {
    texture_path: "effect\\ring_yellow.tga",
    duration_ms: 1_500,
    height: 2.0 * OFFICIAL_EFFECT_UNIT,
    bottom_radius: 2.0 * OFFICIAL_EFFECT_UNIT,
    top_radius: 5.0 * OFFICIAL_EFFECT_UNIT,
    alpha_max: 0.5,
    additive: false,
    rotation_speed: 4.0,
    grow: AuraGrow::BothRadii,
    duplicates: 4,
    duplicate_delay_ms: 100,
};

/// Auras of an instant Fury cast: the cast cone (`beginCastEffectId: 12`) and
/// the expanding rings (`effectIdOnCaster: 261`).
static FURY_INSTANT_AURAS: [SkillAuraSpec; 2] = [BEGIN_SPELL_AURA, FURY_RINGS_AURA];

fn completion(path: &'static str, height: f32, scale: f32) -> Option<CompletionEffect> {
    Some(CompletionEffect {
        path,
        offset: Vector3::new(0.0, height, 0.0),
        scale,
    })
}

fn plain(cast_circle_color: Color) -> SkillVisuals {
    SkillVisuals {
        cast_circle_color,
        completion: None,
        cast_aura: None,
        instant_auras: &[],
        body_flash: None,
        light: (cast_circle_color, 25.0),
    }
}

/// Returns the cast visuals for a skill. Unknown skills still get the generic
/// element-colored cast ring, only the completion effect is absent.
pub fn skill_visuals(skill_id: SkillId, element: u32) -> SkillVisuals {
    match skill_id.0 {
        // Forging: hammer strike into the ground -> dust puff at the feet.
        skill_ids::BS_HAMMERFALL | skill_ids::BS_HAMMERFALL_AMBIENT => SkillVisuals {
            cast_circle_color: crate::world::element_color(element),
            completion: Some(CompletionEffect {
                // The hammer slam itself (the effect texture is `hammer.bmp`).
                path: HAMMERFALL_EFFECT,
                offset: Vector3::new(0.0, 0.0, 0.0),
                scale: 1.2,
            }),
            cast_aura: None,
            instant_auras: &[],
            body_flash: None,
            light: (Color::rgb_u8(210, 190, 150), 20.0),
        },
        // Warp portal: the ground cast circle is the cast visual; the open
        // portal itself is a skill unit and rendered separately.
        skill_ids::AL_WARP => SkillVisuals {
            cast_circle_color: Color::rgb_u8(160, 130, 255),
            completion: None,
            cast_aura: None,
            instant_auras: &[],
            body_flash: None,
            light: (Color::rgb_u8(160, 120, 255), 40.0),
        },
        // Blessing: the animated angel above the head is what players expect.
        skill_ids::AL_BLESSING => SkillVisuals {
            cast_circle_color: Color::rgb_u8(255, 230, 160),
            completion: completion(ANGEL_EFFECT, ANGEL_HEIGHT, 1.0),
            cast_aura: None,
            instant_auras: &[],
            body_flash: None,
            light: (Color::rgb_u8(255, 235, 180), 45.0),
        },
        // Angelus: ring of blessing angels around the character.
        skill_ids::AL_ANGELUS => SkillVisuals {
            cast_circle_color: Color::rgb_u8(255, 230, 160),
            completion: completion(BLESSING_ANGELS_EFFECT, BLESSING_ANGELS_HEIGHT, 1.0),
            cast_aura: None,
            instant_auras: &[],
            body_flash: None,
            light: (Color::rgb_u8(255, 235, 180), 45.0),
        },
        // Incagi: blessing-of-souls sparkles around the torso.
        skill_ids::AL_INCAGI => SkillVisuals {
            cast_circle_color: Color::rgb_u8(255, 230, 160),
            completion: completion(BLESSING_OF_SOULS_EFFECT, TORSO_HEIGHT, 0.9),
            cast_aura: None,
            instant_auras: &[],
            body_flash: None,
            light: (Color::rgb_u8(255, 230, 160), 35.0),
        },
        // Concentration: generic sparkles around the torso.
        skill_ids::AC_CONCENTRATION => SkillVisuals {
            cast_circle_color: Color::rgb_u8(255, 230, 160),
            completion: completion(SPARKLE_EFFECT, TORSO_HEIGHT, 0.9),
            cast_aura: None,
            instant_auras: &[],
            body_flash: None,
            light: (Color::rgb_u8(255, 230, 160), 35.0),
        },
        // Zen (`CH_SOULCOLLECT`): two-second self cast. The official client keeps
        // the idle pose, plays the yellow `EF_BEGINSPELL` cone for the start of
        // the cast and flashes the caster's body yellow (`EF_PORTAL5`).
        skill_ids::CH_SOULCOLLECT => SkillVisuals {
            cast_circle_color: Color::rgb_u8(255, 230, 160),
            completion: None,
            cast_aura: Some(BEGIN_SPELL_AURA),
            instant_auras: &[],
            body_flash: Some(ZEN_FLASH),
            light: (YELLOW_AURA_COLOR, 40.0),
        },
        // Fury (`MO_EXPLOSIONSPIRITS`): instant self buff. The official client
        // keeps the idle pose and plays the `EF_BEGINSPELL` cone plus the four
        // expanding `EF_GUMGANG2` rings around the caster.
        skill_ids::MO_EXPLOSIONSPIRITS => SkillVisuals {
            cast_circle_color: Color::rgb_u8(255, 230, 160),
            completion: None,
            cast_aura: None,
            instant_auras: &FURY_INSTANT_AURAS,
            body_flash: None,
            light: (YELLOW_AURA_COLOR, 45.0),
        },
        _ => plain(crate::world::element_color(element)),
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::skill_visuals;

    #[test]
    fn crafting_has_a_completion_effect() {
        let visuals = skill_visuals(SkillId(110), 0);
        assert!(visuals.completion.as_ref().is_some_and(|effect| effect.path.ends_with(".str")));
    }

    #[test]
    fn blessing_uses_the_animated_angel_above_the_head() {
        let visuals = skill_visuals(SkillId(34), 0);
        let effect = visuals.completion.expect("blessing completion effect");

        assert!(
            effect.path.contains("angel_fluttering"),
            "blessing should use the angel effect, got {}",
            effect.path
        );
        assert!(
            effect.offset.y > 10.0,
            "the angel has to be anchored above the head, not at the feet"
        );
        assert!(effect.scale >= 0.9);
    }

    #[test]
    fn unknown_skills_have_no_completion_effect() {
        assert!(skill_visuals(SkillId(1), 3).completion.is_none());
    }

    #[test]
    fn cast_circle_colors_follow_the_element_for_unknown_skills() {
        let fire = skill_visuals(SkillId(1), 3);
        let water = skill_visuals(SkillId(1), 1);
        assert_ne!(fire.cast_circle_color, water.cast_circle_color);
    }

    #[test]
    fn zen_has_a_cast_aura_and_a_body_flash() {
        let visuals = skill_visuals(SkillId(401), 0);
        let aura = visuals.cast_aura.expect("Zen cast aura");

        assert!(aura.texture_path.contains("ring_yellow"));
        assert!(aura.height > 0.0 && aura.top_radius > aura.bottom_radius);

        let flash = visuals.body_flash.expect("Zen body flash");
        assert_eq!(flash.hold_ms, 200);
        assert_eq!(flash.fade_ms, 600);
        assert!(flash.color.red > 0.9 && flash.color.blue < 0.1);
    }

    #[test]
    fn fury_plays_the_begin_spell_cone_and_the_expanding_rings_on_an_instant_cast() {
        let visuals = skill_visuals(SkillId(270), 0);
        assert!(visuals.cast_aura.is_none(), "Fury has no cast time in the pre-renewal skill db");
        assert_eq!(visuals.instant_auras.len(), 2);
        assert!(visuals.instant_auras.iter().all(|aura| aura.texture_path.contains("ring_yellow")));
        assert_eq!(visuals.instant_auras[1].duplicates, 4);
        assert!(visuals.instant_auras[1].duplicate_delay_ms > 0);
    }
}
