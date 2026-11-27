//! `CRagEffect::SakuraFireArrow()` - `RagEffect.cpp:9509-9546` - and the
//! `EF_KOUENKA` dispatch that wraps it (`RagEffect.cpp:21180-21185`).
//!
//! `EF_KOUENKA` is a composite: every frame runs [`sakura_fire_arrow`], which
//! launches one prim 124 (`PP_KOUENKA`) sakura sprite per frame for the first
//! twenty frames, and frame 0 additionally spawns the two
//! `TexturedQuadSpin("effect\\alpha_center.tga", index, 2)` sheets.
//!
//! RNG order matters: the sound pick (`ClampedRandom(3) + 1`, frames 12-15)
//! happens before any launch, and each launch draws `maxHeight`
//! (`ClampedRandom(3)`), `size` (`ClampedRandom(6)`), the orbit angle
//! (`ClampedRandom(360)`), `height[0..3]` (`ClampedRandom(360)` each) and
//! finally the `SetAction` row (`ClampedRandom(3)`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::quad_spin::TexturedQuadSpin;

/// `effect\\alpha_center.tga` - the two spinning sheets of the Kouenka
/// dispatch (`RagEffect.cpp:21183-21185`).
const KOUENKA_QUAD_TEXTURE: &str = "effect\\alpha_center.tga";
/// `이팩트\sakura01.spr` / `.act` - `SakuraFireArrow`'s sprite
/// (`RagEffect.cpp:9541-9542`).
const SAKURA_SPRITE: &str = "이팩트\\sakura01.spr";
const SAKURA_ACTION: &str = "이팩트\\sakura01.act";

/// `case EF_KOUENKA:` - `RagEffect.cpp:21180-21185`.
#[derive(Debug)]
pub struct Kouenka {
    lifetime: i32,
}

impl Kouenka {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Kouenka {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        sakura_fire_arrow(runtime);

        if runtime.frame == 0 {
            // `for (int index = 0; index < 2; ++index)
            //      this->TexturedQuadSpin("effect\\alpha_center.tga", index, 2);`
            let quads = TexturedQuadSpin::new(KOUENKA_QUAD_TEXTURE, 2, 2, runtime.lifetime);

            for phase in 0..2i32 {
                quads.spawn(runtime, phase);
            }
        }

        true
    }
}

/// `CRagEffect::SakuraFireArrow()` - `RagEffect.cpp:9509-9546`.
fn sakura_fire_arrow(runtime: &mut EffectRuntime) {
    // `CopyVector(this->render.position, this->state.parent->render.position);`
    runtime.copy_parent_position();

    if (12..=15).contains(&runtime.frame) {
        let wave = format!("effect\\EF_FireArrow{}.wav", runtime.rng.clamped_random(3) + 1);

        runtime.play_sound(&wave);
    }

    if runtime.frame >= 20 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = runtime.position;

    // RNG order per launch: maxHeight, size, orbit angle, height[0..3],
    // `SetAction` row.
    let max_height = (runtime.rng.clamped_random(3) + 2) as f32 * 0.15;
    let size = (runtime.rng.clamped_random(6) + 22) as f32 / 70.0;
    let angle = runtime.rng.clamped_random(360) as i16;
    let mut heights = [0.0f32; 4];

    for height in &mut heights {
        *height = runtime.rng.clamped_random(360) as f32;
    }

    let action = runtime.rng.clamped_random(3);

    let sakura = runtime.launch(PrimitiveId::Kouenka, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(sakura);
    let angles = super::super::angles::Angles::shared();

    state.duration = lifetime;
    state.slots[0].life = 1;
    state.slots[0].max_height = max_height;
    state.size = size;
    state.slots[0].vec_b_pre[1] = position[1];

    let angle = angle as usize;

    state.slots[0].vec_b_pre[0] = angles.raw_cosine(angle) * 8.0 + position[0];
    state.slots[0].vec_b_pre[2] = angles.raw_sine(angle) * 8.0 + position[2];

    state.slots[0].height[0] = heights[0];
    state.slots[0].height[1] = heights[1];
    state.slots[0].height[2] = heights[2];
    state.slots[0].height[3] = heights[3];
    state.slots[1].life = 0;
    state.slots[2].life = 0;
    state.slots[3].life = 0;
    state.alpha = 250.0;
    state.sprite = Some(SAKURA_SPRITE);
    state.action = Some(SAKURA_ACTION);
    // `SetAction(ClampedRandom(3), 36, 0)` - the row, `curMotion` reset and
    // `render.motionType`; `state.repeatAnim` is never written by `SetAction`
    // and keeps the constructor's 1, so `AnimMotion` loops.
    state.action_index = action;
    state.init_segment();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn kouenka_spawns_one_sakura_per_frame_for_twenty_frames() {
        let mut effect = Kouenka::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);
        runtime.position = [7.0, 8.0, 9.0];

        step(&mut effect, &mut runtime);

        // Frame 0: one sakura + the two quad sheets.
        assert_eq!(runtime.primitives.len(), 3);

        for _ in 0..19 {
            step(&mut effect, &mut runtime);
        }

        // Frames 0..=19 add one sakura each, frames 20+ add nothing.
        assert_eq!(runtime.primitives.len(), 22);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 22);
    }

    #[test]
    fn the_sakura_sprite_uses_the_official_assets_and_slots() {
        let mut effect = Kouenka::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);
        runtime.position = [7.0, 8.0, 9.0];

        // `on_process` alone, so `PrimKouenka` has not run its same-frame pass
        // yet (it would already have floated the sprite and faded the alpha).
        assert!(effect.on_process(&mut runtime));

        let sakura = &runtime.primitives[0].state;
        assert_eq!(sakura.count, PrimitiveId::Kouenka);
        assert_eq!(sakura.sprite, Some("이팩트\\sakura01.spr"));
        assert_eq!(sakura.action, Some("이팩트\\sakura01.act"));
        assert!((0..3).contains(&sakura.action_index));
        assert_eq!(sakura.alpha, 250.0);
        assert_eq!(sakura.duration, 300);
        // `SetAction` clears `curMotion`; `state.repeatAnim` stays 1.
        assert_eq!(sakura.cur_motion, 0);
        assert_eq!(sakura.repeat_anim, 1);
        assert_eq!(sakura.slots[0].life, 1);
        assert!(sakura.slots[1..].iter().all(|slot| slot.life == 0));
        assert!(sakura.slots[0].max_height >= 0.3 - 1e-6 && sakura.slots[0].max_height <= 0.75 + 1e-6);
        // The orbit radius is 8 around the caster on x/z, on the caster's y.
        let dx = sakura.slots[0].vec_b_pre[0] - 7.0;
        let dz = sakura.slots[0].vec_b_pre[2] - 9.0;
        assert!((dx * dx + dz * dz - 64.0).abs() < 1e-2);
        assert_eq!(sakura.slots[0].vec_b_pre[1], 8.0);
        assert_eq!(sakura.records.len(), 1, "InitSegment ran at spawn");
    }

    #[test]
    fn the_fire_arrow_wave_plays_on_frames_twelve_to_fifteen() {
        let mut effect = Kouenka::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);

        for _ in 0..12 {
            step(&mut effect, &mut runtime);
        }

        assert!(runtime.sounds.is_empty(), "frames 0-11 are silent");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds.len(), 1);
        assert!(runtime.sounds[0].starts_with("effect\\EF_FireArrow"));
        assert!(runtime.sounds[0].ends_with(".wav"));

        for _ in 0..3 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.sounds.len(), 4, "frames 12, 13, 14 and 15");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds.len(), 4);
    }

    #[test]
    fn the_two_quad_sheets_land_on_the_caster_position() {
        let mut effect = Kouenka::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);
        runtime.position = [1.0, 2.0, 3.0];

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[1].state.count, PrimitiveId::SpinQuad);
        assert_eq!(runtime.primitives[2].state.count, PrimitiveId::SpinQuad);
        assert_eq!(runtime.primitives[1].state.texture, Some("effect\\alpha_center.tga"));
        assert_eq!(runtime.primitives[1].state.slots[0].height[4], 2.0);
        assert_eq!(runtime.primitives[2].state.slots[0].rise_angle, 22);
        assert_eq!(runtime.primitives[1].state.position, [1.0, 2.0, 3.0]);
    }
}
