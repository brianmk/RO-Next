//! Exact port of the `.str` player - `CRagEffect::ProcessEZ2STR`
//! (`RagEffect.cpp:1402-1523`) and `InitEZ2STRFrame`
//! (`RagEffect.cpp:1386-1398`).
//!
//! This is the correction that matters most for the 62 `.str` effects the
//! pre-renewal skills play. The official player **does not interpolate between
//! keyframes**. It advances a per-layer key cursor when the *next* key's frame
//! equals the effect frame counter and otherwise holds the previous transform:
//!
//! ```cpp
//! if (elapsed != 0 && (key + 1)->frame == this->state.stateCnt) { ++currentKey; ... }
//! if (key->type == 0) memcpy(current, &key->transform);   // absolute
//! else                current->x += key->transform.x;     // additive delta
//! ```
//!
//! The current loader instead interpolates with easing (`world/effect/mod.rs`),
//! which plays every `.str` smoother and offset. The replacement also carries
//! the four independent corners (`ax[0..4]`, `ay[0..4]`) and two UV sets that
//! the single-quad renderer cannot express.
//!
//! `state_cnt` is owned by the player here (rather than by `CRagEffect`) so the
//! key stepping can be tested without a renderer. `CRagEffect::OnProcess`
//! advances the same counter after the body runs; when the player replaces the
//! live path, the effect's counter must be handed to it.

/// `EZeffectTransform` - one keyframe's transform.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    /// UV size of the first texture stage.
    pub us: f32,
    pub vs: f32,
    /// Second UV set.
    pub u2: f32,
    pub v2: f32,
    pub us2: f32,
    pub vs2: f32,
    /// Four independent corners.
    pub ax: [f32; 4],
    pub ay: [f32; 4],
    /// Per-corner colours.
    pub cr_r: f32,
    pub cr_g: f32,
    pub cr_b: f32,
    pub cr_a: f32,
    pub rz: f32,
    /// 0 = absolute transform, non-zero = additive delta key.
    pub kind: i32,
    /// Texture animation mode (0..=5).
    pub anitype: i32,
    pub anidelta: f32,
    pub aniframe: f32,
}

/// One `EZeffectKeyFrame`.
#[derive(Debug, Clone, Default)]
pub struct KeyFrame {
    /// Absolute effect frame at which this key becomes active.
    pub frame: i32,
    pub transform: Transform,
}

/// One `EZeffectClip`.
#[derive(Debug, Clone, Default)]
pub struct Clip {
    pub texture_count: u32,
    pub frames: Vec<KeyFrame>,
}

/// Per-layer runtime state of `ProcessEZ2STR` (`RagEffectState` subset).
#[derive(Debug, Clone)]
pub struct Ez2StrPlayer {
    pub state_cnt: i32,
    pub current_key: Vec<i32>,
    pub current_layer: i32,
    pub end_layer: i32,
    pub loop_count: i32,
    pub loop_limit: i32,
    pub transforms: Vec<Transform>,
    pub layer_drawn: Vec<bool>,
    /// `float ratio;` - declared outside the layer loop in the original.
    ratio: f32,
}

impl Ez2StrPlayer {
    /// `InitEZ2STRFrame()` - `RagEffect.cpp:1386-1398`.
    pub fn new(clips: &[Clip]) -> Self {
        let mut player = Self {
            state_cnt: 0,
            current_key: vec![0; clips.len()],
            current_layer: 0,
            end_layer: 0,
            loop_count: 0,
            loop_limit: 0,
            transforms: vec![Transform::default(); clips.len()],
            layer_drawn: vec![false; clips.len()],
            ratio: 0.0,
        };
        player.init_frame(clips);
        player
    }

    fn init_frame(&mut self, clips: &[Clip]) {
        self.current_layer = 0;
        self.state_cnt = 0;
        self.end_layer = clips.len() as i32;

        if clips.first().map(|clip| clip.frames.is_empty()).unwrap_or(true) {
            self.end_layer -= 1;
        }

        for key in &mut self.current_key {
            *key = 0;
        }
    }

    /// `CRagEffect::ProcessEZ2STR` - advances all layers for the current
    /// `state_cnt` and returns `false` when every loop has finished.
    pub fn process(&mut self, clips: &[Clip]) -> bool {
        if self.current_layer >= self.end_layer {
            self.loop_count += 1;

            if self.loop_count > self.loop_limit {
                return false;
            }

            self.init_frame(clips);
        }

        for layer in 0..clips.len() {
            self.layer_drawn[layer] = false;

            let clip = &clips[layer];

            if clip.frames.is_empty() {
                continue;
            }

            let mut key_index = self.current_key[layer];

            if key_index == -1 {
                continue;
            }

            let mut index = key_index as usize;
            let key = &clip.frames[index];
            let elapsed = self.state_cnt - key.frame;

            if elapsed < 0 {
                continue;
            }

            // Advance when the next key starts on exactly this frame. The
            // `index + 1 < len` guard covers a one-key clip, where the original
            // reads one element past the end.
            if elapsed != 0 && index + 1 < clip.frames.len() && clip.frames[index + 1].frame == self.state_cnt {
                self.current_key[layer] += 1;
                key_index = self.current_key[layer];

                if key_index >= clip.frames.len() as i32 - 1 {
                    self.current_key[layer] = -1;
                    self.current_layer += 1;
                }

                // `++key`: the advanced key is still drawn on this frame, even
                // though the cursor is now -1.
                index = (clip.frames.len() - 1).min(key_index.max(0) as usize);
            }

            let key = &clip.frames[index];

            if key.transform.kind == 0 {
                self.transforms[layer] = key.transform;

                if self.current_key[layer] != -1 && index + 1 < clip.frames.len() && clip.frames[index + 1].frame == key.frame {
                    self.current_key[layer] += 1;
                }
            } else {
                self.apply_delta(layer, clip, key, elapsed);
            }

            self.layer_drawn[layer] = true;
        }

        true
    }

    /// The additive-delta branch, `RagEffect.cpp:1475-1523`.
    fn apply_delta(&mut self, layer: usize, clip: &Clip, key: &KeyFrame, elapsed: i32) {
        let delta = key.transform;
        let current = &mut self.transforms[layer];

        current.x += delta.x;
        current.y += delta.y;
        current.u += delta.u;
        current.v += delta.v;
        current.us += delta.us;
        current.vs += delta.vs;
        current.u2 += delta.u2;
        current.v2 += delta.v2;
        current.us2 += delta.us2;
        current.vs2 += delta.vs2;

        for corner in 0..4 {
            current.ax[corner] += delta.ax[corner];
            current.ay[corner] += delta.ay[corner];
        }

        current.cr_r += delta.cr_r;
        current.cr_g += delta.cr_g;
        current.cr_b += delta.cr_b;
        current.cr_a += delta.cr_a;
        current.rz += delta.rz;

        match delta.anitype {
            0 => {}
            1 => {
                current.aniframe += delta.aniframe;
            }
            2 => {
                current.aniframe += delta.anidelta;
                let count = clip.texture_count as f32;
                if current.aniframe >= count {
                    current.aniframe = count - 1.0;
                }
            }
            5 => {
                let frame = (current.aniframe + delta.anidelta * elapsed as f32) as i32;
                let count = clip.texture_count as i32 - 1;

                // QUIRK: the original divides by `count`; a single-texture clip
                // would divide by zero.
                if count == 0 {
                    return;
                }

                let turns = frame / count;
                let frame = if turns & 1 != 0 {
                    count - (frame - turns * count)
                } else {
                    frame - turns * count
                };

                current.aniframe = frame as f32;
            }
            3 => {
                current.aniframe += delta.anidelta;

                let count = clip.texture_count as f32;
                if current.aniframe >= count {
                    self.ratio = (current.aniframe / count) as i32 as f32;
                    current.aniframe -= self.ratio * count;
                }
            }
            4 => {
                // QUIRK: `ratio` is not assigned before this branch's first use
                // in the original (`RagEffect.cpp:1519-1522`); the port keeps
                // the persistent value and documents the deviation.
                current.aniframe -= delta.anidelta;

                if current.aniframe < 0.0 {
                    current.aniframe -= self.ratio * clip.texture_count as f32;
                    self.ratio = (current.aniframe / clip.texture_count as f32) as i32 as f32;

                    if current.aniframe < 0.0 {
                        current.aniframe += clip.texture_count as f32;
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs_key(frame: i32, x: f32) -> KeyFrame {
        KeyFrame {
            frame,
            transform: Transform {
                x,
                kind: 0,
                ..Default::default()
            },
        }
    }

    #[test]
    fn keys_are_held_not_interpolated() {
        let clips = vec![Clip {
            texture_count: 1,
            frames: vec![abs_key(0, 10.0), abs_key(5, 20.0)],
        }];
        let mut player = Ez2StrPlayer::new(&clips);

        let mut seen = Vec::new();
        while player.process(&clips) {
            seen.push(player.transforms[0].x);
            player.state_cnt += 1;
        }

        assert_eq!(seen, vec![10.0, 10.0, 10.0, 10.0, 10.0, 20.0]);
    }

    #[test]
    fn delta_keys_apply_every_frame_while_current() {
        let clips = vec![Clip {
            texture_count: 1,
            frames: vec![
                abs_key(0, 0.0),
                KeyFrame {
                    frame: 2,
                    transform: Transform {
                        x: 1.0,
                        kind: 1,
                        ..Default::default()
                    },
                },
                KeyFrame {
                    frame: 4,
                    transform: Transform {
                        x: 1.0,
                        kind: 1,
                        ..Default::default()
                    },
                },
            ],
        }];
        let mut player = Ez2StrPlayer::new(&clips);

        let mut seen = Vec::new();
        while player.process(&clips) {
            seen.push(player.transforms[0].x);
            player.state_cnt += 1;
        }

        assert_eq!(seen, vec![0.0, 0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn animation_type_three_wraps_and_final_key_holds() {
        let clips = vec![Clip {
            texture_count: 3,
            frames: vec![
                KeyFrame {
                    frame: 0,
                    transform: Transform {
                        kind: 1,
                        anitype: 3,
                        anidelta: 1.0,
                        ..Default::default()
                    },
                },
                KeyFrame {
                    frame: 10,
                    transform: Transform {
                        kind: 1,
                        anitype: 0,
                        ..Default::default()
                    },
                },
            ],
        }];
        let mut player = Ez2StrPlayer::new(&clips);

        let mut frames = Vec::new();
        while player.process(&clips) {
            frames.push(player.transforms[0].aniframe);
            player.state_cnt += 1;
        }

        assert_eq!(frames, vec![1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn loop_limit_zero_stops_after_one_pass() {
        let clips = vec![Clip {
            texture_count: 1,
            frames: vec![abs_key(0, 1.0), abs_key(1, 2.0)],
        }];
        let mut player = Ez2StrPlayer::new(&clips);

        assert!(player.process(&clips));
        player.state_cnt = 1;
        assert!(player.process(&clips));
        player.state_cnt = 2;

        assert!(!player.process(&clips), "one pass with loopLimit 0");
        assert_eq!(player.loop_count, 1);
    }

    #[test]
    fn loop_limit_one_restarts_the_timeline() {
        let clips = vec![Clip {
            texture_count: 1,
            frames: vec![abs_key(0, 1.0), abs_key(1, 2.0)],
        }];
        let mut player = Ez2StrPlayer::new(&clips);
        player.loop_limit = 1;

        assert!(player.process(&clips));
        player.state_cnt = 1;
        assert!(player.process(&clips));
        player.state_cnt = 2;

        assert!(player.process(&clips), "restarts because loopCount 1 <= 1");
        assert_eq!(player.loop_count, 1);
        assert_eq!(player.state_cnt, 0, "InitEZ2STRFrame resets the counter");
    }
}
