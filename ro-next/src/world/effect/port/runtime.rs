//! Per-frame driver for ported effects.
//!
//! Mirrors the official update order:
//!
//! 1. `EffectLists::Update` calls `CEffect::Update` on every live effect,
//! 2. `CEffect::Update` dispatches frame-scheduled events, calls the effect
//!    body (`OnProcess`), then `++stateCnt`,
//! 3. the primitive objects (pushed into `scene.objects` by
//!    `CRagEffect::LaunchEffectPrim`) are processed by the scene on the same
//!    frame, with `master.stateCnt` already incremented,
//! 4. a primitive retires once `++stateCnt > duration`.
//!
//! Effects retire when `stateCnt >= duration` (`Effect.cpp:214-216` in
//! `CRagEffect::OnProcess`'s tail).

use super::angles::Angles;
use super::primitive::{EffectPrimitiveState, MasterState, PrimitiveId};
pub use super::rng::EffectRng;
use super::str_player::{Clip, Ez2StrPlayer};

/// `ProcessEZ2STR(0, -80, 0)` - the argument every pre-existing ported effect
/// passes (`ProcessEZ2STRC10830`, `RagEffect.cpp:1823-1824`, and its siblings).
///
/// This is the **only** home of the `-80` literal: the `.str` renderer
/// (`port/str_render.rs`) takes the value from
/// [`EffectRuntime::str_offset`] and never hard-codes it. Bodies that the
/// official dispatch calls with a different offset (ShieldCharge's
/// `ProcessEZ2STRC65300` = `(0, 0, 0)`, Devotion's `(0, -20, 0)`) override the
/// field through [`EffectRuntime::process_str_with`].
pub const DEFAULT_STR_OFFSET: [f32; 3] = [0.0, -80.0, 0.0];

/// A live `CEffectPrim` owned by an effect.
#[derive(Debug, Clone)]
pub struct PrimitiveInstance {
    pub state: EffectPrimitiveState,
    pub alive: bool,
}

/// `CRenderObject::SetFrameSequence`'s eight arguments
/// (`RenderObject.cpp:625-643`), recorded verbatim so the client can play the
/// caster's ACT range.
///
/// The official call stores the animation/frame range and, when `timed` is
/// non-zero, a deadline. The port keeps all eight values; the client applies
/// the range with [`crate::world::Entity::set_skill_frame_sequence`] and the
/// animation with `set_custom_skill_action` (see `port/visual.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentFrameSequence {
    /// `unsigned char animation` - the ACT block, e.g. `0x60`.
    pub animation: u8,
    /// `unsigned char firstFrame`.
    pub first_frame: u8,
    /// `unsigned char lastFrame`.
    pub last_frame: u8,
    /// `unsigned char finishAnimation`.
    pub finish_animation: u8,
    /// `unsigned char finishFrame` - the loop target once `lastFrame` is hit.
    pub finish_frame: u8,
    /// `int timed` - non-zero arms `sequenceDeadline`.
    pub timed: i32,
    /// `std::uint32_t duration` - milliseconds of the timed sequence.
    pub duration: i32,
    /// `int loop`.
    pub loop_: i32,
}

/// The state a ported effect body runs against.
pub struct EffectRuntime {
    /// `state.stateCnt` of the `CRagEffect` (the frame number the body sees).
    pub frame: i32,
    /// `state.duration` from the effect lifetime table.
    pub lifetime: i32,
    /// `state.tlvertX/tlvertY` - projected anchor on the 640x480 reference
    /// screen. The real client fills this from the camera every frame.
    pub screen: [f32; 2],
    /// `render.position` - the world position, used by 3-D primitives.
    pub position: [f32; 3],
    /// `state.param[0]` - effect parameter (skill level / power) some bodies
    /// use.
    pub param: f32,
    /// `state.param[1]`.
    pub param_secondary: f32,
    /// `parent->GetActRes()->Frame(0, 0)->Cell(0)->y`, used by aura kind 22 to
    /// lift the overlay to the head of the parent sprite.
    pub parent_action_height: Option<f32>,
    /// `state.deltaPos4` equivalents the effect bodies own: the offset to the
    /// target (`deltaPos`), the travelled vector (`deltaPos3`), the facing
    /// (`render.angle`) and the `flag` used by the projectile-style bodies.
    pub target_offset: [f32; 3],
    pub delta_pos3: [f32; 3],
    /// `state.deltaPos2` - the per-step vector some bodies carry across frames.
    pub delta_pos2: [f32; 3],
    pub render_angle: f32,
    /// `CRagEffect::state.longitude` / `.latitude` - the *effect-level*
    /// orientation, distinct from a primitive's own `state.longitude`.
    ///
    /// `CRagEffect::Init` sets `longitude = parent->render.angle`
    /// (`RagEffectInitialization.cpp:22`); `latitude` keeps the state
    /// constructor's zero. Bodies that aim primitives by the effect's facing
    /// (`FireArrow`, `IceArrow`) write both and their spark particles read them
    /// back on later frames.
    ///
    /// Additive: no pre-existing field changed position or meaning, and no
    /// ported body read an effect-level angle before these bodies did.
    pub longitude: f32,
    pub latitude: f32,
    /// `camera->camera.yaw` - used by action-facing effects.
    pub camera_yaw: f32,
    /// `camera->camera.pitch` - read by `FaceCamera` (primitive 155).
    pub camera_pitch: f32,
    /// Terrain height under the effect. The official code calls
    /// `scene->terrain->Height(x, z)` per primitive; the port's hermetic
    /// runtime carries a single sample (`None` = no terrain context).
    pub terrain_height: Option<f32>,
    /// `AM_CHANGE_COLOR` body tint (`None` = `AM_RETURN_COLOR`).
    pub body_tint: Option<[u8; 3]>,
    /// `state.parent->render.angle` - the caster's facing, used by bodies such
    /// as Bowling Bash.
    ///
    /// Storm Kick (`EF_STORMKICK`/`EF_STORMKICK1`) also *writes* it: the
    /// official bodies run `parent->render.angle += 30` / `+= 14` with a single
    /// `> 360.0` subtraction. The port mutates this per-effect copy exactly
    /// like the original; the client can mirror the post-step value back onto
    /// the caster actor (the original rotates the shared actor in place).
    pub parent_angle: f32,
    pub flag: i32,
    /// `state.count` - the hit count some bodies read (e.g. Blitz Beat).
    pub count: i32,
    pub primitives: Vec<PrimitiveInstance>,
    /// Sounds played on the current step (`PlayEffectWaveInline`).
    pub sounds: Vec<String>,
    /// `g_modeMgr.GetGameMode()->scene->player->render.position` - the listener
    /// origin [`Self::play_sound_at_height`] subtracts from the effect
    /// position.
    ///
    /// `PlayEffectWaveAt` (`RagEffect.cpp:22972`) reads the *current* local
    /// player position on every call. The hermetic runtime has no player
    /// object, so this is a spawn-time snapshot: the client's spawn path fills
    /// it from the local entity and tests set it explicitly. The constructor
    /// value `[0, 0, 0]` makes the horizontal offset equal the effect position,
    /// i.e. what a listener at the world origin would hear.
    pub listener_position: [f32; 3],
    /// Positional sounds played on the current step as
    /// `(name, [dx, height, dz])`, mirroring
    /// `g_soundMgr->PlayWave(name, x, y, z, 250, 40, 1.0f, 0)`
    /// (`SoundMgr.cpp:627`) and the `PlayEffectWaveAt` helper built on it.
    ///
    /// Additive channel: [`Self::sounds`] keeps the inline
    /// (`PlayEffectWaveInline`) call shape untouched. The offset is
    /// listener-relative, exactly like the raw `PlayWave` arguments; the audio
    /// layer can turn it into a world position with
    /// `listener_position + offset` (for `PlayEffectWaveAt`, `dx`/`dz`
    /// already are `effect.position - listener_position`).
    pub positional_sounds: Vec<(String, [f32; 3])>,
    /// `MM_QUAKE` requests made by the body so far.
    pub shakes: u32,
    pub rng: EffectRng,
    /// `.str` clips the body opened (`CRagEffect::Init`).
    pub str_clips: Vec<Clip>,
    /// `ProcessEZ2STR` state, created when the body opens a `.str` resource.
    pub str_player: Option<Ez2StrPlayer>,
    /// The `x, y, z` arguments of the body's current `ProcessEZ2STR` call.
    ///
    /// `ProcessEZ2STR` adds them to the projected parent position
    /// (`state.tlvertX/Y/Z`, `RagEffect.cpp:1418-1420`); the `.str` renderer
    /// reads the x/y pair as a raw screen-pixel shift. The official dispatch
    /// uses `(0, -80, 0)` almost everywhere ([`DEFAULT_STR_OFFSET`]), but
    /// ShieldCharge calls `ProcessEZ2STRC65300` (`(0, 0, 0)`,
    /// `RagEffect.cpp:1884-1885`) and Devotion calls `(0, -20, 0)`
    /// (`RagEffect.cpp:24604-24606`), so the value is per-effect runtime state,
    /// not a constant.
    ///
    /// Additive: this field did not exist before; every existing body keeps
    /// the constructor's [`DEFAULT_STR_OFFSET`] because it calls
    /// [`Self::process_str`] unchanged.
    pub str_offset: [f32; 3],
    /// `state.parent->render.colorChannels` request of the current frame - the
    /// argument list of the last `SetColor(alpha, red, green, blue)`
    /// (`RenderObject.cpp:534-547`), with `-1` meaning "leave this channel
    /// unchanged". `None` until a body calls
    /// [`Self::set_parent_color`].
    ///
    /// Additive: the field and helper are new, so no existing body observes
    /// them. Like [`Self::body_tint`] this is a simulation-side request; the
    /// bridge that copies it onto the caster entity is described in
    /// [`Self::set_parent_color`].
    pub parent_color: Option<[i16; 4]>,
    /// `state.parent->render.flags` bits the body requests this frame.
    ///
    /// The official actor clears `render.flags` after every rendered frame
    /// (`GameObject.cpp:435`, `GameActor.cpp:685-687`), so this starts at `0`
    /// on each 16 ms step and only the body's current-frame writes are
    /// visible. [`Self::add_parent_flags`] is the `flags += 8` idiom.
    ///
    /// The bridge in `port/visual.rs` applies [`Self::parent_color`] but not
    /// this word: the client's `Entity` has no per-frame `render.flags`
    /// register. Bit `1` (additive body blend, `Player.cpp:1059`) and bit `8`
    /// (pulse/wave outline, `Player.cpp:1085`) map to persistent status
    /// mechanisms (`set_status_additive`/`start_body_wave`) with different
    /// lifetimes, so translating them would change other systems' state rather
    /// than reproduce the original. The bits stay recorded exactly.
    pub parent_flags: u32,
    /// `state.parent->render.state` write of the effect body, e.g.
    /// `ST_NINJAREADY` (37) in `EF_STORMKICK5` (`RagEffect.cpp:23716`).
    ///
    /// The official actor's state machine selects how `ProcessState` advances
    /// the sprite; this client has no `render.state` equivalent, so the value
    /// is recorded for the bridge and the visible animation is carried by
    /// [`Self::parent_frame_sequence`].
    pub parent_state: Option<i32>,
    /// `state.parent->render.outlineAlpha` write of the effect body -
    /// `RenderObjectState.h:125` (`unsigned char`).
    ///
    /// Written by `EF_ASURABODY` (`RagEffect.cpp:24517`): `stateCnt < 200 ?
    /// (unsigned char)stateCnt : 200`. The official write sits inside the
    /// `(render.flags & 0x800) == 0` guard, and that flag word *is* cleared per
    /// rendered frame, so the write happens on every frame in practice.
    /// `outlineAlpha` itself is **not** cleared per frame -
    /// `CRenderObject::Reset` (`RenderObjectState.cpp:90`) is the only
    /// reset - so [`Self::begin_step`] leaves the value in place and the
    /// last write persists.
    ///
    /// Additive. The client bridge stores it on the caster entity
    /// ([`crate::world::Entity::set_render_outline_alpha`]); the actor
    /// outline passes that consume it (`Player.cpp:1142`,
    /// `RenderObject.cpp:2072`) are ported as
    /// `world::entity::render_phase::outline_alpha` + the flag-0x800 pass in
    /// `Common::render`.
    pub parent_outline_alpha: Option<u8>,
    /// `state.parent->render.effectPhase` write of the effect body -
    /// `RenderObjectState.h:127` (`short`).
    ///
    /// Written by `EF_ASURABODY_MONSTER` (`RagEffect.cpp:24524`):
    /// `stateCnt < 150 ? (short)(stateCnt + 50) : <no write>`. Like
    /// [`Self::parent_outline_alpha`] this field is only reset by
    /// `CRenderObject::Reset` (`RenderObjectState.cpp:75`), so the original
    /// stops writing after frame 149 and the actor keeps the last value, `199`.
    /// The runtime reproduces that persistence: [`Self::begin_step`] does not
    /// clear it.
    ///
    /// Additive. The client bridge stores it on the caster entity
    /// ([`crate::world::Entity::set_render_effect_phase`]); the body render
    /// path has no phase consumer yet, so it is applied-but-record-only.
    pub parent_effect_phase: Option<i16>,
    /// `state.parent->render.tintPhase` write of the effect body -
    /// `RenderObjectState.h:128` (`unsigned char`).
    ///
    /// One field, several writers: `EF_BLUEBODY` writes `stateCnt / 3` while
    /// `stateCnt <= 150` (`RagEffect.cpp:21312-21316`); `EF_MADNESS_BLUE`
    /// pins `20` on every fourth frame (`:21318-21324`); `EF_MADNESS_RED`
    /// does the same through flag `0x40000` (`:21326-21332`); `EF_RED_HIT`
    /// writes `stateCnt * 3` while `stateCnt <= 16` (`:21781-21787`);
    /// `EF_REDLIGHTBODY`/`EF_UNK_1398` write
    /// `(unsigned char)((float)stateCnt * 0.125f)` up to frame 200 and pin
    /// `25` afterwards (`:21793-21802`); `EF_LANDBODY` writes `stateCnt`
    /// (`:24551-24555`).
    ///
    /// `CRenderObject::Reset` (`RenderObjectState.cpp:76`) is the only reset,
    /// so [`Self::begin_step`] leaves the value in place and the last write
    /// persists - the same rule as [`Self::parent_outline_alpha`].
    ///
    /// Additive. The client bridge stores it on the caster entity
    /// ([`crate::world::Entity::set_render_tint_phase`]). Consumers are the
    /// tinted body quads `CRenderObject::DrawTintedQuad` (the
    /// `tintPhase <= 10 / <= 20 / <= 50` alpha ramp,
    /// `RenderObject.cpp:826-846`) and the flag-`0x20000` rise
    /// (`Player.cpp:1206-1218`); neither exists in the client renderer yet,
    /// so it is applied-but-record-only.
    pub parent_tint_phase: Option<u8>,
    /// `state.parent->render.shrinkPhase` write of the effect body -
    /// `RenderObjectState.h:121` (`unsigned char`, the low byte of the
    /// `pulsePhaseState` union).
    ///
    /// `EF_BABYBODY` writes `stateCnt` while `stateCnt <= 45`
    /// (`RagEffect.cpp:24478-24484`). Only `CRenderObject::Reset`
    /// (`RenderObjectState.cpp:75-76`) clears it, so it persists.
    ///
    /// Additive. The client bridge stores it on the caster entity
    /// ([`crate::world::Entity::set_render_shrink_phase`]). Its consumer is
    /// `CRenderObject::ScaleByPulse` behind flag `0x10`
    /// (`RenderObject.cpp:749-767`, `GameActor.cpp:932-940`), ported as
    /// `world::entity::render_phase::pulse_scale` and applied to every actor
    /// layer scale in `Common::render`.
    pub parent_shrink_phase: Option<u8>,
    /// `state.parent->render.runStartTick` write of the effect body -
    /// `RenderObjectState.h:129` (`unsigned char`).
    ///
    /// `EF_RUN` writes `stateCnt` behind flag `0x8000`
    /// (`RagEffect.cpp:23621-23626`). The official `render.flags` word is
    /// cleared after every drawn frame (`GameObject.cpp:435`), so the guard
    /// holds every frame and the write happens each step.
    ///
    /// Persistence: **`CRenderObject::Reset` does not touch this byte**
    /// (`RenderObjectState.cpp:74-94` has no `runStartTick` assignment) and no
    /// other code resets it; it is zero from actor construction and only the
    /// `EF_RUN` body writes it, so the last value persists for the actor's
    /// lifetime. [`Self::begin_step`] leaves it alone.
    ///
    /// Additive. The client bridge stores it on the caster entity
    /// ([`crate::world::Entity::set_render_run_start_tick`]). Its consumers are
    /// `CRenderObject::GetSequenceSpriteOverride`'s accessory-sprite swap
    /// (`RenderObject.cpp:794-800`) and `CRenderObject::ProcessFootprint`'s
    /// dummy-effect selection (`RagEffect.cpp:17127`), neither of which the
    /// client models; applied-but-record-only.
    pub parent_run_start_tick: Option<u8>,
    /// `CRagEffect::NotifyGameMode(char value)` - `RagEffect.cpp:8672-8676`:
    /// on frame 0 it sends `MM_SONG_TALK` with `value` to the current mode.
    ///
    /// `EF_TALK_FROSTJOKE` passes `1` (`RagEffect.cpp:24052-24054`) and
    /// `EF_TALK_SCREAM` passes `0` (`:24055-24057`). The mode handler
    /// (`GameMode.cpp:4972-5030`) reads `BA_frostjoke.txt` / `DC_scream.txt`,
    /// picks a random line (`rand() % 117 + 1` / `rand() % 114 + 1`), prefixes
    /// the player name and sends it as a chat packet
    /// (`CZ_MOVE_ITEM_FROM_BODY_TO_STORE`, the classic "talk" packet).
    ///
    /// Additive. **Recorded only**: the client has no localised
    /// `BA_frostjoke.txt`/`DC_scream.txt` asset path, no `MM_SONG_TALK`
    /// equivalent and no chat-packet send hook for it, so the value is kept
    /// for a later bridge. It is not an actor-render register.
    pub game_mode_notify: Option<i8>,
    /// The last `state.parent->SetFrameSequence(...)` call of the effect body.
    ///
    /// `EF_STORMKICK5` is the only ported caller so far:
    /// `SetFrameSequence(0x60, 2, 2, 0x60, 2, 1, 25, 0)`
    /// (`RagEffect.cpp:23717`). The client applies it to the caster entity (see
    /// `port/visual.rs`), restarting the 25 ms sequence only when
    /// [`Self::parent_frame_sequence_frame`] changes.
    pub parent_frame_sequence: Option<ParentFrameSequence>,
    /// The effect frame on which [`Self::parent_frame_sequence`] was last
    /// written (`-1` before the first write). This is the port-only change
    /// stamp the client uses to avoid re-applying the same sequence every
    /// 16 ms step.
    pub parent_frame_sequence_frame: i32,
}

impl EffectRuntime {
    pub fn new(lifetime: i32, screen: [f32; 2], seed: u64) -> Self {
        Self {
            frame: 0,
            lifetime,
            screen,
            position: [0.0; 3],
            param: 0.0,
            param_secondary: 0.0,
            parent_action_height: None,
            target_offset: [0.0; 3],
            delta_pos3: [0.0; 3],
            delta_pos2: [0.0; 3],
            render_angle: 0.0,
            longitude: 0.0,
            latitude: 0.0,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            terrain_height: None,
            body_tint: None,
            parent_angle: 0.0,
            flag: 0,
            count: 0,
            primitives: Vec::new(),
            sounds: Vec::new(),
            listener_position: [0.0; 3],
            positional_sounds: Vec::new(),
            shakes: 0,
            rng: EffectRng::new(seed),
            str_clips: Vec::new(),
            str_player: None,
            str_offset: DEFAULT_STR_OFFSET,
            parent_color: None,
            parent_flags: 0,
            parent_state: None,
            parent_outline_alpha: None,
            parent_effect_phase: None,
            parent_tint_phase: None,
            parent_shrink_phase: None,
            parent_run_start_tick: None,
            game_mode_notify: None,
            parent_frame_sequence: None,
            parent_frame_sequence_frame: -1,
        }
    }

    /// `CRagEffect::LaunchEffectPrim` - creates a primitive and returns its
    /// index (the port's stand-in for the `CEffectPrim*`).
    ///
    /// `CEffectPrim::Initialize` (`EffectPrimitiveState.cpp:72-82`) starts the
    /// primitive at the master's position (`state.orgPos =
    /// master.render.position; render.position = state.orgPos`). Bodies whose
    /// motion has no `FollowMaster` (prim 154's `PrimFlashFade`, prim 155's
    /// relative slot vectors) depend on that spawn anchor.
    pub fn launch(&mut self, id: PrimitiveId, delta_pos2: [f32; 3]) -> usize {
        self.primitives.push(PrimitiveInstance {
            state: EffectPrimitiveState {
                count: id,
                delta_pos2,
                position: self.position,
                ..Default::default()
            },
            alive: true,
        });

        self.primitives.len() - 1
    }

    pub fn primitive_mut(&mut self, index: usize) -> &mut EffectPrimitiveState {
        &mut self.primitives[index].state
    }

    /// `MM_QUAKE` from an effect body.
    pub fn quake(&mut self) {
        self.shakes += 1;
    }

    /// `CRenderObject::Face(x, z)` - `ActorGeometry.cpp:47-64`.
    pub fn face(&mut self, x: f32, z: f32) {
        let delta_x = x - self.position[0];
        let delta_z = z - self.position[2];

        if delta_x != 0.0 || delta_z != 0.0 {
            let mut angle = ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0;
            self.render_angle = angle;

            if angle >= 360.0 {
                angle -= 360.0;
                self.render_angle = angle;
            }

            if angle < 0.0 {
                self.render_angle = angle + 360.0;
            }
        }
    }

    /// `CRenderObject::Heading(x, z, targetX, targetZ)` -
    /// `ActorGeometry.cpp:30-45`. The effect bodies call it through a
    /// `CEffectPrim` (which inherits `CRenderObject`), so this is an associated
    /// function: the original reads no object state.
    ///
    /// Quirk kept: only one `360` subtraction and one `360` addition.
    pub fn heading(x: f32, z: f32, target_x: f32, target_z: f32) -> f32 {
        let delta_x = target_x - x;
        let delta_z = target_z - z;
        let mut angle = ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0;

        if angle >= 360.0 {
            angle -= 360.0;
        }

        if angle < 0.0 {
            angle += 360.0;
        }

        angle
    }

    /// `CRagEffect::Init`'s effect-level facing
    /// (`RagEffectInitialization.cpp:22`): `state.longitude =
    /// parent->render.angle`. `state.latitude` keeps the constructor zero.
    ///
    /// The port cannot do this in [`Self::new`]: the client fills
    /// [`Self::parent_angle`] *after* construction, and the holder steps the
    /// body directly instead of going through [`step`]. The bodies that read
    /// the effect-level angle therefore call this on their first frame, which
    /// is exactly the frame the original sees the value on.
    pub fn init_effect_angles(&mut self) {
        self.longitude = self.parent_angle;
    }

    /// `CRenderObject::CalcDist(x, z)` - `ActorGeometry.cpp:66-70`.
    /// `CEffectPrim::Get_Angle` - `EffectPrimitiveState.cpp:51-70`. Returns the
    /// table index whose cosine the normalised vector still exceeds.
    pub fn get_angle(&self, x: f32, y: f32) -> i16 {
        let angles = super::angles::Angles::shared();
        let length = (x * x + y * y).sqrt();
        let normal_y = y / length;
        let normal_x = x / length;

        for index in 0..=180i16 {
            if normal_x >= angles.raw_cosine(index as usize) {
                return if normal_y >= 0.0 { index } else { 360 - index };
            }
        }

        180
    }

    pub fn calc_dist(&self, x: f32, z: f32) -> f32 {
        let delta_x = x - self.position[0];
        let delta_z = z - self.position[2];

        (delta_z * delta_z + delta_x * delta_x).sqrt()
    }

    /// `PlayEffectWaveInline(*this, name)`.
    pub fn play_sound(&mut self, name: &str) {
        self.sounds.push(name.to_owned());
    }

    /// `PlayEffectWaveAt(effect, name, height)` - `RagEffect.cpp:22972`:
    ///
    /// ```text
    /// g_soundMgr->PlayWave(name,
    ///     effect.render.position.x - player->render.position.x,
    ///     height,
    ///     effect.render.position.z - player->render.position.z,
    ///     250, 40, 1.0f, 0);
    /// ```
    ///
    /// [`Self::listener_position`] stands in for the local player position, so
    /// the recorded offset is `[dx, height, dz]` with the same horizontal
    /// parts the official helper computes.
    pub fn play_sound_at_height(&mut self, name: &str, height: f32) {
        let listener = self.listener_position;
        self.play_sound_at(name, [self.position[0] - listener[0], height, self.position[2] - listener[2]]);
    }

    /// Raw `g_soundMgr->PlayWave(name, dx, height, dz, ...)` with explicit
    /// listener-relative coordinates, e.g. Holy Cross' literal `(0, 0, 0)`
    /// (`RagEffect.cpp:25264-25270`).
    pub fn play_sound_at(&mut self, name: &str, offset: [f32; 3]) {
        self.positional_sounds.push((name.to_owned(), offset));
    }

    /// `CRagEffect::Init` opening the `.str` resource of an effect.
    pub fn open_str(&mut self, clips: Vec<Clip>) {
        let player = Ez2StrPlayer::new(&clips);
        self.str_clips = clips;
        self.str_player = Some(player);
    }

    /// `CRagEffect::ProcessEZ2STR` for the runtime's current
    /// [`Self::str_offset`] (the constructor default is `(0, -80, 0)`;
    /// [`Self::process_str_with`] overrides it per call site).
    ///
    /// Returns `false` when every loop of the resource has finished, which
    /// makes the body retire the effect (the original does `return false`).
    /// The player shares the effect's frame counter, and `InitEZ2STRFrame`
    /// resets it on a loop - so the runtime frame is synced back
    /// afterwards.
    pub fn process_str(&mut self) -> bool {
        let Some(player) = &mut self.str_player else {
            return true;
        };

        player.state_cnt = self.frame;

        let alive = player.process(&self.str_clips);

        self.frame = player.state_cnt;

        alive
    }

    /// `ProcessEZ2STR(x, y, z)` with an explicit screen offset.
    ///
    /// Same player/timeline work as [`Self::process_str`]; the only addition is
    /// that the offset the `.str` renderer reads from [`Self::str_offset`] is
    /// set to the official call site's arguments. ShieldCharge passes
    /// `(0, 0, 0)` (`RagEffect.cpp:1884-1885`, via `ProcessEZ2STRC65300`) and
    /// Devotion passes `(0, -20, 0)` (`RagEffect.cpp:24604-24606`).
    pub fn process_str_with(&mut self, offset: [f32; 3]) -> bool {
        self.str_offset = offset;
        self.process_str()
    }

    /// `state.parent->SetColor(alpha, red, green, blue)`
    /// (`RenderObject.cpp:534-547`).
    ///
    /// The four arguments are recorded verbatim, including the `-1` channels
    /// the original leaves untouched. The official call mutates the shared
    /// caster actor; the port's hermetic runtime cannot reach that actor from a
    /// body (see the module-level note below), so the request is exposed here
    /// for the client bridge. `None` means no `SetColor` call has happened.
    ///
    /// **Application status:** the client bridge in `port/visual.rs` applies
    /// this request to the caster entity on every step
    /// (`Entity::set_status_tint`) and clears it when the effect retires. A
    /// channel still equal to `-1` keeps its previously applied value,
    /// exactly like `SetColor`.
    pub fn set_parent_color(&mut self, alpha: i16, red: i16, green: i16, blue: i16) {
        self.parent_color = Some([alpha, red, green, blue]);
    }

    /// `state.parent->render.state = value`.
    ///
    /// Stored verbatim; see [`Self::parent_state`] for why the client does not
    /// drive a state machine from it.
    pub fn set_parent_state(&mut self, state: i32) {
        self.parent_state = Some(state);
    }

    /// `state.parent->render.outlineAlpha = alpha`.
    ///
    /// Stored verbatim; see [`Self::parent_outline_alpha`] for why the client
    /// does not drive the outline pass from it (recorded only) and why
    /// [`Self::begin_step`] does not clear it.
    pub fn set_parent_outline_alpha(&mut self, alpha: u8) {
        self.parent_outline_alpha = Some(alpha);
    }

    /// `state.parent->render.effectPhase = phase`.
    ///
    /// Stored verbatim; see [`Self::parent_effect_phase`] for the persistence
    /// rule and the record-only status.
    pub fn set_parent_effect_phase(&mut self, phase: i16) {
        self.parent_effect_phase = Some(phase);
    }

    /// `state.parent->render.tintPhase = phase`.
    ///
    /// Stored verbatim; see [`Self::parent_tint_phase`] for the persistence
    /// rule and the record-only status.
    pub fn set_parent_tint_phase(&mut self, phase: u8) {
        self.parent_tint_phase = Some(phase);
    }

    /// `state.parent->render.shrinkPhase = phase`.
    ///
    /// Stored verbatim; see [`Self::parent_shrink_phase`].
    pub fn set_parent_shrink_phase(&mut self, phase: u8) {
        self.parent_shrink_phase = Some(phase);
    }

    /// `state.parent->render.runStartTick = tick`.
    ///
    /// Stored verbatim; see [`Self::parent_run_start_tick`].
    pub fn set_parent_run_start_tick(&mut self, tick: u8) {
        self.parent_run_start_tick = Some(tick);
    }

    /// `CRagEffect::NotifyGameMode(value)` - `RagEffect.cpp:8672-8676`.
    ///
    /// Stored verbatim; see [`Self::game_mode_notify`] for why the client does
    /// not dispatch the `MM_SONG_TALK` message yet.
    pub fn notify_game_mode(&mut self, value: i8) {
        self.game_mode_notify = Some(value);
    }

    /// `state.parent->SetFrameSequence(...)` - records the eight arguments and
    /// the effect frame that wrote them.
    pub fn set_parent_frame_sequence(&mut self, sequence: ParentFrameSequence) {
        self.parent_frame_sequence = Some(sequence);
        self.parent_frame_sequence_frame = self.frame;
    }

    /// `state.parent->render.flags += bits` (`if ((flags & bits) == 0)` in the
    /// official bodies; the OR is equivalent because the actor clears the word
    /// every frame).
    pub fn add_parent_flags(&mut self, bits: u32) {
        self.parent_flags |= bits;
    }

    /// Clears the per-frame actor-flag request before the body runs.
    ///
    /// `GameObject.cpp:435` / `GameActor.cpp:685-687` clear `render.flags`
    /// after the frame is drawn, so a body that stops setting bit `8` stops
    /// requesting it on the next step. Both step drivers call this before
    /// `OnProcess`; the colour request is intentionally **not** cleared because
    /// `SetColor` persists on the actor, and neither are
    /// [`Self::parent_outline_alpha`], [`Self::parent_effect_phase`],
    /// [`Self::parent_tint_phase`], [`Self::parent_shrink_phase`] or
    /// [`Self::parent_run_start_tick`]: the first four are only reset by
    /// `CRenderObject::Reset` (`RenderObjectState.cpp:75-90`) and
    /// `runStartTick` is never reset at all, so
    /// `EF_ASURABODY`'s pinned `200`, `EF_ASURABODY_MONSTER`'s frozen `199`
    /// and every later body's last phase survive every step, as in the
    /// original. [`Self::game_mode_notify`] is likewise a one-shot request,
    /// not a per-frame word.
    pub fn begin_step(&mut self) {
        self.parent_flags = 0;
    }

    /// Advances every live primitive by one frame.
    pub fn process_primitives(&mut self) {
        let master = MasterState {
            screen: self.screen,
            position: self.position,
            state_cnt: self.frame,
            duration: self.lifetime,
            master_delta_pos: self.target_offset,
            render_angle: self.render_angle,
            parent_angle: self.parent_angle,
            camera_yaw: self.camera_yaw,
            camera_pitch: self.camera_pitch,
            terrain_height: self.terrain_height,
            ..Default::default()
        };

        let angles = Angles::shared();

        // The primitives share the effect's RNG, which lives next to them in
        // this struct; take it out for the loop to keep the borrows separate.
        let mut rng = std::mem::replace(&mut self.rng, EffectRng::new(1));
        // Port-only outbox: motion primitives record `PlayWave` requests on
        // their own state, the runtime collects them in submission order.
        let mut primitive_sounds = Vec::new();

        for primitive in &mut self.primitives {
            if !primitive.alive {
                continue;
            }

            primitive.alive = primitive.state.on_process(angles, &master, &mut rng);
            primitive_sounds.append(&mut primitive.state.pending_sounds);
        }

        self.rng = rng;
        self.sounds.append(&mut primitive_sounds);
    }
}

/// A ported `CRagEffect` body or dedicated `CEffect` subclass.
///
/// Bodies are owned by the client's effect list
/// (`Box<dyn EffectBase + Send + Sync>`), so the simulation state has to be
/// thread-shareable even though it is only stepped on the render thread.
pub trait PortedEffect: Send + Sync {
    /// `effectLifetime` from `EffectTypes.cpp`.
    fn lifetime(&self) -> i32;

    /// One `OnProcess` step. `runtime.frame` is the `stateCnt` the original
    /// body reads; it is incremented by [`step`] after this returns.
    ///
    /// Returning `false` retires the effect, mirroring `return false` in the
    /// decompiled `OnProcess` bodies.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool;
}

impl<T: PortedEffect + ?Sized> PortedEffect for Box<T> {
    fn lifetime(&self) -> i32 {
        (**self).lifetime()
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        (**self).on_process(runtime)
    }
}

/// One `CEffect::Update` step, including the primitive pass.
///
/// Returns `false` when the effect has finished and must be retired.
pub fn step(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime) -> bool {
    runtime.begin_step();

    if !effect.on_process(runtime) {
        return false;
    }

    runtime.frame += 1;

    if runtime.frame >= runtime.lifetime {
        return false;
    }

    runtime.process_primitives();

    true
}

/// Runs an effect to completion, collecting the state after every frame.
/// Used by the per-effect tests and by the future golden-frame tooling.
pub fn run_to_completion(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime) -> Vec<FrameSnapshot> {
    let mut snapshots = Vec::new();

    loop {
        let alive = step(effect, runtime);
        snapshots.push(FrameSnapshot::capture(runtime));

        if !alive {
            return snapshots;
        }
    }
}

/// A frame's worth of observable state.
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    pub frame: i32,
    pub primitives: Vec<EffectPrimitiveState>,
    pub sounds: Vec<String>,
}

impl FrameSnapshot {
    fn capture(runtime: &EffectRuntime) -> Self {
        Self {
            frame: runtime.frame,
            primitives: runtime.primitives.iter().map(|p| p.state.clone()).collect(),
            sounds: runtime.sounds.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Spawner {
        lifetime: i32,
    }

    impl PortedEffect for Spawner {
        fn lifetime(&self) -> i32 {
            self.lifetime
        }

        fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
            if runtime.frame != 0 {
                return true;
            }

            let index = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);
            state.duration = 10;
            state.radius = 5.0;
            state.pattern = 1;

            true
        }
    }

    #[test]
    fn effect_runs_exactly_lifetime_frames() {
        let mut effect = Spawner { lifetime: 10 };
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        // Frames 0..=9 are simulated, the 10th step retires the effect.
        assert_eq!(snapshots.len(), 10);
        assert_eq!(snapshots.last().unwrap().frame, 10);
    }

    #[test]
    fn primitive_launched_at_frame_zero_is_processed_on_frame_zero() {
        let mut effect = Spawner { lifetime: 10 };
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.state_cnt, 1);
    }

    #[test]
    fn rng_is_deterministic_and_matches_clamped_random() {
        let mut left = EffectRng::new(42);
        let mut right = EffectRng::new(42);

        for _ in 0..100 {
            assert_eq!(left.clamped_random(360), right.clamped_random(360));
        }

        let mut rng = EffectRng::new(7);
        for _ in 0..1000 {
            let value = rng.clamped_random(25);
            assert!((0..25).contains(&value));
        }

        assert_eq!(rng.clamped_random(0), 0);
    }

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-3
    }

    #[test]
    fn heading_matches_the_cardinal_angles() {
        // `Heading(x, z, targetX, targetZ)` = atan2(dx, -dz) in degrees. The
        // values are not exact quarters because the original divides by
        // `3.141592f` (`90.00002` for +x), so compare with a 1e-3 tolerance.
        assert!(close(EffectRuntime::heading(0.0, 0.0, 0.0, -1.0), 0.0));
        assert!(close(EffectRuntime::heading(0.0, 0.0, 1.0, 0.0), 90.0));
        assert!(close(EffectRuntime::heading(0.0, 0.0, 0.0, 1.0), 180.0));
        assert!(close(EffectRuntime::heading(0.0, 0.0, -1.0, 0.0), 270.0));
    }

    #[test]
    fn heading_is_relative_to_the_source_point() {
        // A zero target vector keeps atan2(0, -0) at 180 (the `-0.0` sign),
        // i.e. the original's behaviour for an unchanged position.
        assert!(close(EffectRuntime::heading(5.0, 7.0, 5.0, 7.0), 180.0));
        // Same direction whether the pair is translated or not.
        assert_eq!(
            EffectRuntime::heading(5.0, 7.0, 6.0, 7.0),
            EffectRuntime::heading(0.0, 0.0, 1.0, 0.0)
        );
    }

    #[test]
    fn launched_primitives_start_at_the_master_position() {
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);
        runtime.position = [10.0, 20.0, 30.0];
        let index = runtime.launch(PrimitiveId::Circle2d, [1.0, 2.0, 3.0]);

        let state = &runtime.primitives[index].state;
        assert_eq!(state.position, [10.0, 20.0, 30.0]);
        assert_eq!(state.delta_pos2, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn init_effect_angles_copies_the_parent_facing_into_longitude() {
        let mut runtime = EffectRuntime::new(160, [0.0, 0.0], 1);
        runtime.parent_angle = 135.0;
        runtime.latitude = 12.0;

        runtime.init_effect_angles();

        assert_eq!(runtime.longitude, 135.0);
        // `Init` never assigns latitude; the constructor zero is kept.
        assert_eq!(runtime.latitude, 12.0);
    }

    #[test]
    fn effect_level_angles_start_at_zero() {
        let runtime = EffectRuntime::new(160, [0.0, 0.0], 1);

        assert_eq!(runtime.longitude, 0.0);
        assert_eq!(runtime.latitude, 0.0);
    }

    #[test]
    fn play_sound_at_height_mirrors_play_effect_wave_at() {
        let mut runtime = EffectRuntime::new(160, [0.0, 0.0], 1);
        runtime.position = [13.0, 7.0, 25.0];
        runtime.listener_position = [10.0, 7.0, 20.0];

        runtime.play_sound_at_height("effect\\priest_gloria.wav", 1.0);

        // `PlayWave(name, effect.x - player.x, height, effect.z - player.z)`.
        // The listener's y is never read by the helper.
        assert_eq!(runtime.positional_sounds, vec![("effect\\priest_gloria.wav".to_owned(), [
            3.0, 1.0, 5.0
        ])]);
        assert!(runtime.sounds.is_empty(), "the inline channel stays untouched");
    }

    #[test]
    fn play_sound_at_records_the_raw_play_wave_coordinates() {
        let mut runtime = EffectRuntime::new(160, [0.0, 0.0], 1);

        runtime.play_sound_at("effect\\cru_holy cross.wav", [0.0, 0.0, 0.0]);
        runtime.play_sound_at("_enemy_hit_wind1.wav", [-150.0, 0.0, 2.5]);

        assert_eq!(runtime.positional_sounds, vec![
            ("effect\\cru_holy cross.wav".to_owned(), [0.0, 0.0, 0.0]),
            ("_enemy_hit_wind1.wav".to_owned(), [-150.0, 0.0, 2.5]),
        ]);
        assert!(runtime.sounds.is_empty());
    }
    #[test]
    fn str_offset_defaults_to_the_process_ez2str_minus_80_call() {
        let runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        assert_eq!(runtime.str_offset, DEFAULT_STR_OFFSET);
        assert_eq!(runtime.str_offset, [0.0, -80.0, 0.0]);
    }

    #[test]
    fn process_str_with_records_the_official_screen_offset() {
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        assert!(runtime.process_str_with([0.0, -20.0, 0.0]));
        assert_eq!(runtime.str_offset, [0.0, -20.0, 0.0]);

        // `process_str` keeps the offset the body last set; it is not a reset.
        assert!(runtime.process_str());
        assert_eq!(runtime.str_offset, [0.0, -20.0, 0.0]);
    }

    #[test]
    fn begin_step_clears_the_actor_flag_request_but_keeps_the_color() {
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);
        runtime.add_parent_flags(8);
        runtime.set_parent_color(-1, 255, 255, 160);

        runtime.begin_step();

        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 160]));
    }

    #[test]
    fn render_phase_writes_persist_across_begin_step() {
        // `CRenderObject::Reset` (`RenderObjectState.cpp:75-90`) is the only
        // reset of these fields, so a body's last write survives the per-frame
        // `render.flags` clear - `begin_step` must not clear them.
        let mut runtime = EffectRuntime::new(60, [0.0, 0.0], 1);

        runtime.set_parent_effect_phase(41);
        runtime.set_parent_tint_phase(160);
        runtime.set_parent_shrink_phase(27);
        runtime.set_parent_run_start_tick(9);
        runtime.set_parent_outline_alpha(200);
        runtime.notify_game_mode(1);

        runtime.begin_step();

        assert_eq!(runtime.parent_effect_phase, Some(41));
        assert_eq!(runtime.parent_tint_phase, Some(160));
        assert_eq!(runtime.parent_shrink_phase, Some(27));
        assert_eq!(runtime.parent_run_start_tick, Some(9));
        assert_eq!(runtime.parent_outline_alpha, Some(200));
        assert_eq!(runtime.game_mode_notify, Some(1));
    }

    #[test]
    fn render_phase_writes_start_unset_and_store_their_exact_width() {
        let mut runtime = EffectRuntime::new(60, [0.0, 0.0], 1);

        assert_eq!(runtime.parent_effect_phase, None);
        assert_eq!(runtime.parent_tint_phase, None);
        assert_eq!(runtime.parent_shrink_phase, None);
        assert_eq!(runtime.parent_run_start_tick, None);
        assert_eq!(runtime.game_mode_notify, None);

        // `unsigned char` / `short` narrowing happens at the call site, exactly
        // like `static_cast<unsigned char>(...)` / `static_cast<short>(...)`.
        runtime.set_parent_tint_phase(u8::MAX);
        runtime.set_parent_effect_phase(i16::MIN);

        assert_eq!(runtime.parent_tint_phase, Some(255));
        assert_eq!(runtime.parent_effect_phase, Some(i16::MIN));
    }

    #[test]
    fn parent_state_and_frame_sequence_are_recorded_verbatim() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);
        runtime.frame = 15;

        runtime.set_parent_state(37);
        runtime.set_parent_frame_sequence(ParentFrameSequence {
            animation: 0x60,
            first_frame: 2,
            last_frame: 2,
            finish_animation: 0x60,
            finish_frame: 2,
            timed: 1,
            duration: 25,
            loop_: 0,
        });

        assert_eq!(runtime.parent_state, Some(37));
        assert_eq!(runtime.parent_frame_sequence_frame, 15);
        assert_eq!(
            runtime.parent_frame_sequence,
            Some(ParentFrameSequence {
                animation: 0x60,
                first_frame: 2,
                last_frame: 2,
                finish_animation: 0x60,
                finish_frame: 2,
                timed: 1,
                duration: 25,
                loop_: 0,
            })
        );

        // `render.state` and the sequence persist across the per-frame flag
        // clear; only `render.flags` is a one-frame request.
        runtime.begin_step();
        assert_eq!(runtime.parent_state, Some(37));
        assert!(runtime.parent_frame_sequence.is_some());
    }
}
