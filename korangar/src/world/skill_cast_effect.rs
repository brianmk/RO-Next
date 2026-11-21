//! Visual effects for skill casts of other entities.
//!
//! Other clients play the caster's skill-cast animation *and* a skill effect
//! when they receive `ZC_USESKILL_ACK` (`UseSkillAckPacket2019`, header 0x0B1A
//! on the 2019 server). Korangar never handled that packet, so ambient actors
//! (self-buffs, crafting, warp portal casts, ...) looked completely static.
//!
//! The 2019 client data set shipped with this server does not contain the
//! classic skill `.str` effects (there is no `blessing.str`, `incagi.str`,
//! `hammerfall.str`, `warp.str`, ...; only newer 4th-job style effects are in
//! the GRFs). For the ambient plaza shows this mapping therefore uses
//! representative effects that *do* exist in the data:
//!
//! * crafting (`BS_HAMMERFALL`, a hammer strike into the ground) uses the
//!   ground dust puff,
//! * the priest self-buffs use a golden halo ring / blue chill cloud,
//! * a warp portal cast uses the magic circle at the caster's feet.
//!
//! The cast *animation* itself (the RO ACT skill action) is played for every
//! skill regardless of this table, so ambient acts are visible even when no
//! representative effect file exists.

use cgmath::Vector3;
use ragnarok_packets::SkillId;

use crate::graphics::Color;

/// Official skill ids used by this table (`db/re/skill_db.yml` ids are stable
/// across the 2019 pre-renewal database).
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
}

/// A representative `.str` effect for a skill cast.
pub struct SkillCastEffect {
    /// Effect file, relative to `data\texture\effect\`.
    pub path: &'static str,
    /// Offset from the caster's feet in world units.
    pub offset: Vector3<f32>,
    /// Color/intensity of the point light spawned with the effect.
    pub light_color: Color,
    pub light_intensity: f32,
}

const HALO_EFFECT: &str = "efst_glowofnewyear\\halo.str";
const CHILL_EFFECT: &str = "chill.str";
const DUST_EFFECT: &str = "dust.str";
const MAGIC_CIRCLE_EFFECT: &str = "efst_magiccircle\\mc.str";

/// Returns the representative cast effect for a skill, if there is one.
///
/// Skills without an entry still get the cast animation, just no `.str`
/// effect.
pub fn skill_cast_effect(skill_id: SkillId) -> Option<SkillCastEffect> {
    let (path, offset, light_color, light_intensity) = match skill_id.0 {
        // Crafting: hammer strike into the ground -> dust puff at the feet.
        skill_ids::BS_HAMMERFALL | skill_ids::BS_HAMMERFALL_AMBIENT => {
            (DUST_EFFECT, Vector3::new(0.0, 0.0, 0.0), Color::rgb_u8(210, 190, 150), 20.0)
        }
        // Warp portal cast -> magic circle at the feet.
        skill_ids::AL_WARP => (
            MAGIC_CIRCLE_EFFECT,
            Vector3::new(0.0, 0.0, 0.0),
            Color::rgb_u8(160, 120, 255),
            40.0,
        ),
        // Agility up -> blue speed cloud around the caster.
        skill_ids::AL_INCAGI => (CHILL_EFFECT, Vector3::new(0.0, 5.0, 0.0), Color::rgb_u8(150, 200, 255), 30.0),
        // Angelus / Blessing -> golden halo around the caster.
        skill_ids::AL_ANGELUS | skill_ids::AL_BLESSING | skill_ids::AC_CONCENTRATION => {
            (HALO_EFFECT, Vector3::new(0.0, 8.0, 0.0), Color::rgb_u8(255, 230, 160), 40.0)
        }
        _ => return None,
    };

    Some(SkillCastEffect {
        path,
        offset,
        light_color,
        light_intensity,
    })
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::skill_cast_effect;

    #[test]
    fn ambient_plaza_skills_have_a_representative_effect() {
        // BS_HAMMERFALL (crafting), AL_WARP (portal), AL_INCAGI, AL_ANGELUS,
        // AL_BLESSING
        for skill_id in [5, 27, 29, 33, 34] {
            let effect = skill_cast_effect(SkillId(skill_id)).unwrap_or_else(|| panic!("missing effect for skill {skill_id}"));
            assert!(effect.path.ends_with(".str"), "effect path should be a .str file");
            assert!(effect.light_intensity > 0.0);
        }
    }

    #[test]
    fn ambient_crafting_and_concentration_have_effects() {
        // 110 = the ambient crafting skill id, 45 = AC_CONCENTRATION.
        assert!(skill_cast_effect(SkillId(110)).is_some());
        assert!(skill_cast_effect(SkillId(45)).is_some());
    }

    #[test]
    fn unknown_skills_do_not_get_a_fake_effect() {
        // 1 = SM_SWORD, 28 = AL_HEAL: not part of the ambient show.
        assert!(skill_cast_effect(SkillId(1)).is_none());
        assert!(skill_cast_effect(SkillId(28)).is_none());
    }
}
