//! Client-side day/night and random weather.
//!
//! The server already owns the authoritative day/night flag on rAthena
//! (`EFST_SKE`), but the CondaRO server used for local play does not have a
//! configured cycle. This module is a deliberately small, self-contained
//! environment for the current milestone:
//!
//! * a 15-minute day followed by a 15-minute night, with a one-minute dusk and
//!   dawn cross-fade;
//! * random weather in Prontera only (snow, sakura, leaves, clouds, fog, or
//!   clear), changing every 1-3 minutes;
//! * the existing `ZC_NOTIFY_EFFECT2` weather still works on every map and
//!   takes priority over the random Prontera weather.
//!
//! It composes the map's RSW lighting instead of replacing it, so the authored
//! mood of a map is preserved during the day and the night is a tinted,
//! darkened version of the same map.

use cgmath::{InnerSpace, Vector2, Vector3};
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize};
use crate::loaders::TextureLoader;
use crate::renderer::EffectRenderer;
use crate::world::effect::render_world_quad_blended;
use crate::world::{Camera, Map, SceneFog, Weather, WeatherKind};

/// Length of the day half of the cycle.
pub const DAY_SECONDS: f32 = 15.0 * 60.0;
/// Length of the night half of the cycle.
pub const NIGHT_SECONDS: f32 = 15.0 * 60.0;
/// Full day/night cycle.
pub const CYCLE_SECONDS: f32 = DAY_SECONDS + NIGHT_SECONDS;

/// A one-minute dusk/dawn transition keeps the 15/15 split but avoids a hard
/// switch at the boundary.
const TWILIGHT_SECONDS: f32 = 60.0;
const DUSK_START: f32 = DAY_SECONDS - TWILIGHT_SECONDS;
const DAWN_START: f32 = CYCLE_SECONDS - TWILIGHT_SECONDS;

const RANDOM_WEATHER_MIN_SECONDS: f32 = 60.0;
const RANDOM_WEATHER_MAX_SECONDS: f32 = 180.0;
const FIRST_WEATHER_DELAY_SECONDS: f32 = 15.0;

const NIGHT_AMBIENT_SCALE: f32 = 0.28;
const NIGHT_SUN_SCALE: f32 = 0.12;
const NIGHT_TINT: Color = Color::rgb(0.62, 0.72, 1.0);
const NIGHT_BACKGROUND: Color = Color::rgb_u8(4, 8, 20);
const DAY_PRONTERA_BACKGROUND: Color = Color::rgb_u8(78, 116, 170);

/// Per-frame values the renderer needs. Everything is copied so the client can
/// borrow the environment mutably for weather rendering later in the frame.
#[derive(Clone, Copy, Debug)]
pub struct EnvironmentSample {
    pub ambient_light_color: Color,
    pub directional_light_direction: Vector3<f32>,
    pub directional_light_color: Color,
    pub background_color: Color,
    pub scene_fog: Option<SceneFog>,
}

/// Day/night clock and the current local weather.
pub struct Environment {
    elapsed: f32,
    weather: Weather,
    sun_texture: Option<std::sync::Arc<crate::graphics::Texture>>,
    server_weather: Option<WeatherKind>,
    random_weather: Option<WeatherKind>,
    random_timer: f32,
    rng: u32,
    active_map_is_prontera: bool,
    server_weather_seen: bool,
    server_night: Option<bool>,
    server_night_factor: f32,
}

impl Environment {
    pub fn new(texture_loader: &TextureLoader) -> Self {
        Self {
            elapsed: 0.0,
            weather: Weather::new(texture_loader),
            sun_texture: texture_loader
                .get_or_load("effect\\sun_y_01.bmp", crate::loaders::ImageType::Color)
                .ok(),
            server_weather: None,
            random_weather: None,
            random_timer: FIRST_WEATHER_DELAY_SECONDS,
            // Deterministic seed: the same weather sequence every launch is
            // easier to debug; the sequence is still random-looking.
            rng: 0x5EED_2026,
            active_map_is_prontera: false,
            server_weather_seen: false,
            server_night: None,
            server_night_factor: 0.0,
        }
    }

    /// Advances the clock and the Prontera random-weather timer.
    ///
    /// `map_name` is the server map name (with or without `.rsw`). `outdoor`
    /// comes from the map-type packet; indoor maps keep their authored light
    /// and never receive local weather.
    pub fn update(&mut self, delta_seconds: f32, map_name: Option<&str>, outdoor: bool) {
        if delta_seconds.is_finite() && delta_seconds > 0.0 {
            self.elapsed = (self.elapsed + delta_seconds) % CYCLE_SECONDS;
        }

        self.active_map_is_prontera = map_name.is_some_and(is_prontera);

        if self.active_map_is_prontera && outdoor && !self.server_weather_seen {
            self.random_timer -= delta_seconds;
            if self.random_timer <= 0.0 {
                self.random_weather = self.next_random_weather();
                self.random_timer = self.next_random_delay();
            }
        } else {
            self.random_weather = None;
        }

        // The server's EFST_SKE is authoritative when it is present. When it
        // is absent (a server without an enabled day/night cycle), the local
        // 15/15 clock is the visual fallback.
        if let Some(server_night) = self.server_night {
            let target = if server_night { 1.0 } else { 0.0 };
            let step = (delta_seconds / 2.0).clamp(0.0, 1.0);
            self.server_night_factor += (target - self.server_night_factor) * step;
        }

        let current = self.server_weather.or(self.random_weather);
        self.weather.set_kind(current, self.elapsed);
    }

    /// rAthena sends `EFST_SKE` (status 160) as the night signal. Only the
    /// local player's status drives the view, like the official client.
    pub fn set_server_night(&mut self, night: bool) {
        self.server_night = Some(night);
    }

    /// Server `ZC_NOTIFY_EFFECT2` weather. It wins over the local Prontera
    /// random weather and still works on every outdoor map.
    pub fn set_server_effect(&mut self, effect_id: i32) -> bool {
        // 161 is `EF_RAIN`, a no-op in the official client. The local Prontera
        // weather script uses it as a light-weight "clear" control so that a
        // server can end a random weather period without a stop packet.
        self.server_weather_seen = true;

        if effect_id == 161 {
            self.server_weather = None;
            return true;
        }

        match WeatherKind::from_effect(effect_id) {
            Some(kind) => {
                self.server_weather = Some(kind);
                true
            }
            None => false,
        }
    }

    pub fn clear_server_weather(&mut self) {
        self.server_weather = None;
    }

    /// Clears local and server weather, used on map changes and screen changes.
    pub fn clear_weather(&mut self) {
        self.server_weather = None;
        self.random_weather = None;
        self.random_timer = FIRST_WEATHER_DELAY_SECONDS;
        self.server_weather_seen = false;
        self.server_night = None;
        self.weather.clear();
    }

    /// Local `/weather <name>` support. `clear` removes all weather.
    pub fn force_weather(&mut self, name: &str) -> bool {
        if name.eq_ignore_ascii_case("clear") || name.eq_ignore_ascii_case("none") {
            self.server_weather = None;
            self.random_weather = None;
            self.server_weather_seen = true;
            self.random_timer = self.next_random_delay();
            self.weather.clear();
            return true;
        }

        match WeatherKind::from_name(name) {
            Some(kind) => {
                self.server_weather = Some(kind);
                self.server_weather_seen = true;
                true
            }
            None => false,
        }
    }

    pub fn current_weather(&self) -> Option<WeatherKind> {
        self.weather.kind()
    }

    /// Current in-game hour, derived from the 15/15 cycle (0..24).
    pub fn hour(&self) -> f32 {
        (self.elapsed / CYCLE_SECONDS) * 24.0
    }

    /// 0 during the day, 1 at night. Uses the server's SKE state when
    /// available, otherwise the local 15/15 clock.
    pub fn night_factor(&self) -> f32 {
        match self.server_night {
            Some(_) => self.server_night_factor,
            None => night_factor(self.elapsed),
        }
    }

    pub fn sample(&self, map: &Map) -> EnvironmentSample {
        let night = self.night_factor();

        let base_ambient = map.ambient_light_color();
        let (_, base_sun) = map.directional_light();

        let ambient_light_color = modulate(base_ambient, NIGHT_AMBIENT_SCALE, night);
        let directional_light_color = modulate(base_sun, NIGHT_SUN_SCALE, night);

        EnvironmentSample {
            ambient_light_color,
            directional_light_direction: self.celestial_direction(),
            directional_light_color,
            background_color: self.background_color(map, night),
            scene_fog: map.scene_fog().map(|fog| SceneFog {
                color: lerp_color(fog.color, NIGHT_BACKGROUND, night),
                ..fog
            }),
        }
    }

    /// Renders the active weather and the visible sun/moon. The environment
    /// clock drives both, so random weather and the sun stay in sync.
    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, screen_size: ScreenSize) {
        self.render_sun(renderer, camera, screen_size);
        self.weather.render(renderer, camera, screen_size, self.elapsed);
    }

    fn render_sun(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, screen_size: ScreenSize) {
        let Some(texture) = self.sun_texture.as_ref() else {
            return;
        };
        let night = self.night_factor();
        if night > 0.95 {
            return;
        }

        let direction = self.celestial_direction();
        if direction.y <= 0.0 {
            return;
        }

        let focus = camera.focus_point();
        let position = focus + direction * 700.0;
        let right = camera.look_up_vector().cross(camera.view_direction()).normalize();
        let up = camera.view_direction().cross(right).normalize();

        let half_size = 24.0 * (1.0 - night * 0.45);
        let corners = [
            position - right * half_size + up * half_size,
            position + right * half_size + up * half_size,
            position + right * half_size - up * half_size,
            position - right * half_size - up * half_size,
        ];
        let texture_coordinates = [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
        ];

        render_world_quad_blended(
            renderer,
            camera,
            position,
            screen_size,
            texture.clone(),
            Color::rgba(1.0, 0.94, 0.78, (1.0 - night) * 0.92),
            corners,
            texture_coordinates,
            BlendFactor::One,
            BlendFactor::One,
        );

        // A larger, fainter halo.
        let halo = 60.0 * (1.0 - night * 0.45);
        let halo_corners = [
            position - right * halo + up * halo,
            position + right * halo + up * halo,
            position + right * halo - up * halo,
            position - right * halo - up * halo,
        ];
        render_world_quad_blended(
            renderer,
            camera,
            position,
            screen_size,
            texture.clone(),
            Color::rgba(1.0, 0.78, 0.45, (1.0 - night) * 0.12),
            halo_corners,
            texture_coordinates,
            BlendFactor::One,
            BlendFactor::One,
        );
    }

    /// Direction toward the sun during the day and the moon at night.
    pub fn celestial_direction(&self) -> Vector3<f32> {
        if self.night_factor() >= 0.5 {
            let phase = ((self.elapsed - DAY_SECONDS) / NIGHT_SECONDS).rem_euclid(1.0);
            Vector3::new(
                -(std::f32::consts::PI * phase).cos(),
                (std::f32::consts::PI * phase).sin().max(0.15),
                0.0,
            )
            .normalize()
        } else {
            let phase = (self.elapsed / DAY_SECONDS).clamp(0.0, 1.0);
            Vector3::new(
                (std::f32::consts::PI * phase).cos(),
                (std::f32::consts::PI * phase).sin().max(0.15),
                0.0,
            )
            .normalize()
        }
    }

    fn background_color(&self, map: &Map, night: f32) -> Color {
        let base = map.sky_background_color().unwrap_or(Color::BLACK);

        // Prontera has no useful sky colour in this data set. Give it a simple
        // sky so the 15/15 cycle is visible without touching other maps.
        let day = if self.active_map_is_prontera && is_black(base) {
            DAY_PRONTERA_BACKGROUND
        } else {
            base
        };

        lerp_color(day, NIGHT_BACKGROUND, night)
    }

    fn next_random_weather(&mut self) -> Option<WeatherKind> {
        // 40% clear, 20% clouds, 10% each fog/snow/sakura/leaves.
        match self.next_u32() % 10 {
            0..=3 => None,
            4..=5 => Some(WeatherKind::Clouds),
            6 => Some(WeatherKind::Fog),
            7 => Some(WeatherKind::Snow),
            8 => Some(WeatherKind::Sakura),
            _ => Some(WeatherKind::Leaves),
        }
    }

    fn next_random_delay(&mut self) -> f32 {
        let span = RANDOM_WEATHER_MAX_SECONDS - RANDOM_WEATHER_MIN_SECONDS;
        RANDOM_WEATHER_MIN_SECONDS + (self.next_u32() as f32 / u32::MAX as f32) * span
    }

    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG: deterministic, no dependency, good enough for
        // choosing a weather kind every few minutes.
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.rng
    }
}

fn is_prontera(map_name: &str) -> bool {
    map_name.strip_suffix(".rsw").unwrap_or(map_name).eq_ignore_ascii_case("prontera")
}

fn is_black(color: Color) -> bool {
    color.red <= f32::EPSILON && color.green <= f32::EPSILON && color.blue <= f32::EPSILON
}

fn night_factor(phase_seconds: f32) -> f32 {
    let phase = phase_seconds.rem_euclid(CYCLE_SECONDS);

    if phase < DUSK_START {
        0.0
    } else if phase < DAY_SECONDS {
        smoothstep((phase - DUSK_START) / TWILIGHT_SECONDS)
    } else if phase < DAWN_START {
        1.0
    } else {
        1.0 - smoothstep((phase - DAWN_START) / TWILIGHT_SECONDS)
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn modulate(color: Color, night_scale: f32, night: f32) -> Color {
    let scale = 1.0 + (night_scale - 1.0) * night;
    let tint = lerp_color(Color::WHITE, NIGHT_TINT, night);

    Color::rgba(
        color.red * scale * tint.red,
        color.green * scale * tint.green,
        color.blue * scale * tint.blue,
        color.alpha,
    )
}

fn lerp_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::rgba(
        from.red + (to.red - from.red) * amount,
        from.green + (to.green - from.green) * amount,
        from.blue + (to.blue - from.blue) * amount,
        from.alpha + (to.alpha - from.alpha) * amount,
    )
}

#[cfg(test)]
mod tests {
    use super::{CYCLE_SECONDS, DAY_SECONDS, night_factor};

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < 1e-4, "{left} != {right}");
    }

    #[test]
    fn day_and_night_are_fifteen_minutes_each() {
        assert_close(night_factor(0.0), 0.0);
        assert_close(night_factor(DAY_SECONDS - 61.0), 0.0);
        assert_close(night_factor(DAY_SECONDS), 1.0);
        assert_close(night_factor(DAY_SECONDS + 1.0), 1.0);
        assert!(night_factor(CYCLE_SECONDS - 1.0) < 0.01);
        assert_close(night_factor(CYCLE_SECONDS), 0.0);
    }

    #[test]
    fn twilight_cross_fades_are_monotonic() {
        let mut previous = night_factor(DAY_SECONDS - 60.0);
        for step in 1..=60 {
            let current = night_factor(DAY_SECONDS - 60.0 + step as f32);
            assert!(current >= previous);
            previous = current;
        }
    }
}
