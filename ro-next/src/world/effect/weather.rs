//! Map weather effects sent as `ZC_NOTIFY_EFFECT2` (0x01F3): snow, sakura,
//! autumn leaves, fog and clouds. The 2019 server sends the effect ids on map
//! entry from the RSW map flags; the original client turns them into ambient
//! particles/sheets around the player. RO-Next now renders all of them
//! procedurally in a full-screen post-processing shader, so no weather
//! textures or CPU particle quads are required.

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

    /// Procedural weather shader id (0 is clear).
    pub fn shader_id(self) -> u32 {
        match self {
            Self::Snow => 1,
            Self::Sakura => 2,
            Self::Leaves => 3,
            Self::Clouds => 4,
            Self::Fog => 5,
            Self::Clouds2 => 4,
        }
    }
}

/// The currently active weather, if any.
pub struct Weather {
    kind: Option<Kind>,
    started: f32,
    /// Development override (`RO_NEXT_WEATHER`); cleared weather snapshots do
    /// not remove it, so a shader can be inspected without logging in.
    forced_kind: Option<Kind>,
}

impl Weather {
    pub fn new() -> Self {
        // Hidden development override: `RO_NEXT_WEATHER=sakura` lets the
        // procedural weather shader be inspected without logging in.
        let forced_kind = std::env::var("RO_NEXT_WEATHER").ok().and_then(|name| Kind::from_name(&name));
        Self {
            kind: forced_kind,
            started: 0.0,
            forced_kind,
        }
    }

    pub fn kind(&self) -> Option<Kind> {
        self.kind.or(self.forced_kind)
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
        self.kind = self.forced_kind;
    }

    /// `[kind, time, intensity, seed]` for the procedural weather shader.
    pub fn shader_state(&self, time: f32, zoom: f32) -> [f32; 4] {
        let Some(kind) = self.kind.or(self.forced_kind) else {
            return [0.0; 4];
        };
        [kind.shader_id() as f32, (time - self.started).max(0.0), 1.0, zoom]
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
