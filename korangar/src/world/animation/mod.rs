use std::sync::{Arc, LazyLock};

#[cfg(feature = "debug")]
use cgmath::SquareMatrix;
use cgmath::{Array, Matrix4, Point3, Transform, Vector2, Vector3, Zero};
use korangar_container::Cacheable;
use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, Direction, EntityId};
use rust_state::RustState;

#[cfg(feature = "debug")]
use crate::graphics::DebugRectangleInstruction;
use crate::graphics::{Color, EntityInstruction};
use crate::loaders::Sprite;
use crate::world::{ActionEvent, Actions, Camera, EntityType};

pub(crate) const TILE_SIZE: f32 = 10.0;
pub(crate) const SPRITE_SCALE: f32 = 1.4;
const PICKUP_ANIMATION_FACTOR: f32 = 25.0;

#[allow(dead_code)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum AnimationActionType {
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
}

impl AnimationActionType {
    pub fn action_base_offset(&self, entity_type: EntityType) -> usize {
        match entity_type {
            EntityType::Hidden | EntityType::Player => match self {
                AnimationActionType::Idle => 0,
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
                _ => 0,
            },
            EntityType::Npc | EntityType::Monster => match self {
                AnimationActionType::Idle => 0,
                AnimationActionType::Walk => 1,
                AnimationActionType::Attack1 => 2,
                AnimationActionType::Hurt => 3,
                AnimationActionType::Die => 4,
                _ => 0,
            },
            EntityType::Warp => 0,
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
        }
    }

    pub fn idle(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Idle;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = true;
    }

    pub fn attack(&mut self, entity_type: EntityType, attack_duration: u32, critical: bool, client_tick: ClientTick) {
        self.action_type = match critical {
            true => AnimationActionType::Attack3,
            false => AnimationActionType::Attack1,
        };
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
        self.time = client_tick.0.wrapping_sub(self.start_time.0);
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

        let frame_time = match animation_state.duration {
            Some(duration) if !animation_state.looping => animation_state.time * animation.frames.len() as u32 / duration.max(1),
            _ => (animation_state.time as f32 / factor) as u32,
        };

        let frame_index = match animation_state.looping {
            true => frame_time as usize % animation.frames.len(),
            false => (frame_time as usize).min(animation.frames.len().saturating_sub(1)),
        };

        // Remove Doridori animation from Player
        if self.entity_type == EntityType::Player && animation_state.action_type == AnimationActionType::Idle {
            &animation.frames[0]
        } else {
            &animation.frames[frame_index]
        }
    }

    pub fn calculate_world_matrix(
        &self,
        camera: &dyn Camera,
        frame: &AnimationFrame,
        entity_position: Point3<f32>,
        scale: f32,
    ) -> Matrix4<f32> {
        // Offset the image to below the ground by frame.offset.y.
        // Add 0.5 to change from center of pixel to the lower border of pixel
        let origin_y = -frame.offset.y as f32 + 0.5;
        // TODO - TBD : Change the entity z coordinate to 0.0.
        // Add 1.0 in z-coordinate, because the entity is at point with z = 1.0.
        // The operation is performed beforehand to correctly rotate the billboard.
        let origin = Point3::new(0.0, origin_y, 0.0) * SPRITE_SCALE / TILE_SIZE + Vector3::unit_z();
        let size = Vector2::new(frame.size.x as f32, frame.size.y as f32) * (SPRITE_SCALE / TILE_SIZE) * scale;
        camera.billboard_matrix(entity_position, origin, size)
    }

    pub fn get_texture_coordinates(&self) -> (Vector2<f32>, Vector2<f32>) {
        let cell_count = Vector2::new(1, 1);
        let cell_position = Vector2::new(0, 0);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        (texture_size, texture_position)
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
        scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position, scale);

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
            let color = frame_part.color * fade_alpha;

            instructions.push(EntityInstruction {
                world: world_matrix,
                frame_part_transform: frame_part.affine_matrix,
                texture_position,
                texture_size,
                frame_size,
                depth_offset,
                extra_depth_offset: 0.005 * index as f32,
                curvature,
                color,
                mirror: frame_part.mirror,
                entity_id,
                add_to_picker,
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
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position, scale);
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
    use ragnarok_packets::ClientTick;

    use super::{AnimationActionType, AnimationState};
    use crate::world::EntityType;

    #[test]
    fn zero_duration_and_empty_animation_data_do_not_panic() {
        use ragnarok_packets::ClientTick;

        use super::{AnimationData, AnimationState};
        use crate::world::EntityType;

        // The crash report from the Geffenia maps: a zero duration animation
        // state divides by zero in `is_animation_over`.
        let mut state = AnimationState::new(EntityType::Monster, ClientTick(0));
        state.attack(EntityType::Monster, 0, false, ClientTick(0));
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
        assert_eq!(AnimationActionType::Idle.action_base_offset(EntityType::Player), 0);
        assert_eq!(AnimationActionType::Walk.action_base_offset(EntityType::Player), 1);
        assert_eq!(AnimationActionType::Sit.action_base_offset(EntityType::Player), 2);
        assert_eq!(AnimationActionType::Pickup.action_base_offset(EntityType::Player), 3);
        assert_eq!(AnimationActionType::Attack1.action_base_offset(EntityType::Player), 5);
        assert_eq!(AnimationActionType::Die.action_base_offset(EntityType::Player), 8);
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
