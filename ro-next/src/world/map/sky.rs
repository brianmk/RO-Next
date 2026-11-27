//! Map sky and scene fog derived from `mapskydata.lub`.
//!
//! The classic client reads `data\luafiles514\lua files\mapskydata\
//! mapskydata.lub` (`MapSkyData` in [`crate::world::library`]). The table
//! carries a background colour, a "use the background colour as fog" flag, a
//! star effect flag and the cloud descriptors. Maps that are not part of the
//! table (and maps without the lua file) keep the previous appearance: black
//! background and no fog.
//!
//! This client implements the two pieces that the renderer can express without
//! a skybox mesh or a cloud particle system: the sky colour is used as the
//! clear colour of the forward pass (the "skybox colour") and `BG_Fog` fades
//! distant geometry into that colour.

use cgmath::Vector3;

use crate::graphics::Color;
use crate::world::MapSkyData;

/// Scene fog of a map: the colour to fade into and the distance window.
#[derive(Debug, Clone, Copy)]
pub struct SceneFog {
    pub color: Color,
    /// Distance (world units) where the fog starts.
    pub near: f32,
    /// Distance (world units) where geometry is fully fogged.
    pub far: f32,
}

impl SceneFog {
    /// Builds the fog window of a map from its `BG_Fog` flag.
    ///
    /// The RSW/GND data carries no fog distances, so the window is derived from
    /// the map size: fog starts after a quarter and is full slightly before the
    /// far corner of the map. The colour is the map background colour, or the
    /// ambient light colour for maps that only carry the fog flag.
    fn from_map_sky(data: &MapSkyData, map_size: Vector3<f32>, ambient_color: Color) -> Option<Self> {
        if !data.background_fog() {
            return None;
        }

        let diagonal = (map_size.x * map_size.x + map_size.z * map_size.z).sqrt().max(1.0);
        let near = (diagonal * 0.25).max(50.0);
        let far = (diagonal * 0.9).max(near + 1.0);

        Some(Self {
            color: data.background_color().unwrap_or(ambient_color),
            near,
            far,
        })
    }
}

/// Sky and fog presentation of one map.
#[derive(Debug, Clone)]
pub struct MapSky {
    background_color: Option<Color>,
    fog: Option<SceneFog>,
}

impl MapSky {
    /// Derives the runtime sky/fog presentation from the `mapskydata.lub` entry
    /// of the map. The default entry (no data) produces no sky and no fog.
    pub fn from_map_sky_data(data: &MapSkyData, map_size: Vector3<f32>, ambient_color: Color) -> Self {
        Self {
            background_color: data.background_color(),
            fog: SceneFog::from_map_sky(data, map_size, ambient_color),
        }
    }

    /// Sky/background colour of the map. `None` keeps the black background.
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Scene fog, `None` for maps without `BG_Fog`.
    pub fn fog(&self) -> Option<SceneFog> {
        self.fog
    }
}
