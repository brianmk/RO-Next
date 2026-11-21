use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::Path;

use crate::interface::components::equipment_stats::EquipmentStats;
use crate::interface::components::paper_doll::PaperDoll;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::{Player, ResourceMetadata};

/// Total width of the equipment window. The content is split into two equally
/// sized columns (paper doll and stats), so this is derived from the width of
/// the paper doll canvas.
const WINDOW_WIDTH: f32 = 488.0;

pub struct EquipmentWindow<A, B> {
    items_path: A,
    player_path: B,
}

impl<A, B> EquipmentWindow<A, B> {
    pub fn new(items_path: A, player_path: B) -> Self {
        Self { items_path, player_path }
    }
}

impl<A, B> CustomWindow<ClientState> for EquipmentWindow<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, Player>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Equipment)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Equipment",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: WINDOW_WIDTH,
            maximum_width: WINDOW_WIDTH,
            elements: (
                split! {
                    gaps: 8.0,
                    children: (
                        PaperDoll::new(self.items_path),
                        EquipmentStats::new(self.items_path, self.player_path),
                    ),
                },
            ),
        }
    }
}
