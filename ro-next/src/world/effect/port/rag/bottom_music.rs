//! `CRagEffect::Bottom_Music(const char* name, char kind)` -
//! `RagEffect.cpp:12276-12384` - and `CRagEffect::Bottom_Out(const char* name,
//! char kind)` - `RagEffect.cpp:11938-12036`.
//!
//! Both are spawn-once bodies of the bard/dancer "bottom" songs: frame 0 arms
//! one field-panel primitive (`PP_BOTTOM` 47 for `Bottom_Music`, `PP_BOTTOMOUT`
//! 49 for `Bottom_Out`) with the caller's texture, then the motion in
//! [`super::super::primitive::bottom`] animates it and `RenderFieldPanel`
//! draws it. Later frames only refresh `render.position` from the caster;
//! they do not spawn anything.
//!
//! `kind` selects texture, slot distances and the panel's colour case
//! (`slots[0].flag1[2]`):
//!
//! | kind | source call | texture | distance | flag1[2] |
//! | --- | --- | --- | --- | --- |
//! | 1 | Lullaby | caller | 5 | 2 |
//! | 2 | Drumbattlefield | `melody_a/b` (RNG 2) | 3 | 2 + 3 trail slots |
//! | 3 | Richmankim | caller | 3 | 3 |
//! | 4 | Fortunekiss | caller | 15 | 2 + `height[9] = 10` |
//! | 5 | Intoabyss | one of three gems (RNG 3) | 3 | 4 |
//! | 6 | Appleidun | caller | 8 | 4 |
//! | 7 | Ringnibelungen | caller | 3 | 7 (spins 10/frame) |
//! | 8 | Poembragi | `spell_01..08` (RNG 8) | 3 | 2 |
//!
//! The RNG order is part of the body: the kind-5/2/8 texture draw happens
//! *before* the single `ClampedRandom(360)` for `rotStart`.
//!
//! `Bottom_Out` is the same panel geometry with one or two slots, the
//! `ClampedRandom(11)` start phase and a `ClampedRandom(6)` /
//! `ClampedRandom(20)` distance (kind 0 / 1). Every slot draws `process` first,
//! then `distance`.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::Bottom_Music` - one prim 47 field panel.
#[derive(Debug)]
pub struct BottomMusic {
    name: &'static str,
    kind: i8,
    lifetime: i32,
}

impl BottomMusic {
    pub fn new(name: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { name, kind, lifetime }
    }

    /// `kind == 5` - one of the three gem textures.
    fn gem_texture(runtime: &mut EffectRuntime) -> &'static str {
        match runtime.rng.clamped_random(3) {
            0 => "effect\\레드젬스톤.bmp",
            1 => "effect\\블루젬스톤.bmp",
            _ => "effect\\옐로우젬스톤.bmp",
        }
    }

    /// `kind == 2` - one of the two melody notes.
    fn melody_texture(runtime: &mut EffectRuntime) -> &'static str {
        if runtime.rng.clamped_random(2) == 0 {
            "effect\\melody_a.bmp"
        } else {
            "effect\\melody_b.bmp"
        }
    }

    /// `kind == 8` - one of the eight spell glyphs.
    fn spell_texture(runtime: &mut EffectRuntime) -> &'static str {
        const SPELLS: [&str; 8] = [
            "effect\\spell_01.bmp",
            "effect\\spell_02.bmp",
            "effect\\spell_03.bmp",
            "effect\\spell_04.bmp",
            "effect\\spell_05.bmp",
            "effect\\spell_06.bmp",
            "effect\\spell_07.bmp",
            "effect\\spell_08.bmp",
        ];

        SPELLS[runtime.rng.clamped_random(8) as usize]
    }

    /// The `Distance` chain of `RagEffect.cpp:12343-12353`.
    fn distance(kind: i8) -> f32 {
        match kind {
            1 => 5.0,
            4 => 15.0,
            6 => 8.0,
            9 => 1.5,
            _ => 3.0,
        }
    }
}

impl PortedEffect for BottomMusic {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::Bottom_Music` - `RagEffect.cpp:12276-12384`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = this->state.parent->render.position;`
        // (the runtime anchor already is that position)

        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        let kind = self.kind;
        let lifetime = runtime.lifetime;
        let position = EffectRuntime::official_position(runtime.position);

        let index = runtime.launch(PrimitiveId::Bottom, [0.0, 0.0, 0.0]);

        // The texture branches draw before `rotStart` below, exactly like the
        // original's source order. Both draws happen before the primitive
        // borrow so the RNG and the state can be borrowed separately.
        let texture = if kind == 5 {
            Self::gem_texture(runtime)
        } else if kind == 2 {
            Self::melody_texture(runtime)
        } else if kind == 8 {
            Self::spell_texture(runtime)
        } else {
            self.name
        };
        let rot_start = runtime.rng.clamped_random(360) as i16;

        let state = runtime.primitive_mut(index);

        // `RSF_ALPHA | RSF_CUSTOM_BLEND`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(texture);

        state.size = 0.0;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        let slot = &mut state.slots[0];

        slot.life = 1;
        slot.process = 0;
        slot.distance = Self::distance(kind);
        slot.height[8] = 0.0;
        slot.alpha_b = 200;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_pre[1] = position[1] - 6.0;
        slot.rot_start = rot_start;
        slot.full_display_angle = 45;
        slot.flag1[0] = 0;
        slot.flag1[2] = 2;
        slot.height[7] = 0.0;

        if kind == 4 {
            slot.height[9] = 10.0;

            return true;
        }

        if kind == 2 {
            for slot in &mut state.slots[1..] {
                slot.life = 1;
                slot.alpha_b = 0;
            }

            return true;
        }

        if kind == 3 {
            state.slots[0].flag1[2] = 3;

            return true;
        }

        if kind == 5 || kind == 6 {
            state.slots[0].flag1[2] = 4;
        } else if kind == 7 {
            state.slots[0].flag1[2] = 7;
        } else if kind == 9 {
            state.slots[0].flag1[2] = 9;
            state.slots[0].flag1[3] = 6;
        }

        true
    }
}

/// `CRagEffect::Bottom_Out` - one or two prim 49 field panels.
#[derive(Debug)]
pub struct BottomOut {
    name: &'static str,
    kind: i8,
    lifetime: i32,
}

impl BottomOut {
    pub fn new(name: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { name, kind, lifetime }
    }
}

impl PortedEffect for BottomOut {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::Bottom_Out` - `RagEffect.cpp:11938-12036`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, parent.render.position);`
        // (the runtime anchor already is that position)

        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        let kind = self.kind;
        let lifetime = runtime.lifetime;
        let position = EffectRuntime::official_position(runtime.position);

        let index = runtime.launch(PrimitiveId::BottomOut, [0.0, 0.0, 0.0]);

        // `int count = 2; if (kind == 1) count = 1;`
        let count = if kind == 1 { 1 } else { 2 };

        // Every slot draws `process` first, then (for kind 0/1) `distance`.
        // The draws happen before the primitive borrow for the same reason as
        // `BottomMusic`.
        let mut randoms = [(0i32, 0.0f32); 2];

        for random in randoms.iter_mut().take(count) {
            let process = runtime.rng.clamped_random(11);

            if kind == 0 {
                random.1 = runtime.rng.clamped_random(6) as f32 + 5.0;
            } else if kind == 1 {
                random.1 = runtime.rng.clamped_random(20) as f32 * 0.1 + 3.0;
            }

            random.0 = process;
        }

        let state = runtime.primitive_mut(index);

        // `RSF_ALPHA | RSF_CUSTOM_BLEND`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.name);

        for slot in &mut state.slots {
            slot.life = 0;
        }

        for (index, slot) in state.slots.iter_mut().take(count).enumerate() {
            slot.life = 1;
            slot.process = randoms[index].0;
            slot.distance = randoms[index].1;

            slot.height[8] = 0.0;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = position[0];
            slot.vec_b_now[2] = position[2];

            if kind == 0 {
                slot.vec_b_pre[1] = position[1];
            } else if kind == 1 {
                slot.vec_b_pre[1] = position[1] - 5.0;
            }

            slot.full_display_angle = 45;
            slot.flag1[0] = 0;
            slot.flag1[2] = 2;
            slot.height[7] = 0.0;
            slot.flag1[1] = kind;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, PortedEffect};

    #[test]
    fn lullaby_arms_the_panel_on_frame_zero() {
        let mut effect = BottomMusic::new("effect\\zz.bmp", 1, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 7);
        runtime.position = [10.0, 20.0, 30.0];

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Bottom);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, 29999);
        assert_eq!(state.texture, Some("effect\\zz.bmp"));
        assert_eq!(state.size, 0.0);
        assert_eq!(state.slots[1].life, 0);
        assert_eq!(state.slots[2].life, 0);
        assert_eq!(state.slots[3].life, 0);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 0);
        assert_eq!(slot.distance, 5.0);
        assert_eq!(slot.alpha_b, 200);
        assert_eq!(slot.height[8], 0.0);
        assert_eq!(slot.height[7], 0.0);
        assert_eq!(slot.vec_b_now[0], 10.0);
        assert_eq!(slot.vec_b_now[2], 30.0);
        // Official Y-down: port y 20 is -20, then the -6 lift.
        assert_eq!(slot.vec_b_pre[1], -26.0);
        assert_eq!(slot.full_display_angle, 45);
        assert_eq!(slot.flag1[0], 0);
        assert_eq!(slot.flag1[2], 2);

        // Frame 1 changes nothing and spawns no second primitive.
        runtime.frame = 1;
        effect.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 0);
    }

    #[test]
    fn drumbattlefield_trims_the_second_melody_and_arms_three_trail_slots() {
        let seed = 1234;
        let mut reference = crate::world::effect::port::rng::EffectRng::new(seed);
        let expected_texture = if reference.clamped_random(2) == 0 {
            "effect\\melody_a.bmp"
        } else {
            "effect\\melody_b.bmp"
        };

        let mut effect = BottomMusic::new("", 2, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], seed);

        effect.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture, Some(expected_texture));
        assert_eq!(state.slots[0].distance, 3.0);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 0);
        }
    }

    #[test]
    fn poembragi_cycles_the_eight_spell_glyphs() {
        for seed in 0..8u64 {
            let mut reference = crate::world::effect::port::rng::EffectRng::new(seed);
            let pick = reference.clamped_random(8);
            let expected = format!("effect\\spell_0{}.bmp", pick + 1);

            let mut effect = BottomMusic::new("", 8, 29999);
            let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], seed);
            effect.on_process(&mut runtime);

            assert_eq!(runtime.primitives[0].state.texture, Some(expected.as_str()), "seed {seed}");
        }
    }

    #[test]
    fn intoabyss_picks_one_of_the_three_gems_before_the_rotation() {
        let seed = 99;
        let mut reference = crate::world::effect::port::rng::EffectRng::new(seed);
        let expected_texture = match reference.clamped_random(3) {
            0 => "effect\\레드젬스톤.bmp",
            1 => "effect\\블루젬스톤.bmp",
            _ => "effect\\옐로우젬스톤.bmp",
        };
        let expected_rotation = reference.clamped_random(360) as i16;

        let mut effect = BottomMusic::new("", 5, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], seed);
        effect.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture, Some(expected_texture));
        assert_eq!(state.slots[0].rot_start, expected_rotation);
        assert_eq!(state.slots[0].flag1[2], 4);
    }

    #[test]
    fn kind_five_and_six_use_the_glow_panel_and_kind_seven_spins() {
        let mut six = BottomMusic::new("effect\\idun_apple.bmp", 6, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 1);
        six.on_process(&mut runtime);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture, Some("effect\\idun_apple.bmp"));
        assert_eq!(state.slots[0].distance, 8.0);
        assert_eq!(state.slots[0].flag1[2], 4);

        let mut seven = BottomMusic::new("effect\\twirl.bmp", 7, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 1);
        seven.on_process(&mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 7);
    }

    #[test]
    fn fortunekiss_offsets_the_panel_and_richmankim_uses_case_three() {
        let mut four = BottomMusic::new("effect\\kiss.bmp", 4, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 1);
        four.on_process(&mut runtime);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].distance, 15.0);
        assert_eq!(state.slots[0].height[9], 10.0);
        assert_eq!(state.slots[0].flag1[2], 2);

        let mut three = BottomMusic::new("effect\\pocket.bmp", 3, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 1);
        three.on_process(&mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 3);
    }

    #[test]
    fn bottom_out_arms_two_slots_for_kind_zero() {
        let seed = 4242;
        let mut reference = crate::world::effect::port::rng::EffectRng::new(seed);
        let expected = [
            (reference.clamped_random(11), reference.clamped_random(6)),
            (reference.clamped_random(11), reference.clamped_random(6)),
        ];

        let mut effect = BottomOut::new("effect\\safeline.bmp", 0, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], seed);
        runtime.position = [3.0, 4.0, 5.0];

        effect.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::BottomOut);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.texture, Some("effect\\safeline.bmp"));

        for index in 0..2 {
            let slot = &state.slots[index];
            assert_eq!(slot.life, 1);
            assert_eq!(slot.process, expected[index].0);
            assert_eq!(slot.distance, expected[index].1 as f32 + 5.0);
            assert_eq!(slot.alpha_b, 0);
            assert_eq!(slot.height[8], 0.0);
            assert_eq!(slot.vec_b_now[0], 3.0);
            assert_eq!(slot.vec_b_now[2], 5.0);
            // Official Y-down: the parent's port y of 4 is -4 here.
            assert_eq!(slot.vec_b_pre[1], -4.0);
            assert_eq!(slot.full_display_angle, 45);
            assert_eq!(slot.flag1[1], 0);
            assert_eq!(slot.flag1[2], 2);
        }

        assert_eq!(state.slots[2].life, 0);
        assert_eq!(state.slots[3].life, 0);
    }

    #[test]
    fn bottom_out_kind_one_arms_one_lifted_slot() {
        let seed = 77;
        let mut reference = crate::world::effect::port::rng::EffectRng::new(seed);
        let expected_process = reference.clamped_random(11);
        let expected_distance = reference.clamped_random(20) as f32 * 0.1 + 3.0;

        let mut effect = BottomOut::new("effect\\ring.tga", 1, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], seed);
        runtime.position = [1.0, 9.0, 2.0];

        effect.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].life, 1);
        assert_eq!(state.slots[0].process, expected_process);
        assert_eq!(state.slots[0].distance, expected_distance);
        // Official Y-down: port y 9 is -9, then the kind's -5 lift.
        assert_eq!(state.slots[0].vec_b_pre[1], -14.0);
        assert_eq!(state.slots[0].flag1[1], 1);
        assert_eq!(state.slots[1].life, 0);
    }
}
