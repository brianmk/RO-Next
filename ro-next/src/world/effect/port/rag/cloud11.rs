//! `CRagEffect::Cloud11Burst(kind)` - `RagEffect.cpp:16089-16157`.
//!
//! Four slot-driven rising wind columns using `cloud11.tga` (`PP_WIND`). Used
//! by the portal effects and the Storm Kick family.
//!
//! The helper itself only builds the primitive; the dispatch case adds the
//! per-`EF_*` layers. `EF_PORTAL4` (Wind Walk) is the one with a caster layer
//! (`RagEffect.cpp:23300-23311`):
//!
//! ```cpp
//! case EF_PORTAL4:
//!   this->Cloud11Burst(0);
//!   if (stateCnt == 0) { position = parent.position; PlayWave("effect\\윈드워크.wav"); }
//!   if (stateCnt >= 5 && stateCnt <= 25) {
//!     if ((parent->render.flags & 1) == 0) parent->render.flags += 1;
//!     parent->SetColor(-1, 220, 250, 220);
//!   }
//!   break;
//! ```
//!
//! That green body tint rides the port's [`EffectRuntime::set_parent_color`]
//! bridge ([`ParentTint`]) and requests `render.flags` bit 1 for the same
//! window.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The dispatch-level caster layers of an `EF_*` case built on
/// `Cloud11Burst`: an inclusive `stateCnt` window that requests
/// `render.flags` and calls `state.parent->SetColor(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentTint {
    /// First `stateCnt` of the window (inclusive).
    pub first_frame: i32,
    /// Last `stateCnt` of the window (inclusive).
    pub last_frame: i32,
    /// The four `SetColor(alpha, red, green, blue)` arguments verbatim; `-1`
    /// leaves a channel unchanged.
    pub color: [i16; 4],
    /// `render.flags` bits the case adds while the window is active.
    pub flags: u32,
}

#[derive(Debug)]
pub struct Cloud11Burst {
    kind: i32,
    lifetime: i32,
    /// `(frame, wave)` sounds the dispatch block schedules.
    waves: Vec<(i32, &'static str)>,
    /// Dispatch-level caster tint/flags window (`EF_PORTAL4` only).
    tint: Option<ParentTint>,
}

impl Cloud11Burst {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self {
            kind,
            lifetime,
            waves: Vec::new(),
            tint: None,
        }
    }

    pub fn with_waves(mut self, waves: &[(i32, &'static str)]) -> Self {
        self.waves = waves.to_vec();
        self
    }

    /// Adds the dispatch case's caster colour/flags window.
    pub fn with_tint(mut self, tint: ParentTint) -> Self {
        self.tint = Some(tint);
        self
    }
}

impl PortedEffect for Cloud11Burst {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        for wave in self.waves.iter().filter(|(at, _)| *at == frame) {
            runtime.play_sound(wave.1);
        }

        // `if (this->state.stateCnt >= 5 && this->state.stateCnt <= 25) { ... }`
        if let Some(tint) = self.tint
            && (tint.first_frame..=tint.last_frame).contains(&frame)
        {
            runtime.add_parent_flags(tint.flags);
            runtime.set_parent_color(tint.color[0], tint.color[1], tint.color[2], tint.color[3]);
        }

        if frame != 0 {
            return true;
        }

        // `CopyVector(this->render.position, this->state.parent->render.position);`
        runtime.copy_parent_position();

        let lifetime = runtime.lifetime;
        let kind = self.kind;

        // The distance/rise random draws happen per slot, in slot order, before
        // the primitive is mutated (same RNG order as the original).
        let mut distances = [0.0f32; 4];
        let mut rises = [0i16; 4];

        for index in 0..4usize {
            if kind >= 2 {
                distances[index] = if kind == 2 {
                    14.0 - index as f32 * 2.0
                } else if kind == 3 {
                    6.0 - index as f32
                } else {
                    runtime.rng.clamped_random(101) as f32 * 0.03 + 4.5
                };
            } else {
                distances[index] = if kind == 1 {
                    runtime.rng.clamped_random(101) as f32 * 0.01 + 2.5
                } else {
                    runtime.rng.clamped_random(101) as f32 * 0.03 + 4.5
                };
            }

            rises[index] = (runtime.rng.clamped_random(21) + 80) as i16;
        }

        let cloud = runtime.launch(PrimitiveId::Wind, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(cloud);

        state.duration = lifetime;
        state.size = if kind == 1 { 8.0 } else { 5.0 };
        state.texture = Some("effect\\cloud11.tga");

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 30;

            slot.max_height = if kind == 1 {
                index as f32 * 2.0 + 3.0
            } else if kind == 2 {
                48.0 - index as f32 * 5.0
            } else if kind == 3 {
                28.0 - index as f32 * 5.0
            } else {
                index as f32 * 2.0 + 5.0
            };

            slot.rot_start = (index as i16) * 90;
            slot.alpha_b = 0;

            slot.alpha_t = if kind >= 2 { 2 } else { 1 };
            slot.distance = distances[index];
            slot.rise_angle = rises[index];

            slot.height = [1.0; 21];
            slot.flag1 = [0; 21];
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn portal4_spawns_four_wind_slots() {
        let mut effect = Cloud11Burst::new(0, 200).with_waves(&[(0, "effect\\윈드워크.wav")]);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\윈드워크.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Wind);
        assert_eq!(state.duration, 200);
        assert_eq!(state.size, 5.0);
        assert_eq!(state.texture, Some("effect\\cloud11.tga"));

        for (index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            // Spawned at 30, one motion step added 3.
            assert_eq!(slot.full_display_angle, 33);
            assert_eq!(slot.rot_start, ((index as i16) * 90 + 5) % 360);
            assert_eq!(slot.alpha_t, 1);
            assert_eq!(slot.max_height, index as f32 * 2.0 + 5.0);
            // kind 0: (rand(101) * 0.03) + 4.5
            assert!((4.5..7.6).contains(&slot.distance));
            assert_eq!(slot.alpha_b, 10);
            assert!((80..=100).contains(&slot.rise_angle));
        }
    }

    #[test]
    fn portal5_uses_the_larger_size_and_kind_two_layout() {
        let mut effect = Cloud11Burst::new(1, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.size, 8.0);
        assert_eq!(state.slots[0].max_height, 3.0);
        assert_eq!(state.slots[0].alpha_t, 1);
        assert!((2.5..3.6).contains(&state.slots[0].distance));
    }

    #[test]
    fn storm_kick_uses_alpha_t_two_and_descending_heights() {
        let mut effect = Cloud11Burst::new(2, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].alpha_t, 2);
        assert_eq!(state.slots[0].max_height, 48.0);
        assert_eq!(state.slots[0].distance, 14.0);
    }

    #[test]
    fn slots_advance_once_per_frame() {
        let mut effect = Cloud11Burst::new(0, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 2);
    }

    /// `EF_PORTAL4`'s registration (`RagEffect.cpp:23300-23311`): kind 0, the
    /// inline Wind Walk wave on frame 0 and the caster tint/flags window.
    fn wind_walk() -> Cloud11Burst {
        Cloud11Burst::new(0, 200)
            .with_waves(&[(0, "effect\\윈드워크.wav")])
            .with_tint(ParentTint {
                first_frame: 5,
                last_frame: 25,
                color: [-1, 220, 250, 220],
                flags: 1,
            })
    }

    /// Layer 1: one `PP_WIND` primitive with the official four rising columns
    /// (`Cloud11Burst(0)`), heights 5/7/9/11, rotations 0/90/180/270.
    #[test]
    fn wind_walk_has_the_four_official_wind_columns() {
        let mut effect = wind_walk();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Wind);
        assert_eq!(state.duration, 200);
        assert_eq!(state.size, 5.0);
        assert_eq!(state.texture, Some("effect\\cloud11.tga"));

        for (index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1, "slot {index}");
            assert_eq!(slot.max_height, index as f32 * 2.0 + 5.0);
            assert_eq!(slot.rot_start, ((index as i16) * 90 + 5) % 360);
            assert_eq!(slot.alpha_t, 1);
            assert!((4.5..7.6).contains(&slot.distance));
            assert!((80..=100).contains(&slot.rise_angle));
        }
    }

    /// Layer 2: `effect\윈드워크.wav`, exactly once, on frame 0 and with the
    /// archive's single backslash separator.
    #[test]
    fn wind_walk_plays_the_official_wave_once_on_frame_zero() {
        let mut effect = wind_walk();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\윈드워크.wav".to_owned()]);

        // `step` does not drain the record channel (the holder does), so clear
        // it before the next frame to prove the wave does not replay.
        runtime.sounds.clear();
        step(&mut effect, &mut runtime);
        assert!(runtime.sounds.is_empty(), "the wave must not replay");
    }

    /// Layer 3: `SetColor(-1, 220, 250, 220)` + `render.flags += 1` on frames
    /// 5-25 only. The colour request persists after the window (the official
    /// `SetColor` mutates the actor), the flag word does not
    /// (`GameObject.cpp:435` clears it every frame).
    #[test]
    fn wind_walk_tints_the_caster_for_frames_five_to_twenty_five() {
        let mut effect = wind_walk();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        for frame in 0..=26 {
            step(&mut effect, &mut runtime);

            if (5..=25).contains(&frame) {
                assert_eq!(runtime.frame, frame + 1);
                assert_eq!(
                    runtime.parent_color,
                    Some([-1, 220, 250, 220]),
                    "frame {frame} must carry SetColor(-1, 220, 250, 220)"
                );
                assert_eq!(runtime.parent_flags & 1, 1, "frame {frame} must request render.flags bit 1");
            } else if frame < 5 {
                assert_eq!(runtime.parent_color, None, "frame {frame}");
                assert_eq!(runtime.parent_flags & 1, 0, "frame {frame}");
            } else {
                // Frame 26 ends the window: the per-frame flag word is cleared,
                // the persistent colour request stays.
                assert_eq!(runtime.parent_color, Some([-1, 220, 250, 220]));
                assert_eq!(runtime.parent_flags & 1, 0);
            }
        }
    }

    /// The official ``CopyVector(render.position, parent.position)`` on frame
    /// 0.
    #[test]
    fn wind_walk_reanchors_on_the_caster_on_frame_zero() {
        let mut effect = wind_walk();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);
        runtime.position = [10.0, 20.0, 30.0];
        runtime.parent_position = Some([1.0, 2.0, 3.0]);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.position, [1.0, 2.0, 3.0]);
    }
}
