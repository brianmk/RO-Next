use std::sync::{Arc, LazyLock};

#[cfg(feature = "debug")]
use cgmath::SquareMatrix;
use cgmath::{Array, Matrix4, Point3, Transform, Vector2, Vector3, Zero};
use ragnarok_packets::{ClientTick, Direction, EntityId};
use ro_next_container::Cacheable;
use ro_next_interface::element::StateElement;
use rust_state::RustState;

#[cfg(feature = "debug")]
use crate::graphics::DebugRectangleInstruction;
use crate::graphics::{Color, EntityInstruction};
use crate::loaders::{Imf, Sprite};
use crate::world::{ActionEvent, Actions, Camera, EntityType};

pub(crate) const TILE_SIZE: f32 = 10.0;
pub(crate) const SPRITE_SCALE: f32 = 1.4;
const PICKUP_ANIMATION_FACTOR: f32 = 25.0;

#[allow(dead_code)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AnimationActionType {
    Attack1,
    Attack2,
    Attack3,
    Die,
    Freeze1,
    Freeze2,
    Hurt,
    #[default]
    Idle,
    Pickup,
    ReadyFight,
    Sit,
    Skill,
    Special,
    Walk,
    /// Explicit ACT action index (rustnarok's generic motion table: dance,
    /// song, kick, throw, gunslinger, Madogear, Doram actions, ...).
    Custom(usize),
}

impl AnimationActionType {
    pub fn action_base_offset(&self, entity_type: EntityType) -> usize {
        match entity_type {
            EntityType::Hidden | EntityType::Player => match self {
                // The player body ACT layout has two standing blocks: the
                // classic block 0..7 is the *weaponless* relaxed pose, while
                // the armed stand pose is the ready-fight block 32..39. Every
                // per-job weapon ACT (kRO 2019 data set) only animates the
                // ready-fight (32..39) and attack (80..95) blocks, so idling on
                // block 0 would make the equipped weapon layer invisible
                // (official `CPc::JobActionBase(ST_STAND) = 32`).
                AnimationActionType::Idle => 4,
                AnimationActionType::Walk => 1,
                AnimationActionType::Sit => 2,
                AnimationActionType::Pickup => 3,
                AnimationActionType::ReadyFight => 4,
                AnimationActionType::Attack1 => 5,
                AnimationActionType::Hurt => 6,
                AnimationActionType::Freeze1 => 7,
                AnimationActionType::Die => 8,
                AnimationActionType::Freeze2 => 9,
                AnimationActionType::Attack2 => 10,
                AnimationActionType::Attack3 => 11,
                AnimationActionType::Skill => 12,
                AnimationActionType::Custom(index) => *index,
                _ => 0,
            },
            // Pets, homunculi and mercenaries use the non-player ACT layout
            // (idle 0, walk 1, attack 2, hurt 3, die 4).
            EntityType::Npc | EntityType::Monster | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary => match self {
                AnimationActionType::Idle => 0,
                AnimationActionType::Walk => 1,
                AnimationActionType::Attack1 => 2,
                AnimationActionType::Hurt => 3,
                AnimationActionType::Die => 4,
                AnimationActionType::Custom(index) => *index,
                _ => 0,
            },
            EntityType::Warp => 0,
        }
    }
}

/// How a looping animation advances when it reaches its last frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimationLoopMode {
    /// Jump from the last frame back to the first (native ACT looping).
    #[default]
    Repeat,
    /// Play the frames forwards, then backwards (`0..n-1, n-2..1`). Useful for
    /// animations whose last frame does not line up with the first one.
    PingPong,
}

/// Resolve a frame index for a looping animation.
/// Maps an action frame index onto the official `first..=last` / `loop_to..`
/// sequence of `RenderObject.cpp`'s `SetFrameSequence`.
fn sequence_frame_index(index: usize, first: u8, last: u8, loop_to: u8) -> usize {
    let first = first as usize;
    let last = last as usize;
    let loop_to = loop_to.clamp(first as u8, last as u8) as usize;

    if last <= first {
        return first;
    }

    let span = last - first + 1;
    if index < span {
        return first + index;
    }

    let loop_span = last - loop_to + 1;
    first + (loop_to - first) + (index - span) % loop_span
}

fn looped_frame_index(frame_time: u32, frame_count: usize, loop_mode: AnimationLoopMode) -> usize {
    if frame_count <= 1 {
        return 0;
    }

    match loop_mode {
        AnimationLoopMode::Repeat => frame_time as usize % frame_count,
        AnimationLoopMode::PingPong => {
            // With two frames a ping-pong loop is identical to repeating.
            if frame_count == 2 {
                return frame_time as usize % frame_count;
            }

            let cycle = frame_count * 2 - 2;
            let position = frame_time as usize % cycle;
            match position < frame_count {
                true => position,
                false => cycle - position,
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationState {
    action_type: AnimationActionType,
    action_base_offset: usize,
    start_time: ClientTick,
    time: u32,
    duration: Option<u32>,
    factor: Option<f32>,
    looping: bool,
    /// Playback speed multiplier. `1.0` is the native ACT timing; the hunter
    /// falcon uses a faster value because its flap loop looks too slow with the
    /// raw ACT delays.
    speed: f32,
    /// How the animation loops once it reaches the last frame.
    loop_mode: AnimationLoopMode,
    /// Official per-skill frame range: play `first..=last` and continue with
    /// `loop_to..=last` (`RenderObject.cpp` `SetFrameSequence`). `None` plays
    /// the whole action.
    frame_sequence: Option<(u8, u8, u8)>,
}

impl AnimationState {
    pub fn new(entity_type: EntityType, start_time: ClientTick) -> Self {
        let action_type = AnimationActionType::Idle;
        Self {
            action_type,
            action_base_offset: action_type.action_base_offset(entity_type),
            start_time,
            time: 0,
            duration: None,
            factor: None,
            looping: true,
            speed: 1.0,
            loop_mode: AnimationLoopMode::default(),
            frame_sequence: None,
        }
    }

    /// Select how a looping animation wraps at its end (repeat or ping-pong).
    pub fn with_loop_mode(mut self, loop_mode: AnimationLoopMode) -> Self {
        self.loop_mode = loop_mode;
        self
    }

    /// Change the loop mode of an existing animation.
    #[allow(dead_code)]
    pub fn set_loop_mode(&mut self, loop_mode: AnimationLoopMode) {
        self.loop_mode = loop_mode;
    }

    #[allow(dead_code)]
    pub fn loop_mode(&self) -> AnimationLoopMode {
        self.loop_mode
    }

    /// Base action index of the animation currently being played.
    ///
    /// This is the value the official client calls `previousAnimation` when it
    /// looks up per-motion appearance tables such as `DrawOnTop`
    /// (`CCharacterRender::DrawOnTop` / `CPc::IsDrawOnTop`).
    pub fn action_index(&self) -> usize {
        self.action_base_offset
    }

    /// Set a playback speed multiplier (e.g. `1.5` for a 50 % faster loop).
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.01);
        self
    }

    /// Change the playback speed of an existing animation.
    #[allow(dead_code)]
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.01);
    }

    pub fn idle(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Idle;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = true;
    }

    /// Play an attack swing.
    ///
    /// `action` selects the ACT block: [`AnimationActionType::Attack1`] (2) for
    /// the non-player layout, or [`AnimationActionType::Attack2`] (80..87) /
    /// [`AnimationActionType::Attack3`] (88..95) for the player attack blocks.
    /// The equipped weapon ACT only animates one of the player blocks (its data
    /// depends on the job/weapon pair), so the caller picks the block that the
    /// weapon layer actually draws.
    pub fn attack(&mut self, entity_type: EntityType, attack_duration: u32, action: AnimationActionType, client_tick: ClientTick) {
        self.action_type = action;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        // A server-provided attack duration can be zero (e.g. some instant
        // attack packets); a zero duration must not be used as a divisor.
        self.duration = Some(attack_duration.max(1));
        self.factor = None;
        self.looping = false;
    }

    /// Sit down. The sit pose loops until the entity stands up again.
    pub fn sit(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Sit;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = true;
    }

    /// Stand up again (back to the idle animation).
    pub fn stand_up(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.idle(entity_type, client_tick);
    }

    /// Play the skill-cast animation for `cast_time` milliseconds.
    ///
    /// Casts with a cast time loop the ACT skill action (the original client
    /// repeats the casting gesture for as long as the cast bar is filled) and
    /// return to idle when the cast time is over. Instant casts
    /// (`looping == false`, `cast_time == 0`) play the whole ACT skill action
    /// once at its natural frame speed and return to idle when it is over.
    pub fn skill(&mut self, entity_type: EntityType, cast_time: u32, looping: bool, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Skill;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        // A non-looping cast has no artificial duration: the ACT file defines how
        // long the full animation takes, which is what the original client shows.
        self.duration = match looping {
            true => Some(cast_time.max(1)),
            false => None,
        };
        self.factor = None;
        self.looping = looping;
    }

    /// Play an explicit ACT action index (`CasterAction::Action`).
    ///
    /// Used for the rustnarok generic motion table (dance/song/kick/shot/...)
    /// whose action index cannot be expressed by one of the named actions.
    pub fn custom(&mut self, entity_type: EntityType, action_index: usize, cast_time: u32, looping: bool, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Custom(action_index);
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = match looping {
            true => Some(cast_time.max(1)),
            false => None,
        };
        self.factor = None;
        self.looping = looping;
    }

    /// Restricts the current action to the official frame sequence
    /// (`first..=last`, then `loop_to..=last`) for `duration_ms`.
    pub fn frame_sequence(&mut self, first: u8, last: u8, loop_to: u8, duration_ms: u32, client_tick: ClientTick) {
        self.frame_sequence = Some((first, last, loop_to));
        self.start_time = client_tick;
        self.factor = None;
        self.looping = duration_ms == 0;
        self.duration = (duration_ms > 0).then_some(duration_ms);
    }

    /// Combat-ready stance (official `READYFIGHT` action, `SM_ENDURE`).
    ///
    /// The stance loops until the next action replaces it, like every other
    /// non-timed pose.
    pub fn ready_fight(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::ReadyFight;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = true;
    }

    pub fn pickup(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Pickup;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(PICKUP_ANIMATION_FACTOR);
        self.looping = false;
    }

    pub fn walk(&mut self, entity_type: EntityType, movement_speed: usize, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Walk;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(movement_speed as f32 * 100.0 / 150.0 / 5.0);
        self.looping = true;
    }

    pub fn dead(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Die;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = false;
    }

    pub fn is_attack(&self) -> bool {
        matches!(
            self.action_type,
            AnimationActionType::Attack1 | AnimationActionType::Attack2 | AnimationActionType::Attack3
        )
    }

    pub fn is_pickup(&self) -> bool {
        self.action_type == AnimationActionType::Pickup
    }

    pub fn is_walking(&self) -> bool {
        self.action_type == AnimationActionType::Walk
    }

    pub fn is_dead(&self) -> bool {
        self.action_type == AnimationActionType::Die
    }

    pub fn is_sitting(&self) -> bool {
        self.action_type == AnimationActionType::Sit
    }

    pub fn is_skill(&self) -> bool {
        self.action_type == AnimationActionType::Skill
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        let elapsed = client_tick.0.wrapping_sub(self.start_time.0);
        self.time = (elapsed as f32 * self.speed) as u32;
    }
}

#[derive(RustState, Clone, StateElement)]
pub struct AnimationData {
    pub animation_pair: Vec<AnimationPair>,
    pub animations: Vec<Animation>,
    pub delays: Vec<f32>,
    #[hidden_element]
    pub entity_type: EntityType,
}

impl Cacheable for AnimationData {
    fn size(&self) -> usize {
        // We cache animations only by count.
        0
    }
}

#[derive(RustState, Clone, StateElement)]
pub struct AnimationPair {
    pub sprites: Arc<Sprite>,
    pub actions: Arc<Actions>,
}

#[derive(RustState, Clone, StateElement)]
pub struct Animation {
    #[hidden_element]
    pub frames: Vec<AnimationFrame>,
}

#[derive(Clone)]
pub struct AnimationFrame {
    pub event: Option<ActionEvent>,
    pub offset: Vector2<i32>,
    pub top_left: Vector2<i32>,
    pub size: Vector2<i32>,
    pub frame_parts: Vec<AnimationFramePart>,
    #[cfg(feature = "debug")]
    pub horizontal_matrix: Matrix4<f32>,
    #[cfg(feature = "debug")]
    pub vertical_matrix: Matrix4<f32>,
}

#[derive(Clone)]
pub struct AnimationFramePart {
    pub animation_index: usize,
    pub sprite_number: usize,
    pub offset: Vector2<i32>,
    pub size: Vector2<i32>,
    pub mirror: bool,
    pub angle: f32,
    pub color: Color,
    pub affine_matrix: Matrix4<f32>,
}

impl Default for AnimationFramePart {
    fn default() -> AnimationFramePart {
        AnimationFramePart {
            animation_index: usize::MAX,
            sprite_number: usize::MAX,
            offset: Vector2::<i32>::zero(),
            size: Vector2::<i32>::zero(),
            mirror: Default::default(),
            angle: Default::default(),
            color: Default::default(),
            affine_matrix: Matrix4::<f32>::zero(),
        }
    }
}

/// A 1x1 transparent frame that is used when the animation data of an entity
/// is unusable (no actions, no delays or an action without frames). Such data
/// used to panic while rendering (division by zero / out of bounds index).
fn fallback_frame() -> &'static AnimationFrame {
    static FALLBACK_FRAME: LazyLock<AnimationFrame> = LazyLock::new(|| AnimationFrame {
        event: None,
        offset: Vector2::zero(),
        top_left: Vector2::zero(),
        size: Vector2::new(1, 1),
        frame_parts: Vec::new(),
        #[cfg(feature = "debug")]
        horizontal_matrix: Matrix4::identity(),
        #[cfg(feature = "debug")]
        vertical_matrix: Matrix4::identity(),
    });

    &FALLBACK_FRAME
}

/// Computes the animation duration for an action in milliseconds.
///
/// The ACT delay is stored per action/direction and multiplied by 50 ms (the
/// conversion the original client implies), so an action with 6 frames and a
/// delay of 4.0 takes 6 * 4.0 * 50 = 1200 ms. The longest direction wins so
/// that a one-shot animation is never cut before the last frame.
pub(crate) fn duration_from_frame_counts(action_base_offset: usize, frame_counts: &[usize], delays: &[f32]) -> u32 {
    if frame_counts.is_empty() || delays.is_empty() {
        return 0;
    }

    let mut longest = 0.0f32;

    for direction in 0..8 {
        let action_index = action_base_offset + direction;
        let frame_count = frame_counts[action_index % frame_counts.len()];
        let delay = delays[action_index % delays.len()];

        if frame_count == 0 || delay <= 0.0 {
            continue;
        }

        longest = longest.max(frame_count as f32 * delay * 50.0);
    }

    longest.max(1.0) as u32
}

impl AnimationData {
    /// Whether the ACT block starting at `action` (8 directions) contains at
    /// least one drawable frame.
    ///
    /// `action` is the block index (`32` for ready-fight, `80`/`88` for the
    /// player attack blocks), matching [`AnimationActionType::action_base_offset`].
    /// A trimmed ACT can contain only placeholder clips (`sprite_number ==
    /// usize::MAX`) for a block; such a block draws nothing.
    pub fn action_is_drawable(&self, action: usize) -> bool {
        self.animations
            .iter()
            .skip(action * 8)
            .take(8)
            .flat_map(|animation| animation.frames.iter())
            .flat_map(|frame| frame.frame_parts.iter())
            .any(|part| part.sprite_number != usize::MAX)
    }

    pub fn is_animation_over(&self, animation_state: &AnimationState) -> bool {
        if self.delays.is_empty() || self.animations.is_empty() {
            return true;
        }

        let animation_action_index = animation_state.action_type.action_base_offset(self.entity_type) * 8;

        let delay_index = animation_action_index % self.delays.len();
        let animation_index = animation_action_index % self.animations.len();

        let delay = self.delays[delay_index];
        let animation = &self.animations[animation_index];

        if animation.frames.is_empty() {
            return true;
        }

        let factor = animation_state.factor.map(|factor| delay * factor).unwrap_or_else(|| delay * 50.0);
        let factor = if factor > 0.0 { factor } else { 50.0 };

        // Duration of the real ACT action (the longest of the 8 directions) in
        // milliseconds. This is what the game files define as the length of the
        // animation, independent of the camera direction.
        let action_duration = self.action_duration_ms(animation_state);

        // An official frame sequence runs exactly for its configured duration.
        if animation_state.frame_sequence.is_some()
            && let Some(duration) = animation_state.duration
        {
            return animation_state.time >= duration;
        }

        // A looping animation (for example a cast with a cast time) finishes when
        // its duration is over, but always plays at least one full cycle so that a
        // very short cast does not cut the casting gesture off after one or two
        // frames.
        if let Some(duration) = animation_state.duration
            && animation_state.looping
        {
            return animation_state.time >= duration.max(action_duration);
        }

        // One-shot animations without an artificial duration (skill casts) run for
        // the full duration of the ACT action.
        if animation_state.duration.is_none() && action_duration > 0 {
            return animation_state.time >= action_duration;
        }

        let frame_time = animation_state
            .duration
            // A zero duration would divide by zero; a zero time cast is over
            // immediately, so treat it as the first frame.
            .map(|duration| animation_state.time * animation.frames.len() as u32 / duration.max(1))
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);

        frame_time as usize > animation.frames.len()
    }

    /// Total duration of the animation for the action behind the given state in
    /// milliseconds (the longest duration over all 8 directions, as defined by
    /// the ACT frame delays).
    pub fn action_duration_ms(&self, animation_state: &AnimationState) -> u32 {
        let frame_counts: Vec<usize> = self.animations.iter().map(|animation| animation.frames.len()).collect();

        duration_from_frame_counts(animation_state.action_base_offset, &frame_counts, &self.delays)
    }

    /// Duration of the skill-cast animation of this entity in milliseconds,
    /// used to keep effects alive for at least as long as the cast
    /// animation.
    pub fn skill_animation_duration_ms(&self) -> u32 {
        duration_from_frame_counts(
            AnimationActionType::Skill.action_base_offset(self.entity_type),
            &self.animations.iter().map(|animation| animation.frames.len()).collect::<Vec<_>>(),
            &self.delays,
        )
    }

    pub fn get_frame(&self, animation_state: &AnimationState, camera: &dyn Camera, direction: Direction) -> &AnimationFrame {
        if self.delays.is_empty() || self.animations.is_empty() {
            return fallback_frame();
        }

        let camera_direction = camera.camera_direction();
        let direction = (camera_direction + u16::from(direction) as usize) & 7;
        let animation_action_index = animation_state.action_type.action_base_offset(self.entity_type) * 8 + direction;

        let delay_index = animation_action_index % self.delays.len();
        let animation_index = animation_action_index % self.animations.len();

        let delay = self.delays[delay_index];
        let animation = &self.animations[animation_index];

        if animation.frames.is_empty() {
            return fallback_frame();
        }

        let factor = animation_state.factor.map(|factor| delay * factor).unwrap_or_else(|| delay * 50.0);
        // A zero delay in an ACT file must not divide by zero.
        let factor = if factor > 0.0 { factor } else { 50.0 };

        // Official per-skill sequences play the ACT timing of the range and
        // then loop its last frames for the remaining duration.
        let frame_index = if let Some((first, last, loop_to)) = animation_state.frame_sequence {
            let frame_time = (animation_state.time as f32 / factor) as usize;
            sequence_frame_index(frame_time, first, last, loop_to)
        } else {
            let frame_time = match animation_state.duration {
                Some(duration) if !animation_state.looping => animation_state.time * animation.frames.len() as u32 / duration.max(1),
                _ => (animation_state.time as f32 / factor) as u32,
            };

            // The sit pose must be static: the classic client shows a stable sitting
            // pose, and cycling the sit action frames/directions makes the character
            // (especially the head) bob around. Always use the first frame of the
            // sitting action instead.
            match animation_state.is_sitting() {
                true => 0,
                false => match animation_state.looping {
                    true => looped_frame_index(frame_time, animation.frames.len(), animation_state.loop_mode),
                    false => (frame_time as usize).min(animation.frames.len().saturating_sub(1)),
                },
            }
        };

        // Remove Doridori animation from Player
        if self.entity_type == EntityType::Player && animation_state.action_type == AnimationActionType::Idle {
            &animation.frames[0]
        } else {
            &animation.frames[frame_index]
        }
    }

    /// Action index and frame index currently shown for the given state,
    /// camera and facing direction.
    ///
    /// Per-motion appearance tables (`DrawOnTop`/`2DLayerDir`) are indexed by
    /// the action number (the official `previousAnimation`) and the frame
    /// number (the official `curMotion`), so the renderer needs both without
    /// selecting the actual frame texture.
    pub fn motion(&self, animation_state: &AnimationState, camera: &dyn Camera, direction: Direction) -> Option<(usize, usize)> {
        if self.delays.is_empty() || self.animations.is_empty() {
            return None;
        }

        let camera_direction = camera.camera_direction();
        let direction = (camera_direction + u16::from(direction) as usize) & 7;
        let animation_action_index = animation_state.action_index() * 8 + direction;

        let delay_index = animation_action_index % self.delays.len();
        let animation_index = animation_action_index % self.animations.len();

        let delay = self.delays[delay_index];
        let animation = &self.animations[animation_index];

        if animation.frames.is_empty() {
            return None;
        }

        let factor = animation_state.factor.map(|factor| delay * factor).unwrap_or_else(|| delay * 50.0);
        let factor = if factor > 0.0 { factor } else { 50.0 };

        let frame_index = if let Some((first, last, loop_to)) = animation_state.frame_sequence {
            let frame_time = (animation_state.time as f32 / factor) as usize;
            sequence_frame_index(frame_time, first, last, loop_to)
        } else {
            let frame_time = match animation_state.duration {
                Some(duration) if !animation_state.looping => animation_state.time * animation.frames.len() as u32 / duration.max(1),
                _ => (animation_state.time as f32 / factor) as u32,
            };

            match animation_state.is_sitting() {
                true => 0,
                false => match animation_state.looping {
                    true => looped_frame_index(frame_time, animation.frames.len(), animation_state.loop_mode),
                    false => (frame_time as usize).min(animation.frames.len().saturating_sub(1)),
                },
            }
        };

        let frame_index = if self.entity_type == EntityType::Player && animation_state.action_type == AnimationActionType::Idle {
            0
        } else {
            frame_index
        };

        Some((animation_state.action_index(), frame_index))
    }

    pub fn calculate_world_matrix(
        &self,
        camera: &dyn Camera,
        frame: &AnimationFrame,
        entity_position: Point3<f32>,
        scale: f32,
        y_scale: f32,
    ) -> Matrix4<f32> {
        // Offset the image to below the ground by frame.offset.y.
        // Add 0.5 to change from center of pixel to the lower border of pixel
        let origin_y = -frame.offset.y as f32 + 0.5;
        // TODO - TBD : Change the entity z coordinate to 0.0.
        // Add 1.0 in z-coordinate, because the entity is at point with z = 1.0.
        // The operation is performed beforehand to correctly rotate the billboard.
        let origin = Point3::new(0.0, origin_y, 0.0) * SPRITE_SCALE / TILE_SIZE + Vector3::unit_z();
        let size = Vector2::new(frame.size.x as f32 * scale, frame.size.y as f32 * scale * y_scale) * (SPRITE_SCALE / TILE_SIZE);
        camera.billboard_matrix(entity_position, origin, size)
    }

    pub fn get_texture_coordinates(&self) -> (Vector2<f32>, Vector2<f32>) {
        let cell_count = Vector2::new(1, 1);
        let cell_position = Vector2::new(0, 0);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        (texture_size, texture_position)
    }

    /// Whether the `.imf` layer order puts the head/headgear parts behind the
    /// body for the current action/frame.
    fn head_behind_body(&self, imf: &Imf, animation_state: &AnimationState, camera: &dyn Camera, direction: Direction) -> bool {
        if self.delays.is_empty() || self.animations.is_empty() {
            return false;
        }

        let camera_direction = camera.camera_direction();
        let direction = (camera_direction + u16::from(direction) as usize) & 7;
        let action_index = animation_state.action_type.action_base_offset(self.entity_type) * 8 + direction;
        let animation_index = action_index % self.animations.len();
        let animation = &self.animations[animation_index];
        if animation.frames.is_empty() {
            return false;
        }

        let delay = self.delays[action_index % self.delays.len()];
        let factor = animation_state.factor.map(|factor| delay * factor).unwrap_or(delay * 50.0);
        let factor = if factor > 0.0 { factor } else { 50.0 };
        let frame_time = match animation_state.duration {
            Some(duration) if !animation_state.looping => animation_state.time * animation.frames.len() as u32 / duration.max(1),
            _ => (animation_state.time as f32 / factor) as u32,
        };
        let frame_index = match animation_state.is_sitting() {
            true => 0,
            false => match animation_state.looping {
                true => looped_frame_index(frame_time, animation.frames.len(), animation_state.loop_mode),
                false => (frame_time as usize).min(animation.frames.len().saturating_sub(1)),
            },
        };
        let frame_index = if self.entity_type == EntityType::Player && animation_state.action_type == AnimationActionType::Idle {
            0
        } else {
            frame_index
        };

        imf.head_behind_body(action_index, frame_index)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        camera: &dyn Camera,
        add_to_picker: bool,
        entity_id: EntityId,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        direction: Direction,
        fade_alpha: f32,
        tint: Color,
        scale: f32,
        imf: Option<&Imf>,
        additive: bool,
        flat: bool,
        depth_boost: f32,
        y_scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let head_behind_body = imf.is_some_and(|imf| self.head_behind_body(imf, animation_state, camera, direction));
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position, scale, y_scale);

        for (index, frame_part) in frame.frame_parts.iter().enumerate() {
            // Frames without usable sprites are merged into a placeholder frame
            // part (with `usize::MAX` indices); it must not be rendered, and an
            // action file may reference fewer textures than the sprite has.
            let Some(animation_pair) = self.animation_pair.get(frame_part.animation_index) else {
                continue;
            };
            let Some(texture) = animation_pair.sprites.textures.get(frame_part.sprite_number) else {
                continue;
            };

            let frame_size = Vector2::new(frame.size.x as f32, frame.size.y as f32);

            let (texture_size, texture_position) = self.get_texture_coordinates();
            let (depth_offset, curvature) = camera.calculate_depth_offset_and_curvature(&world_matrix, SPRITE_SCALE, SPRITE_SCALE);

            let position = world_matrix.transform_point(Point3::from_value(0.0));
            let distance = camera.distance_to(position);
            // Optional tint of the whole entity (status effect colors such as
            // the red Fury glow, or short body flashes such as the Zen cast
            // flash). White means "unchanged".
            let color = Color::rgba(
                frame_part.color.red * tint.red,
                frame_part.color.green * tint.green,
                frame_part.color.blue * tint.blue,
                frame_part.color.alpha * tint.alpha,
            ) * fade_alpha;

            instructions.push(EntityInstruction {
                world: world_matrix,
                frame_part_transform: frame_part.affine_matrix,
                texture_position,
                texture_size,
                frame_size,
                depth_offset,
                extra_depth_offset: if head_behind_body && frame_part.animation_index > 0 {
                    // Head/headgear layers (animation part index > 0) are
                    // behind the body when the IMF says so.
                    -0.01 - 0.005 * index as f32
                } else {
                    0.005 * index as f32 + depth_boost
                },
                curvature,
                color,
                mirror: frame_part.mirror,
                entity_id,
                add_to_picker,
                additive,
                flat,
                texture: texture.clone(),
                distance,
            });
        }
    }

    #[cfg(feature = "debug")]
    #[allow(clippy::too_many_arguments)]
    pub fn render_debug(
        &self,
        instructions: &mut Vec<DebugRectangleInstruction>,
        camera: &dyn Camera,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        direction: Direction,
        color_external: Color,
        color_internal: Color,
        scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position, scale, 1.0);
        instructions.push(DebugRectangleInstruction {
            world: world_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_matrix * frame.horizontal_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_matrix * frame.vertical_matrix,
            color: color_external,
        });

        for frame_part in frame.frame_parts.iter() {
            instructions.push(DebugRectangleInstruction {
                world: world_matrix * frame_part.affine_matrix,
                color: color_internal,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sequence_frame_index;

    #[test]
    fn frame_sequences_play_the_range_and_loop_the_tail() {
        // Asura: attack frames 0..7, hold frame 7.
        assert_eq!(sequence_frame_index(0, 0, 7, 7), 0);
        assert_eq!(sequence_frame_index(6, 0, 7, 7), 6);
        assert_eq!(sequence_frame_index(7, 0, 7, 7), 7);
        assert_eq!(sequence_frame_index(20, 0, 7, 7), 7);
        // Chain Combo: only frame 5.
        assert_eq!(sequence_frame_index(3, 5, 5, 5), 5);
        // Combo Finish: frames 1..2, loop frame 2.
        assert_eq!(sequence_frame_index(0, 1, 2, 2), 1);
        assert_eq!(sequence_frame_index(1, 1, 2, 2), 2);
        assert_eq!(sequence_frame_index(2, 1, 2, 2), 2);
        // Chain Crush: frames 3..4, loop frame 4.
        assert_eq!(sequence_frame_index(5, 3, 4, 4), 4);
    }

    use ragnarok_packets::ClientTick;

    use super::{AnimationActionType, AnimationLoopMode, AnimationState, looped_frame_index};
    use crate::world::EntityType;

    #[test]
    fn zero_duration_and_empty_animation_data_do_not_panic() {
        use ragnarok_packets::ClientTick;

        use super::{AnimationData, AnimationState};
        use crate::world::EntityType;

        // The crash report from the Geffenia maps: a zero duration animation
        // state divides by zero in `is_animation_over`.
        let mut state = AnimationState::new(EntityType::Monster, ClientTick(0));
        state.attack(EntityType::Monster, 0, AnimationActionType::Attack1, ClientTick(0));
        state.duration = Some(0);

        let animation_data = AnimationData {
            animation_pair: Vec::new(),
            animations: vec![super::Animation {
                frames: vec![super::fallback_frame().clone()],
            }],
            delays: vec![100.0],
            entity_type: EntityType::Monster,
        };

        // Must not divide by zero; the exact result is not important here.
        let _ = animation_data.is_animation_over(&state);

        // An entity without any usable animation data must also be handled.
        let empty_animation_data = AnimationData {
            animation_pair: Vec::new(),
            animations: Vec::new(),
            delays: Vec::new(),
            entity_type: EntityType::Monster,
        };

        assert!(empty_animation_data.is_animation_over(&AnimationState::new(EntityType::Monster, ClientTick(0))));
    }

    #[test]
    fn player_idle_uses_the_armed_ready_fight_block() {
        // The kRO player body/weapon ACTs animate the armed stand pose in the
        // ready-fight block 32..39; block 0..7 is the weaponless relaxation
        // pose. Idling on block 0 made every equipped weapon layer invisible
        // because no per-job weapon ACT has frames there.
        let state = AnimationState::new(EntityType::Player, ClientTick(0));
        assert_eq!(state.action_index(), 4);
        assert_eq!(state.action_index() * 8, 32);

        // Non-players keep the non-player layout (idle is ACT block 0).
        assert_eq!(AnimationState::new(EntityType::Monster, ClientTick(0)).action_index(), 0);
        assert_eq!(AnimationState::new(EntityType::Npc, ClientTick(0)).action_index(), 0);
    }

    #[test]
    fn player_attack_uses_a_player_attack_block() {
        // ST_THROW (block 5 = 40..47) is not a weapon swing block; the player
        // attack animations are 80..87 (Attack2) and 88..95 (Attack3).
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.attack(EntityType::Player, 100, AnimationActionType::Attack2, ClientTick(0));
        assert_eq!(state.action_index(), 10);
        assert_eq!(state.action_index() * 8, 80);

        // Monsters keep the non-player attack block (2).
        let mut monster = AnimationState::new(EntityType::Monster, ClientTick(0));
        monster.attack(EntityType::Monster, 100, AnimationActionType::Attack1, ClientTick(0));
        assert_eq!(monster.action_index(), 2);
    }

    #[test]
    fn action_is_drawable_only_for_blocks_with_real_frames() {
        // Mirrors the kRO 2019 weapon ACT layout: the armed stand pose is in
        // block 4 (32..39), the swing in block 10 (80..87), and every other
        // block is a hidden placeholder. The player idle/attack mapping must
        // therefore target block 4 / 10 instead of the classic idle block 0.
        let hidden = super::fallback_frame().clone();
        let visible = super::AnimationFrame {
            frame_parts: vec![super::AnimationFramePart {
                sprite_number: 0,
                ..Default::default()
            }],
            ..hidden.clone()
        };
        let animations = (0..104)
            .map(|action| super::Animation {
                frames: vec![match action / 8 {
                    4 | 10 => visible.clone(),
                    _ => hidden.clone(),
                }],
            })
            .collect();
        let animation_data = super::AnimationData {
            animation_pair: Vec::new(),
            animations,
            delays: vec![100.0; 104],
            entity_type: EntityType::Player,
        };

        assert!(!animation_data.action_is_drawable(0));
        assert!(animation_data.action_is_drawable(4));
        assert!(animation_data.action_is_drawable(10));
        assert!(!animation_data.action_is_drawable(11));
    }

    #[test]
    fn action_duration_uses_the_longest_direction_and_the_act_delays() {
        use super::duration_from_frame_counts;

        // Player skill action: 8 directions with 6 frames each and delay 4.0.
        let mut frame_counts = vec![3; 104];
        for direction in 0..8 {
            frame_counts[12 * 8 + direction] = 6;
        }
        let delays = vec![4.0f32; 104];

        assert_eq!(duration_from_frame_counts(12 * 8, &frame_counts, &delays), 1200);
    }

    #[test]
    fn action_duration_uses_the_longest_direction() {
        use super::duration_from_frame_counts;

        let mut frame_counts = vec![3usize; 104];
        frame_counts[12 * 8] = 4;
        frame_counts[12 * 8 + 3] = 9;
        let delays = vec![4.0f32; 104];

        // The longest direction (9 * 4.0 * 50 ms) wins, so the animation is not cut
        // before the last frame of the direction that is actually drawn.
        assert_eq!(duration_from_frame_counts(12 * 8, &frame_counts, &delays), 1800);
    }

    #[test]
    fn instant_skill_cast_plays_the_full_act_action() {
        use super::{Animation, AnimationData};

        let animation_data = AnimationData {
            animation_pair: Vec::new(),
            animations: (0..104)
                .map(|_| Animation {
                    frames: (0..6).map(|_| super::fallback_frame().clone()).collect(),
                })
                .collect(),
            delays: vec![4.0; 104],
            entity_type: EntityType::Player,
        };

        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));
        state.skill(EntityType::Player, 0, false, ClientTick(0));

        // 6 frames * 4.0 delay * 50 ms = 1200 ms; the cast animation must not be
        // terminated early ("some of the effect happens but not the whole
        // animation").
        state.update(ClientTick(1_000));
        assert!(!animation_data.is_animation_over(&state));
        state.update(ClientTick(1_250));
        assert!(animation_data.is_animation_over(&state));
    }

    #[test]
    fn player_action_offsets_match_the_ro_act_file() {
        // Idle is the armed ready-fight block: the classic stand block 0..7 is
        // the weaponless pose and no per-job weapon ACT animates it.
        assert_eq!(AnimationActionType::Idle.action_base_offset(EntityType::Player), 4);
        assert_eq!(AnimationActionType::Walk.action_base_offset(EntityType::Player), 1);
        assert_eq!(AnimationActionType::Sit.action_base_offset(EntityType::Player), 2);
        assert_eq!(AnimationActionType::Pickup.action_base_offset(EntityType::Player), 3);
        assert_eq!(AnimationActionType::ReadyFight.action_base_offset(EntityType::Player), 4);
        // `Attack1` is the ST_THROW block (40..47), used by throw skills, not by
        // the auto attack; the weapon swings in `Attack2`/`Attack3`.
        assert_eq!(AnimationActionType::Attack1.action_base_offset(EntityType::Player), 5);
        assert_eq!(AnimationActionType::Die.action_base_offset(EntityType::Player), 8);
        assert_eq!(AnimationActionType::Attack2.action_base_offset(EntityType::Player), 10);
        assert_eq!(AnimationActionType::Attack3.action_base_offset(EntityType::Player), 11);
        assert_eq!(AnimationActionType::Skill.action_base_offset(EntityType::Player), 12);
    }

    #[test]
    fn sitting_and_standing_switch_the_animation() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));

        state.sit(EntityType::Player, ClientTick(10));
        assert!(state.is_sitting());
        assert!(!state.is_skill());
        assert!(state.looping);

        state.stand_up(EntityType::Player, ClientTick(20));
        assert!(!state.is_sitting());
        assert!(state.looping);
    }

    #[test]
    fn cast_time_casts_loop_the_cast_animation() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));

        state.skill(EntityType::Player, 1200, true, ClientTick(10));
        assert!(state.is_skill());
        assert!(!state.is_sitting());
        assert!(state.looping);
        assert_eq!(state.duration, Some(1200));
    }

    #[test]
    fn instant_casts_play_the_cast_animation_once_at_natural_speed() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));

        state.skill(EntityType::Player, 0, false, ClientTick(10));
        assert!(state.is_skill());
        assert!(!state.looping);
        // No artificial duration: the ACT file defines the animation length, so the
        // full animation is played instead of squeezing it into a fixed time.
        assert_eq!(state.duration, None);
    }
}

#[cfg(test)]
mod loop_mode_tests {
    use super::{AnimationLoopMode, looped_frame_index};

    #[test]
    fn repeat_wraps_to_the_first_frame() {
        let frames = |t: u32| looped_frame_index(t, 4, AnimationLoopMode::Repeat);
        assert_eq!([frames(0), frames(1), frames(2), frames(3), frames(4)], [0, 1, 2, 3, 0]);
    }

    #[test]
    fn pingpong_reverses_without_repeating_the_end_frames() {
        let frames = |t: u32| looped_frame_index(t, 4, AnimationLoopMode::PingPong);
        // 0,1,2,3,2,1 then back to 0.
        assert_eq!([frames(0), frames(1), frames(2), frames(3), frames(4), frames(5), frames(6)], [
            0, 1, 2, 3, 2, 1, 0
        ]);
    }

    #[test]
    fn pingpong_handles_short_and_empty_animations() {
        assert_eq!(looped_frame_index(5, 0, AnimationLoopMode::PingPong), 0);
        assert_eq!(looped_frame_index(5, 1, AnimationLoopMode::PingPong), 0);
        assert_eq!(looped_frame_index(5, 2, AnimationLoopMode::PingPong), 1);
    }
}
