//! `CRagEffect::SMA(char flag)` - `RagEffect.cpp:2621-2646` - the `EF_STIN3`
//! dispatch that wraps it (`RagEffect.cpp:21744-21750`), and the two
//! intermediate ring bodies it feeds:
//! `CRagEffect::TexturedBlueRing` (`RagEffect.cpp:16329-16395`) and
//! `CRagEffect::BlueRingStack` (`RagEffect.cpp:5829-5877`).
//!
//! [`sma`] is the shared method body: frame 0 launches prim 120 (`PP_SMA`)
//! with four drifting slots whose `flag1[0]` is the `flag` argument, `process`
//! starts at `index - 3`, `height[1]` is `0.8` (the per-frame drift) and the
//! slot positions sit at the caster minus ten units on `y` plus
//! `ClampedRandom(5)`. Prim 120 has `totalTexture = 0` and submits nothing; its
//! renderer (`RenderSMA3`) launches the children below on every render frame.
//! `EF_STIN3` (`SMA(1)`) adds the frame-0 `T_에너지방출.wav` wave on top.
//!
//! [`TexturedBlueRing`] (`EF_SMA2`) and [`BlueRingStack`] (`EF_SMA_BLACK2`)
//! both launch prim 121 (`PP_SMA2`) and only differ in where the ring texture
//! comes from and in `slots[].alphaT`: `99` selects the grey/black colour of
//! `RenderSMA2`'s `pw = 99` branch, while everything else keeps the blue
//! `(90, 90, 255)` branch. `BlueRingStack` also stops at slot 3 and lets the
//! `index * 0.2` distance ladder give it the four-ring stack.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::SMA(char flag)` - `RagEffect.cpp:2621-2646`.
///
/// Shared by `EF_SMA` (`flag = 0`), `EF_STIN3` (`1`) and `EF_SMA_BLACK`
/// (`99`); only `EF_STIN3` is registered (the other two are outside the
/// pre-renewal backlog).
pub fn sma(runtime: &mut EffectRuntime, flag: i8) {
    // `CopyVector(this->render.position, this->state.parent->render.position);`
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = runtime.position;
    let target = runtime.target_offset;

    // Per-slot `ClampedRandom(5)` in slot order.
    let mut y_offsets = [0.0f32; 4];

    for offset in &mut y_offsets {
        *offset = runtime.rng.clamped_random(5) as f32;
    }

    let launcher = runtime.launch(PrimitiveId::Sma, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(launcher);

    state.duration = lifetime;
    // `state.totalTexture = 0;` - the launcher primitive has no texture.
    state.total_texture = 0;
    state.size = 0.0;
    state.delta_pos = target;
    // `CopyVector(this->state.lastPrim->render.position, this->render.position);`
    state.position = position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = index as i32 - 3;
        slot.alpha_b = 0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = (position[1] - 10.0) + y_offsets[index];
        slot.rot_start = 0;
        slot.height[1] = 0.8;
        slot.flag1[0] = flag;
    }
}

/// `case EF_STIN3:` - `RagEffect.cpp:21744-21750`.
///
/// `if (stateCnt == 0) { render.position = parent.position;
/// PlayEffectWave(*this, "effect\T_에너지방출.wav"); } this->SMA(1);`
pub struct Stin3 {
    lifetime: i32,
}

impl Stin3 {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Stin3 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\T_에너지방출.wav");
        }

        sma(runtime, 1);

        true
    }
}

/// `CRagEffect::TexturedBlueRing(const char* name)` -
/// `RagEffect.cpp:16329-16395`.
#[derive(Debug)]
pub struct TexturedBlueRing {
    texture: &'static str,
    lifetime: i32,
}

impl TexturedBlueRing {
    pub fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

impl PortedEffect for TexturedBlueRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let target = runtime.target_offset;
        let delta_pos2 = runtime.delta_pos2;

        let ring = runtime.launch(PrimitiveId::Sma2, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.size = 4.0;
        state.delta_pos = target;
        state.delta_pos2 = delta_pos2;

        for index in 0..3usize {
            let step = index as f32;
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 315;
            slot.max_height = 13.0 - 2.0 * step;
            slot.rot_start = (index * 90) as i16;
            slot.alpha_b = 0;
            slot.distance = step * 0.2 + if delta_pos2[1] == 0.0 { 3.4 } else { 2.4 };
            slot.rise_angle = (55 - 5 * index as i32) as i16;
            slot.vec_b_now[1] = 0.0;
        }

        state.slots[3].life = 0;

        for slot in &mut state.slots {
            for value in 0..21usize {
                slot.height[value] = 0.0;
                slot.flag1[value] = 0;
            }
        }

        true
    }
}

/// `CRagEffect::BlueRingStack()` - `RagEffect.cpp:5829-5877`.
#[derive(Debug)]
pub struct BlueRingStack {
    lifetime: i32,
}

impl BlueRingStack {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for BlueRingStack {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let target = runtime.target_offset;
        let delta_pos2 = runtime.delta_pos2;

        let ring = runtime.launch(PrimitiveId::Sma2, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some("effect\\ring_blue.tga");
        state.size = 4.0;
        state.delta_pos = target;
        state.delta_pos2 = delta_pos2;

        for index in 0..4usize {
            // `if (index == 3) { slots[3].life = 0; return; }` - slot 3 never
            // gets the `alphaT = 99` / height initialisation.
            if index == 3 {
                state.slots[3].life = 0;

                return true;
            }

            let step = index as f32;
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 315;
            slot.max_height = 13.0 - (step + step);
            slot.rot_start = (index * 90) as i16;
            slot.alpha_b = 0;
            slot.distance = step * 0.2 + if delta_pos2[1] == 0.0 { 3.4 } else { 2.4 };
            slot.rise_angle = ((11 - index as i32) * 5) as i16;
            slot.vec_b_now[1] = 0.0;

            for value in 0..21usize {
                slot.height[value] = 0.0;
                slot.flag1[value] = 0;
            }

            slot.alpha_t = 99;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn stin3_launches_the_textureless_sma_primitive() {
        let mut effect = Stin3::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);
        runtime.position = [10.0, 20.0, 30.0];
        runtime.target_offset = [40.0, 20.0, 60.0];

        // `on_process` alone: the same-frame primitive pass would already have
        // advanced every slot's `process` counter (`PrimSMA`).
        assert!(effect.on_process(&mut runtime));

        let sma = &runtime.primitives[0].state;
        assert_eq!(sma.count, PrimitiveId::Sma);
        assert_eq!(sma.duration, 200);
        assert_eq!(sma.total_texture, 0);
        assert_eq!(sma.texture, None, "the launcher draws nothing itself");
        assert_eq!(sma.delta_pos, [40.0, 20.0, 60.0]);
        assert_eq!(sma.position, [10.0, 20.0, 30.0]);

        for (index, slot) in sma.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.process, index as i32 - 3);
            assert_eq!(slot.flag1[0], 1, "SMA(1) is EF_STIN3's flag");
            assert_eq!(slot.height[1], 0.8);
            assert_eq!(slot.rot_start, 0);
            assert_eq!(slot.vec_b_now[0], 10.0);
            assert_eq!(slot.vec_b_now[2], 30.0);
            // `render.position.y - 10 + ClampedRandom(5)` stays in [10, 15).
            assert!((10.0..15.0).contains(&slot.vec_b_now[1]));
        }
    }

    #[test]
    fn the_sma_body_runs_its_launch_only_on_frame_zero() {
        let mut effect = Stin3::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
    }

    #[test]
    fn stin3_plays_the_energy_wave_once_on_frame_zero() {
        let mut effect = Stin3::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);

        step(&mut effect, &mut runtime);

        assert_eq!(
            runtime.sounds,
            vec!["effect\\T_에너지방출.wav".to_owned()],
            "the literal keeps EF_STIN3's capital T"
        );
        assert!(runtime.positional_sounds.is_empty(), "`PlayEffectWave` is the inline channel");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds.len(), 1, "frame 0 only");
    }

    #[test]
    fn sma2_launches_the_blue_ring_stack() {
        let mut effect = TexturedBlueRing::new("effect\\ring_blue.tga", 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 7);
        runtime.target_offset = [3.0, 4.0, 5.0];
        runtime.delta_pos2 = [12.0, 0.0, 0.0];

        step(&mut effect, &mut runtime);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Sma2);
        assert_eq!(ring.duration, 100);
        assert_eq!(ring.render_flag, 5);
        assert_eq!(ring.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(ring.size, 4.0);
        assert_eq!(ring.delta_pos, [3.0, 4.0, 5.0]);
        assert_eq!(ring.delta_pos2, [12.0, 0.0, 0.0]);
        assert_eq!(ring.slots[0].max_height, 13.0);
        assert_eq!(ring.slots[1].max_height, 11.0);
        assert_eq!(ring.slots[2].max_height, 9.0);
        assert_eq!(ring.slots[0].rise_angle, 55);
        assert_eq!(ring.slots[2].rise_angle, 45);
        // `deltaPos2.y == 0` keeps the outer ring distances 3.4/3.6/3.8.
        assert!((ring.slots[0].distance - 3.4).abs() < 1e-6);
        assert!((ring.slots[2].distance - 3.8).abs() < 1e-6);
        assert_eq!(ring.slots[3].life, 0);
        // `TexturedBlueRing` never writes `alphaT`, so the blue branch is used.
        assert!(ring.slots.iter().all(|slot| slot.alpha_t == 0));
    }

    #[test]
    fn a_non_zero_launcher_extra_shrinks_the_ring_distances() {
        let mut effect = TexturedBlueRing::new("effect\\ring_blue.tga", 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 7);
        runtime.delta_pos2 = [12.0, 1.0, 0.0];

        step(&mut effect, &mut runtime);

        let ring = &runtime.primitives[0].state;
        assert!((ring.slots[0].distance - 2.4).abs() < 1e-6);
        assert!((ring.slots[2].distance - 2.8).abs() < 1e-6);
    }

    #[test]
    fn black_ring_stack_marks_the_grey_render_branch() {
        let mut effect = BlueRingStack::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 7);
        runtime.target_offset = [1.0, 2.0, 3.0];
        runtime.delta_pos2 = [20.0, 1.0, 0.0];

        step(&mut effect, &mut runtime);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Sma2);
        assert_eq!(ring.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(ring.delta_pos, [1.0, 2.0, 3.0]);
        assert_eq!(ring.delta_pos2, [20.0, 1.0, 0.0]);
        assert!(ring.slots[..3].iter().all(|slot| slot.alpha_t == 99));
        assert_eq!(ring.slots[3].life, 0);
        assert!((ring.slots[1].distance - 2.6).abs() < 1e-6);
    }
}
