//! Map weather effects sent as `ZC_NOTIFY_EFFECT2` (0x01F3): snow, sakura,
//! autumn leaves, fog and clouds. The 2019 server sends the effect ids on map
//! entry from the RSW map flags; the original client turns them into ambient
//! particles/sheets around the player. This is a compact world-space version:
//! falling billboards for the three particle weathers and soft translucent
//! sheets for fog/clouds, all driven by the animation time.

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
}

/// The currently active weather, if any.
pub struct Weather {
    kind: Option<Kind>,
    started: f32,
    snow: Arc<Texture>,
    sakura: Arc<Texture>,
    leaf: Arc<Texture>,
    fog: Arc<Texture>,
    cloud: Arc<Texture>,
}

impl Weather {
    pub fn new(texture_loader: &TextureLoader) -> Option<Self> {
        let load = |path: &str| texture_loader.get_or_load(path, ImageType::Color).ok();
        Some(Self {
            kind: None,
            started: 0.0,
            snow: load("effect\\snow_a.bmp")?,
            sakura: load("effect\\j_sakura01.bmp")?,
            leaf: load("effect\\leaf01.bmp")?,
            fog: load("effect\\fog1.tga")?,
            cloud: load("effect\\cloud1.tga")?,
        })
    }

    /// Applies a weather effect id. Unknown ids are ignored, so the generic
    /// `ZC_NOTIFY_EFFECT2` stream can be passed through unchanged.
    pub fn set_effect(&mut self, effect: i32, time: f32) -> bool {
        match Kind::from_effect(effect) {
            Some(kind) => {
                self.kind = Some(kind);
                self.started = time;
                true
            }
            None => false,
        }
    }

    pub fn clear(&mut self) {
        self.kind = None;
    }

    pub fn kind(&self) -> Option<Kind> {
        self.kind
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
                    Kind::Clouds2 => (self.cloud.clone(), Color::rgba(0.8, 0.85, 1.0, 0.08), 55.0, 110.0),
                    _ => unreachable!(),
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
    fn weather_effect_ids_map_to_the_five_families() {
        assert_eq!(Kind::from_effect(162), Some(Kind::Snow));
        assert_eq!(Kind::from_effect(163), Some(Kind::Sakura));
        assert_eq!(Kind::from_effect(333), Some(Kind::Leaves));
        assert_eq!(Kind::from_effect(515), Some(Kind::Fog));
        assert_eq!(Kind::from_effect(233), Some(Kind::Clouds));
        assert_eq!(Kind::from_effect(516), Some(Kind::Clouds2));
        assert_eq!(Kind::from_effect(9999), None);
    }
}
