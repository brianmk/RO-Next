//! Refine UI (`UIRefiningWnd`) of the 2016+ rAthena refine flow.
//!
//! On CondaRO the refine NPCs (`npc/merchants/refine.txt`) and the `@refineui`
//! atcommand open this window because `feature.refineui` is enabled
//! (`conf/battle/feature.conf:85`). The server sends `ZC_OPEN_REFINING_UI`
//! (0x0AA0) without any item list, so the window lists the player's own
//! refinable equipment and asks the server for the materials of the picked item
//! with `CZ_REFINING_SELECT_ITEM` (0x0AA1). The answering
//! `ZC_REFINING_MATERIAL_LIST` (0x0AA2) contains the success chance, the zeny
//! cost and the Blacksmith Blessing requirement per material; the actual
//! attempt is `CZ_REQ_REFINING` (0x0AA3) and closing the window sends
//! `CZ_CLOSE_REFINING_UI` (0x0AA4).
//!
//! The old Blacksmith-skill refine list (`ZC_NOTIFY_WEAPONITEMLIST`, 0x0221)
//! is handled by [`super::ItemServiceWindow`] instead.

use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, Selector, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::item_service::{ItemServiceEntry, ItemServicePathExt, RefiningMaterial, RefiningUiPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Which step of the refine flow the window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefiningMode {
    /// Pick the item that should be refined.
    SelectItem,
    /// Pick the material (and start the refine) for the selected item.
    SelectMaterial,
}

/// Window of the NPC/atcommand refine UI.
pub struct RefiningWindow {
    mode: RefiningMode,
}

impl RefiningWindow {
    pub fn new(mode: RefiningMode) -> Self {
        Self { mode }
    }

    fn header(mode: RefiningMode) -> &'static str {
        match mode {
            RefiningMode::SelectItem => "Select the equipment you want to refine.",
            RefiningMode::SelectMaterial => "Select the material for the refine attempt.",
        }
    }
}

impl CustomWindow<ClientState> for RefiningWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Refining)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let mode = self.mode;

        window! {
            title: "Refine",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 420.0,
            maximum_width: 700.0,
            maximum_height: 640.0,
            elements: (
                text! {
                    text: client_state().item_service().refining().status_message(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::header(mode),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: selected_item_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                either! {
                    selector: ComputedSelector::<_, bool>::new_default(move |_: &ClientState| mode == RefiningMode::SelectMaterial),
                    on_true: scroll_view! {
                        scroll_bar: true,
                        fit_content_height: true,
                        children: RefiningMaterialList::new(client_state().item_service().refining().materials()),
                    },
                    on_false: scroll_view! {
                        scroll_bar: true,
                        fit_content_height: true,
                        children: RefiningCandidateList::new(client_state().item_service().refining().candidates()),
                    },
                },
                either! {
                    selector: blessing_available(),
                    on_true: split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: blessing_text(),
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            state_button! {
                                text: "Use Blacksmith Blessings",
                                state: client_state().item_service().refining().use_blacksmith_blessing(),
                                event: Toggle(client_state().item_service().refining().use_blacksmith_blessing()),
                            },
                        ),
                    },
                    on_false: text! {
                        text: "",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                },
                button! {
                    text: "Close",
                    event: |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::CloseRefiningUi);
                    },
                },
            ),
        }
    }
}

/// Selector for the Blacksmith Blessing requirement line.
fn blessing_text() -> impl Selector<ClientState, String> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(|state: &ClientState| {
        let blessing = *client_state().item_service().refining().blacksmith_blessing().follow_safe(state);
        format!("This refine level requires {blessing} Blacksmith Blessing(s) to protect the item.")
    })
}

/// Selector that is `true` when the server offers Blacksmith Blessings.
fn blessing_available() -> impl Selector<ClientState, bool> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(|state: &ClientState| {
        *client_state().item_service().refining().blacksmith_blessing().follow_safe(state) > 0
    })
}

/// Selector for the name of the item whose materials are shown.
fn selected_item_text() -> impl Selector<ClientState, String> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(|state: &ClientState| {
        client_state()
            .item_service()
            .refining()
            .item()
            .follow_safe(state)
            .as_ref()
            .map(|item| format!("Selected: {}", item.label()))
            .unwrap_or_default()
    })
}

/// Rows of the item picker.
struct RefiningCandidateList<A> {
    entries_path: A,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A> RefiningCandidateList<A> {
    fn new(entries_path: A) -> Self {
        Self {
            entries_path,
            rows: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for RefiningCandidateList<A>
where
    A: Path<ClientState, Vec<ItemServiceEntry>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        use ro_next_interface::prelude::*;

        with_single_resolver(resolvers, |resolver| {
            let entries = state.get(&self.entries_path);

            if self.rows.len() != entries.len() {
                self.rows.clear();

                for index in 0..entries.len() {
                    let entry_path = self.entries_path.index(index).manually_asserted();
                    let text_path = entry_path;
                    let click_path = entry_path;

                    let display_text = ComputedSelector::new_default(move |state: &ClientState| text_path.follow_safe(state).label());

                    self.rows.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: display_text,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: "Select",
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let inventory_index = state.get(&click_path).index;
                                    queue.queue(InputEvent::SelectRefiningItem { inventory_index });
                                },
                            },
                        ),
                    }));
                }
            }

            self.rows.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.rows.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

/// Rows of the material picker.
struct RefiningMaterialList<A> {
    materials_path: A,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A> RefiningMaterialList<A> {
    fn new(materials_path: A) -> Self {
        Self {
            materials_path,
            rows: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for RefiningMaterialList<A>
where
    A: Path<ClientState, Vec<RefiningMaterial>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        use ro_next_interface::prelude::*;

        with_single_resolver(resolvers, |resolver| {
            let materials = state.get(&self.materials_path);

            if self.rows.len() != materials.len() {
                self.rows.clear();

                for index in 0..materials.len() {
                    let material_path = self.materials_path.index(index).manually_asserted();
                    let text_path = material_path;
                    let click_path = material_path;

                    let display_text = ComputedSelector::new_default(move |state: &ClientState| {
                        let material = text_path.follow_safe(state);
                        format!("{} — {}% success — {} zeny", material.name, material.chance, material.zeny)
                    });

                    self.rows.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: display_text,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: "Refine",
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let material_item_id = state.get(&click_path).item_id;
                                    let blacksmith_blessing = *state
                                        .get(&client_state().item_service().refining().use_blacksmith_blessing());
                                    queue.queue(InputEvent::RefineWithMaterial {
                                        material_item_id,
                                        blacksmith_blessing,
                                    });
                                },
                            },
                        ),
                    }));
                }
            }

            self.rows.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.rows.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}
