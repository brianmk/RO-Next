//! Map weather effects sent as `ZC_NOTIFY_EFFECT2` (0x01F3): snow, sakura,
//! autumn leaves, fog and clouds. The 2019 server sends the effect ids on map
//! entry from the RSW map flags; the original client turns them into ambient
//! particles/sheets around the player. This is a compact world-space version:
//! falling billboards for the three particle weathers and soft translucent
//! sheets for fog/clouds, all driven by the village clock / animation time.
//!
//! The textures are optional: a data set without (for example) the Korean
//! sakura texture still gets snow/fog/clouds instead of losing every weather
//! layer. This was the failure mode of the previous version, which required
//! all five textures and silently disabled the whole subsystem.

use std::f32::consts::TAU;
use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use super::render_world_quad_blended;
use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// `EF_SNOW` / `EF_SAKURA` / `EF_MAPLE` / `EF_CLOUD3` / `EF_CLOUD4` /
/// `EF_CLOUD5`.
const SNOW: i32 = 162;
const SAKURA: i32 = 163;
const LEAVES: i32 = 333;
const CLOUDS: i32 = 233;
const FOG: i32 = 515;
const CLOUDS2: i32 = 516;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Snow,
    Sakura,
    Leaves,
    Clouds,
    Fog,
    Clouds2,
}

impl Kind {
    pub fn from_effect(effect: i32) -> Option<Self> {
        match effect {
            SNOW => Some(Self::Snow),
            SAKURA => Some(Self::Sakura),
            LEAVES => Some(Self::Leaves),
            CLOUDS => Some(Self::Clouds),
            FOG => Some(Self::Fog),
            CLOUDS2 => Some(Self::Clouds2),
            _ => None,
        }
    }

    /// User-facing name used by the local `/weather` command.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "snow" => Some(Self::Snow),
            "sakura" | "cherry" => Some(Self::Sakura),
            "leaves" | "leaf" | "maple" => Some(Self::Leaves),
            "clouds" | "cloud" => Some(Self::Clouds),
            "fog" | "mist" => Some(Self::Fog),
            "clouds2" | "cloud2" => Some(Self::Clouds2),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Snow => "snow",
            Self::Sakura => "sakura",
            Self::Leaves => "leaves",
            Self::Clouds => "clouds",
            Self::Fog => "fog",
            Self::Clouds2 => "clouds2",
        }
    }
}

/// The currently active weather, if any.
pub struct Weather {
    kind: Option<Kind>,
    started: f32,
    snow: Option<Arc<Texture>>,
    sakura: Option<Arc<Texture>>,
    leaf: Option<Arc<Texture>>,
    fog: Option<Arc<Texture>>,
    cloud: Option<Arc<Texture>>,
    cloud2: Option<Arc<Texture>>,
}

impl Weather {
    pub fn new(texture_loader: &TextureLoader) -> Self {
        let load = |path: &str| texture_loader.get_or_load(path, ImageType::Color).ok();
        Self {
            kind: None,
            started: 0.0,
            // `snow_a.bmp` is the official snow particle texture.
            snow: load("effect\\snow_a.bmp"),
            // These two are shipped under `texture\나무잡초꽃\`, not `texture\effect\`.
            sakura: load("나무잡초꽃\\j_sakura01.bmp"),
            leaf: load("나무잡초꽃\\leaf01.bmp"),
            fog: load("effect\\fog1.tga"),
            cloud: load("effect\\cloud1.tga"),
            cloud2: load("effect\\cloud2.tga"),
        }
    }

    pub fn kind(&self) -> Option<Kind> {
        self.kind
    }

    /// Replaces the active kind. Re-selecting the same kind keeps the
    /// animation phase, so a server snapshot that repeats every frame does not
    /// restart the particles.
    pub fn set_kind(&mut self, kind: Option<Kind>, time: f32) {
        if self.kind != kind {
            self.kind = kind;
            self.started = time;
        }
    }

    pub fn clear(&mut self) {
        self.kind = None;
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, screen_size: ScreenSize, time: f32) {
        let Some(kind) = self.kind else {
            return;
        };
        let elapsed = (time - self.started).max(0.0);
        let focus = camera.focus_point();
        let right = camera.look_up_vector().cross(camera.view_direction()).normalize();
        let up = camera.view_direction().cross(right).normalize();

        match kind {
            Kind::Snow | Kind::Sakura | Kind::Leaves => {
                let (texture, color, count, fall) = match kind {
                    Kind::Snow => (self.snow.clone(), Color::rgba(0.95, 0.97, 1.0, 0.85), 140, 3.0),
                    Kind::Sakura => (self.sakura.clone(), Color::rgba(1.0, 0.75, 0.85, 0.9), 90, 2.2),
                    Kind::Leaves => (self.leaf.clone(), Color::rgba(1.0, 0.75, 0.35, 0.9), 80, 1.8),
                    _ => unreachable!(),
                };

                let Some(texture) = texture else {
                    return;
                };

                for index in 0..count {
                    let seed = index as f32;
                    let angle = seed * 2.399_963 + elapsed * 0.15;
                    let radius = 20.0 + (seed * 7.13).fract() * 60.0;
                    let phase = (seed * 0.37 + elapsed * fall / 60.0).fract();
                    let height = 70.0 * (1.0 - phase);
                    let position =
                        focus + Vector3::new(angle.cos() * radius, height, angle.sin() * radius) + Vector3::new(elapsed * 0.6, 0.0, 0.0);
                    let size = 1.2 + (seed * 3.7).fract() * 1.8;
                    let corners = [
                        position - right * size,
                        position + right * size,
                        position - up * size,
                        position + up * size,
                    ];
                    render_world_quad_blended(
                        renderer,
                        camera,
                        position,
                        screen_size,
                        texture.clone(),
                        color,
                        corners,
                        [
                            Vector2::new(0.0, 0.0),
                            Vector2::new(1.0, 0.0),
                            Vector2::new(0.0, 1.0),
                            Vector2::new(1.0, 1.0),
                        ],
                        BlendFactor::One,
                        BlendFactor::One,
                    );
                }
            }
            Kind::Fog | Kind::Clouds | Kind::Clouds2 => {
                let (texture, color, height, size) = match kind {
                    Kind::Fog => (self.fog.clone(), Color::rgba(0.75, 0.78, 0.8, 0.12), 35.0, 90.0),
                    Kind::Clouds => (self.cloud.clone(), Color::rgba(0.9, 0.92, 1.0, 0.10), 70.0, 80.0),
                    Kind::Clouds2 => (
                        self.cloud2.clone().or_else(|| self.cloud.clone()),
                        Color::rgba(0.8, 0.85, 1.0, 0.08),
                        55.0,
                        110.0,
                    ),
                    _ => unreachable!(),
                };

                let Some(texture) = texture else {
                    return;
                };

                for index in 0..10 {
                    let seed = index as f32;
                    let angle = seed * TAU / 10.0 + elapsed * 0.02;
                    let radius = 50.0 + (seed * 11.7).fract() * 50.0;
                    let position = focus + Vector3::new(angle.cos() * radius, height, angle.sin() * radius);
                    let drift = (elapsed * 0.8) % 40.0;
                    let position = Point3::new(position.x + drift, position.y, position.z);
                    let half = Vector3::new(size, 0.0, size * 0.6);
                    let corners = [position - half, position + half, position - half * 0.8, position + half * 0.8];
                    render_world_quad_blended(
                        renderer,
                        camera,
                        position,
                        screen_size,
                        texture.clone(),
                        color,
                        corners,
                        [
                            Vector2::new(0.0, 0.0),
                            Vector2::new(1.0, 0.0),
                            Vector2::new(0.0, 1.0),
                            Vector2::new(1.0, 1.0),
                        ],
                        BlendFactor::One,
                        BlendFactor::One,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kind;

    #[test]
    fn weather_effect_ids_map_to_the_families() {
        assert_eq!(Kind::from_effect(162), Some(Kind::Snow));
        assert_eq!(Kind::from_effect(163), Some(Kind::Sakura));
        assert_eq!(Kind::from_effect(333), Some(Kind::Leaves));
        assert_eq!(Kind::from_effect(515), Some(Kind::Fog));
        assert_eq!(Kind::from_effect(233), Some(Kind::Clouds));
        assert_eq!(Kind::from_effect(516), Some(Kind::Clouds2));
        assert_eq!(Kind::from_effect(9999), None);
    }

    #[test]
    fn names_are_stable_for_the_local_command() {
        assert_eq!(Kind::from_name("SNOW"), Some(Kind::Snow));
        assert_eq!(Kind::from_name("cherry"), Some(Kind::Sakura));
        assert_eq!(Kind::from_name("maple"), Some(Kind::Leaves));
        assert_eq!(Kind::from_name("unknown"), None);
    }
}
