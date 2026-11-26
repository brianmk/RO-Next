//! Bridge from the ported effect simulation to the client's audio engine.
//!
//! The official effect bodies do not own a mixer: they call
//! `g_soundMgr->PlayWave(name, effect.x - player.x, height, effect.z -
//! player.z, 250, 40, 1.0f, 0)` (`RagEffect.cpp:152-164` for
//! `PlayEffectWave`/`PlayEffectWaveInline`, `RagEffect.cpp:22972` for
//! `PlayEffectWaveAt`). The port records those calls on the hermetic
//! [`EffectRuntime`](super::runtime::EffectRuntime) instead:
//!
//! * [`sounds`](super::runtime::EffectRuntime::sounds) - the inline names
//!   (`runtime.play_sound`), plus the `PlayWave` requests the primitive pass
//!   appends after the body (`EffectRuntime::process_primitives`).
//! * [`positional_sounds`](super::runtime::EffectRuntime::positional_sounds) -
//!   `PlayEffectWaveAt` and the raw calls with explicit listener-relative
//!   coordinates (`runtime.play_sound_at`).
//!
//! [`take_sounds`] drains both channels in recorded order. The offsets are
//! **listener-relative** (`[dx, height, dz]`), exactly like the raw `PlayWave`
//! arguments: [`PortedSound::world_position`] adds them to the runtime's
//! listener snapshot to get the world spot the audio engine's spatial track
//! expects. That is the same coordinate the descriptor effects hand to
//! `AudioEngine::play_spatial_sound_effect`; [`AudioEffectSink`] reuses that
//! exact call, so ported effects share the client's existing mixer, listener
//! and muted/headless backend instead of adding a second mixing path.
//!
//! The drain is a pure function over the two vectors and [`SoundSink`] is the
//! only interface to audio, so the play order and the offsets are testable
//! without an audio device.

use cgmath::Point3;
use ro_next_audio::AudioEngine;

use super::runtime::EffectRuntime;
use crate::loaders::GameFileLoader;

/// `PlayWave`'s `maximumDistance` argument, in world units
/// (`g_soundMgr->PlayWave(..., 250, 40, 1.0f, 0)`). The descriptor effect
/// sounds use the same 250-unit spatial range
/// (`Client::spawn_item_use_effect`, `Client::spawn_teleport_vanish`).
const EFFECT_SOUND_RANGE: f32 = 250.0;

/// One sound recorded by a ported effect body or primitive.
#[derive(Debug, Clone, PartialEq)]
pub struct PortedSound {
    /// Archive path relative to `data\wav`, exactly as the body spelled it,
    /// e.g. `effect\EF_Bash.wav`. [`AudioEngine::load`] resolves that shape.
    pub name: String,
    /// The official `PlayWave` coordinates `[dx, height, dz]`, relative to the
    /// listener. `None` is the inline `PlayEffectWave`/`PlayEffectWaveInline`
    /// channel, which always plays at `[effect.x - listener.x, 0.0, effect.z -
    /// listener.z]` (`PlayEffectWave` passes the height literal `0.0f`).
    pub offset: Option<[f32; 3]>,
}

impl PortedSound {
    /// Resolves the listener-relative offset to a world position.
    ///
    /// `effect_position` is the effect anchor and `listener_position` the
    /// spawn-time local-player snapshot of
    /// [`EffectRuntime::listener_position`](super::runtime::EffectRuntime::listener_position).
    /// Adding the snapshot back to the offset recovers the effect position the
    /// official helper used; the only difference from the original is that a
    /// listener that has moved since the effect spawned is not re-sampled.
    pub fn world_position(&self, effect_position: [f32; 3], listener_position: [f32; 3]) -> Point3<f32> {
        let offset = self.offset.unwrap_or([
            effect_position[0] - listener_position[0],
            0.0,
            effect_position[2] - listener_position[2],
        ]);

        Point3::new(
            listener_position[0] + offset[0],
            listener_position[1] + offset[1],
            listener_position[2] + offset[2],
        )
    }
}

/// Drains both recorded sound channels in recorded order.
///
/// The inline channel comes first: `EffectRuntime::process_primitives` appends
/// the primitives' `PlayWave` requests to it after the body ran, so that list
/// is itself in emission order, and no ported body records on both channels in
/// one step today (`rag::positional` is the only positional user). The two
/// vectors carry no cross-channel sequence numbers, so this concatenation is
/// the most faithful order the frozen runtime can express.
pub fn take_sounds(sounds: &mut Vec<String>, positional_sounds: &mut Vec<(String, [f32; 3])>) -> Vec<PortedSound> {
    let mut drained: Vec<PortedSound> = std::mem::take(sounds)
        .into_iter()
        .map(|name| PortedSound { name, offset: None })
        .collect();

    drained.extend(std::mem::take(positional_sounds).into_iter().map(|(name, offset)| PortedSound {
        name,
        offset: Some(offset),
    }));

    drained
}

/// Receives the sounds of the port, in recorded order.
///
/// The client implements this with [`AudioEffectSink`]; the tests record the
/// plays instead of opening an audio device.
pub trait SoundSink {
    /// Plays one recorded sound at its resolved world position.
    fn play(&mut self, sound: &PortedSound, position: Point3<f32>);
}

/// Drains the runtime channels into `sink`, in recorded order.
///
/// This is the whole drain: every sound is played exactly once, at the world
/// position its listener-relative offset resolves to. It is called after each
/// 16 ms step of
/// [`PortedEffectHolder::update_with_sounds`](super::holder::PortedEffectHolder::update_with_sounds),
/// so a catch-up
/// frame still emits the sounds of an earlier step before those of a later one.
pub fn drain_into(runtime: &mut EffectRuntime, sink: &mut dyn SoundSink) {
    let effect_position = runtime.position;
    let listener_position = runtime.listener_position;

    for sound in take_sounds(&mut runtime.sounds, &mut runtime.positional_sounds) {
        sink.play(&sound, sound.world_position(effect_position, listener_position));
    }
}

/// Plays ported effect sounds through the client's audio engine.
///
/// The two calls are the ones the existing descriptor effects use for their
/// official sound (`Client::spawn_item_use_effect`): `AudioEngine::load` takes
/// the path relative to the `data\wav` archive folder and
/// `play_spatial_sound_effect` plays it on the engine's spatial sub-track over
/// 250 world units (`EFFECT_SOUND_RANGE`). Missing or still-loading files are
/// queued by the engine exactly like every other sound effect; the port never
/// touches the mixer directly.
pub struct AudioEffectSink<'a> {
    audio_engine: &'a AudioEngine<GameFileLoader>,
}

impl<'a> AudioEffectSink<'a> {
    pub fn new(audio_engine: &'a AudioEngine<GameFileLoader>) -> Self {
        Self { audio_engine }
    }
}

impl SoundSink for AudioEffectSink<'_> {
    fn play(&mut self, sound: &PortedSound, position: Point3<f32>) {
        let sound_effect = self.audio_engine.load(&sound.name);
        self.audio_engine
            .play_spatial_sound_effect(sound_effect, position, EFFECT_SOUND_RANGE);
    }
}

/// Records the drains instead of playing them; tests only.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) plays: Vec<(String, Option<[f32; 3]>, Point3<f32>)>,
}

#[cfg(test)]
impl SoundSink for RecordingSink {
    fn play(&mut self, sound: &PortedSound, position: Point3<f32>) {
        self.plays.push((sound.name.clone(), sound.offset, position));
    }
}

#[cfg(test)]
mod tests {
    use super::super::runtime::{PortedEffect, step};
    use super::*;

    struct MixedChannels;

    impl PortedEffect for MixedChannels {
        fn lifetime(&self) -> i32 {
            10
        }

        fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
            if runtime.frame == 0 {
                runtime.play_sound("effect\\EF_Signum.wav");
                runtime.play_sound("effect\\EF_Bash.wav");
                runtime.play_sound_at("effect\\cru_holy cross.wav", [3.0, 1.0, 5.0]);
            }

            true
        }
    }

    #[test]
    fn take_sounds_keeps_the_recorded_order_and_exact_offsets() {
        let mut effect = MixedChannels;
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        let drained = take_sounds(&mut runtime.sounds, &mut runtime.positional_sounds);

        assert_eq!(drained, vec![
            PortedSound {
                name: "effect\\EF_Signum.wav".to_owned(),
                offset: None,
            },
            PortedSound {
                name: "effect\\EF_Bash.wav".to_owned(),
                offset: None,
            },
            PortedSound {
                name: "effect\\cru_holy cross.wav".to_owned(),
                offset: Some([3.0, 1.0, 5.0]),
            },
        ]);

        // A drain is destructive: nothing is replayed on the next one.
        assert!(runtime.sounds.is_empty());
        assert!(runtime.positional_sounds.is_empty());
        assert!(take_sounds(&mut runtime.sounds, &mut runtime.positional_sounds).is_empty());
    }

    #[test]
    fn inline_sounds_play_at_the_effect_position_with_height_zero() {
        // `PlayEffectWave`/`PlayEffectWaveInline` pass `height = 0.0f`, so the
        // effect's own y is never used.
        let sound = PortedSound {
            name: "effect\\EF_Bash.wav".to_owned(),
            offset: None,
        };

        let position = sound.world_position([13.0, 99.0, 25.0], [10.0, 7.0, 20.0]);

        assert_eq!(position, Point3::new(13.0, 7.0, 25.0));
    }

    #[test]
    fn positional_sounds_add_the_listener_snapshot_to_their_offset() {
        // `PlayEffectWaveAt(..., 1.0)`: `[dx, height, dz]` is relative to the
        // listener snapshot, not a world position.
        let sound = PortedSound {
            name: "effect\\priest_gloria.wav".to_owned(),
            offset: Some([3.0, 1.0, 5.0]),
        };

        let position = sound.world_position([13.0, 0.0, 25.0], [10.0, 7.0, 20.0]);

        assert_eq!(position, Point3::new(13.0, 8.0, 25.0));
    }
}
