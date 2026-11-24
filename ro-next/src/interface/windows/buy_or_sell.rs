use ro_next_interface::window::{CustomWindow, Window};
use ragnarok_packets::{BuyOrSellOption, ShopId};

use super::WindowClass;
use crate::input::InputEvent;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct BuyOrSellWindow {
    shop_id: ShopId,
}

impl BuyOrSellWindow {
    pub fn new(shop_id: ShopId) -> Self {
        Self { shop_id }
    }
}

impl CustomWindow<ClientState> for BuyOrSellWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BuyOrSell)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: client_state().localization().buy_or_sell_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                button! {
                    text: client_state().localization().buy_button_text(),
                    event: InputEvent::BuyOrSell {
                        shop_id: self.shop_id,
                        buy_or_sell: BuyOrSellOption::Buy,
                    },
                },
                button! {
                    text: client_state().localization().sell_button_text(),
                    event: InputEvent::BuyOrSell {
                        shop_id: self.shop_id,
                        buy_or_sell: BuyOrSellOption::Sell,
                    },
                },
            ),
        }
    }
}
