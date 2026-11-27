//! Effects ported by the "k" parallel batch (ids whose enum variant was
//! missing).
//!
//! These are the dispatch arms that were already ported *in spirit* - the
//! shared body exists and is registered for its sibling ids - but whose
//! official id had no `EffectId` variant, so `registry.rs` could not name them.
//!
//! | effect | id | dispatch | shared body |
//! | --- | --- | --- | --- |
//! | `EF_PEONG_BLACK` | 1207 | `RagEffect.cpp:22254-22261` | `CRagEffect::SonicSlashFan(1)` (`:16587-16641`) |
//!
//! The other ids in this batch (`EF_BLOODDRAIN2` 1052, `EF_NEW_HEAVENS_DRIVE`
//! 1399, `EF_HEAL_BLUE_2` 2427, `EF_RG_COIN_BLUE` 2436, `EF_RG_COIN_BLUE_2`
//! 2437) are pure aliases of a body that is already public, so they live
//! entirely in `registry_batch_k.rs`.
//!
//! ## Ring aliases
//!
//! [`SizedRing`] ports the 21 dispatch arms that are one or two
//! `CRagEffect::TexturedQuadRingSized` calls with fixed arguments. The helper
//! already existed in `rag/asura.rs` but was private; it is now `pub(crate)`
//! and unchanged. The official arms are:
//!
//! | effect | id | dispatch | lifetime |
//! | --- | --- | --- | --- |
//! | `EF_NEW_MOON_SLASH` | 1418 | `RagEffect.cpp:22334-22341` | 1418 (`EffectTypes.cpp:1427`) |
//! | `EF_NEW_HEAL_BLUE` | 1556 | `:22419-22431` | 60 (`:1565`) |
//! | `EF_NEW_HEAL_WHITE` | 1560 | `:22457-22464` | 40 (`:1569`) |
//! | `EF_NEW_HEAL_YELLOW` | 1565 | `:22465-22472` | 40 (`:1574`) |
//! | `EF_NEW_SAFETYWALL` | 1677 | `:22610-22622` | 60 (`:1686`) |
//! | `EF_NEW_PNEUMA` | 1872 | `:22638-22645` | 1872 (`:1881`) |
//! | `EF_NEW_SW_CAST` | 1876 | `:22646-22653` | 1876 (`:1885`) |
//! | `EF_ABR_MOTHER` | 1902 | `:22688-22695` | 1902 (`:1911`) |
//! | `EF_ABR_INFINITY_RING` | 1910 | `:22654-22663` | 1910 (`:1919`) |
//! | `EF_NEW_D_CANNON_ENTITY` | 1918 | `:22672-22679` | 1918 (`:1927`) |
//! | `EF_FLAME_TRAP_RING` | 1923 | `:22432-22444` | 1923 (`:1932`) |
//! | `EF_NEW_PRAMEN_RING` | 1932 | `:22445-22456` | 1932 (`:1941`) |
//! | `EF_NEW_BATTLE_W_ENTITY` | 1938 | `:22664-22671` | 1938 (`:1947`) |
//! | `EF_PNEUMATICUS_PROCELLA_RING` | 1959 | `:22680-22687` | 1959 (`:1968`) |
//! | `EF_SINCERE_FAITH_RING` | 1963 | `:22696-22703` | 1963 (`:1972`) |
//! | `EF_POWERFUL_FAITH_RING` | 1986 | `:22704-22711` | 1986 (`:1995`) |
//! | `EF_FIRM_FAITH_RING` | 1998 | `:22712-22719` | 1998 (`:2007`) |
//! | `EF_RESEARCH_REPORT_RING` | 2240 | `:22720-22727` | 2240 (`:2249`) |
//! | `EF_WOODEN_WARRIOR_RING` | 2249 | `:22728-22735` | 2249 (`:2258`) |
//! | `EF_WOODEN_FAIRY_RING` | 2255 | `:22736-22743` | 2255 (`:2264`) |
//! | `EF_BEGINSPELL_RING_YELLOW` | 2367 | `:22818-22826` | 40 (`:2376`) |
//!
//! The lifetime entries that repeat the effect id are the decompiled table's
//! literal values, not typos. Four arms (`EF_NEW_HEAL_BLUE`,
//! `EF_FLAME_TRAP_RING`, `EF_NEW_PRAMEN_RING`, `EF_NEW_SAFETYWALL`) call raw
//! `PlayWave(..., -100.0f, ...)` instead of `PlayEffectWave`, so [`RingSound`]
//! keeps the two sound channels distinct. `EF_ABR_INFINITY_RING` is the only
//! arm wrapped in `if (this->state.parent != nullptr)`; the hermetic runtime
//! has no null-parent state (its `position` is always the caster anchor), so
//! that guard's false arm is unreachable and the arm runs unconditionally.
//!
//! ## Why `SonicSlashFan(1)` is transliterated here
//!
//! `rag/sonic_slash_fan.rs` keeps `sonic_slash_fan` private and exposes only
//! the two fixed-kind wrappers (`EnergyDrain3` = kind 0, `EnergyDrainBlack` =
//! kind 2); that module is owned by another round. The kind-1 branch below is
//! a byte-for-byte transliteration of the same `RagEffect.cpp:16587-16641`
//! body, including the RNG order (face x draw(7), face z draw(7), grey
//! draw(55), size draw(6)) and the zero-radius `CalcDist` quirk
//! (`lastPrim->CalcDist(render.position)` measures the freshly seeded
//! primitive against its own anchor, so it is always exactly zero).

#![allow(dead_code)]

use super::angles::Angles;
use super::math::matrix_mult;
use super::primitive::PrimitiveId;
use super::rag::asura::textured_quad_ring_sized;
use super::runtime::{EffectRuntime, PortedEffect};

/// The sound call the official dispatch arm makes on frame 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingSound {
    /// `PlayEffectWave(*this, "effect\\EF_BeginSpell.wav")` - an inline wave.
    Wave,
    /// Raw `g_soundMgr->PlayWave("effect\\EF_BeginSpell.wav", dx, -100.0f,
    /// dz, 250, 40, 1.0f, 0)` - a positional wave at height `-100`.
    MinusOneHundred,
}

/// A dispatch arm that is nothing but the already-ported
/// `CRagEffect::TexturedQuadRingSized` (`rag/asura.rs`, `RagEffect.cpp:17437`)
/// with fixed arguments.
///
/// Every arm in the ring-alias table writes `render.position =
/// state.parent->render.position` first; [`EffectRuntime::position`] already is
/// that anchor, so there is nothing left to copy. The official arms then gate
/// on `stateCnt == 0`; the helper repeats that gate itself, and it is what
/// actually decides whether a ring launches.
#[derive(Debug)]
pub struct SizedRing {
    lifetime: i32,
    sound: RingSound,
    calls: &'static [(i16, &'static str, i8)],
}

impl SizedRing {
    pub const fn new(lifetime: i32, sound: RingSound, calls: &'static [(i16, &'static str, i8)]) -> Self {
        Self { lifetime, sound, calls }
    }
}

impl PortedEffect for SizedRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            match self.sound {
                RingSound::Wave => runtime.play_sound("effect\\EF_BeginSpell.wav"),
                RingSound::MinusOneHundred => runtime.play_sound_at_height("effect\\EF_BeginSpell.wav", -100.0),
            }
        }

        for &(base_alpha, name, kind) in self.calls {
            textured_quad_ring_sized(runtime, base_alpha, name, kind);
        }

        true
    }
}

/// `SonicSlashFan(1)` - `RagEffect.cpp:16587-16641`, kind-1 branch at
/// `:16622-16626`.
fn sonic_slash_fan_kind_one(runtime: &mut EffectRuntime) {
    if runtime.frame > 4 {
        return;
    }

    let angles = Angles::shared();
    let position = runtime.position;
    let mut angle = -90;

    while angle < 180 {
        // `Face(ClampedRandom(7) + render.position.x - 3.0f,
        //       ClampedRandom(7) + render.position.z - 3.0f)`.
        // Decompiled argument order: x draw first, then z draw.
        let face_x = runtime.rng.clamped_random(7) as f32 + position[0] - 3.0;
        let face_z = runtime.rng.clamped_random(7) as f32 + position[2] - 3.0;
        let face = if face_x != position[0] || face_z != position[2] {
            EffectRuntime::heading(position[0], position[2], face_x, face_z)
        } else {
            0.0
        };

        // `kind == 1`: one grey `ClampedRandom(55) + 200`.
        let colour = runtime.rng.clamped_random(55) + 200;
        let size = runtime.rng.clamped_random(6) as f32 * 0.1 + 1.0;

        let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spline);

        state.duration = 60;
        state.delta_pos2 = [0.0, -10.0, 0.0];
        // Quirk: the primitive's own anchor and the effect anchor are the same
        // position, so `CalcDist` is zero.
        state.radius = 0.0;
        state.render_angle = face;
        state.longitude = -state.render_angle;
        let speed = state.radius / state.duration as f32;
        state.speed = speed + speed;
        state.grav_speed = 2.0;
        let rise = -(state.grav_speed / state.duration as f32);
        state.grav_accel = rise + rise;
        state.lati_speed = -2.0;
        let spread = -(state.lati_speed / state.duration as f32);
        state.lati_accel = spread + spread;
        state.roll = angle as f32;
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos3 = matrix_mult([0.0, 0.0, 1.0], &state.transform.values);
        state.rot_axis = state.delta_pos3;
        state.red = colour;
        state.green = colour;
        state.blue = colour;
        state.size = size;
        state.sprite = Some("이팩트\\particle1.spr");
        state.action = Some("이팩트\\particle1.act");
        state.num_segment = 7;
        state.init_segment();

        angle += 90;
    }
}

/// `EF_PEONG_BLACK` (1207) - `CRagEffect::SonicSlashFan(1)` plus the caster
/// tint of `RagEffect.cpp:22254-22261`.
///
/// Lifetime 200 (`EffectTypes.cpp:1216`).
#[derive(Debug)]
pub struct PeongBlack {
    lifetime: i32,
}

impl PeongBlack {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for PeongBlack {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        sonic_slash_fan_kind_one(runtime);

        if (50..=80).contains(&runtime.frame) {
            runtime.add_parent_flags(8);
            let channel = ((125 - runtime.frame) * 2) as i16;
            runtime.set_parent_color(-1, channel, channel, 250);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::EffectId;

    use super::*;
    use crate::world::effect::port::primitive::EffectPrimitiveState;
    use crate::world::effect::port::registry_batch_k;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime(seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], seed);
        runtime.position = [10.0, 0.0, 10.0];

        runtime
    }

    fn ring_runtime(lifetime: i32) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], 21);
        runtime.position = [10.0, 0.0, 10.0];

        runtime
    }

    fn spawn_ring(effect: EffectId) -> Box<dyn PortedEffect> {
        registry_batch_k::spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"))
    }

    /// The size table of `TexturedQuadRingSized` (`RagEffect.cpp:17437-17608`).
    fn expected_size(kind: i8) -> f32 {
        match kind {
            1 => 8.0,
            2 => 5.0,
            3 => 10.0,
            4 => 11.0,
            5 => 4.0,
            6 => 12.0,
            15 => 7.0,
            20 => 24.0,
            21 => 25.0,
            22 => 26.0,
            23 => 27.0,
            31 => 31.0,
            42 => 42.0,
            43 => 43.0,
            44 => 44.0,
            45 => 45.0,
            46 => 46.0,
            47 => 47.0,
            48 => 48.0,
            49 => 49.0,
            50 => 50.0,
            _ => 5.0,
        }
    }

    fn assert_ring(state: &EffectPrimitiveState, texture: &str, kind: i8, base_alpha: i16, duration: i32, stagger: bool) {
        assert_eq!(state.count, PrimitiveId::Casting3d);
        assert_eq!(state.count.id(), 30);
        assert_eq!(state.texture, Some(texture));
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, duration);
        assert_eq!(state.size, expected_size(kind));

        let max_height = if kind == 2 {
            [25.0, 24.0, 23.0, 22.0]
        } else if kind == 22 {
            [15.0, 14.0, 13.0, 12.0]
        } else {
            [20.0, 19.0, 18.0, 17.0]
        };
        let rot_start = [180i16, 270, 0, 90];
        let alpha_bonus = [135i16, 90, 45, 0];

        for (index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.full_display_angle, 360);
            assert_eq!(slot.max_height, max_height[index]);
            assert_eq!(slot.rot_start, rot_start[index]);
            assert_eq!(slot.distance, 4.1);
            assert_eq!(slot.rise_angle, 80);
            assert_eq!(slot.alpha_t, kind as i16);
            assert_eq!(slot.vec_b_now, [0.0; 3]);
            assert_eq!(slot.height, [0.0; 21]);
            assert_eq!(slot.flag1, [0; 21]);

            if stagger {
                assert_eq!(slot.alpha_b, 0);
                assert_eq!(slot.process, index as i32 * -5);
            } else {
                assert_eq!(slot.alpha_b, base_alpha + alpha_bonus[index]);
                assert_eq!(slot.process, 0);
            }
        }
    }

    fn assert_inline_sound(runtime: &EffectRuntime) {
        assert_eq!(runtime.sounds, vec!["effect\\EF_BeginSpell.wav".to_owned()]);
        assert!(runtime.positional_sounds.is_empty());
    }

    fn assert_positional_sound(runtime: &EffectRuntime) {
        assert!(runtime.sounds.is_empty());
        assert_eq!(runtime.positional_sounds, vec![("effect\\EF_BeginSpell.wav".to_owned(), [
            10.0, -100.0, 10.0
        ])]);
    }

    #[test]
    fn single_ring_shapes_match_their_official_arguments() {
        // `RagEffect.cpp:22334-22341` and `:22445-22456`.
        let cases = [
            (
                EffectId::NewMoonSlash,
                1418,
                "effect\\new_comet\\new_comet_cast_ring\\ring_rainbow.tga",
                20,
                45,
                false,
            ),
            (EffectId::NewPramenRing, 1932, "effect\\new_pramen\\ring_blue.tga", 5, 90, true),
        ];

        for (effect, lifetime, texture, kind, base_alpha, positional) in cases {
            let mut body = spawn_ring(effect);
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");

            let mut runtime = ring_runtime(lifetime);
            body.on_process(&mut runtime);

            assert_eq!(runtime.primitives.len(), 1, "{effect:?}");
            assert_ring(&runtime.primitives[0].state, texture, kind, base_alpha, lifetime, false);
            // A lifetime above 60 keeps the helper from raising the duration.
            assert_eq!(runtime.lifetime, lifetime);

            if positional {
                assert_positional_sound(&runtime);
            } else {
                assert_inline_sound(&runtime);
            }
        }
    }

    #[test]
    fn short_double_ring_shapes_stagger_and_lose_their_alpha() {
        // `RagEffect.cpp:22419-22431`, `:22457-22464`, `:22465-22472`,
        // `:22610-22622` and `:22818-22826`.
        let cases = [
            (EffectId::NewHealBlue, 60, "effect\\ring_blue.tga", [20, 21], 60, true),
            (EffectId::NewHealWhite, 40, "effect\\ring_white.tga", [22, 22], 56, false),
            (EffectId::NewHealYellow, 40, "effect\\ring_yellow.tga", [23, 23], 56, false),
            (EffectId::NewSafetywall, 60, "effect\\ring_white.tga", [31, 31], 60, true),
            (EffectId::BeginspellRingYellow, 40, "effect\\ring_yellow.tga", [0, 0], 56, false),
        ];

        for (effect, lifetime, texture, kinds, duration, positional) in cases {
            let mut body = spawn_ring(effect);
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");

            let mut runtime = ring_runtime(lifetime);
            body.on_process(&mut runtime);

            assert_eq!(runtime.primitives.len(), 2, "{effect:?}");
            assert_eq!(runtime.lifetime, duration, "{effect:?}");

            assert_ring(&runtime.primitives[0].state, texture, kinds[0], 45, duration, true);
            assert_ring(&runtime.primitives[1].state, texture, kinds[1], 25, duration, true);

            if positional {
                assert_positional_sound(&runtime);
            } else {
                assert_inline_sound(&runtime);
            }
        }
    }

    #[test]
    fn long_double_ring_shapes_keep_their_official_alpha_and_textures() {
        // The remaining `TexturedQuadRingSized` arms: two 45/25 rings each,
        // with the decompiled texture and kind per id.
        let white = "effect\\ring_white.tga";
        let cases = [
            (EffectId::NewPneuma, 1872, white, 42, false),
            (EffectId::NewSwCast, 1876, white, 43, false),
            (EffectId::AbrMother, 1902, "effect\\new_abr_mother\\abr_mother.tga", 0, false),
            (
                EffectId::AbrInfinityRing,
                1910,
                "effect\\new_abr_infinity\\ring_infinty.tga",
                0,
                false,
            ),
            (
                EffectId::NewDCannonEntity,
                1918,
                "effect\\new_d_cannon\\ABR_DUAL_CANNON.tga",
                0,
                false,
            ),
            (EffectId::FlameTrapRing, 1923, "effect\\ring_red.tga", 44, true),
            (
                EffectId::NewBattleWEntity,
                1938,
                "effect\\new_battle_w\\ABR_BATTLE_WARIOR.tga",
                0,
                false,
            ),
            (
                EffectId::PneumaticusProcellaRing,
                1959,
                "effect\\pneumaticusprocella\\pneumaticusprocella_cast\\ring_yellow.tga",
                0,
                false,
            ),
            (EffectId::SincereFaithRing, 1963, white, 45, false),
            (EffectId::PowerfulFaithRing, 1986, white, 46, false),
            (EffectId::FirmFaithRing, 1998, white, 47, false),
            (EffectId::ResearchReportRing, 2240, white, 48, false),
            (EffectId::WoodenWarriorRing, 2249, white, 49, false),
            (EffectId::WoodenFairyRing, 2255, white, 50, false),
        ];

        for (effect, lifetime, texture, kind, positional) in cases {
            let mut body = spawn_ring(effect);
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");

            let mut runtime = ring_runtime(lifetime);
            body.on_process(&mut runtime);

            assert_eq!(runtime.primitives.len(), 2, "{effect:?}");
            assert_eq!(runtime.lifetime, lifetime, "{effect:?}");

            assert_ring(&runtime.primitives[0].state, texture, kind, 45, lifetime, false);
            assert_ring(&runtime.primitives[1].state, texture, kind, 25, lifetime, false);

            if positional {
                assert_positional_sound(&runtime);
            } else {
                assert_inline_sound(&runtime);
            }
        }
    }

    #[test]
    fn research_report_kind_48_overrides_the_size_chain() {
        // `RagEffect.cpp:22720-22727`; kind 48 is applied after the size table.
        let mut body = spawn_ring(EffectId::ResearchReportRing);
        let mut runtime = ring_runtime(2240);

        body.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 2);
        for primitive in &runtime.primitives {
            assert_eq!(primitive.state.size, 48.0);
        }
    }

    #[test]
    fn peong_black_spawns_three_grey_trails_on_the_first_five_frames() {
        let mut body = PeongBlack::new(200);
        let mut runtime = runtime(1207);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleSpline);
            assert_eq!(state.duration, 60);
            assert_eq!(state.roll, (index as i32 * 90 - 90) as f32);
            assert_eq!(state.red, state.green);
            assert_eq!(state.green, state.blue);
            assert!(state.red >= 200 && state.red < 255);
            assert_eq!(state.radius, 0.0);
            assert_eq!(state.speed, 0.0);
            assert_eq!(state.num_segment, 7);
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert_eq!(state.action, Some("이팩트\\particle1.act"));
        }

        for _ in 0..4 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 15);

        // Frame 5 is outside `stateCnt > 4`.
        step(&mut body, &mut runtime);
        assert_eq!(runtime.primitives.len(), 15);
    }

    #[test]
    fn peong_black_rng_order_is_face_x_face_z_grey_size() {
        let mut body = PeongBlack::new(200);
        let mut runtime = runtime(42);
        let mut expected = EffectRuntime::new(200, [0.0, 0.0], 42);
        let position = runtime.position;

        let mut draws = [(0.0f32, 0.0f32, 0i32, 0i32); 3];
        for draw in &mut draws {
            let face_x = expected.rng.clamped_random(7) as f32 + position[0] - 3.0;
            let face_z = expected.rng.clamped_random(7) as f32 + position[2] - 3.0;
            let colour = expected.rng.clamped_random(55) + 200;
            let size = expected.rng.clamped_random(6);

            *draw = (face_x, face_z, colour, size);
        }

        step(&mut body, &mut runtime);

        for (primitive, draw) in runtime.primitives.iter().zip(draws) {
            let state = &primitive.state;
            assert_eq!((state.red, state.green, state.blue), (draw.2, draw.2, draw.2));
            assert_eq!(state.size, draw.3 as f32 * 0.1 + 1.0);

            let expected_face = if draw.0 != position[0] || draw.1 != position[2] {
                EffectRuntime::heading(position[0], position[2], draw.0, draw.1)
            } else {
                0.0
            };
            assert_eq!(state.render_angle, expected_face);
        }
    }

    #[test]
    fn peong_black_tints_the_caster_on_frames_50_to_80() {
        let mut body = PeongBlack::new(200);
        let mut runtime = runtime(1207);

        runtime.frame = 49;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_color, None);

        runtime.frame = 50;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 150, 150, 250]));
        assert_eq!(runtime.parent_flags, 8);

        runtime.begin_step();
        runtime.frame = 81;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_flags, 0);
    }

    #[test]
    fn peong_black_lifetime_is_200() {
        // `EffectTypes.cpp:1216`
        assert_eq!(PeongBlack::new(200).lifetime(), 200);
    }
}
