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

pub mod aura_glow;
pub mod circle;
pub mod circle3d;
pub mod cylinder;
pub mod flash;
pub mod flash_fade;
pub mod forest_light;
pub mod half_sphere;
pub mod halo;
pub mod height_ramp;
pub mod intimidate;
pub mod orbit;
pub mod particle3d;
pub mod particle3d_gravity;
pub mod particle_up;
pub mod pulse_kind;
pub mod pulsing_columns;
pub mod quadhorn;
pub mod ripple_rings;
pub mod rising_fan;
pub mod spin_quad;
pub mod stacked_rings;
pub mod texture2d;
pub mod texture3d;
pub mod texture_spin;
pub mod wandering_flames;
pub mod wind;

use super::angles::Angles;
use super::math::Matrix;
use super::rng::EffectRng;

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

/// The parts of the master `CRagEffect` and the frame context that primitives
/// read.
#[derive(Debug, Clone)]
pub struct MasterState {
    /// `CRagEffect::state.tlvertX/tlvertY` - projected anchor on the 640x480
    /// reference screen.
    pub screen: [f32; 2],
    /// `CRagEffect::render.position` - the world position. 3-D primitives
    /// anchor to this instead of the screen.
    pub position: [f32; 3],
    /// `CRagEffect::state.stateCnt` and `.duration`.
    pub state_cnt: i32,
    pub duration: i32,
    /// `CRagEffect::state.deltaPos` (target offset), read by `pattern & 0x400`.
    pub master_delta_pos: [f32; 3],
    /// `CRagEffect::render.angle` (facing), read by `pattern & 0x400`.
    pub render_angle: f32,
    /// `CRagEffect::state.parent->render.angle` - the caster actor's facing,
    /// read by `PP_HALFSPHERE` (prim 104), which aligns the shell to its
    /// parent rather than to the effect's own `render.angle`.
    pub parent_angle: f32,
    /// Camera pitch / yaw and `g_cameraYawOffset`, read by `pattern`
    /// 0x40/0x200.
    pub camera_pitch: f32,
    pub camera_yaw: f32,
    pub camera_yaw_offset: f32,
    /// Terrain height under the primitive, read by `pattern & 4`.
    pub terrain_height: Option<f32>,
    /// `g_masterReachDistance`.
    pub master_reach_distance: f32,
    /// `pattern & 0x10` writes the master's `deltaPos` through this cell.
    pub delta_pos_out: std::cell::Cell<[f32; 3]>,
    /// `pattern & 4` sets the master's `flag`.
    pub flag_out: std::cell::Cell<bool>,
}

impl Default for MasterState {
    fn default() -> Self {
        Self {
            screen: [0.0; 2],
            position: [0.0; 3],
            state_cnt: 0,
            duration: 0,
            master_delta_pos: [0.0; 3],
            render_angle: 0.0,
            parent_angle: 0.0,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
            camera_yaw_offset: 90.0,
            terrain_height: None,
            master_reach_distance: 3.0,
            delta_pos_out: std::cell::Cell::new([0.0; 3]),
            flag_out: std::cell::Cell::new(false),
        }
    }
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
    /// `PP_3DCIRCLE` (12)
    Circle3d,
    /// `PP_3DSPHERE` (14)
    Sphere3d,
    /// `PP_3DQUADHORN` (9)
    QuadHorn3d,
    /// `PP_3DTEXTURE` (1)
    Texture3d,
    /// `PP_3DCROSSTEXTURE` (16)
    CrossTexture3d,
    /// `PP_3DCYLINDER` (13)
    Cylinder3d,
    /// `PP_3DPARTICLE` (5)
    Particle3d,
    /// `PP_3DPARTICLEGRAVITY` (6)
    Particle3dGravity,
    /// `PP_WIND` (62)
    Wind,
    /// `PP_INTIMIDATE` (43)
    Intimidate,
    /// `PP_AURA` / prim 82 (`PrimAuraGlow`)
    AuraGlow,
    /// prim 137 (`PrimHeightRamp`)
    HeightRamp,
    /// prim 89 (TexturedQuadSpin)
    SpinQuad,
    /// prim 59 (`PrimParticleUp`)
    ParticleUp,
    /// prim 60 (`PrimPulsingColumns`)
    PulsingColumns,
    /// prim 63 (`PrimRippleRings`)
    RippleRings,
    /// prim 130 (`PrimRisingFan`)
    RisingFan,
    /// prim 131 (`PrimRisingFan2`)
    RisingFan2,
    /// prim 39 (TexturedHalo)
    Halo,
    /// prim 143 (`PrimTextureSpin`)
    TextureSpin,
    /// prim 7 (`PP_3DPARTICLEORBIT`)
    Orbit,
    /// prim 66 (`ForestLight`) - spawned only, the original has no motion case.
    ForestLight,
    /// prim 64 (`TexturedPulseKind`) - spawned only, no motion case either.
    PulseKind,
    /// prim 155 (`PrimWanderingFlames`) - the Fork Bolt twin glow squares.
    WanderingFlames,
    /// `PP_HALFSPHERE` (104) - `CRagEffect::TexturedPulse`'s shell.
    HalfSphere,
    /// prim 154 (`PrimFlashFade`) - `CRagEffect::TexturedBeam`'s blade fins.
    FlashFade,
    /// prim 103 - `CRagEffect::TexturedMark`'s stacked rings. The official
    /// enum has no symbol for it (`EffectPrimId.h` jumps from 102 to 104), so
    /// the port names it after its renderer, `RenderStackedRings`.
    StackedRings,
}

impl PrimitiveId {
    /// The numeric id the official client stores in `state.count`.
    pub const fn id(self) -> u16 {
        match self {
            Self::Texture2d => 0,
            Self::Flash2d => 3,
            Self::Circle2d => 4,
            Self::Circle3d => 12,
            Self::Sphere3d => 14,
            Self::QuadHorn3d => 9,
            Self::Texture3d => 15,
            Self::CrossTexture3d => 16,
            Self::Cylinder3d => 13,
            Self::Particle3d => 5,
            Self::Particle3dGravity => 6,
            Self::Wind => 62,
            Self::Intimidate => 43,
            Self::AuraGlow => 82,
            Self::HeightRamp => 137,
            Self::SpinQuad => 89,
            Self::ParticleUp => 59,
            Self::PulsingColumns => 60,
            Self::RippleRings => 63,
            Self::RisingFan => 130,
            Self::RisingFan2 => 131,
            Self::Halo => 39,
            Self::TextureSpin => 143,
            Self::Orbit => 7,
            Self::ForestLight => 66,
            Self::PulseKind => 64,
            Self::WanderingFlames => 155,
            Self::HalfSphere => 104,
            Self::FlashFade => 154,
            Self::StackedRings => 103,
        }
    }
}

/// `EffectPrimitiveRecord` - one segment record of a multi-segment primitive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimitiveRecord {
    pub position: [f32; 3],
    pub corners: [[f32; 3]; 4],
    pub radius: f32,
    pub scale: f32,
    pub angle: f32,
    pub matrix: Matrix,
    pub alpha: f32,
    pub color: u32,
}

impl Default for PrimitiveRecord {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            corners: [[0.0; 3]; 4],
            radius: 0.0,
            scale: 0.0,
            angle: 0.0,
            matrix: Matrix::default(),
            alpha: 0.0,
            color: 0,
        }
    }
}

/// `EffectPrimitiveSlot` - one of the four slot records a primitive can drive
/// (winds, auras, sprite bursts). `EffectPrimitiveState.h` layout order kept.
#[derive(Debug, Clone, Copy)]
pub struct EffectPrimitiveSlot {
    /// `/* 0000 */ life` (0 = unused, 1 = active)
    pub life: i8,
    /// `/* 0002 */ alphaB`
    pub alpha_b: i16,
    /// `/* 0004 */ alphaT`
    pub alpha_t: i16,
    /// `/* 0006 */ fullDisplayAngle`
    pub full_display_angle: i16,
    /// `/* 0008 */ maxHeight`
    pub max_height: f32,
    /// `/* 000C */ process`
    pub process: i32,
    /// `/* 0010 */ rotStart`
    pub rot_start: i16,
    /// `/* 0014 */ height[21]`
    pub height: [f32; 21],
    /// `/* 0068 */ flag1[21]`
    pub flag1: [i8; 21],
    /// `/* 007E */ riseAngle`
    pub rise_angle: i16,
    /// `/* 0080 */ distance`
    pub distance: f32,
    /// `/* 0084 */ vectors[4]` union: vecBNow / vecTNow / vecBPre / vecTPre
    pub vec_b_now: [f32; 3],
    pub vec_t_now: [f32; 3],
    pub vec_b_pre: [f32; 3],
    pub vec_t_pre: [f32; 3],
}

impl Default for EffectPrimitiveSlot {
    fn default() -> Self {
        Self {
            life: 0,
            alpha_b: 0,
            alpha_t: 0,
            full_display_angle: 0,
            max_height: 0.0,
            process: 0,
            rot_start: 0,
            height: [0.0; 21],
            flag1: [0; 21],
            rise_angle: 0,
            distance: 0.0,
            vec_b_now: [0.0; 3],
            vec_t_now: [0.0; 3],
            vec_b_pre: [0.0; 3],
            vec_t_pre: [0.0; 3],
        }
    }
}

/// `EffectPrimitiveState`, in the original field order. Offsets are kept in
/// comments so a ported line can be matched against the decompilation.
#[derive(Debug, Clone)]
pub struct EffectPrimitiveState {
    /// `/* 01DC */ pattern`
    pub pattern: u32,
    /// `state.commonPattern`.
    pub common_pattern: u32,
    /// `state.type` (the original's `20/50` render class selector).
    pub type_field: i32,
    /// `/* 0208 */ playWaveType` - `1` makes primitive 6 play
    /// `effect\EF_Coin1..3.wav` when a particle bounces off the terrain.
    pub play_wave_type: u32,
    /// Port-only outbox: wave requests raised by a primitive. The runtime
    /// drains it into [`super::runtime::EffectRuntime::sounds`] each frame,
    /// standing in for the direct `g_soundMgr->PlayWave(...)` call.
    pub pending_sounds: Vec<String>,
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
    /// Number of frames in `texture_frames` (the original's `totalTexture`).
    pub frame_count: u32,
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
    /// `/* 02A4 */ changeSpeed3d`, `/* 02B0 */ changeAccel3d`
    pub change_speed3d: [f32; 3],
    pub change_accel3d: [f32; 3],
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
    /// `/* 02FC */ innerSpeed`, `/* 0300 */ outerSpeed`
    pub inner_speed: f32,
    pub outer_speed: f32,
    /// `/* 0304 */ innerAccel`, `/* 0308 */ outerAccel`
    pub inner_accel: f32,
    pub outer_accel: f32,
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
    /// `gravSpeed`, `gravAccel` (orbit primitives).
    pub grav_speed: f32,
    pub grav_accel: f32,
    /// `/* 0354 */ widthSize`, `/* 0358 */ widthSpeed`, `/* 035C */ widthAccel`
    pub width_size: f32,
    pub width_speed: f32,
    pub width_accel: f32,
    /// `/* 0360 */ heightSize`, `/* 0364 */ heightSpeed`, `/* 0368 */
    /// heightAccel`
    pub height_size: f32,
    pub height_speed: f32,
    pub height_accel: f32,

    /// `/* 0388 */ deltaPos`, `/* 0394 */ deltaPos2`, `/* 03A0 */ deltaPos3`
    pub delta_pos: [f32; 3],
    pub delta_pos2: [f32; 3],
    pub delta_pos3: [f32; 3],
    /// `/* 037C */ orgPos`
    pub org_pos: [f32; 3],
    /// `/* 0370 */ speed3d`
    pub speed3d: [f32; 3],
    /// `/* 02BC */ speed`, `/* 02C0 */ accel`
    pub speed: f32,
    pub accel: f32,
    /// `/* 0160 */ transform` - 12-float affine matrix.
    pub transform: Matrix,
    /// `/* 0190 */ rotTransform`
    pub rot_transform: Matrix,
    /// `/* 01F4 */ numSegment`
    pub num_segment: i32,
    /// `/* 06B8 */ records[50]`
    pub records: Vec<PrimitiveRecord>,
    /// `/* 03E8 */ slots[4]`
    pub slots: [EffectPrimitiveSlot; 4],

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
    /// `/* 013C */ render.angle` - the primitive's own facing.
    pub render_angle: f32,
    /// `/* 01C4 */ renderFlag` - `RSF_*` bits; `0x200` is the additive blend
    /// used by the ring primitives.
    pub render_flag: u32,
    /// `.spr` asset of a sprite particle (`SetSpriteName`).
    pub sprite: Option<&'static str>,
    /// `.act` asset of a sprite particle (`SetActionName`).
    pub action: Option<&'static str>,
    /// Second texture of two-texture primitives (`textures[1]`).
    pub texture_secondary: Option<&'static str>,
    /// Animation row selected by `SetAction(index, ...)`.
    pub action_index: i32,
    /// Third texture of three-texture primitives (`textures[2]`).
    pub texture_tertiary: Option<&'static str>,
    /// Animated texture sequence (`textures[0..totalTexture]` of the animated
    /// effect primitives, e.g. `torch_red01..13`, `W_c_00..50`).
    pub texture_frames: Vec<&'static str>,
    /// Texture asset the body loads for this primitive (`g_texMgr.Load(...)`),
    /// relative to `data\` as the official paths are written. `None` for
    /// sprite (`.spr/.act`) primitives.
    pub texture: Option<&'static str>,
    /// `/* 01CC */ type` - per-primitive number used by `pattern & 0x20`.
    pub primitive_type: i32,
    /// `/* 014C */ masterReached`
    pub master_reached: bool,
    /// `/* 01FC */ RGBCycleCnt`, `/* 0234 */ RGBCycleDelay`
    pub rgb_cycle_cnt: u32,
    pub rgb_cycle_delay: u32,
    /// `chRadiSpeed` - enables the chPoint radius cycle.
    pub ch_radi_speed: u32,
    /// `/* 02E4 */ v1`, `/* 02E8 */ v2`
    pub v1: f32,
    pub v2: f32,
    /// `/* 036C */ maxHeightSize`
    pub max_height_size: f32,
    /// `deltaPosAccel` - used by `pattern & 0x400` and `changePoint`.
    pub delta_pos_accel: [f32; 3],
    /// Frame count of the attached `.act` action (`AnimMotion`).
    pub action_frame_count: u32,
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
            common_pattern: 0,
            type_field: 0,
            play_wave_type: 0,
            pending_sounds: Vec::new(),
            pattern: 0,
            state_cnt: 0,
            duration: 100,
            fade_in_cnt: -1,
            fade_out_cnt: -1,
            change_point: -1,
            total_texture: 1,
            anim_speed: 4,
            frame_count: 1,
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
            change_speed3d: [0.0001; 3],
            change_accel3d: [0.0001; 3],
            change_height_speed: 0.0001,
            change_height_accel: 0.0001,
            change_width_accel: 0.0001,
            change_width_speed: 0.0001,
            max_alpha: 254.0,
            min_alpha: 0.0,
            inner_size: 0.0,
            outer_size: 0.0,
            inner_speed: 0.0,
            outer_speed: 0.0,
            inner_accel: 0.0,
            outer_accel: 0.0,
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
            grav_speed: 0.0,
            grav_accel: 0.0,
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
            delta_pos3: [0.0; 3],
            org_pos: [0.0; 3],
            speed3d: [0.0; 3],
            speed: 0.0,
            accel: 0.0,
            transform: Matrix::default(),
            rot_transform: Matrix::default(),
            num_segment: 1,
            records: Vec::new(),
            slots: [EffectPrimitiveSlot::default(); 4],
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
            render_angle: 0.0,
            render_flag: 1,
            sprite: None,
            action: None,
            action_index: -1,
            texture_frames: Vec::new(),
            texture: None,
            texture_secondary: None,
            texture_tertiary: None,
            primitive_type: 0,
            master_reached: false,
            rgb_cycle_cnt: 0,
            rgb_cycle_delay: 0,
            ch_radi_speed: 0,
            v1: 0.0,
            // `EffectPrim.cpp:126-130`: the constructor starts with
            // `v1 = 0`, `u1Bits = 0`, `v2 = 1.0`, `u2 = 1.0`. The 3-D texture
            // renderer samples `[v1, v2]`; leaving `v2` at zero would make
            // every plane sample a single texel row.
            v2: 1.0,
            max_height_size: 100.0,
            delta_pos_accel: [0.0; 3],
            action_frame_count: 1,
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
    pub fn on_process(&mut self, angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
        let alive = match self.count {
            PrimitiveId::Circle2d => {
                circle::on_process(self, master);
                true
            }
            PrimitiveId::Flash2d => {
                flash::on_process(self, angles, master);
                true
            }
            PrimitiveId::Circle3d => {
                circle3d::on_process(self, angles, master);
                true
            }
            PrimitiveId::Sphere3d => {
                self.on_process_sphere(master);
                true
            }
            PrimitiveId::Texture2d => texture2d::on_process(self, angles, master, rng),
            PrimitiveId::QuadHorn3d => {
                quadhorn::on_process(self, angles, master);
                true
            }
            PrimitiveId::Texture3d | PrimitiveId::CrossTexture3d => texture3d::on_process(self, angles, master, rng),
            PrimitiveId::Cylinder3d => {
                cylinder::on_process(self, angles, master);
                true
            }
            PrimitiveId::Particle3d => particle3d::on_process(self, angles, master, rng),
            PrimitiveId::Particle3dGravity => particle3d_gravity::on_process(self, angles, master, rng),
            PrimitiveId::Wind => {
                wind::on_process(self);
                true
            }
            PrimitiveId::Intimidate => {
                intimidate::on_process(self);
                true
            }
            PrimitiveId::AuraGlow => {
                aura_glow::on_process(self, master);
                true
            }
            PrimitiveId::HeightRamp => {
                height_ramp::on_process(self, angles);
                true
            }
            PrimitiveId::SpinQuad => {
                spin_quad::on_process(self, master);
                true
            }
            PrimitiveId::ParticleUp => {
                particle_up::on_process(self, angles, master, rng);
                true
            }
            PrimitiveId::PulsingColumns => {
                pulsing_columns::on_process(self, angles, master);
                true
            }
            PrimitiveId::RippleRings => {
                ripple_rings::on_process(self, angles, master);
                true
            }
            PrimitiveId::RisingFan => {
                rising_fan::rising_fan(self, master);
                true
            }
            PrimitiveId::RisingFan2 => {
                rising_fan::rising_fan2(self, master);
                true
            }
            PrimitiveId::Halo => {
                halo::on_process(self, master);
                true
            }
            PrimitiveId::TextureSpin => {
                texture_spin::on_process(self, master);
                true
            }
            PrimitiveId::Orbit => {
                orbit::on_process(self, angles, master);
                self.process_alpha(master);
                self.shift_segment();
                true
            }
            PrimitiveId::ForestLight => {
                forest_light::on_process(self);
                true
            }
            PrimitiveId::PulseKind => {
                pulse_kind::on_process(self);
                true
            }
            PrimitiveId::WanderingFlames => {
                wandering_flames::on_process(self, angles, master, rng);
                true
            }
            PrimitiveId::HalfSphere => {
                half_sphere::on_process(self, angles, master);
                true
            }
            PrimitiveId::FlashFade => {
                flash_fade::on_process(self, master);
                true
            }
            PrimitiveId::StackedRings => {
                stacked_rings::on_process(self, rng);
                true
            }
        };

        if !alive {
            return false;
        }

        self.state_cnt += 1;

        self.state_cnt <= self.duration
    }

    /// `PP_3DSPHERE` - `EffectPrimitiveMotion.cpp:8225-8234`.
    fn on_process_sphere(&mut self, master: &MasterState) {
        self.org_pos = [
            master.position[0] + self.delta_pos2[0],
            master.position[1] + self.delta_pos2[1],
            master.position[2] + self.delta_pos2[2],
        ];
        self.position = self.org_pos;
        self.radius_speed = self.radius_accel + self.radius_speed;
        self.radius = self.radius_speed + self.radius;
        self.longitude = self.long_speed + self.longitude;
        self.process_alpha(master);
    }

    fn ensure_records(&mut self) {
        let needed = self.num_segment.max(0) as usize;

        if self.records.len() < needed {
            self.records.resize(needed, PrimitiveRecord::default());
        }
    }

    /// `CEffectPrim::InitSegment` - `EffectPrimitiveRecords.cpp:9-35`.
    pub fn init_segment(&mut self) {
        self.ensure_records();

        for index in 0..self.num_segment.max(0) as usize {
            let record = &mut self.records[index];
            record.position = self.position;
            record.radius = self.radius;
            record.scale = self.size;
            record.angle = self.longitude;
            record.matrix = self.transform;
            record.alpha = self.alpha;
            record.color = record_color(self.alpha, self.red, self.green, self.blue);
        }
    }

    /// `CEffectPrim::ShiftSegment` - `EffectPrimitiveMotion.cpp:3018-3050`.
    pub fn shift_segment(&mut self) {
        self.ensure_records();

        let segments = self.num_segment.max(0) as usize;

        for index in (1..segments).rev() {
            let previous = self.records[index - 1];
            let record = &mut self.records[index];

            record.position = previous.position;
            record.corners = previous.corners;
            record.radius = previous.radius;
            record.angle = previous.angle;
            record.matrix = previous.matrix;

            let level = self.alpha / segments as f32;
            record.alpha = self.alpha - index as f32 * level;
            record.color = record_color(record.alpha, self.red, self.green, self.blue);

            let level = self.size / segments as f32;
            record.scale = self.size - index as f32 * level / 2.0;
        }

        if segments > 0 {
            let first = &mut self.records[0];
            first.position = self.position;
            first.radius = self.radius;
            first.scale = self.size;
            first.angle = self.longitude;
            first.matrix = self.transform;
            first.alpha = self.alpha;
            first.color = record_color(self.alpha, self.red, self.green, self.blue);
        }
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
        self.argb = ((alpha as u32 * 256 + self.red as u8 as u32) * 256 + self.green as u8 as u32) * 256 + self.blue as u8 as u32;
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

        if self.is_disappear == 0 && self.duration - self.state_cnt > master.duration - master.state_cnt + 1 {
            self.is_disappear = 1;
            self.pattern &= !0x80u32;
            self.alpha_speed = -(self.alpha / (master.duration - master.state_cnt) as f32);
        }
    }
}

/// Packed record colour as computed by `InitSegment`/`ShiftSegment`.
fn record_color(alpha: f32, red: i32, green: i32, blue: i32) -> u32 {
    let alpha = alpha as i32 as u32;

    (alpha << 24) + ((red as u8 as u32) << 16) + ((green as u8 as u32) << 8) + (blue as u8 as u32)
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
            assert!(state.on_process(&angles, &master, &mut EffectRng::new(1)));
            assert_eq!(state.state_cnt, expected + 1);
        }

        assert!(!state.on_process(&angles, &master, &mut EffectRng::new(1)));
    }
}
