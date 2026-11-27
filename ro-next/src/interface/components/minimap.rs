//! The minimap view: a top-down image of the current map with a marker for the
//! player, rendered inside the minimap window.

use ragnarok_packets::TilePosition;
use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{ComputedSelector, HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, PathExt, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::minimap::MinimapPathExt;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Size of the square map view in interface (unscaled) pixels.
pub const MINIMAP_VIEW_SIZE: f32 = 192.0;
/// Padding between the map image and the window frame.
const PADDING: f32 = 4.0;
/// Size of the fallback marker (used when no arrow texture is available).
const FALLBACK_MARKER_SIZE: f32 = 7.0;
/// The arrow texture is drawn at `view_size / this` pixels.
const MARKER_SIZE_DIVISOR: f32 = 13.0;
/// Size of a server-placed minimap marker in interface pixels.
const MARKER_SIZE: f32 = 5.0;
/// Height of the map name/coordinate row below the minimap image.
const LOCATION_ROW_HEIGHT: f32 = 16.0;
/// Gap between the minimap image and the location row.
const LOCATION_ROW_GAP: f32 = 2.0;

pub struct MinimapView<Text, Transparent> {
    loading_text: Text,
    transparent_frame: Transparent,
    view_size: f32,
    /// Cached map name/coordinate line, recomputed from the state every frame.
    location_text: ComputedSelector<fn(&ClientState) -> String, String>,
}

impl<Text, Transparent> MinimapView<Text, Transparent> {
    pub fn new(loading_text: Text, transparent_frame: Transparent, view_size: f32) -> Self {
        Self {
            loading_text,
            transparent_frame,
            view_size,
            location_text: ComputedSelector::new_default(minimap_location_text as fn(&ClientState) -> String),
        }
    }
}

/// Formats the map/tile line shown below the minimap image.
///
/// Returns an empty string while no map is loaded (the row is then omitted).
/// If the map is known but the player tile is not (first frames after a map
/// change), only the map name is shown.
fn format_location(map_name: &str, position: Option<TilePosition>) -> String {
    if map_name.is_empty() {
        return String::new();
    }

    match position {
        Some(position) => format!("{map_name}  {}, {}", position.x, position.y),
        None => map_name.to_owned(),
    }
}

/// Computes [`format_location`] from the current minimap state.
fn minimap_location_text(state: &ClientState) -> String {
    format_location(
        client_state().minimap().map_name().follow_safe(state),
        *client_state().minimap().player_position().follow_safe(state),
    )
}

impl<Text, Transparent> Element<ClientState> for MinimapView<Text, Transparent>
where
    Text: Path<ClientState, String>,
    Transparent: Path<ClientState, bool>,
{
    type LayoutInfo = Area;

    fn create_layout_info(&mut self, _: &State<ClientState>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) -> Area {
        with_single_resolver(resolvers, |resolver| resolver.with_height(self.view_size))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Area,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let area = *layout_info;
        let transparent_frame = *state.get(&self.transparent_frame);

        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let background_color = *state.get(&color_tokens.minimap_background());
        let fallback_marker_color = *state.get(&color_tokens.minimap_marker_fallback());
        let marker_outline_color = *state.get(&color_tokens.minimap_marker_outline());
        let overlay_secondary_text = *state.get(&color_tokens.overlay_secondary_text());
        let loading_font_size = *state.get(&typography.font_button());
        let location_font_size = *state.get(&typography.font_caption());

        // In transparent mode the map fills the whole element and no dark frame
        // is drawn, so only the generated map image is visible.
        let inner = if transparent_frame {
            area
        } else {
            layout.add_rectangle(
                area,
                CornerDiameter::uniform(4.0),
                background_color,
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );

            Area {
                left: area.left + PADDING,
                top: area.top + PADDING,
                width: (area.width - PADDING * 2.0).max(1.0),
                height: (area.height - PADDING * 2.0).max(1.0),
            }
        };

        let texture = state.get(&client_state().minimap().texture()).clone();

        let Some(texture) = texture else {
            let text: &str = state.get(&self.loading_text);

            layout.add_text(
                inner,
                text,
                loading_font_size,
                overlay_secondary_text,
                overlay_secondary_text,
                HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            return;
        };

        let texture_size = texture.get_size();
        let texture_width = texture_size.width as f32;
        let texture_height = texture_size.height as f32;

        // Keep a row free below the map image for the map name and tile
        // coordinates. The window is only 210x250 with a 192 pixel view and a
        // transparent-frame toggle, so the row has to live inside the view;
        // appending another element to the window would be clipped by the
        // window's fixed height.
        let map_bounds = Area {
            left: inner.left,
            top: inner.top,
            width: inner.width,
            height: (inner.height - LOCATION_ROW_HEIGHT - LOCATION_ROW_GAP).max(1.0),
        };

        let scale = (map_bounds.width / texture_width).min(map_bounds.height / texture_height);
        let draw_width = texture_width * scale;
        let draw_height = texture_height * scale;

        let map_area = Area {
            left: map_bounds.left + (map_bounds.width - draw_width) / 2.0,
            top: map_bounds.top + (map_bounds.height - draw_height) / 2.0,
            width: draw_width,
            height: draw_height,
        };

        layout.add_texture(map_area, texture, Color::WHITE, true);

        // Current map and tile below the map image. The selector recomputes
        // the line from the minimap state on every frame, so both the map
        // change and every tile step are reflected live. `Shrink` keeps long
        // map names inside the 210 pixel window.
        let location_text: &String = state.get(&self.location_text);

        if !location_text.is_empty() {
            layout.add_text(
                Area {
                    left: inner.left,
                    top: inner.top + inner.height - LOCATION_ROW_HEIGHT,
                    width: inner.width,
                    height: LOCATION_ROW_HEIGHT,
                },
                location_text,
                location_font_size,
                overlay_secondary_text,
                // Black outline keeps the coordinates readable over bright or
                // busy parts of the map image.
                Color::BLACK,
                HorizontalAlignment::Center { offset: 0.0, border: 4.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        // Server-placed NPC/quest markers (`ZC_COMPASS`, 0x0144).
        let markers_path = client_state().minimap().markers();
        let markers = state.get(&markers_path);
        if !markers.is_empty() {
            let cells_width = *state.get(&client_state().minimap().cells_width()) as f32;
            let cells_height = *state.get(&client_state().minimap().cells_height()) as f32;

            if cells_width > 0.0 && cells_height > 0.0 {
                layout.with_layer(|layout| {
                    for marker in markers {
                        let center_x = map_area.left + draw_width * ((marker.position.x as f32 + 0.5) / cells_width);
                        let center_y = map_area.top + draw_height * (1.0 - (marker.position.y as f32 + 0.5) / cells_height);
                        let color = Color::from_rgba_bytes(marker.color);

                        layout.add_rectangle(
                            Area {
                                left: center_x - MARKER_SIZE / 2.0,
                                top: center_y - MARKER_SIZE / 2.0,
                                width: MARKER_SIZE,
                                height: MARKER_SIZE,
                            },
                            CornerDiameter::uniform(1.0),
                            color,
                            marker_outline_color,
                            ShadowPadding::uniform(0.5),
                        );
                    }
                });
            }
        }

        // Client-side navigation route (`/navi`, search window). The route is
        // computed with `world/pathing.rs` and only exists for destinations on
        // the current map.
        let cells_width = *state.get(&client_state().minimap().cells_width()) as f32;
        let cells_height = *state.get(&client_state().minimap().cells_height()) as f32;

        if cells_width > 0.0 && cells_height > 0.0 {
            let navigation_path = client_state().navigation();
            let navigation = state.get(&navigation_path);
            let route_color = Color::from_rgba_bytes([255, 200, 60, 190]);
            let route_outline = Color::from_rgba_bytes([40, 30, 0, 190]);
            let destination_color = Color::from_rgba_bytes([255, 120, 40, 255]);

            if !navigation.route().is_empty() {
                layout.with_layer(|layout| {
                    for tile in navigation.route() {
                        let center_x = map_area.left + draw_width * ((tile.x as f32 + 0.5) / cells_width);
                        let center_y = map_area.top + draw_height * (1.0 - (tile.y as f32 + 0.5) / cells_height);

                        layout.add_rectangle(
                            Area {
                                left: center_x - 1.5,
                                top: center_y - 1.5,
                                width: 3.0,
                                height: 3.0,
                            },
                            CornerDiameter::uniform(1.0),
                            route_color,
                            route_outline,
                            ShadowPadding::uniform(0.0),
                        );
                    }
                });
            }

            if let Some(position) = navigation.destination().and_then(|destination| destination.position) {
                let center_x = map_area.left + draw_width * ((position.x as f32 + 0.5) / cells_width);
                let center_y = map_area.top + draw_height * (1.0 - (position.y as f32 + 0.5) / cells_height);

                layout.with_layer(|layout| {
                    layout.add_rectangle(
                        Area {
                            left: center_x - 4.0,
                            top: center_y - 4.0,
                            width: 8.0,
                            height: 8.0,
                        },
                        CornerDiameter::uniform(2.0),
                        destination_color,
                        marker_outline_color,
                        ShadowPadding::uniform(1.0),
                    );
                });
            }
        }

        // Player marker: an arrow that points into the direction the player is
        // facing, positioned by the player's tile relative to the map size in
        // cells.
        let Some(player_position) = *state.get(&client_state().minimap().player_position()) else {
            return;
        };

        if cells_width <= 0.0 || cells_height <= 0.0 {
            return;
        }

        let marker_center_x = map_area.left + draw_width * ((player_position.x as f32 + 0.5) / cells_width);
        let marker_center_y = map_area.top + draw_height * (1.0 - (player_position.y as f32 + 0.5) / cells_height);

        let marker_textures = state.get(&client_state().minimap().marker_textures()).clone();
        let marker_direction = *state.get(&client_state().minimap().player_direction());
        let marker_size = (self.view_size / MARKER_SIZE_DIVISOR).max(FALLBACK_MARKER_SIZE + 2.0);

        // All rectangles of a layer are drawn before its textures, so the marker
        // has to be put into a new layer on top of the map texture.
        layout.with_layer(|layout| {
            if let Some(marker_texture) = marker_arrow_texture(&marker_textures, marker_direction) {
                let marker_area = Area {
                    left: marker_center_x - marker_size / 2.0,
                    top: marker_center_y - marker_size / 2.0,
                    width: marker_size,
                    height: marker_size,
                };

                layout.add_texture(marker_area, marker_texture, Color::WHITE, true);
            } else {
                let marker_area = Area {
                    left: marker_center_x - FALLBACK_MARKER_SIZE / 2.0,
                    top: marker_center_y - FALLBACK_MARKER_SIZE / 2.0,
                    width: FALLBACK_MARKER_SIZE,
                    height: FALLBACK_MARKER_SIZE,
                };

                layout.add_rectangle(
                    marker_area,
                    CornerDiameter::uniform(1.5),
                    fallback_marker_color,
                    marker_outline_color,
                    ShadowPadding::uniform(1.0),
                );
            }
        });
    }
}

/// Pick the arrow texture for the given facing direction, if one was generated.
fn marker_arrow_texture(
    marker_textures: &[std::sync::Arc<crate::graphics::Texture>],
    direction: u8,
) -> Option<std::sync::Arc<crate::graphics::Texture>> {
    let direction = direction as usize % crate::loaders::MARKER_DIRECTION_COUNT;

    marker_textures.get(direction).cloned()
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::TilePosition;

    use super::format_location;

    #[test]
    fn location_text_shows_the_map_and_tile() {
        assert_eq!(
            format_location("prontera", Some(TilePosition { x: 156, y: 187 })),
            "prontera  156, 187"
        );
    }

    #[test]
    fn location_text_handles_missing_data() {
        let position = Some(TilePosition { x: 156, y: 187 });

        assert_eq!(format_location("", position), "");
        assert_eq!(format_location("", None), "");
        // The map is known but the marker position is not set yet.
        assert_eq!(format_location("prontera", None), "prontera");
    }
}
