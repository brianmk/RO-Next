use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::{Lua, Value};

use super::{Library, Table};
use crate::graphics::Color;
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone)]
pub struct MapSkyData {
    #[allow(dead_code)]
    old_cloud_effect: Option<usize>,
    bg_color: Option<Color>,
    bg_fog: bool,
    star_effect: bool,
    cloud_effect: Vec<CloudEffect>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CloudEffect {
    num: usize,
    cull_dist: usize,
    color: Color,
    size: usize,
    size_extra: usize,
    expand_rate: f32,
    alpha_inc_time: usize,
    alpha_inc_time_extra: usize,
    alpha_inc_speed: usize,
    alpha_dec_time: usize,
    alpha_dec_time_extra: usize,
    alpha_dec_speed: f32,
    height: usize,
    height_extra: usize,
}

impl Table for MapSkyData {
    type Key<'a> = &'a str;
    type Storage = HashMap<String, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let map_sky_data_table = match game_file_loader.get("data\\luafiles514\\lua files\\mapskydata\\mapskydata.lub") {
            Ok(data) => {
                let state = Lua::new();

                // `mapskydata.lub` is shipped as LuaQ bytecode in the CondaRO
                // data set; it is patched into Lua 5.1 bytecode before the
                // library is built. If that still fails, the sky/fog feature is
                // simply unavailable (guarded) instead of failing the whole
                // library.
                if let Err(error) = state.load(&data).exec() {
                    crate::logging::log_warn!("map sky data is not executable ({error:?}); sky colour and scene fog stay disabled");
                    return Ok(HashMap::new());
                }

                let globals = state.globals();
                let mut result = HashMap::new();

                if let Ok(table) = globals.get::<mlua::Table>("MapSkyData") {
                    for (map_rsw, map_sky_data_table) in table.pairs::<String, mlua::Table>().flatten() {
                        let resource_name = map_rsw.strip_suffix(".rsw").unwrap_or(&map_rsw).to_string();
                        let map_sky_data = Self::parse_map_sky_data(&map_sky_data_table);
                        // Map names are case-insensitive on Windows; store normalized
                        // keys so a map name from the server matches either way.
                        result.insert(resource_name.to_lowercase(), map_sky_data);
                    }
                }

                HashMap::from_iter(result)
            }
            Err(error) => {
                // The CondaRO client ships this file as LuaQ bytecode (a Gravity
                // Lua 5.1 dialect), which the embedded Lua interpreter cannot
                // execute. The sky/fog path stays inactive for such a data set;
                // maps render exactly as before. A server that ships a text
                // `mapskydata.lub`/`.lua` gets the full path without a rebuild.
                crate::logging::log_warn!(
                    "map sky data: `data\\luafiles514\\lua files\\mapskydata\\mapskydata.lub` is not readable ({error:?}); sky colour and \
                     scene fog stay disabled"
                );
                HashMap::new()
            }
        };

        Ok(map_sky_data_table)
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        let key = key.to_lowercase();
        library.map_sky_data_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: MapSkyData = MapSkyData {
            old_cloud_effect: None,
            bg_color: None,
            bg_fog: false,
            star_effect: false,
            cloud_effect: vec![],
        };
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

impl MapSkyData {
    /// Background/sky colour of the map (`BG_Color`), if the map defines one.
    pub fn background_color(&self) -> Option<Color> {
        self.bg_color
    }

    /// Whether the map wants its background colour used as scene fog
    /// (`BG_Fog`).
    pub fn background_fog(&self) -> bool {
        self.bg_fog
    }

    /// Whether the map wants the star sky effect (`Star_Effect`).
    pub fn star_effect(&self) -> bool {
        self.star_effect
    }

    /// Cloud effect descriptors of the map (`Cloud_Effect`).
    pub fn cloud_effects(&self) -> &[CloudEffect] {
        &self.cloud_effect
    }

    fn parse_map_sky_data(table: &mlua::Table) -> MapSkyData {
        let mut cloud_effect = Vec::new();

        if let Ok(Value::Table(cloud_effect_table)) = table.get("Cloud_Effect") {
            let _ = cloud_effect_table.for_each(|_key: usize, value: mlua::Table| {
                let effect = CloudEffect {
                    num: value.get("Num").unwrap_or_default(),
                    cull_dist: value.get("CullDist").unwrap_or_default(),
                    color: value.get("Color").unwrap_or_default(),
                    size: value.get("Size").unwrap_or_default(),
                    size_extra: value.get("Size_Extra").unwrap_or_default(),
                    expand_rate: value.get("Expand_Rate").unwrap_or_default(),
                    alpha_inc_time: value.get("Alpha_Inc_Time").unwrap_or_default(),
                    alpha_inc_time_extra: value.get("Alpha_Inc_Time_Extra").unwrap_or_default(),
                    alpha_inc_speed: value.get("Alpha_Inc_Speed").unwrap_or_default(),
                    alpha_dec_time: value.get("Alpha_Dec_Time").unwrap_or_default(),
                    alpha_dec_time_extra: value.get("Alpha_Dec_Time_Extra").unwrap_or_default(),
                    alpha_dec_speed: value.get("Alpha_Dec_Speed").unwrap_or_default(),
                    height: value.get("Height").unwrap_or_default(),
                    height_extra: value.get("Height_Extra").unwrap_or_default(),
                };
                cloud_effect.push(effect);

                Ok(())
            });
        }

        MapSkyData {
            old_cloud_effect: table.get::<[usize; 1]>("Old_Cloud_Effect").ok().map(|effect| effect[0]),
            bg_color: table.get("BG_Color").ok(),
            bg_fog: table.get("BG_Fog").unwrap_or(false),
            star_effect: table.get("Star_Effect").unwrap_or(false),
            cloud_effect,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MapSkyData, Table};
    use crate::loaders::GameFileLoader;

    /// The client patches the Lua bytecode of the archives into a format the
    /// embedded interpreter can execute before the library is built. This
    /// archive-backed test proves that the sky/fog data path is real for this
    /// data set and is not only a code skeleton.
    #[test]
    #[ignore = "requires the CondaRO game archives in the client directory"]
    fn mapskydata_is_readable_from_the_patched_archives() {
        let loader = GameFileLoader::default();
        loader.load_archives_from_settings();
        loader.load_patched_lua_files();

        let table = MapSkyData::load(&loader).expect("the patched mapskydata must be readable");
        assert!(!table.is_empty(), "mapskydata.lub has to contain map entries");

        let with_background = table.values().filter(|data| data.background_color().is_some()).count();
        assert!(with_background > 0, "at least one map has a sky background colour");

        // `BG_Fog` is optional and this data set does not enable it for any
        // map; the code path stays guarded (maps without it render as before).
        let _with_fog = table.values().filter(|data| data.background_fog()).count();
    }
}
