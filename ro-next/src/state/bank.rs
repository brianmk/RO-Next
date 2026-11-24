//! Bank window state for the rAthena banking system.
//!
//! The server exposes the bank through the `CZ_REQ_BANKING_*` request family
//! and answers with `ZC_ACK_OPEN_BANKING` / `ZC_BANKING_CHECK` /
//! `ZC_ACK_BANKING_DEPOSIT` / `ZC_ACK_BANKING_WITHDRAW`
//! (`src/map/clif.cpp:7327-7535` on the PACKETVER 20190605 server). The vault
//! itself is account-wide and only exists when `feature.banking` is enabled and
//! the map does not carry the `nobank` mapflag.
//!
//! The client keeps the last vault balance and the two text fields here so the
//! window can be reopened (or opened by an NPC through `ZC_UI_OPEN`) without
//! losing the values that were already typed.

use ro_next_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

/// Maximum amount of zeny that can be held in the bank (`MAX_BANK_ZENY`,
/// `src/common/mmo.hpp:83`).
const MAX_BANK_ZENY: u64 = i32::MAX as u64;

/// Parses a zeny amount typed into one of the bank text boxes.
///
/// Accepts plain digits and the `1,000,000` grouping that players commonly
/// type. A zero or unparsable amount is rejected so the window can keep the
/// field untouched instead of sending a request the server always refuses
/// (`pc_bank_deposit` rejects `money <= 0`, `pc.cpp:17085`).
pub fn parse_zeny_amount(input: &str) -> Option<u32> {
    let cleaned: String = input.chars().filter(|character| !matches!(character, ',' | '_' | ' ')).collect();
    let value = cleaned.parse::<u64>().ok()?;

    if value == 0 {
        return None;
    }

    Some(value.min(MAX_BANK_ZENY) as u32)
}

/// Human readable status for an `e_BANKING_DEPOSIT_ACK` value
/// (`src/map/clif.hpp:93`).
pub fn deposit_result_message(reason: u16) -> &'static str {
    match reason {
        0 => "Deposit complete.",
        1 => "The deposit failed.",
        2 => "You do not have enough zeny to deposit that much.",
        3 => "The bank vault would overflow.",
        _ => "The deposit failed.",
    }
}

/// Human readable status for an `e_BANKING_WITHDRAW_ACK` value
/// (`src/map/clif.hpp:100`).
pub fn withdraw_result_message(reason: u16) -> &'static str {
    match reason {
        0 => "Withdrawal complete.",
        1 => "The bank does not hold that much zeny.",
        2 => "The withdrawal failed.",
        _ => "The withdrawal failed.",
    }
}

/// State of the bank window.
#[derive(Clone, Default, PartialEq, RustState, StateElement, Serialize, Deserialize)]
pub struct Bank {
    #[hidden_element]
    active: bool,
    /// Last known bank vault amount (`ZC_BANKING_CHECK` / the transaction
    /// acks).
    #[hidden_element]
    bank_vault: u64,
    #[hidden_element]
    deposit_input: String,
    #[hidden_element]
    withdraw_input: String,
    #[hidden_element]
    status_message: String,
}

impl Bank {
    /// Mark the window as open and show a neutral waiting message. The balance
    /// and the typed amounts are kept, only the status line is reset.
    pub fn open(&mut self) {
        self.active = true;
        self.status_message = "Waiting for the server...".to_owned();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.status_message.clear();
    }

    pub fn bank_vault(&self) -> u64 {
        self.bank_vault
    }

    pub fn deposit_input(&self) -> &str {
        &self.deposit_input
    }

    pub fn withdraw_input(&self) -> &str {
        &self.withdraw_input
    }

    pub fn set_balance(&mut self, bank_vault: u64) {
        self.bank_vault = bank_vault;
        self.active = true;
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }

    /// Applies a deposit acknowledgement. Returns the message that should be
    /// shown in the window.
    pub fn apply_deposit(&mut self, reason: u16, bank_vault: u64) -> &'static str {
        self.bank_vault = bank_vault;
        self.active = true;
        let message = deposit_result_message(reason);
        self.status_message = message.to_owned();

        if reason == 0 {
            self.deposit_input.clear();
        }

        message
    }

    /// Applies a withdrawal acknowledgement.
    pub fn apply_withdraw(&mut self, reason: u16, bank_vault: u64) -> &'static str {
        self.bank_vault = bank_vault;
        self.active = true;
        let message = withdraw_result_message(reason);
        self.status_message = message.to_owned();

        if reason == 0 {
            self.withdraw_input.clear();
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::{Bank, deposit_result_message, parse_zeny_amount, withdraw_result_message};

    #[test]
    fn zeny_input_accepts_grouping_and_rejects_empty_values() {
        assert_eq!(parse_zeny_amount("1000"), Some(1000));
        assert_eq!(parse_zeny_amount(" 1,000,000 "), Some(1_000_000));
        assert_eq!(parse_zeny_amount("0"), None);
        assert_eq!(parse_zeny_amount(""), None);
        assert_eq!(parse_zeny_amount("abc"), None);
        assert_eq!(parse_zeny_amount("99999999999"), Some(i32::MAX as u32));
    }

    #[test]
    fn deposit_and_withdraw_messages_cover_the_server_enums() {
        assert_eq!(deposit_result_message(0), "Deposit complete.");
        assert_eq!(deposit_result_message(2), "You do not have enough zeny to deposit that much.");
        assert_eq!(deposit_result_message(3), "The bank vault would overflow.");
        assert_eq!(withdraw_result_message(0), "Withdrawal complete.");
        assert_eq!(withdraw_result_message(1), "The bank does not hold that much zeny.");
        assert_eq!(withdraw_result_message(99), "The withdrawal failed.");
    }

    #[test]
    fn balance_updates_and_successful_inputs_are_cleared() {
        let mut bank = Bank::default();

        bank.set_balance(5_000);
        assert_eq!(bank.bank_vault(), 5_000);

        bank.deposit_input = "100".to_owned();
        assert_eq!(bank.apply_deposit(0, 5_100), "Deposit complete.");
        assert_eq!(bank.bank_vault(), 5_100);
        assert!(bank.deposit_input().is_empty());

        bank.withdraw_input = "100".to_owned();
        assert_eq!(bank.apply_withdraw(1, 5_100), "The bank does not hold that much zeny.");
        assert_eq!(bank.bank_vault(), 5_100);
        // A failed request keeps the typed amount for a retry.
        assert_eq!(bank.withdraw_input(), "100");
    }
}
