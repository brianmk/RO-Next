//! Port of `CEffectPrim` - the single state machine behind every effect
//! primitive in the official client.
//!
//! `EffectPrimitiveState.h` defines one ~0xF8E0-byte struct with `slots[4]`,
//! `records[50]`, `triangles[512]` and ~120 scalars; `state.count` is the
//! primitive id and two switches choose the per-frame behaviour
//! (`EffectPrimitiveMotion.cpp:8024`, `EffectPrimitiveRendering.cpp:222`).
//!
//! The port keeps that shape. Only the fields used by the ported primitives are
//! present so far; add fields as functions are ported, never re-order or
//! re-purpose one.

pub mod circle;
pub mod flash;

use super::angles::Angles;

/// One fixed-function-pipeline vertex as the official client submits it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// `rhw` = 1 / w; the client always submits 0.999999 for effect triangles.
    pub rhw: f32,
    pub u: f32,
    pub v: f32,
    /// Packed `argb` as computed by `ProcessAlpha`.
    pub diffuse: u32,
}

/// One submitted triangle (the client submits triangles, not quads).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderTriangle {
    pub vertices: [Vertex; 3],
    pub texture: u16,
}

/// The parts of the master `CRagEffect` that primitives read.
#[derive(Debug, Clone, Copy, Default)]
pub struct MasterState {
    /// `CRagEffect::state.tlvertX/tlvertY` - projected anchor on the 640x480
    /// reference screen.
    pub screen: [f32; 2],
    /// `CRagEffect::state.stateCnt` and `.duration`.
    pub state_cnt: i32,
    pub duration: i32,
}

/// Which per-frame branch a primitive runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveId {
    /// `PP_2DTEXTURE` (0)
    Texture2d,
    /// `PP_2DFLASH` (3)
    Flash2d,
    /// `PP_2DCIRCLE` (4)
    Circle2d,
}

impl PrimitiveId {
    /// The numeric id the official client stores in `state.count`.
    pub const fn id(self) -> u16 {
        match self {
            Self::Texture2d => 0,
            Self::Flash2d => 3,
            Self::Circle2d => 4,
        }
    }
}

/// `EffectPrimitiveState`, in the original field order. Offsets are kept in
/// comments so a ported line can be matched against the decompilation.
#[derive(Debug, Clone)]
pub struct EffectPrimitiveState {
    /// `/* 01DC */ pattern`
    pub pattern: u32,
    /// `/* 01E0 */ stateCnt`
    pub state_cnt: i32,
    /// `/* 01E4 */ duration`
    pub duration: i32,
    /// `/* 01EC */ fadeInCnt`
    pub fade_in_cnt: i32,
    /// `/* 01F0 */ fadeOutCnt`
    pub fade_out_cnt: i32,
    /// `/* 0200 */ changePoint`
    pub change_point: i32,
    /// `/* 0204 */ totalTexture`
    pub total_texture: u32,
    /// `/* 01E8 */ animSpeed`
    pub anim_speed: i32,
    /// `/* 0254 */ repeatAnim`
    pub repeat_anim: u8,

    /// `/* 0248 */ redSpeed` / `greenSpeed` / `blueSpeed`
    pub red_speed: i32,
    pub green_speed: i32,
    pub blue_speed: i32,

    /// `/* 0258 */ chPoint`
    pub ch_point: Vec<f32>,
    /// `/* 0264 */ chVal1`
    pub ch_val1: Vec<f32>,
    /// `/* 0270 */ chVal2`
    pub ch_val2: Vec<f32>,

    /// `/* 0288 */ arcAngle`
    pub arc_angle: f32,
    /// `/* 029C */ changeSpeed`, `/* 02A0 */ changeAccel`
    pub change_speed: f32,
    pub change_accel: f32,
    /// `/* 02CC */ changeHeightSpeed`, `/* 02D0 */ changeHeightAccel`
    pub change_height_speed: f32,
    pub change_height_accel: f32,
    /// `/* 02C4 */ changeWidthAccel`, `/* 02C8 */ changeWidthSpeed`
    pub change_width_accel: f32,
    pub change_width_speed: f32,

    /// `/* 02D4 */ maxAlpha`, `/* 02D8 */ minAlpha`
    pub max_alpha: f32,
    pub min_alpha: f32,
    /// `/* 02F4 */ innerSize`, `/* 02F8 */ outerSize`
    pub inner_size: f32,
    pub outer_size: f32,
    /// `/* 0318 */ size`, `/* 031C */ sizeSpeed`, `/* 0320 */ sizeAccel`
    pub size: f32,
    pub size_speed: f32,
    pub size_accel: f32,
    /// `/* 0324 */ alpha`, `/* 0328 */ alphaSpeed`, `/* 032C */ alphaDelta`
    pub alpha: f32,
    pub alpha_speed: f32,
    pub alpha_delta: f32,
    /// `/* 030C */ longitude`, `/* 0310 */ latitude`, `/* 0314 */ roll`
    pub longitude: f32,
    pub latitude: f32,
    pub roll: f32,
    /// `/* 0330 */ longSpeed`, `/* 0334 */ latiSpeed`, `/* 0338 */ rollSpeed`
    pub long_speed: f32,
    pub lati_speed: f32,
    pub roll_speed: f32,
    /// `/* 033C */ longAccel`, `/* 0340 */ latiAccel`, `/* 0344 */ rollAccel`
    pub long_accel: f32,
    pub lati_accel: f32,
    pub roll_accel: f32,
    /// `/* 0348 */ radius`, `/* 034C */ radiusSpeed`, `/* 0350 */ radiusAccel`
    pub radius: f32,
    pub radius_speed: f32,
    pub radius_accel: f32,
    /// `/* 0354 */ widthSize`, `/* 0358 */ widthSpeed`, `/* 035C */ widthAccel`
    pub width_size: f32,
    pub width_speed: f32,
    pub width_accel: f32,
    /// `/* 0360 */ heightSize`, `/* 0364 */ heightSpeed`, `/* 0368 */ heightAccel`
    pub height_size: f32,
    pub height_speed: f32,
    pub height_accel: f32,

    /// `/* 0388 */ deltaPos`, `/* 0394 */ deltaPos2`
    pub delta_pos: [f32; 3],
    pub delta_pos2: [f32; 3],

    /// `/* 013C */ render.position`
    pub position: [f32; 3],
    /// `/* 01C0 */ argb`
    pub argb: u32,
    /// `/* 023C */ red`, `/* 0240 */ green`, `/* 0244 */ blue`
    pub red: i32,
    pub green: i32,
    pub blue: i32,

    /// `/* 01D8 */ colorCycleIndex`
    pub color_cycle_index: i32,
    /// `/* 0150 */ alphaSpeedCycle`
    pub alpha_speed_cycle: u32,
    /// `/* 0154 */ alphaCycle`
    pub alpha_cycle: u32,
    /// `/* 015C */ isDisappear`
    pub is_disappear: u32,
    /// `/* 01D4 */ chCnt`
    pub ch_cnt: u32,
    /// `/* 01D0 */ count` - the primitive id.
    pub count: PrimitiveId,
    /// `/* 0104 */ render.curMotion` - current texture index of animated prims.
    pub cur_motion: u32,
}

impl Default for EffectPrimitiveState {
    /// `EffectPrimitiveState::EffectPrimitiveState(RenderObjectState&)` -
    /// `EffectPrim.cpp:8-150`. These are *not* zeros: the constructor sets
    /// `duration = 100`, `alpha = 254`, `maxAlpha = 254`, `alphaDelta = 1`,
    /// `changePoint = -1`, `arcAngle = 36`, `red/green/blue = 255`, and the
    /// `change*` fields to `0.0001`. Effects rely on them.
    fn default() -> Self {
        Self {
            pattern: 0,
            state_cnt: 0,
            duration: 100,
            fade_in_cnt: -1,
            fade_out_cnt: -1,
            change_point: -1,
            total_texture: 1,
            anim_speed: 4,
            repeat_anim: 1,
            red_speed: 0,
            green_speed: 0,
            blue_speed: 0,
            ch_point: Vec::new(),
            ch_val1: Vec::new(),
            ch_val2: Vec::new(),
            arc_angle: 36.0,
            change_speed: 0.0001,
            change_accel: 0.0001,
            change_height_speed: 0.0001,
            change_height_accel: 0.0001,
            change_width_accel: 0.0001,
            change_width_speed: 0.0001,
            max_alpha: 254.0,
            min_alpha: 0.0,
            inner_size: 0.0,
            outer_size: 0.0,
            size: 1.0,
            size_speed: 0.0,
            size_accel: 0.0,
            alpha: 254.0,
            alpha_speed: 0.0,
            alpha_delta: 1.0,
            longitude: 0.0,
            latitude: 0.0,
            roll: 0.0,
            long_speed: 0.0,
            lati_speed: 0.0,
            roll_speed: 0.0,
            long_accel: 0.0,
            lati_accel: 0.0,
            roll_accel: 0.0,
            radius: 0.0,
            radius_speed: 0.0,
            radius_accel: 0.0,
            width_size: 0.0,
            width_speed: 0.0,
            width_accel: 0.0,
            height_size: 0.0,
            height_speed: 0.0,
            height_accel: 0.0,
            delta_pos: [0.0; 3],
            delta_pos2: [0.0; 3],
            position: [0.0; 3],
            argb: u32::MAX,
            red: 255,
            green: 255,
            blue: 255,
            color_cycle_index: 0,
            alpha_speed_cycle: 0,
            alpha_cycle: 0,
            is_disappear: 0,
            ch_cnt: 0,
            count: PrimitiveId::Texture2d,
            cur_motion: 0,
        }
    }
}

impl EffectPrimitiveState {
    /// One `CEffectPrim::OnProcess` step for the supported primitives.
    ///
    /// Returns `false` when the primitive has expired, mirroring the tail of
    /// `EffectPrimitiveMotion.cpp:10155-10163`:
    ///
    /// ```cpp
    /// ++this->state.stateCnt;
    /// if (this->state.stateCnt > this->state.duration) return false;
    /// return true;
    /// ```
    pub fn on_process(&mut self, angles: &Angles, master: &MasterState) -> bool {
        match self.count {
            PrimitiveId::Circle2d => circle::on_process(self, master),
            PrimitiveId::Flash2d => flash::on_process(self, angles, master),
            PrimitiveId::Texture2d => {}
        }

        self.state_cnt += 1;

        self.state_cnt <= self.duration
    }

    /// `ProcessAlpha()` - `EffectPrimitiveColor.cpp:26-77`.
    pub fn process_alpha(&mut self, master: &MasterState) {
        if self.alpha_cycle != 0 {
            let mut index = self.color_cycle_index;

            if index < 0 || index as usize > self.ch_point.len().saturating_sub(1) {
                self.color_cycle_index = 0;
                index = 0;
            }

            if self.state_cnt as f32 == self.ch_point[index as usize] {
                self.alpha = self.ch_val1[index as usize];
                self.max_alpha = self.ch_val2[index as usize];
                self.color_cycle_index = index + 1;
            }

            self.limit_alpha(master);
        } else {
            if self.alpha_speed_cycle != 0 {
                let mut index = self.color_cycle_index;

                if index < 0 || index as usize >= self.ch_point.len() {
                    self.color_cycle_index = 0;
                    index = 0;
                }

                if self.state_cnt as f32 == self.ch_point[index as usize] {
                    self.alpha_speed = self.ch_val1[index as usize];
                    self.color_cycle_index = index + 1;
                }
            }

            self.limit_alpha(master);
        }

        if self.state_cnt == self.fade_out_cnt {
            self.pattern &= !0x80u32;
            self.alpha_speed = -(self.alpha / (self.duration - self.state_cnt) as f32) * self.alpha_delta;
        }

        let sum = self.alpha_speed + self.alpha;
        let minimum = if self.min_alpha > sum { self.min_alpha } else { sum };
        let bounded = if self.max_alpha < minimum { self.max_alpha } else { minimum };
        self.alpha = bounded;

        let alpha = bounded as i32;
        self.argb = ((alpha as u32 * 256 + self.red as u8 as u32) * 256 + self.green as u8 as u32) * 256
            + self.blue as u8 as u32;
    }

    /// `LimitAlpha(CEffectPrim&)` - `EffectPrimitiveColor.cpp:6-22`.
    fn limit_alpha(&mut self, master: &MasterState) {
        if (self.pattern & 0x80u32) != 0 {
            if self.alpha_speed < 0.0 {
                if self.alpha <= self.min_alpha {
                    self.alpha_speed *= -1.0;
                }
            } else if self.alpha >= self.max_alpha {
                self.alpha_speed *= -1.0;
            }
        }

        if self.is_disappear == 0
            && self.duration - self.state_cnt > master.duration - master.state_cnt + 1
        {
            self.is_disappear = 1;
            self.pattern &= !0x80u32;
            self.alpha_speed = -(self.alpha / (master.duration - master.state_cnt) as f32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_clamps_to_max_without_fade_out() {
        let master = MasterState {
            state_cnt: 0,
            duration: 100,
            ..Default::default()
        };
        let mut state = EffectPrimitiveState {
            alpha: 250.0,
            alpha_speed: 10.0,
            max_alpha: 255.0,
            duration: 100,
            // QUIRK: C++ zero-initialises this, which would make frame 0 the
            // fade-out frame; every effect that cares sets it explicitly.
            fade_out_cnt: 100,
            red: 255,
            green: 128,
            blue: 0,
            ..Default::default()
        };

        state.process_alpha(&master);

        assert_eq!(state.alpha, 255.0);
        assert_eq!(state.argb, ((255u32 * 256 + 255) * 256 + 128) * 256);
    }

    #[test]
    fn fade_out_starts_on_its_frame() {
        let master = MasterState {
            state_cnt: 20,
            duration: 100,
            ..Default::default()
        };
        let mut state = EffectPrimitiveState {
            alpha: 200.0,
            alpha_delta: 1.0,
            max_alpha: 255.0,
            duration: 100,
            state_cnt: 20,
            fade_out_cnt: 20,
            ..Default::default()
        };

        state.process_alpha(&master);

        assert_eq!(state.alpha_speed, -2.5);
        assert_eq!(state.alpha, 197.5);
    }

    #[test]
    fn primitive_lives_through_its_duration_frame() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Circle2d,
            duration: 4,
            ..Default::default()
        };

        // Four alive steps (stateCnt 0..3), the fifth increments to 5 > 4.
        for expected in 0..4 {
            assert!(state.on_process(&angles, &master));
            assert_eq!(state.state_cnt, expected + 1);
        }

        assert!(!state.on_process(&angles, &master));
    }
}
