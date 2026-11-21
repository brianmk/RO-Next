//! The minimap view: a top-down image of the current map with a marker for the
//! player, rendered inside the minimap window.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::State;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::minimap::MinimapPathExt;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Size of the square map view in interface (unscaled) pixels.
const VIEW_SIZE: f32 = 192.0;
const PADDING: f32 = 4.0;
const MARKER_SIZE: f32 = 7.0;

const BACKGROUND_COLOR: Color = Color::rgb_u8(22, 24, 28);
const MARKER_COLOR: Color = Color::rgb_u8(80, 255, 110);
const MARKER_OUTLINE_COLOR: Color = Color::rgba_u8(0, 0, 0, 200);

pub struct MinimapView<P> {
    loading_text: P,
}

impl<P> MinimapView<P> {
    pub fn new(loading_text: P) -> Self {
        Self { loading_text }
    }
}

impl<P> Element<ClientState> for MinimapView<P>
where
    P: rust_state::Path<ClientState, String>,
{
    type LayoutInfo = Area;

    fn create_layout_info(&mut self, _: &State<ClientState>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) -> Area {
        with_single_resolver(resolvers, |resolver| resolver.with_height(VIEW_SIZE))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Area,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let area = *layout_info;

        layout.add_rectangle(
            area,
            CornerDiameter::uniform(4.0),
            BACKGROUND_COLOR,
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        let inner = Area {
            left: area.left + PADDING,
            top: area.top + PADDING,
            width: (area.width - PADDING * 2.0).max(1.0),
            height: (area.height - PADDING * 2.0).max(1.0),
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

        // Player marker: a small green square with a dark outline, positioned by
        // the player's tile relative to the map size in cells.
        let Some(player_position) = *state.get(&client_state().minimap().player_position()) else {
            return;
        };

        let cells_width = *state.get(&client_state().minimap().cells_width()) as f32;
        let cells_height = *state.get(&client_state().minimap().cells_height()) as f32;

        if cells_width <= 0.0 || cells_height <= 0.0 {
            return;
        }

        let marker_center_x = map_area.left + draw_width * (player_position.x as f32 / cells_width);
        let marker_center_y = map_area.top + draw_height * (player_position.y as f32 / cells_height);

        let marker_area = Area {
            left: marker_center_x - MARKER_SIZE / 2.0,
            top: marker_center_y - MARKER_SIZE / 2.0,
            width: MARKER_SIZE,
            height: MARKER_SIZE,
        };

        // All rectangles of a layer are drawn before its textures, so the marker
        // has to be put into a new layer on top of the map texture.
        layout.with_layer(|layout| {
            layout.add_rectangle(
                marker_area,
                CornerDiameter::uniform(1.5),
                MARKER_COLOR,
                MARKER_OUTLINE_COLOR,
                ShadowPadding::uniform(1.0),
            );
        });
    }
}
