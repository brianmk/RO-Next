//! The minimap view: a top-down image of the current map with a marker for the
//! player, rendered inside the minimap window.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::minimap::MinimapPathExt;
use crate::state::{ClientState, ClientStatePathExt, client_state};

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

const BACKGROUND_COLOR: Color = Color::rgb_u8(22, 24, 28);
const FALLBACK_MARKER_COLOR: Color = Color::rgb_u8(80, 255, 110);
const FALLBACK_MARKER_OUTLINE_COLOR: Color = Color::rgba_u8(0, 0, 0, 200);

pub struct MinimapView<Text, Transparent> {
    loading_text: Text,
    transparent_frame: Transparent,
    view_size: f32,
}

impl<Text, Transparent> MinimapView<Text, Transparent> {
    pub fn new(loading_text: Text, transparent_frame: Transparent, view_size: f32) -> Self {
        Self {
            loading_text,
            transparent_frame,
            view_size,
        }
    }
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

        // In transparent mode the map fills the whole element and no dark frame
        // is drawn, so only the generated map image is visible.
        let inner = if transparent_frame {
            area
        } else {
            layout.add_rectangle(
                area,
                CornerDiameter::uniform(4.0),
                BACKGROUND_COLOR,
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
                FontSize(13.0),
                Color::monochrome_u8(200),
                Color::monochrome_u8(200),
                HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            return;
        };

        let texture_size = texture.get_size();
        let texture_width = texture_size.width as f32;
        let texture_height = texture_size.height as f32;

        let scale = (inner.width / texture_width).min(inner.height / texture_height);
        let draw_width = texture_width * scale;
        let draw_height = texture_height * scale;

        let map_area = Area {
            left: inner.left + (inner.width - draw_width) / 2.0,
            top: inner.top + (inner.height - draw_height) / 2.0,
            width: draw_width,
            height: draw_height,
        };

        layout.add_texture(map_area, texture, Color::WHITE, true);

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
                        let color = Color::rgba_u8(marker.color[0], marker.color[1], marker.color[2], marker.color[3]);

                        layout.add_rectangle(
                            Area {
                                left: center_x - MARKER_SIZE / 2.0,
                                top: center_y - MARKER_SIZE / 2.0,
                                width: MARKER_SIZE,
                                height: MARKER_SIZE,
                            },
                            CornerDiameter::uniform(1.0),
                            color,
                            FALLBACK_MARKER_OUTLINE_COLOR,
                            ShadowPadding::uniform(0.5),
                        );
                    }
                });
            }
        }

        // Player marker: an arrow that points into the direction the player is
        // facing, positioned by the player's tile relative to the map size in
        // cells.
        let Some(player_position) = *state.get(&client_state().minimap().player_position()) else {
            return;
        };

        let cells_width = *state.get(&client_state().minimap().cells_width()) as f32;
        let cells_height = *state.get(&client_state().minimap().cells_height()) as f32;

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
                    FALLBACK_MARKER_COLOR,
                    FALLBACK_MARKER_OUTLINE_COLOR,
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
