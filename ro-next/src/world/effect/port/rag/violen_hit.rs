//! `EF_VIOLEN_HIT` (2420) - the official `CSpriteEffect` body.
//!
//! The official entity is a `CSpriteEffect` (`EffectLists.cpp:5129`), not a
//! `CRagEffect`, so there is no case in `RagEffect.cpp`; the whole body is its
//! sprite setup (`SpriteEffect.cpp:65-75`, `:289-295`):
//!
//! * the shared defaults - `renderOptions |= 0x200`, `frameDelay = 2.0`, `alpha
//!   = 255`, `effectLifetime = 100` (`SpriteEffect.cpp:71-73`);
//! * the Violen case - one `몬스터\violen_hit.spr`/`.act` pair, `loopMode = 1`
//!   (`MT_LOOP`), `depthOffset = 0.001`, `frameDelay = 4.0`
//!   (`SpriteEffect.cpp:289-295`).
//!
//! `CSpriteEffect::Open` creates exactly one `CSpriteParticle`
//! (`SpriteEffect.cpp:51-62`), so the port launches one `PP_3DPARTICLE`.
//! `frameDelay` is the number of 16 ms effect frames between ACT frame
//! advances (`SpriteParticle::UpdateFrame`, `SpriteParticle.cpp:103-117`), the
//! port's `anim_speed`, and `MT_LOOP` is the port's `repeat_anim = 1`.
//!
//! `depthOffset` is only added to the sprite vertices' `rhw` (the DX9
//! screen-space depth bias, `SpriteParticle.cpp:293-296`); it has no
//! position/scale counterpart in the port's sprite pass.
//!
//! The client selects this id as the defender hit effect of a
//! Violen/`JT_CP_VIOLEN` attacker (`RenderObject.cpp:3818-3821`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CSpriteEffect::SetupParticle` - `SpriteEffect.cpp:71-73`.
const LIFETIME: i32 = 100;
/// `particle->frameDelay = 4.0f` - `SpriteEffect.cpp:294`.
const FRAME_DELAY: i32 = 4;

#[derive(Debug, Default)]
pub struct ViolenHit;

impl PortedEffect for ViolenHit {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let particle = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(particle);

        // `renderOptions |= 0x200` (`SpriteEffect.cpp:71`).
        state.pattern |= 0x200;
        state.duration = lifetime;
        state.anim_speed = FRAME_DELAY;
        // `loopMode = MT_LOOP` (`SpriteEffect.cpp:292`).
        state.repeat_anim = 1;
        // `particle->alpha = 255` (`SpriteEffect.cpp:72`); the primitive's
        // default alpha is 254 and `ProcessAlpha` clamps to `maxAlpha`.
        state.alpha = 255.0;
        state.max_alpha = 255.0;
        state.sprite = Some("몬스터\\violen_hit.spr");
        state.action = Some("몬스터\\violen_hit.act");
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn violen_hit_is_the_official_looping_monster_sprite() {
        let mut effect = ViolenHit;
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 2420);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1, "CSpriteEffect::Open creates one particle");
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Particle3d);
        assert_eq!(state.pattern & 0x200, 0x200);
        assert_eq!(state.duration, LIFETIME);
        assert_eq!(state.anim_speed, 4);
        assert_eq!(state.repeat_anim, 1, "loopMode = MT_LOOP");
        assert_eq!(state.alpha, 255.0);
        assert_eq!(state.sprite, Some("몬스터\\violen_hit.spr"));
        assert_eq!(state.action, Some("몬스터\\violen_hit.act"));
        assert_eq!(state.num_segment, 1);
    }

    #[test]
    fn violen_hit_lifetime_is_the_sprite_effect_default() {
        let effect = ViolenHit;
        assert_eq!(effect.lifetime(), 100, "SpriteEffect.cpp:73 effectLifetime = 100");
    }
}
