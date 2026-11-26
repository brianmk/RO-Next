//! The Rogue coin effects (`EF_RG_COIN`, `EF_RG_COIN2..7`, `EF_RG_COIN_BLUE`,
//! `EF_RG_COIN_BLUE_2`) - dispatch blocks at `RagEffect.cpp:20997-21100`.
//!
//! Each coin stage plays a wave on a fixed frame and spawns spray textures
//! through `TexturedSpray` / `TexturedSprayKind`, which are frame-gated: all
//! sprays of a cast appear on frame 0 only.

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::spray;

/// One spray call per frame attempt.
pub struct SprayCall {
    pub name: &'static str,
    pub spin: f32,
    pub kind: i32,
    /// Number of times the dispatch block calls it (loops included).
    pub repeat: usize,
    /// `false` = `TexturedSpray` (`coin_a` style), `true` = `TexturedSprayKind`.
    pub kinded: bool,
}

pub struct RogueCoin {
    waves: Vec<(i32, &'static str)>,
    sprays: Vec<SprayCall>,
    lifetime: i32,
}

impl RogueCoin {
    pub fn new(lifetime: i32) -> Self {
        Self {
            waves: Vec::new(),
            sprays: Vec::new(),
            lifetime,
        }
    }

    pub fn with_waves(mut self, waves: &[(i32, &'static str)]) -> Self {
        self.waves = waves.to_vec();
        self
    }

    pub fn with_sprays(mut self, sprays: Vec<SprayCall>) -> Self {
        self.sprays = sprays;
        self
    }

    pub fn coin2(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[
                (1, "effect\\복호격.wav"),
                (11, "effect\\복호격.wav"),
                (21, "effect\\복호격.wav"),
                (31, "effect\\복호격.wav"),
            ])
            .with_sprays(vec![
                SprayCall { name: "effect\\shield.bmp", spin: 5.0, kind: 0, repeat: 4, kinded: true },
                SprayCall { name: "effect\\sword.bmp", spin: 9.0, kind: 0, repeat: 4, kinded: true },
            ])
    }

    pub fn coin3(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[(21, "effect\\디스암.wav")])
            .with_sprays(vec![
                SprayCall { name: "유저인터페이스\\item\\골드룩스.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\더개리슨.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\더블샷건.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\더스크.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
            ])
    }

    pub fn coin4(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[
                (1, "effect\\복호격.wav"),
                (11, "effect\\복호격.wav"),
                (21, "effect\\복호격.wav"),
                (31, "effect\\복호격.wav"),
            ])
            .with_sprays(vec![
                SprayCall { name: "유저인터페이스\\item\\루드라의활.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\아트로스의흉기.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\모닝스타.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\바스타드소드.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\글레이브.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\블레이드휩.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\드래곤슬레이어.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\배틀액스.bmp", spin: 5.0, kind: 0, repeat: 1, kinded: true },
            ])
    }

    pub fn coin5(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[
                (1, "effect\\복호격.wav"),
                (11, "effect\\복호격.wav"),
                (21, "effect\\복호격.wav"),
                (31, "effect\\복호격.wav"),
            ])
            .with_sprays(vec![
                SprayCall { name: "유저인터페이스\\item\\갈색반지.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\금반지.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\금방울.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\꽃반지.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\낡은펜던트.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\눈구슬반지.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\상자.bmp", spin: 4.0, kind: 1, repeat: 1, kinded: true },
                SprayCall { name: "유저인터페이스\\item\\수제초콜렛.bmp", spin: 5.0, kind: 1, repeat: 1, kinded: true },
            ])
    }

    pub fn coin6(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[
                (1, "effect\\복호격.wav"),
                (11, "effect\\복호격.wav"),
                (21, "effect\\복호격.wav"),
                (31, "effect\\복호격.wav"),
            ])
            .with_sprays(vec![
                SprayCall { name: "effect\\melody_a.bmp", spin: 6.0, kind: 1, repeat: 2, kinded: true },
                SprayCall { name: "effect\\melody_b.bmp", spin: 6.0, kind: 1, repeat: 2, kinded: true },
            ])
    }

    pub fn coin7(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[(26, "effect\\LG_PINPOINTATTACK.wav")])
            .with_sprays(vec![
                SprayCall { name: "effect\\shield.bmp", spin: 5.0, kind: 7, repeat: 4, kinded: true },
                SprayCall { name: "effect\\sword.bmp", spin: 9.0, kind: 7, repeat: 4, kinded: true },
            ])
    }

    pub fn coin_blue(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[
                (1, "effect\\복호격.wav"),
                (11, "effect\\복호격.wav"),
                (21, "effect\\복호격.wav"),
                (31, "effect\\복호격.wav"),
            ])
            .with_sprays(vec![
                SprayCall { name: "effect\\shield.bmp", spin: 5.0, kind: 8, repeat: 4, kinded: true },
                SprayCall { name: "effect\\sword.bmp", spin: 9.0, kind: 8, repeat: 4, kinded: true },
            ])
    }

    pub fn coin_blue_2(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[
                (1, "effect\\복호격.wav"),
                (11, "effect\\복호격.wav"),
                (21, "effect\\복호격.wav"),
                (31, "effect\\복호격.wav"),
            ])
            .with_sprays(vec![
                SprayCall { name: "effect\\shield.bmp", spin: 5.0, kind: 9, repeat: 4, kinded: true },
                SprayCall { name: "effect\\sword.bmp", spin: 9.0, kind: 9, repeat: 4, kinded: true },
            ])
    }

    pub fn coin(lifetime: i32) -> Self {
        Self::new(lifetime)
            .with_waves(&[(0, "effect\\rog_steal coin.wav")])
            .with_sprays(vec![SprayCall { name: "effect\\coin_a.bmp", spin: 3.5, kind: 1, repeat: 30, kinded: false }])
    }
}

impl PortedEffect for RogueCoin {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        for &(at, wave) in &self.waves {
            if at == frame {
                runtime.play_sound(wave);
            }
        }

        for call in &self.sprays {
            for _ in 0..call.repeat {
                if call.kinded {
                    spray::spray_kind(runtime, call.name, call.spin, call.kind);
                } else {
                    spray::spray(runtime, call.name);
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn coin2_waves_on_the_four_stages_and_sprays_on_frame_zero() {
        let mut effect = RogueCoin::coin2(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 341);

        for _ in 0..32 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.sounds.len(), 4);
        assert_eq!(runtime.primitives.len(), 8, "4 shield + 4 sword on frame 0");
        assert!(runtime.primitives.iter().all(|p| p.state.texture.is_some()));
    }

    #[test]
    fn steal_coin_sprays_thirty_coin_sprites() {
        let mut effect = RogueCoin::coin(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 341);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\rog_steal coin.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 30);
        assert!(
            runtime
                .primitives
                .iter()
                .all(|p| p.state.texture == Some("effect\\coin_a.bmp"))
        );
    }

    #[test]
    fn coin7_wave_lands_on_frame_twenty_six() {
        let mut effect = RogueCoin::coin7(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 341);

        step(&mut effect, &mut runtime);
        assert!(runtime.sounds.is_empty());

        while runtime.frame < 26 {
            step(&mut effect, &mut runtime);
        }
        assert!(runtime.sounds.is_empty());

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\LG_PINPOINTATTACK.wav".to_owned()]);
    }

    #[test]
    fn colored_coins_use_their_own_kind() {
        let mut effect = RogueCoin::coin_blue(3000);
        let mut runtime = EffectRuntime::new(3000, [0.0, 0.0], 341);
        step(&mut effect, &mut runtime);
        assert!(runtime.primitives.iter().all(|p| p.state.slots[0].flag1[0] == 8));

        let mut effect = RogueCoin::coin_blue_2(3000);
        let mut runtime = EffectRuntime::new(3000, [0.0, 0.0], 341);
        step(&mut effect, &mut runtime);
        assert!(runtime.primitives.iter().all(|p| p.state.slots[0].flag1[0] == 9));
    }
}
