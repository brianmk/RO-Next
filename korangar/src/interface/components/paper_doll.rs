//! Paper doll view of the equipped items.
//!
//! The equipment window of the original client shows the equipped items on a
//! figure of the character, with a dedicated slot for every equipment position.
//! This component recreates that layout. The figure itself is drawn with simple
//! rounded rectangles (korangar does not render 3D character models inside the
//! interface) and the item slots are placed around it, at the position that
//! corresponds to the body part the item is worn on.

use std::sync::Arc;

use korangar_interface::MouseMode;
use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use ragnarok_packets::EquipPosition;
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::ItemSource;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::ClientState;
use crate::world::ResourceMetadata;

/// Width of the paper doll canvas.
const CANVAS_WIDTH: f32 = 220.0;
/// Height of the paper doll canvas.
const CANVAS_HEIGHT: f32 = 342.0;
/// Edge length of a single equipment slot.
const SLOT_SIZE: f32 = 42.0;

const SLOT_BACKGROUND_COLOR: Color = Color::rgba_u8(24, 24, 28, 150);
const SLOT_HOVERED_BACKGROUND_COLOR: Color = Color::rgb_u8(70, 70, 76);
const SLOT_DRAG_BACKGROUND_COLOR: Color = Color::rgb_u8(150, 150, 70);
const SLOT_DRAG_HOVERED_BACKGROUND_COLOR: Color = Color::rgb_u8(70, 150, 150);
const SLOT_BORDER_COLOR: Color = Color::rgba_u8(120, 120, 130, 110);
const SLOT_FILLED_BORDER_COLOR: Color = Color::rgba_u8(88, 130, 180, 200);
const SLOT_LABEL_COLOR: Color = Color::rgba_u8(160, 160, 170, 130);
const CANVAS_COLOR: Color = Color::rgba_u8(0, 0, 0, 50);
const FIGURE_COLOR: Color = Color::rgb_u8(78, 74, 88);
const FIGURE_HIGHLIGHT_COLOR: Color = Color::rgb_u8(92, 88, 104);
const REFINE_COLOR: Color = Color::rgb_u8(255, 190, 80);
const REFINE_BACKGROUND_COLOR: Color = Color::rgba_u8(0, 0, 0, 170);

struct SlotDefinition {
    equip_position: EquipPosition,
    /// Name shown in the tooltip when the slot is empty.
    name: &'static str,
    /// Short label shown in the slot when it is empty.
    label: &'static str,
    /// Position of the slot on the paper doll canvas.
    left: f32,
    top: f32,
}

/// All equipment slots of the paper doll, in drawing order.
const SLOTS: [SlotDefinition; 11] = [
    SlotDefinition {
        equip_position: EquipPosition::HEAD_TOP,
        name: "Head top",
        label: "TOP",
        left: 89.0,
        top: 4.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::AMMO,
        name: "Ammo",
        label: "AMMO",
        left: 166.0,
        top: 4.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::HEAD_MIDDLE,
        name: "Head middle",
        label: "MID",
        left: 12.0,
        top: 52.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::HEAD_LOWER,
        name: "Head lower",
        label: "LOW",
        left: 166.0,
        top: 52.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::ARMOR,
        name: "Armor",
        label: "ARMOR",
        left: 12.0,
        top: 126.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::GARMENT,
        name: "Garment",
        label: "GARM",
        left: 166.0,
        top: 126.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::RIGHT_HAND,
        name: "Right hand (weapon)",
        label: "WPN",
        left: 12.0,
        top: 200.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::LEFT_HAND,
        name: "Left hand (shield)",
        label: "SHLD",
        left: 166.0,
        top: 200.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::LEFT_ACCESSORY,
        name: "Accessory 1",
        label: "ACC1",
        left: 12.0,
        top: 274.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::RIGTH_ACCESSORY,
        name: "Accessory 2",
        label: "ACC2",
        left: 166.0,
        top: 274.0,
    },
    SlotDefinition {
        equip_position: EquipPosition::SHOES,
        name: "Shoes",
        label: "SHOES",
        left: 89.0,
        top: 296.0,
    },
];

const SLOT_COUNT: usize = SLOTS.len();

/// Part of the figure that is drawn on the paper doll canvas.
struct FigurePart {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    corner_diameter: f32,
    color: Color,
}

const FIGURE: [FigurePart; 10] = [
    // Head.
    FigurePart {
        left: 90.0,
        top: 54.0,
        width: 40.0,
        height: 40.0,
        corner_diameter: 20.0,
        color: FIGURE_HIGHLIGHT_COLOR,
    },
    // Neck.
    FigurePart {
        left: 102.0,
        top: 90.0,
        width: 16.0,
        height: 14.0,
        corner_diameter: 5.0,
        color: FIGURE_COLOR,
    },
    // Torso.
    FigurePart {
        left: 76.0,
        top: 100.0,
        width: 68.0,
        height: 82.0,
        corner_diameter: 16.0,
        color: FIGURE_COLOR,
    },
    // Left arm.
    FigurePart {
        left: 56.0,
        top: 106.0,
        width: 16.0,
        height: 74.0,
        corner_diameter: 8.0,
        color: FIGURE_COLOR,
    },
    // Right arm.
    FigurePart {
        left: 148.0,
        top: 106.0,
        width: 16.0,
        height: 74.0,
        corner_diameter: 8.0,
        color: FIGURE_COLOR,
    },
    // Hips.
    FigurePart {
        left: 82.0,
        top: 180.0,
        width: 56.0,
        height: 16.0,
        corner_diameter: 8.0,
        color: FIGURE_COLOR,
    },
    // Left leg.
    FigurePart {
        left: 90.0,
        top: 194.0,
        width: 16.0,
        height: 76.0,
        corner_diameter: 7.0,
        color: FIGURE_COLOR,
    },
    // Right leg.
    FigurePart {
        left: 114.0,
        top: 194.0,
        width: 16.0,
        height: 76.0,
        corner_diameter: 7.0,
        color: FIGURE_COLOR,
    },
    // Left foot.
    FigurePart {
        left: 84.0,
        top: 268.0,
        width: 24.0,
        height: 10.0,
        corner_diameter: 5.0,
        color: FIGURE_COLOR,
    },
    // Right foot.
    FigurePart {
        left: 112.0,
        top: 268.0,
        width: 24.0,
        height: 10.0,
        corner_diameter: 5.0,
        color: FIGURE_COLOR,
    },
];

/// Path to the item that is equipped in a certain equipment position.
#[derive(Clone, Copy)]
pub struct EquipmentSlotPath<A> {
    equip_position: EquipPosition,
    path: A,
}

impl<A> EquipmentSlotPath<A> {
    pub fn new(equip_position: EquipPosition, path: A) -> Self {
        Self { equip_position, path }
    }
}

impl<A> Selector<ClientState, InventoryItem<ResourceMetadata>, false> for EquipmentSlotPath<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        self.follow(state)
    }
}

impl<A> Path<ClientState, InventoryItem<ResourceMetadata>, false> for EquipmentSlotPath<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        self.path.follow_safe(state).iter().find(|item| {
            if let InventoryItemDetails::Equippable { equipped_position, .. } = &item.details {
                return equipped_position.contains(self.equip_position);
            }

            false
        })
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut InventoryItem<ResourceMetadata>> {
        self.path.follow_mut_safe(state).iter_mut().find(|item| {
            if let InventoryItemDetails::Equippable { equipped_position, .. } = &item.details {
                return equipped_position.contains(self.equip_position);
            }

            false
        })
    }
}

struct EquipmentSlotHandler<A> {
    equip_position: EquipPosition,
    items_path: A,
}

impl<A> ClickHandler<ClientState> for EquipmentSlotHandler<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        let item_path = EquipmentSlotPath::new(self.equip_position, self.items_path);

        // Unwrapping here is fine since we only register the handler if the slot has
        // an item.
        let item = state.try_get(&item_path).unwrap().clone();

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Custom {
                mode: MouseInputMode::MoveItem {
                    item,
                    source: ItemSource::Equipment {
                        position: self.equip_position,
                    },
                },
            },
        });
    }
}

impl<A> DropHandler<ClientState> for EquipmentSlotHandler<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveItem { source, item },
        } = mouse_mode
        {
            queue.queue(InputEvent::MoveItem {
                source: *source,
                destination: ItemSource::Equipment {
                    position: self.equip_position,
                },
                item: item.clone(),
            });
        }
    }
}

/// Data calculated during layout of the paper doll.
pub struct PaperDollLayoutInfo {
    canvas: Area,
    slots: [Area; SLOT_COUNT],
}

pub struct PaperDoll<A> {
    item_paths: [EquipmentSlotPath<A>; SLOT_COUNT],
    handlers: [EquipmentSlotHandler<A>; SLOT_COUNT],
    refine_displays: [Option<String>; SLOT_COUNT],
    /// Cached tooltip text per slot (composed name, card slots and random
    /// options), cloned from the pre-composed item metadata.
    tooltips: [Option<Arc<str>>; SLOT_COUNT],
}

impl<A> PaperDoll<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    pub fn new(items_path: A) -> Self {
        Self {
            item_paths: std::array::from_fn(|index| EquipmentSlotPath::new(SLOTS[index].equip_position, items_path)),
            handlers: std::array::from_fn(|index| EquipmentSlotHandler {
                equip_position: SLOTS[index].equip_position,
                items_path,
            }),
            refine_displays: std::array::from_fn(|_| None),
            tooltips: std::array::from_fn(|_| None),
        }
    }

    /// Returns the item that is equipped in the given slot.
    fn get_item<'a>(&'a self, state: &'a State<ClientState>, index: usize) -> Option<&'a InventoryItem<ResourceMetadata>> {
        state.try_get(&self.item_paths[index])
    }
}

impl<A> Element<ClientState> for PaperDoll<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    type LayoutInfo = PaperDollLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        // The refine level is shown on top of the item icon, but we can only
        // store borrows of the state for the duration of the layout, so the
        // text has to be cached in the element.
        for index in 0..SLOT_COUNT {
            let item = state.try_get(&self.item_paths[index]);

            self.refine_displays[index] = match item {
                Some(item) => match &item.details {
                    InventoryItemDetails::Equippable { refinement_level, .. } if *refinement_level > 0 => {
                        Some(format!("+{refinement_level}"))
                    }
                    _ => None,
                },
                None => None,
            };

            self.tooltips[index] = item.map(|item| item.metadata.tooltip_text.clone());
        }

        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(CANVAS_HEIGHT);

            let left = area.left + (area.width - CANVAS_WIDTH).max(0.0) / 2.0;

            let canvas = Area {
                left,
                top: area.top,
                width: area.width.min(CANVAS_WIDTH),
                height: CANVAS_HEIGHT,
            };

            let slots = std::array::from_fn(|index| Area {
                left: canvas.left + SLOTS[index].left,
                top: canvas.top + SLOTS[index].top,
                width: SLOT_SIZE,
                height: SLOT_SIZE,
            });

            PaperDollLayoutInfo { canvas, slots }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use korangar_interface::prelude::*;

        // Background of the paper doll.
        layout.add_rectangle(
            layout_info.canvas,
            CornerDiameter::uniform(14.0),
            CANVAS_COLOR,
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        // The figure itself.
        for part in &FIGURE {
            let area = Area {
                left: layout_info.canvas.left + part.left,
                top: layout_info.canvas.top + part.top,
                width: part.width,
                height: part.height,
            };

            layout.add_rectangle(
                area,
                CornerDiameter::uniform(part.corner_diameter),
                part.color,
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );
        }

        let is_dragging = matches!(layout.get_mouse_mode(), MouseMode::Custom {
            mode: MouseInputMode::MoveItem { .. },
        });

        for (index, slot) in SLOTS.iter().enumerate() {
            let area = layout_info.slots[index];
            let item = self.get_item(state, index);

            let (is_hovered, background_color) = match is_dragging {
                true => match area.check().any_mouse_mode().run(layout) {
                    // Since we are not in default mouse mode we need to mark the window as
                    // hovered.
                    true => {
                        layout.set_hovered();
                        (true, SLOT_DRAG_HOVERED_BACKGROUND_COLOR)
                    }
                    false => (false, SLOT_DRAG_BACKGROUND_COLOR),
                },
                false => match area.check().run(layout) {
                    true => (true, SLOT_HOVERED_BACKGROUND_COLOR),
                    false => (false, SLOT_BACKGROUND_COLOR),
                },
            };

            let border_color = match item.is_some() {
                true => SLOT_FILLED_BORDER_COLOR,
                false => SLOT_BORDER_COLOR,
            };

            layout.add_rectangle(
                area,
                CornerDiameter::uniform(10.0),
                background_color,
                border_color,
                ShadowPadding::uniform(1.0),
            );

            match item {
                Some(item) => {
                    if let Some(texture) = item.metadata.texture.as_ref() {
                        let texture_size = area.width.min(area.height) - 6.0;

                        let texture_area = Area {
                            left: area.left + (area.width - texture_size) / 2.0,
                            top: area.top + (area.height - texture_size) / 2.0,
                            width: texture_size,
                            height: texture_size,
                        };

                        layout.add_texture(texture_area, texture.clone(), Color::WHITE, false);
                    }

                    if let Some(refine_text) = &self.refine_displays[index] {
                        let refine_area = Area {
                            left: area.left + area.width - 26.0,
                            top: area.top + area.height - 16.0,
                            width: 22.0,
                            height: 13.0,
                        };

                        layout.add_rectangle(
                            refine_area,
                            CornerDiameter::uniform(4.0),
                            REFINE_BACKGROUND_COLOR,
                            Color::TRANSPARENT,
                            ShadowPadding::uniform(0.0),
                        );

                        layout.add_text(
                            refine_area,
                            refine_text,
                            FontSize(11.0),
                            REFINE_COLOR,
                            Color::WHITE,
                            HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                            VerticalAlignment::Center { offset: 0.0 },
                            OverflowBehavior::Shrink,
                        );
                    }
                }
                None => {
                    layout.add_text(
                        area,
                        slot.label,
                        FontSize(9.0),
                        SLOT_LABEL_COLOR,
                        SLOT_LABEL_COLOR,
                        HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                }
            }

            if is_hovered {
                if is_dragging {
                    layout.register_drop_handler(&self.handlers[index]);
                } else if item.is_some() {
                    layout.register_click_handler(MouseButton::Left, &self.handlers[index]);
                }

                let tooltip = match &self.tooltips[index] {
                    Some(tooltip) => tooltip.as_ref(),
                    None => slot.name,
                };

                struct PaperDollSlotTooltip;

                layout.add_tooltip(tooltip, PaperDollSlotTooltip.tooltip_id());
            }
        }
    }
}
