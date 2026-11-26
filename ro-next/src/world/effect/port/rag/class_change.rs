//! `CRagEffect::TexturedClassChange(kind)` - `RagEffect.cpp:18730-19370`.
//!
//! The shared sprite-burst helper behind ~80 effect ids (Gunslinger shots,
//! ninja jutsu, elemental fields, trap spawns, ...). Kind 30 (Triple Action) is
//! the only one that spawns at frame 30; every other kind spawns on frame 0.
//!
//! The action rows come from the `SetAction(row, 16, 1)` calls in the switch.
//! `CEffectPrim` inherits `CRenderObject::SetAction(unsigned animation, int
//! motion, int repeat)` (`RenderObject.cpp:494`): the first argument is the
//! action row (`render.previousAnimation`), `16` is the unused `motion` and `1`
//! is `repeat`/`motionType`. Kinds with no call keep the constructor default
//! `previousAnimation = 0` (`EffectPrim.cpp:124`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

pub(crate) struct ClassChange {
    pub sprite: &'static str,
    pub action: &'static str,
    /// Action row: first argument of the original `SetAction(row, 16, 1)`.
    ///
    /// Kinds 8 and 28 pick a row from the camera angle, and kinds 15, 17, 60,
    /// 61, 62 and 63 roll `ClampedRandom`/`rand()`. The port does not
    /// reproduce those dynamic picks (that would also consume RNG state), so
    /// those kinds store the lowest row the original can reach: row 0 for all
    /// of them except kind 62, whose `rand() % 8 + 0x20` starts at 32.
    pub action_row: u8,
    pub anim_speed: Option<i32>,
    pub pattern: Option<u32>,
    pub alpha: Option<f32>,
    pub wave: Option<&'static str>,
}

const NONE: Option<ClassChange> = None;

// Decompilation-derived data table; keep one kind per line.
#[rustfmt::skip]
static CLASS_CHANGE: [Option<ClassChange>; 106] = [
    Some(ClassChange { sprite: "이팩트\\캐슬링.spr", action: "이팩트\\캐슬링.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\블러드러스트.spr", action: "이팩트\\블러드러스트.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: Some("effect\\h_blood_lust.wav") }),
    Some(ClassChange { sprite: "이팩트\\item_thunder.spr", action: "이팩트\\item_thunder.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\item_cloud.spr", action: "이팩트\\item_cloud.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\item_curse.spr", action: "이팩트\\item_curse.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\item_zzz.spr", action: "이팩트\\item_zzz.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\item_rain.spr", action: "이팩트\\item_rain.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\m_ef01.spr", action: "이팩트\\m_ef01.act", action_row: 0, anim_speed: Some(3), pattern: Some(0), alpha: Some(220.0), wave: Some("effect\\ef_firehit.wav") }),
    Some(ClassChange { sprite: "이팩트\\m_ef02.spr", action: "이팩트\\m_ef02.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\m_ef03.spr", action: "이팩트\\m_ef03.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\m_ef04.spr", action: "이팩트\\m_ef04.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\m_ef05.spr", action: "이팩트\\m_ef05.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\m_ef06.spr", action: "이팩트\\m_ef06.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\m_ef07.spr", action: "이팩트\\m_ef07.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\합격_.spr", action: "이팩트\\합격_.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\그림자베기.spr", action: "이팩트\\그림자베기.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\다다미.spr", action: "이팩트\\다다미.act", action_row: 0, anim_speed: Some(6), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\안개베기.spr", action: "이팩트\\안개베기.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\일섬.spr", action: "이팩트\\일섬.act", action_row: 0, anim_speed: Some(2), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\화염진.spr", action: "이팩트\\화염진.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\데스페라도.spr", action: "이팩트\\데스페라도.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\라이트닝스피어.spr", action: "이팩트\\라이트닝스피어.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\블라인드스피어.spr", action: "이팩트\\블라인드스피어.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\포이즌스피어.spr", action: "이팩트\\포이즌스피어.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\프리징스피어.spr", action: "이팩트\\프리징스피어.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\플레어스피어.spr", action: "이팩트\\플레어스피어.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\래피드샤워.spr", action: "이팩트\\래피드샤워.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\매지컬불릿.spr", action: "이팩트\\매지컬불릿.act", action_row: 0, anim_speed: Some(2), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\스프레드.spr", action: "이팩트\\스프레드.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\트래킹.spr", action: "이팩트\\트래킹.act", action_row: 0, anim_speed: Some(2), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\트리플액션.spr", action: "이팩트\\트리플액션.act", action_row: 0, anim_speed: Some(2), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\어스퀘이크.spr", action: "이팩트\\어스퀘이크.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\폭죽_러브.spr", action: "이팩트\\폭죽_러브.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\폭죽_화이트데이.spr", action: "이팩트\\폭죽_화이트데이.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\폭죽_발렌타인.spr", action: "이팩트\\폭죽_발렌타인.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\폭죽_생일.spr", action: "이팩트\\폭죽_생일.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\폭죽_크리스마스.spr", action: "이팩트\\폭죽_크리스마스.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\mobile_ef04.spr", action: "이팩트\\mobile_ef04.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\mashofa.spr", action: "이팩트\\mashofa.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\soule.spr", action: "이팩트\\soule.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    Some(ClassChange { sprite: "이팩트\\기본트랩.spr", action: "이팩트\\기본트랩.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\담황색트랩.spr", action: "이팩트\\담황색트랩.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\암청색트랩.spr", action: "이팩트\\암청색트랩.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\자홍색트랩.spr", action: "이팩트\\자홍색트랩.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\청록색트랩.spr", action: "이팩트\\청록색트랩.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    NONE,
    NONE,
    NONE,
    NONE,
    NONE,
    Some(ClassChange { sprite: "몬스터\\stapo.spr", action: "몬스터\\stapo.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "몬스터\\Ancient_tree.spr", action: "몬스터\\Ancient_tree.act", action_row: 0, anim_speed: None, pattern: Some(0), alpha: None, wave: None }),
    Some(ClassChange { sprite: "몬스터\\ifrit.spr", action: "몬스터\\ifrit.act", action_row: 32, anim_speed: None, pattern: Some(0), alpha: None, wave: None }),
    Some(ClassChange { sprite: "몬스터\\mavka_bullet.spr", action: "몬스터\\mavka_bullet.act", action_row: 0, anim_speed: Some(6), pattern: Some(0), alpha: Some(255.0), wave: None }),
    Some(ClassChange { sprite: "몬스터\\breeze.spr", action: "몬스터\\breeze.act", action_row: 0, anim_speed: Some(3), pattern: Some(0), alpha: Some(205.0), wave: None }),
    Some(ClassChange { sprite: "몬스터\\breeze.spr", action: "몬스터\\breeze.act", action_row: 4, anim_speed: Some(2), pattern: Some(0), alpha: Some(205.0), wave: None }),
    NONE,
    NONE,
    NONE,
    NONE,
    Some(ClassChange { sprite: "이팩트\\거스트.spr", action: "이팩트\\거스트.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\블래스트.spr", action: "이팩트\\블래스트.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\아쿠아플레이.spr", action: "이팩트\\아쿠아플레이.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\업히벌.spr", action: "이팩트\\업히벌.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\와일드스톰.spr", action: "이팩트\\와일드스톰.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\칠리에어.spr", action: "이팩트\\칠리에어.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\커즈드소일.spr", action: "이팩트\\커즈드소일.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\쿨러.spr", action: "이팩트\\쿨러.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\트로픽.spr", action: "이팩트\\트로픽.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\파이로테크닉.spr", action: "이팩트\\파이로테크닉.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\페트롤로지.spr", action: "이팩트\\페트롤로지.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\히터.spr", action: "이팩트\\히터.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\silent_brz.spr", action: "이팩트\\silent_brz.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\magma_flow.spr", action: "이팩트\\magma_flow.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\icemine.spr", action: "이팩트\\icemine.act", action_row: 0, anim_speed: Some(10), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\icemine_.spr", action: "이팩트\\icemine_.act", action_row: 0, anim_speed: Some(10), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\RK_DRAGONBREATH_WATER.spr", action: "이팩트\\RK_DRAGONBREATH_WATER.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\wideweb.spr", action: "이팩트\\wideweb.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\burnt.spr", action: "이팩트\\burnt.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    NONE,
    Some(ClassChange { sprite: "이팩트\\그라운드드리프트.spr", action: "이팩트\\그라운드드리프트.act", action_row: 0, anim_speed: None, pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\나뭇잎_완성.spr", action: "이팩트\\나뭇잎_완성.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    NONE,
    NONE,
    Some(ClassChange { sprite: "이팩트\\Black_Thunder\\Black_Thunder.spr", action: "이팩트\\Black_Thunder\\Black_Thunder.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Black_Thunder\\Black_Thunder_Dark.spr", action: "이팩트\\Black_Thunder\\Black_Thunder_Dark.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Angel_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Angel_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Animal_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Animal_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Devil_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Devil_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Dragon_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Dragon_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Fish_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Fish_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Human_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Human_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Insect_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Insect_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Nothing_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Nothing_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Plant_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Plant_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
    Some(ClassChange { sprite: "이팩트\\Barmund_Magic_Circle\\Undead_Magic_Circle.spr", action: "이팩트\\Barmund_Magic_Circle\\Undead_Magic_Circle.act", action_row: 0, anim_speed: Some(4), pattern: None, alpha: None, wave: None }),
];

pub(crate) fn class_change(kind: u8) -> Option<&'static ClassChange> {
    CLASS_CHANGE.get(kind as usize).and_then(|entry| entry.as_ref())
}

#[derive(Debug)]
pub struct TexturedClassChange {
    kinds: Vec<u8>,
    lifetime: i32,
    /// `(frame, wave)` sounds scheduled by the effect's dispatch block.
    waves: Vec<(i32, &'static str)>,
    /// Frames on which the block sends `MM_QUAKE`.
    quakes: Vec<i32>,
}

impl TexturedClassChange {
    pub fn new(kinds: &[u8], lifetime: i32) -> Self {
        Self {
            kinds: kinds.to_vec(),
            lifetime,
            waves: Vec::new(),
            quakes: Vec::new(),
        }
    }

    pub fn with_waves(mut self, waves: &[(i32, &'static str)]) -> Self {
        self.waves = waves.to_vec();
        self
    }

    pub fn with_quakes(mut self, frames: &[i32]) -> Self {
        self.quakes = frames.to_vec();
        self
    }
}

impl PortedEffect for TexturedClassChange {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if self.quakes.contains(&frame) {
            runtime.quake();
        }

        for wave in self.waves.iter().filter(|(at, _)| *at == frame) {
            runtime.play_sound(wave.1);
        }

        for &kind in &self.kinds {
            // Only kind 30 has a `stage` of 30; everything else fires on frame 0.
            let stage = if kind == 30 { 30 } else { 0 };

            if runtime.frame != stage {
                continue;
            }

            let Some(info) = class_change(kind) else {
                continue;
            };

            if let Some(wave) = info.wave {
                runtime.play_sound(wave);
            }

            let lifetime = runtime.lifetime;
            let primitive = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(primitive);

            state.duration = lifetime;
            state.delta_pos2[1] = 0.0;
            state.render_flag |= 0x200;
            state.pattern |= 0x200;
            state.anim_speed = info.anim_speed.unwrap_or(2);
            state.repeat_anim = 0;
            state.alpha_speed = 0.0;
            state.sprite = Some(info.sprite);
            state.action = Some(info.action);
            state.action_index = i32::from(info.action_row);

            if let Some(pattern) = info.pattern {
                state.pattern = pattern;
            }

            if let Some(alpha) = info.alpha {
                state.alpha = alpha;
            }

            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn every_kind_has_a_sprite_and_action() {
        for kind in 0..=105u8 {
            if let Some(info) = class_change(kind) {
                assert!(!info.sprite.is_empty(), "kind {kind}");
                assert!(!info.action.is_empty(), "kind {kind}");
                assert!(info.sprite.ends_with(".spr"), "kind {kind}: {}", info.sprite);
                assert!(info.action.ends_with(".act"), "kind {kind}: {}", info.action);
            }
        }
    }

    #[test]
    fn kinds_zero_stage_spawn_on_frame_zero() {
        let mut effect = TexturedClassChange::new(&[26], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Particle3d);
        assert_eq!(state.duration, 100);
        assert_eq!(state.render_flag & 0x200, 0x200);
        assert_eq!(state.pattern & 0x200, 0x200);
        assert_eq!(state.anim_speed, class_change(26).and_then(|info| info.anim_speed).unwrap_or(2));
        assert_eq!(state.repeat_anim, 0);
        assert_eq!(state.alpha_speed, 0.0);
        assert!(state.sprite.is_some());
        assert!(state.action.is_some());
        assert_eq!(state.action_index, 0);
    }

    #[test]
    fn kind_without_a_set_action_call_draws_row_zero() {
        // Kind 0 has no `SetAction` call in the switch; the primitive
        // constructor's `previousAnimation = 0` (`EffectPrim.cpp:124`) is the
        // row, so the table's default of 0 is exact.
        let mut effect = TexturedClassChange::new(&[0], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.action_index, 0);
    }

    #[test]
    fn explicit_action_row_is_stored() {
        // Kind 65 (breeze) calls `SetAction(4, 16, 1)`.
        let mut effect = TexturedClassChange::new(&[65], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.action_index, 4);
    }

    #[test]
    fn frame_count_argument_is_not_the_row() {
        // Kind 85 calls `SetAction(0, 10, 1)`: the 10 is the motion/frame-count
        // argument, so the row is 0, not 10.
        let mut effect = TexturedClassChange::new(&[85], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.action_index, 0);
    }

    #[test]
    fn random_action_row_uses_the_lowest_roll() {
        // Kind 62 (ifrit) rolls `rand() % 8 + 0x20`, i.e. rows 32..=39; the
        // port stores the lowest row instead of consuming RNG state.
        let mut effect = TexturedClassChange::new(&[62], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.action_index, 32);
    }

    #[test]
    fn triple_action_waits_for_frame_thirty() {
        let mut effect = TexturedClassChange::new(&[30], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        while runtime.frame < 30 {
            step(&mut effect, &mut runtime);
            assert!(runtime.primitives.is_empty());
        }

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.sprite, class_change(30).map(|info| info.sprite));
        assert_eq!(runtime.primitives[0].state.action_index, 0);
    }

    #[test]
    fn vacuum_spawns_both_of_its_kinds() {
        let mut effect = TexturedClassChange::new(&[64, 65], 1000);
        let mut runtime = EffectRuntime::new(1000, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2);
        let sprites: Vec<_> = runtime.primitives.iter().map(|p| p.state.sprite).collect();
        assert_eq!(sprites[0], class_change(64).map(|info| info.sprite));
        assert_eq!(sprites[1], class_change(65).map(|info| info.sprite));
    }

    #[test]
    fn per_kind_overrides_are_applied() {
        // Kind 7 (m_ef01) overrides animSpeed to 3, pattern to 0 and alpha to 220.
        let mut effect = TexturedClassChange::new(&[7], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 161);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.anim_speed, 3);
        assert_eq!(state.pattern, 0, "pattern override replaces the 0x200 bit");
        assert_eq!(state.alpha, 220.0);
        assert_eq!(runtime.sounds, vec!["effect\\ef_firehit.wav".to_owned()]);
    }
}
