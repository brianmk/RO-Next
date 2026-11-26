//! Hunter falcon (`CGameActor::statusEffects[0]`) flight state.
//!
//! The falcon is not an effect body: the official client creates it as a
//! message-effect actor when the entity gets the `OPTION_FALCON` bit
//! (`GameObject.cpp` `AM_MAKE_BIRD`) and keeps it in `statusEffects[0]`.
//! Falcon-attack skills move that actor:
//!
//! * `CGameActor::ProcessSkillMessage` sends `AM_REQ_ATTACK` to
//!   `GetEffectCounter(skillId)` for `IsFalconAttackSkill()`
//!   (`RenderObject.cpp:5388`, `GameActor.cpp:894-925`); `AM_REQ_ATTACK` starts
//!   a straight flight towards the target (`MsgEffect.cpp:1611-1617`).
//! * While `activeMotion` is set the bird advances by the stored velocity,
//!   faces the direction of travel and plays the flight action; after the last
//!   flight frame it returns to its owner (`MsgEffect.cpp` `MET_BIRD`,
//!   `FollowOwner`).
//!
//! The official falcon sprite (`data\sprite\이팩트\매.act`, 16 actions) stores
//! eight perched animations (actions 0-7, eight flap frames each) and eight
//! one-frame flight animations (actions 8-15). `AnimationState` indexes an
//! animation as `action * 8 + direction`, so the perched action is `0` and the
//! flight action is `1`.

use cgmath::Point3;
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

/// Flight frames before the bird reaches the target.
///
/// `AM_REQ_ATTACK` stores `(target - position) / 10` and the flight segment
/// covers 25 `processTick`s (`MsgEffect.cpp:1611-1617`, `:489-525`). Applying
/// that velocity for all 25 frames would move the bird 2.5x past the target;
/// the port keeps the official 25-frame duration but normalizes the velocity
/// so the bird arrives at the attacked cell (the visible behaviour of the
/// reference client) instead of overshooting it.
const FLIGHT_TICKS: u32 = 25;
/// `processTick` range 25..=35: the bird stays at the target and pulls up
/// before `FollowOwner` takes over (`MsgEffect.cpp:509-518`).
const STRIKE_TICKS: u32 = 10;
/// Frames `FollowOwner` needs to bring the bird back to its perch.
const RETURN_TICKS: u32 = 25;

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
#[derive(Clone, Debug)]
pub(crate) struct FalconFlight {
    start_tick: ClientTick,
    /// Falcon position when `AM_REQ_ATTACK` was received (its perch).
    perch_start: Point3<f32>,
    /// World position of the attacked target.
    target: Point3<f32>,
    /// Direction of travel for the outbound dive.
    direction: Direction,
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
    /// Returns `None` when the target is essentially under the falcon (the
    /// official client would not move the actor either).
    pub fn new(perch: Point3<f32>, target: Point3<f32>, client_tick: ClientTick) -> Option<Self> {
        let horizontal_distance = (target.x - perch.x).hypot(target.z - perch.z);
        if !horizontal_distance.is_finite() || horizontal_distance < 0.5 {
            return None;
        }

        Some(Self {
            start_tick: client_tick,
            perch_start: perch,
            target,
            direction: direction_towards(perch, target),
        })
    }

    /// Falcon pose at `now`, or `None` once the bird is back on its perch.
    ///
    /// `perch_now` is the owner-relative perch position at `now`, so the return
    /// flight follows a moving owner like `FollowOwner` does.
    pub fn sample(&self, now: ClientTick, perch_now: Point3<f32>) -> Option<FalconPose> {
        let ticks = now.0.saturating_sub(self.start_tick.0) / LOGIC_TICK_MS;

        if ticks <= FLIGHT_TICKS {
            let progress = ticks as f32 / FLIGHT_TICKS as f32;
            Some(FalconPose {
                position: lerp(self.perch_start, self.target, progress),
                direction: self.direction,
                flying: true,
            })
        } else if ticks <= FLIGHT_TICKS + STRIKE_TICKS {
            Some(FalconPose {
                position: self.target,
                direction: self.direction,
                flying: true,
            })
        } else if ticks <= FLIGHT_TICKS + STRIKE_TICKS + RETURN_TICKS {
            let progress = (ticks - FLIGHT_TICKS - STRIKE_TICKS) as f32 / RETURN_TICKS as f32;
            Some(FalconPose {
                position: lerp(self.target, perch_now, progress),
                // `FollowOwner` faces the owner while returning.
                direction: direction_towards(self.target, perch_now),
                flying: false,
            })
        } else {
            None
        }
    }
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

fn lerp(from: Point3<f32>, to: Point3<f32>, progress: f32) -> Point3<f32> {
    let progress = progress.clamp(0.0, 1.0);
    from + (to - from) * progress
}

#[cfg(test)]
mod tests {
    use cgmath::InnerSpace;

    use super::*;

    fn point(x: f32, y: f32, z: f32) -> Point3<f32> {
        Point3::new(x, y, z)
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
    fn flight_dives_at_the_target_and_returns_to_the_perch() {
        let perch = point(0.0, 26.0, 0.0);
        let target = point(50.0, 0.0, 0.0);
        let flight = FalconFlight::new(perch, target, ClientTick(1_000)).expect("horizontal target");

        // Start of the flight: still at the perch, already in the flight pose.
        let start = flight.sample(ClientTick(1_000), perch).expect("flight is active");
        assert_eq!(start.position, perch);
        assert!(start.flying);

        // Half-way through the dive the bird is closer to the target.
        let half = flight
            .sample(ClientTick(1_000 + 12 * LOGIC_TICK_MS), perch)
            .expect("flight is active");
        assert!(half.position.x > perch.x && half.position.x < target.x);
        assert!(half.flying);

        // The official 25-frame flight segment ends exactly on the target.
        let arrival = flight
            .sample(ClientTick(1_000 + FLIGHT_TICKS * LOGIC_TICK_MS), perch)
            .expect("flight is active");
        assert!((arrival.position - target).magnitude() < 1e-3, "{:?}", arrival.position);

        // The strike hold keeps the bird on the target.
        let strike = flight
            .sample(ClientTick(1_000 + (FLIGHT_TICKS + 5) * LOGIC_TICK_MS), perch)
            .expect("flight is active");
        assert_eq!(strike.position, target);

        // On the way back the bird plays the perched action and closes the
        // distance to the (possibly moved) perch.
        let returning = flight
            .sample(ClientTick(1_000 + (FLIGHT_TICKS + STRIKE_TICKS + 12) * LOGIC_TICK_MS), perch)
            .expect("flight is active");
        assert!(!returning.flying);
        assert!(returning.position.x < target.x);
        assert!((returning.position - perch).magnitude() < (target - perch).magnitude());

        // Once the return is over the entity falls back to the perched render.
        let over = ClientTick(1_000 + (FLIGHT_TICKS + STRIKE_TICKS + RETURN_TICKS + 1) * LOGIC_TICK_MS);
        assert!(flight.sample(over, perch).is_none());
    }

    #[test]
    fn return_flies_to_the_current_owner_position() {
        let perch = point(0.0, 26.0, 0.0);
        let target = point(30.0, 0.0, 0.0);
        let flight = FalconFlight::new(perch, target, ClientTick(0)).expect("horizontal target");

        let moved_perch = point(0.0, 26.0, 40.0);
        let end = flight
            .sample(
                ClientTick((FLIGHT_TICKS + STRIKE_TICKS + RETURN_TICKS) * LOGIC_TICK_MS),
                moved_perch,
            )
            .expect("flight is active");
        assert!((end.position - moved_perch).magnitude() < 1e-3);
    }

    #[test]
    fn a_target_under_the_falcon_does_not_start_a_flight() {
        let perch = point(0.0, 26.0, 0.0);
        assert!(FalconFlight::new(perch, point(0.1, 26.0, 0.0), ClientTick(0)).is_none());
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
