use character_slot_preview::{CharacterSlotPreview, CharacterSlotPreviewHandler, OverlayHandler};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::character_slots::{CharacterSlots, CharacterSlotsExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

mod character_slot_preview {
    use std::cell::{Cell, UnsafeCell};
    use std::fmt::Display;

    use korangar_interface::element::store::{ElementStore, ElementStoreMut};
    use korangar_interface::element::{BaseLayoutInfo, Element};
    use korangar_interface::event::{ClickHandler, EventQueue};
    use korangar_interface::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use korangar_interface::layout::tooltip::TooltipExt;
    use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
    use ragnarok_packets::{CharacterInformation, CharacterInformationPathExt};
    use rust_state::{ManuallyAssertExt, Path, State};

    use crate::graphics::{CornerDiameter, ScreenPosition, ScreenSize, ShadowPadding};
    use crate::input::InputEvent;
    use crate::loaders::OverflowBehavior;
    use crate::state::localization::LocalizationPathExt;
    use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, TypographyTokensPathExt};
    use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

    pub struct OverlayHandler<A, B> {
        position: ScreenPosition,
        size: ScreenSize,
        slot: usize,
        switch_request_path: A,
        character_information_path: B,
        window_id: u64,
    }

    impl<A, B> OverlayHandler<A, B> {
        pub fn new(slot: usize, switch_request_path: A, character_information_path: B) -> Self {
            Self {
                position: ScreenPosition { left: 0.0, top: 0.0 },
                size: ScreenSize { width: 0.0, height: 0.0 },
                slot,
                switch_request_path,
                character_information_path,
                window_id: 0,
            }
        }

        fn set_position_size(&mut self, position: ScreenPosition, size: ScreenSize, window_id: u64) {
            self.position = position;
            self.size = size;
            self.window_id = window_id;
        }
    }

    impl<A, B> ClickHandler<ClientState> for OverlayHandler<A, B>
    where
        A: Path<ClientState, Option<usize>>,
        B: Path<ClientState, CharacterInformation, false>,
    {
        fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            use korangar_interface::prelude::*;

            let slot = self.slot;
            let switch_request_path = self.switch_request_path;
            let character_information_path = self.character_information_path;

            let element = ErasedElement::new(fragment! {
                gaps: 4.0,
                children: (
                    button! {
                        text: client_state().localization().delete_button_text(),
                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            // We should not be able to get here if the character is not present, so it's
                            // fine to unwrap.
                            let character_information = state.try_get(&character_information_path).unwrap();
                            let character_id = character_information.character_id;

                            // The character server requires the account e-mail as
                            // the deletion confirmation code, so a separate window
                            // asks for it instead of deleting immediately.
                            queue.queue(InputEvent::OpenCharacterDeleteWindow { character_id });
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                    button! {
                        text: client_state().localization().switch_button_text(),
                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            state.update_value(switch_request_path, Some(slot));
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                    button! {
                        text: client_state().localization().cancel_button_text(),
                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                ),
            });

            queue.queue(Event::OpenOverlay {
                element,
                position: self.position,
                size: self.size,
                window_id: self.window_id,
            });
        }
    }

    // #[derive(RustState)]
    // pub struct CharacterSlotPreviewTheme {
    //     pub background_color: ClientState,
    // }

    pub struct CharacterSlotPreview<P, M, B> {
        character_information: P,
        switch_request: M,
        click_handler: CharacterSlotPreviewHandler<B>,
        overlay_handler: OverlayHandler<M, P>,
        slot: usize,
    }

    impl<P, M, B> CharacterSlotPreview<P, M, B> {
        pub fn new(
            character_information: P,
            switch_request: M,
            click_handler: CharacterSlotPreviewHandler<B>,
            overlay_handler: OverlayHandler<M, P>,
            slot: usize,
        ) -> Self {
            Self {
                character_information,
                switch_request,
                click_handler,
                overlay_handler,
                slot,
            }
        }
    }

    impl<P, M, B> Element<ClientState> for CharacterSlotPreview<P, M, B>
    where
        P: Path<ClientState, CharacterInformation, false>,
        M: Path<ClientState, Option<usize>>,
        B: Path<ClientState, Option<usize>>,
    {
        type LayoutInfo = BaseLayoutInfo;

        fn create_layout_info(
            &mut self,
            _: &State<ClientState>,
            store: ElementStoreMut,
            resolvers: &mut dyn Resolvers<ClientState>,
        ) -> Self::LayoutInfo {
            with_single_resolver(resolvers, |resolver| {
                let area = resolver.with_height(180.0);

                self.overlay_handler.set_position_size(
                    ScreenPosition {
                        left: area.left,
                        top: area.top,
                    },
                    ScreenSize {
                        width: area.width,
                        height: area.height,
                    },
                    store.get_window_id(),
                );

                Self::LayoutInfo { area }
            })
        }

        fn lay_out<'a>(
            &'a self,
            state: &'a State<ClientState>,
            _: ElementStore<'a>,
            layout_info: &'a Self::LayoutInfo,
            layout: &mut WindowLayout<'a, ClientState>,
        ) {
            let color_tokens = client_theme().tokens().color();
            let typography = client_theme().tokens().typography();
            let body_font_size = *state.get(&typography.font_body_lg());
            let title_font_size = *state.get(&typography.font_title());
            let accent_color = *state.get(&color_tokens.text_accent());
            let overlay_text_color = *state.get(&color_tokens.text_overlay());

            if let Some(switch_request) = state.get(&self.switch_request) {
                let is_hoverered = layout_info.area.check().run(layout);

                let background_color = match is_hoverered {
                    true => *state.get(&color_tokens.state_active()),
                    false => *state.get(&color_tokens.state_hover()),
                };
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(25.0),
                    background_color,
                    *state.get(&color_tokens.border()),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                if *switch_request == self.slot {
                    layout.add_text(
                        layout_info.area,
                        state.follow(client_state().localization().cancel_button_text()).as_str(),
                        body_font_size,
                        overlay_text_color,
                        accent_color,
                        HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hoverered {
                        layout.register_click_handler(MouseButton::Left, &self.click_handler.cancel_switch);
                    }
                } else {
                    layout.add_text(
                        layout_info.area,
                        state.follow(client_state().localization().switch_slots_text()).as_str(),
                        body_font_size,
                        overlay_text_color,
                        accent_color,
                        HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hoverered {
                        layout.register_click_handler(MouseButton::Left, &self.click_handler.request_switch);
                    }
                }

                return;
            }

            if let Some(character_information) = state.try_get(&self.character_information) {
                let is_hoverered = layout_info.area.check().run(layout);

                let background_color = match is_hoverered {
                    true => *state.get(&color_tokens.surface_highlight()),
                    false => *state.get(&color_tokens.surface_selected()),
                };
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(25.0),
                    background_color,
                    *state.get(&color_tokens.border()),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                layout.add_text(
                    layout_info.area,
                    &character_information.name,
                    title_font_size,
                    *state.get(&color_tokens.text_value()),
                    accent_color,
                    HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                    VerticalAlignment::Top { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    state.follow(client_state().localization().base_level_text()).as_str(),
                    body_font_size,
                    *state.get(&color_tokens.text_secondary()),
                    accent_color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 30.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    self.click_handler
                        .base_level_str
                        .get_str(self.character_information.manually_asserted().base_level(), state),
                    body_font_size,
                    *state.get(&color_tokens.text_secondary()),
                    accent_color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 44.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    state.follow(client_state().localization().job_level_text()).as_str(),
                    body_font_size,
                    *state.get(&color_tokens.text_secondary()),
                    accent_color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 66.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    self.click_handler
                        .job_level_str
                        .get_str(self.character_information.manually_asserted().job_level(), state),
                    body_font_size,
                    *state.get(&color_tokens.text_secondary()),
                    accent_color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 80.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    state.follow(client_state().localization().map_text()).as_str(),
                    body_font_size,
                    *state.get(&color_tokens.text_secondary()),
                    accent_color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 102.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    // TODO: Replace with a map name lookup
                    character_information
                        .map_name
                        .strip_suffix(".gat")
                        .unwrap_or(&character_information.map_name),
                    body_font_size,
                    *state.get(&color_tokens.text_secondary()),
                    accent_color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 116.0 },
                    OverflowBehavior::Shrink,
                );

                if is_hoverered {
                    layout.register_click_handler(MouseButton::Left, &self.click_handler.select_character);
                    layout.register_click_handler(MouseButton::Right, &self.overlay_handler);

                    {
                        struct PrivateTooltipId;
                        layout.add_tooltip(&character_information.name, PrivateTooltipId.tooltip_id());
                    }
                }
            } else {
                let is_hoverered = layout_info.area.check().run(layout);

                let background_color = match is_hoverered {
                    true => *state.get(&color_tokens.state_disabled()),
                    false => *state.get(&color_tokens.surface_raised()),
                };
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(25.0),
                    background_color,
                    *state.get(&color_tokens.border()),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                layout.add_text(
                    layout_info.area,
                    state.follow(client_state().localization().create_character_slot_text()).as_str(),
                    body_font_size,
                    *state.get(&color_tokens.text_disabled()),
                    accent_color,
                    HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );

                if is_hoverered {
                    layout.register_click_handler(MouseButton::Left, &self.click_handler.create_character);
                }
            }
        }
    }

    struct PartialEqDisplayStr<T> {
        last_value: Cell<Option<T>>,
        text: UnsafeCell<String>,
    }

    impl<T> PartialEqDisplayStr<T> {
        pub fn new() -> Self {
            Self {
                last_value: Cell::default(),
                text: UnsafeCell::default(),
            }
        }
    }

    impl<T> PartialEqDisplayStr<T>
    where
        T: Copy + PartialEq + Display + 'static,
    {
        fn get_str<'a, P>(&'a self, path: P, state: &'a State<ClientState>) -> &'a str
        where
            P: Path<ClientState, T>,
        {
            let value = state.get(&path);

            let last_value = self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                unsafe { *self.text.get() = value.to_string() };
                self.last_value.set(Some(*value));
            }

            unsafe { self.text.as_ref_unchecked() }
        }
    }

    struct SelectCharacter {
        slot: usize,
    }

    impl ClickHandler<ClientState> for SelectCharacter {
        fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            queue.queue(InputEvent::SelectCharacter { slot: self.slot });
        }
    }

    struct CreateCharacter {
        slot: usize,
    }

    impl ClickHandler<ClientState> for CreateCharacter {
        fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            queue.queue(InputEvent::OpenCharacterCreationWindow { slot: self.slot });
        }
    }

    struct CancelSwitch<P> {
        switch_request: P,
    }

    impl<P> ClickHandler<ClientState> for CancelSwitch<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
            state.update_value(self.switch_request, None);
        }
    }

    struct RequestSwitch<P> {
        switch_request: P,
        slot: usize,
    }

    impl<P> ClickHandler<ClientState> for RequestSwitch<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            // We should not be able to get here if there is no switch request, so it's
            // fine to unwrap.
            let origin_slot = state.get(&self.switch_request).unwrap();

            queue.queue(InputEvent::SwitchCharacterSlot {
                origin_slot,
                destination_slot: self.slot,
            });
        }
    }

    pub struct CharacterSlotPreviewHandler<P> {
        select_character: SelectCharacter,
        create_character: CreateCharacter,
        cancel_switch: CancelSwitch<P>,
        request_switch: RequestSwitch<P>,
        base_level_str: PartialEqDisplayStr<i16>,
        job_level_str: PartialEqDisplayStr<i32>,
    }

    impl<P> CharacterSlotPreviewHandler<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        pub fn new(switch_request: P, slot: usize) -> Self {
            Self {
                select_character: SelectCharacter { slot },
                create_character: CreateCharacter { slot },
                cancel_switch: CancelSwitch { switch_request },
                request_switch: RequestSwitch { switch_request, slot },
                base_level_str: PartialEqDisplayStr::new(),
                job_level_str: PartialEqDisplayStr::new(),
            }
        }
    }
}

pub struct CharacterSelectionWindow<C, M> {
    character_slots: C,
    switch_request: M,
}

impl<C, M> CharacterSelectionWindow<C, M> {
    pub fn new(characters: C, switch_request: M) -> Self {
        Self {
            character_slots: characters,
            switch_request,
        }
    }
}

impl<C, M> CustomWindow<ClientState> for CharacterSelectionWindow<C, M>
where
    C: Path<ClientState, CharacterSlots>,
    M: Path<ClientState, Option<usize>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterSelection)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        type RowLayoutInfo = (BaseLayoutInfo, BaseLayoutInfo, BaseLayoutInfo, BaseLayoutInfo, BaseLayoutInfo);

        struct CharacterWrapper<C, M> {
            character_slots: C,
            switch_request: M,
            item_boxes: Vec<Box<dyn Element<ClientState, LayoutInfo = RowLayoutInfo>>>,
        }

        impl<C, M> CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            fn new(character_slots: C, switch_request: M) -> Self {
                Self {
                    character_slots,
                    switch_request,
                    item_boxes: Vec::new(),
                }
            }

            fn correct_element_size(&mut self, state: &State<ClientState>) {
                let character_slots = state.get(&self.character_slots);
                let slot_count = character_slots.get_slot_count();

                // FIX: Very broken check
                if self.item_boxes.len() != slot_count / 5 {
                    self.item_boxes.clear();

                    for row in 0..slot_count / 5 {
                        let slot = row * 5;
                        let path = self.character_slots;

                        self.item_boxes.push(Box::new(split! {
                            gaps: 10.0,
                            children: (
                                CharacterSlotPreview::new(
                                    path.in_slot(slot),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot),
                                    OverlayHandler::new(slot, self.switch_request, path.in_slot(slot)),
                                    slot,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 1),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 1),
                                    OverlayHandler::new(slot + 1, self.switch_request, path.in_slot(slot + 1)),
                                    slot + 1,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 2),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 2),
                                    OverlayHandler::new(slot + 2, self.switch_request, path.in_slot(slot + 2)),
                                    slot + 2,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 3),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 3),
                                    OverlayHandler::new(slot + 3, self.switch_request, path.in_slot(slot + 3)),
                                    slot + 3,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 4),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 4),
                                    OverlayHandler::new(slot + 4, self.switch_request, path.in_slot(slot + 4)),
                                    slot + 4,
                                ),
                            )
                        }));
                    }
                }
            }
        }

        impl<C, M> Element<ClientState> for CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            type LayoutInfo = Vec<RowLayoutInfo>;

            fn create_layout_info(
                &mut self,
                state: &State<ClientState>,
                mut store: ElementStoreMut,
                resolvers: &mut dyn Resolvers<ClientState>,
            ) -> Self::LayoutInfo {
                with_single_resolver(resolvers, |resolver| {
                    self.correct_element_size(state);
                    let (_area, layout_info) = resolver.with_derived(10.0, 0.0, |resolver| {
                        self.item_boxes
                            .iter_mut()
                            .enumerate()
                            .map(|(index, item_box)| item_box.create_layout_info(state, store.child_store(index as u64), resolver))
                            .collect()
                    });

                    layout_info
                })
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a State<ClientState>,
                store: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, ClientState>,
            ) {
                layout.with_layer(|layout| {
                    for (index, item_box) in self.item_boxes.iter().enumerate() {
                        item_box.lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
                    }
                });
            }
        }

        window! {
            title: client_state().localization().select_character_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: 900.0,
            maximum_width: 900.0,
            elements: (
                fragment! {
                    gaps: 8.0,
                    children: (
                        CharacterWrapper::new(self.character_slots, self.switch_request),
                        button! {
                            text: client_state().localization().log_out_button_text(),
                            event: InputEvent::LogOutCharacter,
                        },
                    ),
                },
            ),
        }
    }
}
