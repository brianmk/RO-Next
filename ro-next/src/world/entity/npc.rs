//! The NPC/monster-specific entity state and behaviour (`Npc`).

use super::*;

#[derive(Clone, RustState, StateWindow)]
pub struct Npc {
    pub(crate) common: Common,
}

impl Npc {
    pub fn new(
        library: &Library,
        map: &Map,
        path_finder: &mut PathFinder,
        entity_data: EntityData,
        client_tick: ClientTick,
    ) -> Option<Self> {
        if GR2_MODEL_ACTOR_JOB_IDS.contains(&entity_data.job_id.0) {
            return None;
        }

        let Some(position) = map.get_world_position(entity_data.position.tile_position()) else {
            #[cfg(feature = "debug")]
            ro_next_debug::logging::print_debug!(
                "[{}] NPC with id {:?} is out of map bounds",
                "error".red(),
                entity_data.entity_id
            );
            return None;
        };

        let mut common = Common::new(
            library,
            &entity_data,
            entity_data.position.tile_position(),
            position,
            client_tick,
        );

        if let Some(destination) = entity_data.destination {
            common.move_from_to(
                map,
                path_finder,
                &[],
                entity_data.position.tile_position(),
                destination.tile_position(),
                client_tick,
            );
        }

        Some(Self { common })
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        if self.common.entity_type != EntityType::Monster {
            return;
        }

        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.enemy_bar_width;

        renderer.render_rectangle(
            final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            } + (theme.status_bar.border_size * 2.0),
            theme.status_bar.background_color,
        );

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            },
            theme.status_bar.enemy_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
    }
}

/// The second label line that is drawn under the name of a player, if any.
///
/// Only players in a guild get a guild label; NPCs, monsters and players
/// without a guild keep their single line name label. The name is wrapped in
/// angle brackets the way it is usually shown in Ragnarok Online clients.
pub fn guild_name_label(entity_type: EntityType, guild_name: Option<&str>) -> Option<String> {
    if entity_type != EntityType::Player {
        return None;
    }

    let guild_name = guild_name?.trim();

    if guild_name.is_empty() {
        return None;
    }

    Some(format!("<{guild_name}>"))
}
