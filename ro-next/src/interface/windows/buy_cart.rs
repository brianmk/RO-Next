use std::cmp::Ordering;
use std::fmt::Display;

use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::event::ClickHandler;
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window};
use ro_next_networking::ShopItem;
use rust_state::{ManuallyAssertExt, Path, PathExt, Selector, State, VecIndexExt};

use super::WindowClass;
use crate::InputEvent;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::interface::components::zeny::format_zeny;
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::theme::{DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, ItemCardThemePathExt, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};
use crate::world::ResourceMetadata;

struct PartialEqDisplayStr<T> {
    last_value: Option<T>,
    text: String,
}

impl<T> PartialEqDisplayStr<T> {
    pub fn new() -> Self {
        Self {
            last_value: None,
            text: String::new(),
        }
    }
}

impl<T> PartialEqDisplayStr<T>
where
    T: Clone + PartialEq + Display + 'static,
{
    fn update(&mut self, value: T) {
        if self.last_value.is_none() || self.last_value.as_ref().is_some_and(|last| *last != value) {
            self.text = value.to_string();
            self.last_value = Some(value.clone());
        }
    }

    fn get_str(&self) -> &str {
        &self.text
    }
}

struct ItemLayoutInfo<A> {
    area: Area,
    texture_area: Area,
    text_area: Area,
    children: A,
}

struct ItemElement<A, B> {
    item_path: A,
    children: B,
    amount_string: PartialEqDisplayStr<u32>,
    price_string: PartialEqDisplayStr<u32>,
}

impl<A, B> ItemElement<A, B> {
    fn new(item_path: A, children: B) -> Self {
        Self {
            item_path,
            children,
            amount_string: PartialEqDisplayStr::new(),
            price_string: PartialEqDisplayStr::new(),
        }
    }
}

impl<A, B> Element<ClientState> for ItemElement<A, B>
where
    A: Path<ClientState, ShopItem<(ResourceMetadata, u32)>>,
    B: Element<ClientState>,
{
    type LayoutInfo = ItemLayoutInfo<B::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let (area, (texture_area, text_area, children)) = resolver.with_derived(3.0, 3.0, |resolver| {
                let area = resolver.with_height(34.0);

                let texture_area = Area {
                    width: 34.0,
                    height: 34.0,
                    ..area
                };

                let text_area = Area {
                    left: area.left + 43.0,
                    width: area.width - 43.0,
                    ..area
                };

                let children = self.children.create_layout_info(state, store, resolver);

                (texture_area, text_area, children)
            });

            let item = state.get(&self.item_path);

            self.amount_string.update(item.metadata.1);
            self.price_string.update(item.price.0 * item.metadata.1);

            Self::LayoutInfo {
                area,
                texture_area,
                text_area,
                children,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let item = state.get(&self.item_path);
        let item_card_theme = client_theme().item_card();
        let item_font_size = *state.get(&client_theme().tokens().typography().font_lg());

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(4.0),
            *state.get(&item_card_theme.background_color()),
            *state.get(&item_card_theme.border_color()),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if let Some(texture) = &item.metadata.0.texture {
            layout.add_texture(layout_info.texture_area, texture.clone(), Color::WHITE, false);

            layout.add_text(
                layout_info.texture_area,
                self.amount_string.get_str(),
                item_font_size,
                *state.get(&item_card_theme.text_color()),
                *state.get(&item_card_theme.highlight_color()),
                HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                VerticalAlignment::Bottom { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        layout.add_text(
            layout_info.text_area,
            &item.metadata.0.name,
            item_font_size,
            *state.get(&item_card_theme.text_color()),
            *state.get(&item_card_theme.highlight_color()),
            HorizontalAlignment::Left { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        layout.add_text(
            layout_info.text_area,
            self.price_string.get_str(),
            item_font_size,
            *state.get(&item_card_theme.highlight_color()),
            *state.get(&item_card_theme.text_color()),
            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        self.children.lay_out(state, store, &layout_info.children, layout);
    }
}

struct ItemList<A> {
    cart_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> ItemList<A> {
    fn new(cart_path: A) -> Self {
        Self {
            cart_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for ItemList<A>
where
    A: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use ro_next_interface::prelude::*;

            let cart = state.get(&self.cart_path);

            match cart.len().cmp(&self.elements.len()) {
                Ordering::Less => {
                    self.elements.truncate(cart.len());
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    for index in self.elements.len()..cart.len() {
                        let item_path = self.cart_path.index(index).manually_asserted();

                        fn disabled_cutoff<A>(item_path: A, amount: u32) -> impl Selector<ClientState, bool>
                        where
                            A: Path<ClientState, ShopItem<(ResourceMetadata, u32)>>,
                        {
                            ComputedSelector::new_default(move |state: &ClientState| {
                                let item = item_path.follow_safe(state);

                                item.metadata.1 < amount
                            })
                        }

                        struct RemoveAction<A, B> {
                            item_path: A,
                            cart_path: B,
                            amount: u32,
                        }

                        impl<A, B> RemoveAction<A, B> {
                            fn new(item_path: A, cart_path: B, amount: u32) -> Self {
                                Self {
                                    item_path,
                                    cart_path,
                                    amount,
                                }
                            }
                        }

                        impl<A, B> ClickHandler<ClientState> for RemoveAction<A, B>
                        where
                            A: Path<ClientState, ShopItem<(ResourceMetadata, u32)>>,
                            B: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
                        {
                            fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
                                let item_id = state.get(&self.item_path).item_id;
                                let amount = self.amount;

                                state.update_value_with(self.cart_path, move |cart| {
                                    if let Some(index) = cart.iter_mut().position(|purchase| purchase.item_id == item_id) {
                                        if cart[index].metadata.1 > amount {
                                            cart[index].metadata.1 -= amount;
                                        } else {
                                            cart.remove(index);
                                        }
                                    }
                                });
                            }
                        }

                        let buttons = (split! {
                            gaps: theme().window().gaps(),
                            children: (
                                button! {
                                    text: client_state().localization().ui().quantity_remove_one(),
                                    disabled: disabled_cutoff(item_path, 1),
                                    event: RemoveAction::new(item_path, self.cart_path, 1),
                                },
                                button! {
                                    text: client_state().localization().ui().quantity_remove_ten(),
                                    disabled: disabled_cutoff(item_path, 10),
                                    event: RemoveAction::new(item_path, self.cart_path, 10),
                                },
                                button! {
                                    text: client_state().localization().ui().quantity_remove_hundred(),
                                    disabled: disabled_cutoff(item_path, 100),
                                    event: RemoveAction::new(item_path, self.cart_path, 100),
                                },
                                button! {
                                    text: client_state().localization().ui().quantity_remove_all(),
                                    event: RemoveAction::new(item_path, self.cart_path, u32::MAX),
                                },
                            ),
                        },);

                        self.elements.push(ErasedElement::new(ItemElement::new(item_path, buttons)));
                    }
                }
            }

            self.elements.iter_mut().enumerate().for_each(|(index, element)| {
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
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

/// Shows the total zeny cost of all items in the cart.
struct TotalCost<A> {
    cart_path: A,
    text: String,
}

impl<A> TotalCost<A> {
    fn new(cart_path: A) -> Self {
        Self {
            cart_path,
            text: String::new(),
        }
    }
}

impl<A> Element<ClientState> for TotalCost<A>
where
    A: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let total: u64 = state
            .follow(self.cart_path)
            .iter()
            .map(|item| u64::from(item.price.0) * u64::from(item.metadata.1))
            .sum();

        let total_label = state.get(&client_state().localization().ui().cart_total_label()).clone();
        self.text = format!("{total_label} {}", format_zeny(total));

        with_single_resolver(resolvers, |resolver| resolver.with_height(18.0))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let item_card_theme = client_theme().item_card();
        let total_font_size = *state.get(&client_theme().tokens().typography().font_subtitle());

        layout.add_text(
            *layout_info,
            &self.text,
            total_font_size,
            *state.get(&item_card_theme.text_color()),
            *state.get(&item_card_theme.highlight_color()),
            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}

pub struct BuyCartWindow<A> {
    cart_path: A,
}

impl<A> BuyCartWindow<A> {
    pub fn new(cart_path: A) -> Self {
        Self { cart_path }
    }
}

impl<A> CustomWindow<ClientState> for BuyCartWindow<A>
where
    A: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BuyCart)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: client_state().localization().cart_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            resizable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: client_state().localization().buy_button_text(),
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                let cart = state.get(&self.cart_path);
                                let items = cart
                                    .iter()
                                    .map(|item| ShopItem {
                                        metadata: item.metadata.1,
                                        ..item.clone()
                                    })
                                    .collect();

                                queue.queue(InputEvent::BuyItems { items });
                            }
                        },
                        button! {
                            text: client_state().localization().cancel_button_text(),
                            event: InputEvent::CloseShop,
                        },
                    ),
                },
                // The total has to come before the scroll view: a scroll view consumes
                // all remaining vertical space, so an element after it would not be laid
                // out at all.
                TotalCost::new(self.cart_path),
                scroll_view! {
                    children: ItemList::new(self.cart_path),
                },
            ),
        }
    }
}
