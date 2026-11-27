//! Hunter falcon (`CGameActor::statusEffects[0]`) flight state.
//!
//! The falcon is not an effect body: the official client creates it as a
//! message-effect actor when the entity gets the `OPTION_FALCON` bit
//! (`GameObject.cpp` `AM_MAKE_BIRD`) and keeps it in `statusEffects[0]`.
//!
//! # Official motion chain
//!
//! * `CGameActor::ProcessSkillMessage` sends the attacked target's position to
//!   `GetEffectCounter(skillId)` for `IsFalconAttackSkill()`
//!   (`RenderObject.cpp:5388`, `GameActor.cpp:894-925`); `SendValueMessage`
//!   turns that into `AM_REQ_ATTACK` (`GameObject.cpp:954-958`).
//! * `CMsgEffect::SendMsg` stores `(target - position) / 10` as the velocity
//!   for `MET_BIRD`, sets `activeMotion = 1` and clears `motionInitialized`
//!   (`MsgEffect.cpp:1611-1617`). It deliberately does **not** reset
//!   `processTick` for the bird (unlike the warg types 27/31 in the next
//!   branch), so a re-trigger keeps the current flight frame.
//! * `CMsgEffect::OnProcess` (`MsgEffect.cpp:206-224`) keeps the actor alive
//!   while `activeMotion` is set. The bird advances by the stored velocity once
//!   per 16 ms logic tick (`MsgEffect.cpp:489-525`):
//!   - it faces away from the owner (`Face(owner)` then `angle -= 180`) and
//!     uses flight actions `8..15`;
//!   - x and z move every tick by the velocity;
//!   - y moves only while `processTick <= 10 || processTick >= 25`;
//!   - `processTick == 25` sets `m_finished` and reverses the y velocity, but
//!     the actor keeps moving until `processTick > 35`, then `activeMotion = 0`
//!     (`MsgEffect.cpp:212-221`, `:504-509`).
//!   A fresh flight therefore travels **36** velocity steps: it crosses the
//!   target at step 10 and ends at `perch + 3.6 * (target - perch)`, i.e.
//!   `2.6 * (target - perch)` past the target. There is no strike hold.
//! * Afterwards `FollowOwner` (`MsgEffect.cpp:1500-1531`) homes the bird back
//!   to the owner's **current** position: 4% of the remaining distance per tick
//!   until it is within 25, then 1 world unit per tick until within 12, then 5%
//!   of the remaining distance per tick until within 3; y snaps to the owner's
//!   ground offset. The perched actions `0..7` are used for this return leg.
//! * Re-triggering during the animation only replaces the velocity
//!   (`MsgEffect.cpp:1612-1617`): it starts from the bird's current position
//!   (which may be far off-screen) and keeps `processTick`, so it does not snap
//!   back to the hunter and the remaining frames are not restarted.
//! * Unsummoning sends `AM_DELETE_BIRD` -> `AM_DISAPPEAR`, which sets
//!   `m_finished` (`MsgEffect.cpp:1640-1642`); the entity layer clears the
//!   flight in `Common::set_effect_state`.
//!
//! # Interpreted constants
//!
//! The decompiled y reversal is applied on *every* tick from 25 through 35
//! (`MsgEffect.cpp:504-507`), i.e. `velocity.y *= -1.9` eleven times. That is a
//! divergent geometric series (the bird would jump thousands of units
//! vertically) and cannot be the intended flight. The port applies the
//! `-1.9` factor **once**, at `processTick == 25`, and then uses the reversed
//! vertical velocity for the rest of the post-strike leg. The horizontal rule,
//! which is what makes the bird leave the screen and return from the edge, is
//! taken literally.
//!
//! The official bird sprite (`data\sprite\이팩트\매.act`, 16 actions) stores
//! eight perched animations (actions 0-7, eight flap frames each) and eight
//! one-frame flight animations (actions 8-15). `AnimationState` indexes an
//! animation as `action * 8 + direction`, so the perched action is `0` and the
//! flight action is `1`.

use cgmath::{Point3, Vector3};
use ragnarok_packets::{ClientTick, Direction, SkillId};

/// Flight action of the falcon ACT (`animation + 8 + dir`, actions 8-15).
///
/// The perched state uses the entity layer's `Idle` action (base offset 0),
/// which is why only the flight action needs a constant.
pub(crate) const FALCON_ACTION_FLIGHT: usize = 1;

/// Logic tick of the classic client (`InitTimer(60)`, `Launcher.cpp:1522`).
///
/// `CMsgEffect::OnProcess` advances one `processTick` per logic frame, so the
/// decompiled frame counts below are expressed in these 60 Hz ticks.
const LOGIC_TICK_MS: u32 = 16;

/// `AM_REQ_ATTACK` divides the target offset by 10 (`MsgEffect.cpp:1612-1614`).
const VELOCITY_DIVISOR: f32 = 10.0;

/// The y velocity is only applied while the bird dives and after the strike
/// (`MsgEffect.cpp:499-501`).
const DIVE_Y_END_TICK: i32 = 10;

/// `processTick >= 25` sets `m_finished` (`MsgEffect.cpp:504-506`).
const FLIGHT_FINISH_TICK: i32 = 25;

/// `processTick > 35` clears `activeMotion` (`MsgEffect.cpp:215-219`).
const FLIGHT_END_TICK: i32 = 35;

/// Decompiled post-strike vertical reversal factor (`MsgEffect.cpp:506`).
///
/// Applied once instead of on every tick; see the module docs.
const POST_STRIKE_Y_REVERSAL: f32 = -1.9;

/// `FollowOwner` far-approach threshold and divisor
/// (`MsgEffect.cpp:1513-1519`).
const RETURN_FAR_DISTANCE: f32 = 25.0;
const RETURN_FAR_DIVISOR: f32 = 25.0;

/// `FollowOwner` unit-step threshold (`MsgEffect.cpp:1529`).
const RETURN_MEDIUM_DISTANCE: f32 = 12.0;

/// `FollowOwner` final-approach threshold and divisor
/// (`MsgEffect.cpp:1534-1540`).
const RETURN_STOP_DISTANCE: f32 = 3.0;
const RETURN_STOP_DIVISOR: f32 = 20.0;

/// The bird is created with `SetAnimationSpeed(6.0f)` (`GameObject.cpp:1436`),
/// so its perched `processTick` wraps every six frames
/// (`MsgEffect.cpp:520-531`). Re-triggers keep that counter.
const PERCHED_TICK_CYCLE: i32 = 6;

/// Skill ids for which the official client returns `statusEffects[0]` from
/// `CGameActor::GetEffectCounter` (`IsFalconAttackSkill`,
/// `GameActor.cpp:894-905`).
///
/// Values are from the official `SkillId.h`: `HT_BLITZBEAT` 129,
/// `SN_FALCONASSAULT` 381, `WH_HAWKRUSH` 5326, `WH_HAWKBOOMERANG` 5329 and
/// `WH_WILD_WALK` 6520.
const FALCON_ATTACK_SKILLS: [u16; 5] = [129, 381, 5326, 5329, 6520];

/// Whether `skill_id` makes the companion falcon attack the target.
pub fn is_falcon_attack_skill(skill_id: SkillId) -> bool {
    FALCON_ATTACK_SKILLS.contains(&skill_id.0)
}

/// One falcon-attack flight.
///
/// Mirrors the `CMsgEffect` bird fields that drive the motion: position,
/// `m_velocity`, `processTick`, `activeMotion`, `motionInitialized` and
/// `m_finished`. The actor is stepped once per 16 ms logic tick.
#[derive(Clone, Debug)]
pub(crate) struct FalconFlight {
    /// Current bird position; unlike the perched pose this is independent of
    /// the owner while `activeMotion` is set.
    position: Point3<f32>,
    /// `m_velocity`; recomputed from the current position on a re-trigger.
    velocity: Vector3<f32>,
    /// `processTick`: the flight frame counter. Not reset by a re-trigger.
    process_tick: i32,
    /// `activeMotion`: `true` for the dive/post-strike leg, `false` while
    /// `FollowOwner` returns the bird.
    active_motion: bool,
    /// `motionInitialized`: cleared by every `AM_REQ_ATTACK`, set once the
    /// return leg gets within 25 units of the owner.
    motion_initialized: bool,
    /// `m_finished`: set at `processTick >= 25`, cleared when the actor moves
    /// back to `activeMotion == 0`.
    finished: bool,
    /// The single post-strike y reversal has been applied.
    post_strike_y_reversed: bool,
    /// ACT direction: away from the owner during the flight, toward the owner
    /// while returning/perched.
    direction: Direction,
    /// Last logic tick already simulated.
    last_tick: ClientTick,
    /// The return leg reached the owner; the caller drops the flight and
    /// resumes the perched render.
    done: bool,
}

/// Current falcon pose; the perched pose is derived from the owner, a flight
/// pose carries its own world position.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FalconPose {
    pub position: Point3<f32>,
    pub direction: Direction,
    /// `true` while the falcon plays the flight action instead of the perched
    /// flap.
    pub flying: bool,
}

impl FalconFlight {
    /// Starts a flight from `perch` to `target`.
    ///
    /// This is the `AM_REQ_ATTACK` branch for a bird that is not currently
    /// moving: velocity is `(target - position) / 10`, `activeMotion` is set
    /// and `motionInitialized` is cleared (`MsgEffect.cpp:1611-1617`). A
    /// target on top of the bird is not special-cased: the official client
    /// starts the same 36-frame motion with a zero velocity.
    pub fn new(perch: Point3<f32>, target: Point3<f32>, client_tick: ClientTick) -> Option<Self> {
        let velocity = (target - perch) / VELOCITY_DIVISOR;
        if !velocity.x.is_finite() || !velocity.y.is_finite() || !velocity.z.is_finite() {
            return None;
        }

        Some(Self {
            position: perch,
            velocity,
            process_tick: 0,
            active_motion: true,
            motion_initialized: false,
            finished: false,
            post_strike_y_reversed: false,
            direction: direction_towards(perch, target),
            last_tick: client_tick,
            done: false,
        })
    }

    /// Re-issues `AM_REQ_ATTACK` while the bird is already in the air.
    ///
    /// The official handler only overwrites the velocity (and clears
    /// `motionInitialized`); the position and `processTick` are kept
    /// (`MsgEffect.cpp:1611-1617`). The bird therefore turns from wherever it
    /// currently is (possibly off-screen) and continues with the frames that
    /// are left in the current flight instead of snapping back to the hunter.
    pub fn retarget(&mut self, target: Point3<f32>, client_tick: ClientTick, perch: Point3<f32>) {
        if self.done {
            self.position = perch;
            self.process_tick = 0;
            self.last_tick = client_tick;
        } else {
            self.advance_to(client_tick, perch);
        }

        self.velocity = (target - self.position) / VELOCITY_DIVISOR;
        self.active_motion = true;
        self.motion_initialized = false;
        self.finished = false;
        self.post_strike_y_reversed = false;
        self.done = false;
        self.direction = direction_towards(perch, self.position);
    }

    /// Simulates every complete 16 ms logic tick up to `now` and returns the
    /// current pose, or `None` once the bird is back on its perch.
    pub fn advance_to(&mut self, now: ClientTick, perch: Point3<f32>) -> Option<FalconPose> {
        if self.done {
            return None;
        }

        let ticks = now.0.saturating_sub(self.last_tick.0) / LOGIC_TICK_MS;
        for _ in 0..ticks {
            self.step(perch);
            self.last_tick.0 = self.last_tick.0.saturating_add(LOGIC_TICK_MS);
            if self.done {
                self.last_tick = now;
                break;
            }
        }

        self.pose()
    }

    /// Current pose without advancing the simulation (used by the renderer,
    /// which runs after the per-frame entity update).
    pub fn pose(&self) -> Option<FalconPose> {
        if self.done {
            return None;
        }

        Some(FalconPose {
            position: self.position,
            direction: self.direction,
            flying: self.active_motion,
        })
    }

    /// One `CMsgEffect::OnProcess` bird tick (`MsgEffect.cpp:206-224`,
    /// `:489-531`).
    fn step(&mut self, perch: Point3<f32>) {
        if self.finished {
            if self.process_tick > FLIGHT_END_TICK && self.active_motion {
                // `m_finished` + `processTick > 35`: end the flight. The
                // official actor returns before `FollowOwner` on this tick.
                self.active_motion = false;
                self.finished = false;
                return;
            }
            if !self.active_motion {
                self.done = true;
                return;
            }
        }

        if self.active_motion {
            self.step_flight(perch);
        } else {
            self.step_return(perch);
        }
    }

    /// `MsgEffect.cpp:490-512`: move by `m_velocity`, face away from the owner
    /// and use the flight action.
    fn step_flight(&mut self, perch: Point3<f32>) {
        if horizontal_distance(self.position, perch) > f32::EPSILON {
            // `Face(owner)` then `angle -= 180`, i.e. face away from the owner.
            self.direction = direction_towards(perch, self.position);
        }

        self.position.x += self.velocity.x;
        if self.process_tick <= DIVE_Y_END_TICK || self.process_tick >= FLIGHT_FINISH_TICK {
            self.position.y += self.velocity.y;
        }
        self.position.z += self.velocity.z;

        if self.process_tick >= FLIGHT_FINISH_TICK {
            self.finished = true;
            if !self.post_strike_y_reversed {
                self.post_strike_y_reversed = true;
                self.velocity.y *= POST_STRIKE_Y_REVERSAL;
            }
        }

        self.process_tick += 1;
    }

    /// `FollowOwner` for `MET_BIRD` (`MsgEffect.cpp:1500-1531`) plus the
    /// perched animation counter (`MsgEffect.cpp:514-531`).
    fn step_return(&mut self, perch: Point3<f32>) {
        self.follow_owner(perch);

        // SetAnimationSpeed(6.0f): the perched processTick wraps every six
        // frames and only then advances the flap animation.
        if self.process_tick % PERCHED_TICK_CYCLE == 0 {
            self.process_tick = 0;
        }
        self.process_tick += 1;

        if horizontal_distance(self.position, perch) <= RETURN_STOP_DISTANCE {
            // The official bird hovers within 3 units of the owner and never
            // marks the return "finished". The port drops the actor state here
            // and renders the shared perched pose again.
            self.position = perch;
            self.done = true;
        }
    }

    /// The three-stage homing used by `FollowOwner` for `MET_BIRD`.
    fn follow_owner(&mut self, perch: Point3<f32>) {
        let distance = horizontal_distance(self.position, perch);

        if !self.motion_initialized {
            // Too far from the owner (or first return frame): close 1/25 of
            // the gap per tick. Once within 25 the next stage takes over.
            self.position.x += (perch.x - self.position.x) / RETURN_FAR_DIVISOR;
            self.position.z += (perch.z - self.position.z) / RETURN_FAR_DIVISOR;
            if distance <= RETURN_FAR_DISTANCE {
                self.motion_initialized = true;
            }
        } else if distance > RETURN_MEDIUM_DISTANCE {
            // `position += (owner - position) / dist`: one world unit per tick.
            self.position.x += (perch.x - self.position.x) / distance;
            self.position.z += (perch.z - self.position.z) / distance;
        } else if distance > RETURN_STOP_DISTANCE {
            // Final approach: 5% of the remaining gap per tick.
            self.position.x += (perch.x - self.position.x) / RETURN_STOP_DIVISOR;
            self.position.z += (perch.z - self.position.z) / RETURN_STOP_DIVISOR;
        }

        // The official return leg writes the terrain height minus 25 every
        // tick; the port's perch already carries the owner-relative height.
        self.position.y = perch.y;

        if distance > f32::EPSILON {
            self.direction = direction_towards(self.position, perch);
        }
    }
}

fn horizontal_distance(a: Point3<f32>, b: Point3<f32>) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

/// The direction a sprite at `from` has to face to look at `to`.
///
/// Uses the same self-minus-target convention as `Entity::rotate_towards`, so
/// the flight direction is the same 0..7 ACT direction the entity layer uses
/// for attacks and casts. The dominant axis is chosen before the diagonal so a
/// target that is one unit off-axis still faces straight.
pub(crate) fn direction_towards(from: Point3<f32>, to: Point3<f32>) -> Direction {
    let delta_x = from.x - to.x;
    let delta_z = from.z - to.z;
    let horizontal = delta_x.abs().max(delta_z.abs());

    if horizontal <= f32::EPSILON {
        return Direction::North;
    }

    // Snap to the nearest of the eight directions: straight when the minor
    // axis is less than `tan(22.5°)` (~0.414) of the major one.
    let straight = 0.414;
    let (x, z) = if delta_x.abs() * straight >= delta_z.abs() {
        (delta_x.signum() as isize, 0)
    } else if delta_z.abs() * straight >= delta_x.abs() {
        (0, delta_z.signum() as isize)
    } else {
        (delta_x.signum() as isize, delta_z.signum() as isize)
    };

    Direction::try_from([x, z]).unwrap_or(Direction::North)
}

#[cfg(test)]
mod tests {
    use cgmath::InnerSpace;

    use super::*;

    fn point(x: f32, y: f32, z: f32) -> Point3<f32> {
        Point3::new(x, y, z)
    }

    /// Advance `ticks` logic frames from the flight's current `last_tick`.
    fn advance(flight: &mut FalconFlight, ticks: u32, perch: Point3<f32>) -> Option<FalconPose> {
        let now = ClientTick(flight.last_tick.0 + ticks * LOGIC_TICK_MS);
        flight.advance_to(now, perch)
    }

    #[test]
    fn falcon_attack_skills_are_recognised() {
        for skill in FALCON_ATTACK_SKILLS {
            assert!(is_falcon_attack_skill(SkillId(skill)), "skill {skill}");
        }
        for skill in [5, 13, 129 + 1, 380, 382, 5327] {
            assert!(!is_falcon_attack_skill(SkillId(skill)), "skill {skill}");
        }
    }

    #[test]
    fn flight_uses_the_official_dive_and_post_strike_velocity() {
        let perch = point(0.0, 26.0, 0.0);
        let target = point(100.0, 0.0, 0.0);
        let mut flight = FalconFlight::new(perch, target, ClientTick(0)).expect("finite target");

        // `AM_REQ_ATTACK` itself does not move the bird; the first OnProcess
        // tick does.
        let start = flight.pose().expect("flight is active");
        assert_eq!(start.position, perch);
        assert!(start.flying);

        // Velocity is (target - perch) / 10 = 10 units per tick horizontally.
        let first = advance(&mut flight, 1, perch).expect("flight is active");
        assert!((first.position.x - 10.0).abs() < 1e-4, "{:?}", first.position);
        assert!((first.position.y - 23.4).abs() < 1e-4, "first dive y: {:?}", first.position);

        // The bird crosses the target at step 10 (25% of the 36-step flight).
        advance(&mut flight, 9, perch);
        let on_target = flight.pose().expect("flight is active");
        assert!((on_target.position.x - target.x).abs() < 1e-4, "{:?}", on_target.position);
        assert!((on_target.position.y - 0.0).abs() < 1e-4, "dive y: {:?}", on_target.position);

        // One more step executes processTick 10, the last dive-y frame; the
        // bird is then slightly below the target height and starts gliding
        // level while x/z keep moving.
        advance(&mut flight, 1, perch);
        let dive_end = flight.pose().expect("flight is active");
        assert!((dive_end.position.x - 110.0).abs() < 1e-4, "{:?}", dive_end.position);
        assert!((dive_end.position.y - (-2.6)).abs() < 1e-4, "{:?}", dive_end.position);

        // y holds through processTick 24; at step 24 the bird is 2.4x the
        // perch-target distance from the perch.
        advance(&mut flight, 13, perch);
        let hold = flight.pose().expect("flight is active");
        assert!((hold.position.x - 240.0).abs() < 1e-4, "{:?}", hold.position);
        assert!((hold.position.y - (-2.6)).abs() < 1e-4, "{:?}", hold.position);

        // Step 25 executes processTick 24, so m_finished is still not set and
        // the y reversal has not happened yet.
        advance(&mut flight, 1, perch);
        let before_strike = flight.pose().expect("flight is active");
        assert!((before_strike.position.x - 250.0).abs() < 1e-4, "{:?}", before_strike.position);
        assert!((before_strike.position.y - (-2.6)).abs() < 1e-4, "{:?}", before_strike.position);

        // Step 26 executes processTick 25: it sets m_finished, applies the
        // (single, see module docs) y reversal and continues.
        advance(&mut flight, 1, perch);
        let strike = flight.pose().expect("flight is still moving");
        assert!((strike.position.x - 260.0).abs() < 1e-4, "{:?}", strike.position);
        assert!(
            (strike.position.y - (-5.2)).abs() < 1e-4,
            "post-strike y: {:?}",
            strike.position
        );

        // Ticks 26..=34 move with the reversed y velocity and unchanged x/z.
        advance(&mut flight, 9, perch);
        let post_strike = flight.pose().expect("flight is still moving");
        assert!((post_strike.position.x - 350.0).abs() < 1e-4, "{:?}", post_strike.position);
        assert!(
            (post_strike.position.y - 39.26).abs() < 1e-3,
            "pull-up y: {:?}",
            post_strike.position
        );

        // Tick 35 is the last moving frame; processTick 36 ends activeMotion
        // on the following tick without moving the bird.
        advance(&mut flight, 1, perch);
        let last_moving = flight.pose().expect("flight is still moving");
        assert!((last_moving.position.x - 360.0).abs() < 1e-4, "{:?}", last_moving.position);
        assert!((last_moving.position.y - 44.2).abs() < 1e-3, "{:?}", last_moving.position);
        assert!(last_moving.flying);

        let end = advance(&mut flight, 1, perch).expect("return leg starts");
        assert!(!end.flying);
        assert!((end.position.x - 360.0).abs() < 1e-4, "{:?}", end.position);
    }

    #[test]
    fn flight_leaves_the_screen_past_the_target() {
        // 50 cells is a normal attack range; the official 36-step velocity
        // takes the bird to 3.6x the target offset, 2.6x *past* the target.
        let perch = point(0.0, 0.0, 0.0);
        let target = point(250.0, 0.0, 0.0);
        let mut flight = FalconFlight::new(perch, target, ClientTick(0)).expect("finite target");

        // 36 moving frames, then one tick that only switches to the return leg.
        advance(&mut flight, 37, perch);
        let final_pose = flight.pose().expect("return leg starts");
        let travelled = final_pose.position.x - perch.x;
        let passed_target = final_pose.position.x - target.x;

        assert!((travelled - 900.0).abs() < 1e-3, "travelled {travelled}");
        assert!((passed_target - 650.0).abs() < 1e-3, "past target {passed_target}");
        assert!(
            passed_target > 2.0 * (target.x - perch.x),
            "the bird must be well past the target, not normalised onto it"
        );
    }

    #[test]
    fn retrigger_mid_flight_continues_from_the_current_position() {
        let perch = point(0.0, 0.0, 0.0);
        let first_target = point(100.0, 0.0, 0.0);
        let mut flight = FalconFlight::new(perch, first_target, ClientTick(0)).expect("finite target");

        // Five ticks: 50 units out, processTick 5.
        advance(&mut flight, 5, perch);
        let before = flight.pose().expect("flight is active").position;
        assert!((before.x - 50.0).abs() < 1e-4, "{before:?}");
        let retrigger_tick = flight.last_tick;

        // Re-trigger towards a target 100 units to the south. The bird keeps
        // its current position and flight frame; only the velocity changes.
        flight.retarget(point(0.0, 0.0, 100.0), retrigger_tick, perch);
        let after = flight.pose().expect("flight is active");
        assert_eq!(after.position, before, "re-trigger must not snap the bird back");
        assert_eq!(flight.process_tick, 5, "the flight frame counter is kept");

        // New velocity is (target - current) / 10 = (-5, 0, 10).
        let next = advance(&mut flight, 1, perch).expect("flight is active");
        assert!((next.position.x - 45.0).abs() < 1e-4, "{:?}", next.position);
        assert!((next.position.z - 10.0).abs() < 1e-4, "{:?}", next.position);
        assert_eq!(flight.process_tick, 6);
        assert_eq!(
            u16::from(next.direction),
            u16::from(direction_towards(perch, next.position)),
            "faces away from the owner"
        );
    }

    #[test]
    fn retrigger_after_the_strike_keeps_the_remaining_flight_frames() {
        let perch = point(0.0, 0.0, 0.0);
        let mut flight = FalconFlight::new(perch, point(100.0, 0.0, 0.0), ClientTick(0)).expect("finite target");

        // processTick 30: the bird is 300 units out and already past the strike
        // frame. There are only six moving frames left (30..=35).
        advance(&mut flight, 30, perch);
        assert!((flight.position.x - 300.0).abs() < 1e-4, "{:?}", flight.position);
        let retrigger_tick = flight.last_tick;

        flight.retarget(point(0.0, 0.0, 100.0), retrigger_tick, perch);
        assert!((flight.position.x - 300.0).abs() < 1e-4, "no snap on the late re-trigger");

        // Velocity (-30, 0, 10); steps 30..=35 move six times, then the
        // processTick 36 tick ends the flight without moving.
        advance(&mut flight, 7, perch);
        let end = flight.pose().expect("return leg starts");
        assert!(!end.flying);
        assert!((end.position.x - 120.0).abs() < 1e-3, "{:?}", end.position);
        assert!((end.position.z - 60.0).abs() < 1e-3, "{:?}", end.position);
    }

    #[test]
    fn return_leg_homes_to_the_moved_owner_and_restores_the_perch() {
        let perch = point(0.0, 0.0, 0.0);
        let mut flight = FalconFlight::new(perch, point(100.0, 0.0, 0.0), ClientTick(0)).expect("finite target");

        // Finish the flight: 360 units out, on the return leg.
        advance(&mut flight, 37, perch);
        assert!(!flight.pose().expect("return leg starts").flying);

        // The owner walked 200 units south. The first return frame only closes
        // 1/25 of the gap; it does not teleport to the new perch.
        let moved_perch = point(0.0, 0.0, 200.0);
        let first_return = advance(&mut flight, 1, moved_perch).expect("return leg");
        assert!(!first_return.flying);
        assert!(
            (first_return.position - moved_perch).magnitude() > 300.0,
            "return must home gradually: {:?}",
            first_return.position
        );
        assert!(
            first_return.position.z > 0.0,
            "moving toward the owner: {:?}",
            first_return.position
        );

        // Keep flying back; eventually the perched pose is restored.
        let mut ticks = 1;
        while ticks < 2_000 {
            if advance(&mut flight, 1, moved_perch).is_none() {
                break;
            }
            ticks += 1;
        }

        assert!(flight.pose().is_none(), "perched state restored");
        assert!(
            (flight.position - moved_perch).magnitude() < 1e-3,
            "returned to the moved owner: {:?}",
            flight.position
        );
        assert!(ticks > 20, "the return leg is a gradual flight, not a snap");
    }

    #[test]
    fn a_target_under_the_falcon_keeps_the_official_zero_velocity_flight() {
        let perch = point(0.0, 26.0, 0.0);
        let mut flight = FalconFlight::new(perch, perch, ClientTick(0)).expect("finite target");

        // The official client still runs the 36-frame motion with a zero
        // velocity; only the return leg actually moves the bird.
        advance(&mut flight, 37, perch);
        assert_eq!(flight.position, perch);
        assert!(!flight.pose().expect("return leg starts").flying);
    }

    #[test]
    fn flight_direction_matches_the_entity_facing_convention() {
        // The helper deliberately mirrors `Entity::rotate_towards`
        // (self-minus-target deltas into `Direction::try_from`), so the falcon
        // flies in the same 0..7 direction code a character uses to face the
        // same cell. These assertions pin that shared convention; they are not
        // a compass statement.
        let direction = |from: Point3<f32>, to: Point3<f32>| u16::from(direction_towards(from, to));

        assert_eq!(
            direction(point(0.0, 0.0, 0.0), point(10.0, 0.0, 0.0)),
            u16::from(Direction::West)
        );
        assert_eq!(
            direction(point(0.0, 0.0, 0.0), point(-10.0, 0.0, 0.0)),
            u16::from(Direction::East)
        );
        assert_eq!(
            direction(point(0.0, 0.0, 0.0), point(0.0, 0.0, 10.0)),
            u16::from(Direction::South)
        );
        assert_eq!(
            direction(point(0.0, 0.0, 0.0), point(0.0, 0.0, -10.0)),
            u16::from(Direction::North)
        );
    }
}
