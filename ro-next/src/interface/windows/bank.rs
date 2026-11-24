//! Bank window for the rAthena banking system.
//!
//! The window is opened either from the in-game menu or by an NPC through
//! `ZC_UI_OPEN` (0x0AE2, `OUT_UI_BANK`). Opening sends `CZ_REQ_OPEN_BANKING`
//! (0x09B6) and `CZ_REQ_BANKING_CHECK` (0x09AB); the server answers with
//! `ZC_ACK_OPEN_BANKING` (0x09B7) and `ZC_BANKING_CHECK` (0x09A6).
//! Deposits and withdrawals use `CZ_REQ_BANKING_DEPOSIT` (0x09A7) and
//! `CZ_REQ_BANKING_WITHDRAW` (0x09A9); the transaction acknowledgements carry
//! the resulting vault balance and carried zeny.

use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::prelude::ComputedSelector;
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{PathExt, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::bank::BankPathExt;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

struct DepositFocus;
struct WithdrawFocus;

fn noop_input(_: &State<ClientState>, _: &mut ro_next_interface::event::EventQueue<ClientState>) {}

/// Balance line: the vault and the zeny the character is carrying.
fn balance_text() -> impl Selector<ClientState, String> {
    ComputedSelector::new_default(|state: &ClientState| {
        let bank = client_state().bank().follow_safe(state);
        let carried_zeny = (*client_state().zeny().follow_safe(state)).unwrap_or(0);

        format!("Bank vault: {} zeny   Carried: {} zeny", bank.bank_vault(), carried_zeny)
    })
}

fn status_text() -> impl Selector<ClientState, String> {
    ComputedSelector::new_default(|state: &ClientState| client_state().bank().status_message().follow_safe(state).clone())
}

pub struct BankWindow;

impl CustomWindow<ClientState> for BankWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Bank)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: "Bank",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 380.0,
            elements: (
                text! {
                    text: balance_text(),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: status_text(),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: "The bank vault is shared by every character on the account. rAthena only allows this when the server enables feature.banking and the map is not marked nobank.",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text! {
                            text: "Deposit",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "0",
                            state: client_state().bank().deposit_input(),
                            input_handler: DefaultHandler::<_, _, 20>::new(client_state().bank().deposit_input(), noop_input),
                            focus_id: DepositFocus,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "Deposit zeny",
                            height: 20.0,
                            event: InputEvent::BankDeposit,
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text! {
                            text: "Withdraw",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "0",
                            state: client_state().bank().withdraw_input(),
                            input_handler: DefaultHandler::<_, _, 20>::new(client_state().bank().withdraw_input(), noop_input),
                            focus_id: WithdrawFocus,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "Withdraw zeny",
                            height: 20.0,
                            event: InputEvent::BankWithdraw,
                        },
                    ),
                },
                button! {
                    text: client_state().localization().close_button_text(),
                    event: InputEvent::CloseBank,
                },
            ),
        }
    }
}
