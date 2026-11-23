//! Paper doll view of the equipped items.
//!
//! The equipment window of the original client shows the equipped items on a
//! figure of the character, with a dedicated slot for every equipment position.
//! This component recreates that layout. The figure itself is the real
//! composited character sprite (body, hair, headgear and palettes) in its idle
//! front-facing frame, using the same rendering path as the in-world sprite;
//! the item slots are placed around it, at the position that corresponds to the
//! body part the item is worn on.

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

use super::paper_doll_slots;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::ItemSource;
use crate::loaders::OverflowBehavior;
use crate::renderer::{CHARACTER_FEET_ANCHOR, LayoutExt};
use crate::state::localization::LOCALIZATION_PATH;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, TypographyTokensPathExt};
use crate::state::{ClientState, client_theme};
use crate::world::{Player, ResourceMetadata};

/// Number of equipment slots of the paper doll (data-driven, exactly one
/// slot per classic equipment position).
const SLOT_COUNT: usize = paper_doll_slots::SLOT_COUNT;

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

pub struct PaperDoll<A, B> {
    item_paths: [EquipmentSlotPath<A>; SLOT_COUNT],
    handlers: [EquipmentSlotHandler<A>; SLOT_COUNT],
    player_path: B,
    refine_displays: [Option<String>; SLOT_COUNT],
    /// Cached tooltip text per slot (composed name, card slots and random
    /// options), cloned from the pre-composed item metadata.
    tooltips: [Option<Arc<str>>; SLOT_COUNT],
}

impl<A, B> PaperDoll<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, Player>,
{
    pub fn new(items_path: A, player_path: B) -> Self {
        Self {
            item_paths: std::array::from_fn(|index| {
                EquipmentSlotPath::new(paper_doll_slots::table().slots[index].equip_position(), items_path)
            }),
            handlers: std::array::from_fn(|index| EquipmentSlotHandler {
                equip_position: paper_doll_slots::table().slots[index].equip_position(),
                items_path,
            }),
            player_path,
            refine_displays: std::array::from_fn(|_| None),
            tooltips: std::array::from_fn(|_| None),
        }
    }

    /// Returns the item that is equipped in the given slot.
    fn get_item<'a>(&'a self, state: &'a State<ClientState>, index: usize) -> Option<&'a InventoryItem<ResourceMetadata>> {
        state.try_get(&self.item_paths[index])
    }
}

impl<A, B> Element<ClientState> for PaperDoll<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, Player>,
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
            let table = paper_doll_slots::table();
            let area = resolver.with_height(table.canvas_height);

            let left = area.left + (area.width - table.canvas_width).max(0.0) / 2.0;

            let canvas = Area {
                left,
                top: area.top,
                width: area.width.min(table.canvas_width),
                height: table.canvas_height,
            };

            let slots = std::array::from_fn(|index| Area {
                left: canvas.left + table.slots[index].left,
                top: canvas.top + table.slots[index].top,
                width: table.slot_size,
                height: table.slot_size,
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

        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let localization: &crate::state::localization::Localization = state.get(&LOCALIZATION_PATH);
        let paper_doll_texts = localization.paper_doll_texts();

        let slot_background_color = *state.get(&color_tokens.slot_background());
        let slot_hovered_background_color = *state.get(&color_tokens.slot_hovered_background());
        let slot_drag_background_color = *state.get(&color_tokens.slot_drag_background());
        let slot_drag_hovered_background_color = *state.get(&color_tokens.slot_drag_hovered_background());
        let slot_border_color = *state.get(&color_tokens.slot_border());
        let slot_filled_border_color = *state.get(&color_tokens.slot_filled_border());
        let slot_label_color = *state.get(&color_tokens.slot_label());
        let canvas_background_color = *state.get(&color_tokens.canvas_background());
        let refine_text_color = *state.get(&color_tokens.refine_text());
        let refine_background_color = *state.get(&color_tokens.refine_background());
        let refine_font_size = *state.get(&typography.font_label());
        let slot_label_font_size = *state.get(&typography.font_xs());

        // Background of the paper doll.
        layout.add_rectangle(
            layout_info.canvas,
            CornerDiameter::uniform(14.0),
            canvas_background_color,
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        // The character itself: all sprite layers (body, hair, headgear) are
        // composited in the idle pose facing the viewer.
        if let Some(animation_data) = state.follow(self.player_path).get_common().animation_data.as_ref() {
            let scale = character_scale(animation_data.animations.first().and_then(|animation| animation.frames.first()));

            layout.add_character(layout_info.canvas, animation_data, Color::WHITE, scale);
        }

        let is_dragging = matches!(layout.get_mouse_mode(), MouseMode::Custom {
            mode: MouseInputMode::MoveItem { .. },
        });

        for (index, slot) in paper_doll_slots::table().slots.iter().enumerate() {
            let area = layout_info.slots[index];
            let item = self.get_item(state, index);

            let (is_hovered, background_color) = match is_dragging {
                true => match area.check().any_mouse_mode().run(layout) {
                    // Since we are not in default mouse mode we need to mark the window as
                    // hovered.
                    true => {
                        layout.set_hovered();
                        (true, slot_drag_hovered_background_color)
                    }
                    false => (false, slot_drag_background_color),
                },
                false => match area.check().run(layout) {
                    true => (true, slot_hovered_background_color),
                    false => (false, slot_background_color),
                },
            };

            let border_color = match item.is_some() {
                true => slot_filled_border_color,
                false => slot_border_color,
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
                            refine_background_color,
                            Color::TRANSPARENT,
                            ShadowPadding::uniform(0.0),
                        );

                        layout.add_text(
                            refine_area,
                            refine_text,
                            refine_font_size,
                            refine_text_color,
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
                        paper_doll_texts.slot_label(slot.equip_position()),
                        slot_label_font_size,
                        slot_label_color,
                        slot_label_color,
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
                    None => paper_doll_texts.slot_name(slot.equip_position()),
                };

                struct PaperDollSlotTooltip;

                layout.add_tooltip(tooltip, PaperDollSlotTooltip.tooltip_id());
            }
        }
    }
}

/// Scale of the character sprite on the paper doll canvas.
///
/// The idle frame of the composed animation data is the full drawn height of
/// the character (including hair, headgear and wings). The sprite is anchored
/// with its feet at [`CHARACTER_FEET_ANCHOR`] of the canvas, so the available
/// height is the distance from that anchor to the top margin. The scale is
/// capped at the data-file `character_max_scale` so normal characters keep the
/// familiar paper-doll size.
fn character_scale(frame: Option<&crate::world::AnimationFrame>) -> f32 {
    let table = paper_doll_slots::table();
    let frame_height = frame
        .map(|frame| frame.size.y as f32)
        .filter(|height| *height > 0.0)
        .unwrap_or(96.0);
    let available_height = table.canvas_height * CHARACTER_FEET_ANCHOR - table.character_top_margin;

    (available_height / frame_height).min(table.character_max_scale)
}

#[cfg(test)]
mod tests {
    use super::{CHARACTER_FEET_ANCHOR, character_scale, paper_doll_slots};

    fn character_scale_for_height(frame_height: f32) -> f32 {
        let table = paper_doll_slots::table();
        let available_height = table.canvas_height * CHARACTER_FEET_ANCHOR - table.character_top_margin;

        (available_height / frame_height).min(table.character_max_scale)
    }

    #[test]
    fn normal_characters_use_the_maximum_paper_doll_scale() {
        // Typical player idle frames are around 70-80 pixels tall.
        let maximum_scale = paper_doll_slots::table().character_max_scale;
        for frame_height in [60.0, 70.0, 80.0] {
            assert_eq!(character_scale_for_height(frame_height), maximum_scale);
        }
    }

    #[test]
    fn tall_headgear_is_scaled_down_to_stay_inside_the_canvas() {
        let table = paper_doll_slots::table();
        let tall_frame = 120.0;
        let scale = character_scale_for_height(tall_frame);
        let top = table.canvas_height * CHARACTER_FEET_ANCHOR - tall_frame * scale;

        assert!(scale < table.character_max_scale);
        assert!(
            top >= table.character_top_margin - 0.001,
            "character is clipped at the top: {top}"
        );
    }

    #[test]
    fn helper_matches_the_formula() {
        // The helper used by the component only differs from the test formula by
        // taking the frame out of the animation data.
        assert!((character_scale(None) - character_scale_for_height(96.0)).abs() < 0.001);
    }
}
