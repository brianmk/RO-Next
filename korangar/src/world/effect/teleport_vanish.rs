//! One-shot teleport/logout disappearance effect.
//!
//! The classic client plays a warp-like effect plus `ef_teleportation.wav` when
//! another player (or a population-engine shell) teleports away or logs out.
//! The server tells us about this with the entity-disappear packet
//! (`0x0080`) and the reason `Teleported` / `LoggedOut`; the regular
//! out-of-sight and death disappearances must *not* play it.
//!
//! The effect reuses the already loaded warp portal textures (flat swirl,
//! ground glow and sparkles) and fades out over a short lifetime, then removes
//! itself. One short-lived effect per vanishing entity is cheap, and the client
//! additionally rate-limits spawning so a burst of population shells cannot
//! flood the effect renderer.

use cgmath::Point3;

use crate::graphics::ScreenSize;
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, WarpPortal};

/// Lifetime of the disappear effect in seconds.
const VANISH_DURATION: f32 = 0.7;

/// A short-lived warp/teleport effect at a fixed world position.
pub struct TeleportVanish {
    warp_portal: WarpPortal,
    position: Point3<f32>,
    time: f32,
    gets_deleted: bool,
}

impl TeleportVanish {
    pub fn new(warp_portal: WarpPortal, position: Point3<f32>) -> Self {
        Self {
            warp_portal,
            position,
            time: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for TeleportVanish {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.position)
    }

    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.time += delta_time;

        !self.gets_deleted && self.time < VANISH_DURATION
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };

        // Fade out towards the end of the effect.
        let fade = 1.0 - (self.time / VANISH_DURATION).clamp(0.0, 1.0);

        self.warp_portal
            .render_skill_unit(renderer, camera, self.position, screen_size, self.time, fade);
    }
}
