//! Helpers to display zeny amounts and a small zeny element.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, State};

use crate::loaders::OverflowBehavior;
use crate::state::theme::{DesignTokensPathExt, InterfaceThemePathExt, ItemCardThemePathExt, TypographyTokensPathExt};
use crate::state::{ClientState, client_theme};

/// Formats a zeny amount with thousands separators, e.g. `1,234 z`.
pub fn format_zeny(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3 + 2);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }

    formatted.push_str(" z");
    formatted
}

/// Element that shows the amount of zeny the character has.
///
/// The rAthena 2019 server never sends the *initial* zeny after login: it only
/// pushes `ZC_LONGPAR_CHANGE` when the value changes (`clif_updatestatus` is
/// called from `pc_getzeny`/`pc_payzeny`, not from `clif_parse_LoadEndAck`).
/// The official client initializes its zeny to `0` for that reason, so the
/// element does the same instead of showing a placeholder forever.
pub struct ZenyDisplay<A> {
    zeny_path: A,
    text: String,
}

impl<A> ZenyDisplay<A> {
    pub fn new(zeny_path: A) -> Self {
        Self {
            zeny_path,
            text: String::new(),
        }
    }
}

impl<A> Element<ClientState> for ZenyDisplay<A>
where
    A: Path<ClientState, Option<u64>>,
{
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let zeny = *state.follow(self.zeny_path);

        // `None` means "the server has not sent a zeny update yet"; the
        // official client starts at 0 until the first `ZC_LONGPAR_CHANGE`.
        self.text = format_zeny(zeny.unwrap_or(0));

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

        layout.add_text(
            *layout_info,
            &self.text,
            *state.get(&client_theme().tokens().typography().font_body_lg()),
            *state.get(&item_card_theme.text_color()),
            *state.get(&item_card_theme.highlight_color()),
            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}
