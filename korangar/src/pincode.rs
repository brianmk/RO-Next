//! Client side of the 2019 character-server pincode system.
//!
//! The server sends `HC_ACK_PINCODE` (0x08B9) with a random `pincode_seed`
//! and a state (rAthena `pincode_state`, `char.hpp:113-131`). The client
//! answers with `CZ_PINCODE_CHECK` (0x08B8) or `CZ_PINCODE_SET_NEW` (0x08BA).
//!
//! The four PIN digits are **not** sent in plain text: the server runs
//! `char_pincode_decrypt` (`char.cpp:2103-2143`), which builds a permutation
//! of `0..9` from the seed and maps every transmitted digit through it. This
//! module builds the same permutation and sends the inverse, so the server
//! recovers the digits the player typed.
//!
//! CondaRO runs with `pincode_enabled: no`, so `chlogif_pincode_start`
//! (`char_logif.cpp:52-85`) sends state `0` (passed) and no prompt is shown;
//! the code still works for any server that asks.

/// Number of digits of a pincode (`PINCODE_LENGTH`, `mmo.hpp:167`).
pub const PINCODE_LENGTH: usize = 4;

/// Obfuscate the four digits of a pincode for the given seed.
///
/// `digits` must contain exactly four ASCII digits. The returned bytes are the
/// ones the server expects in `CZ_PINCODE_CHECK` / `CZ_PINCODE_SET_NEW`.
pub fn encrypt_pincode(seed: u32, digits: &str) -> Option<[u8; PINCODE_LENGTH]> {
    let digits = digits.as_bytes();

    if digits.len() != PINCODE_LENGTH || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let table = pincode_permutation(seed);
    let mut encoded = [b'0'; PINCODE_LENGTH];

    for (encoded_digit, digit) in encoded.iter_mut().zip(digits) {
        let target = digit - b'0';
        let position = table.iter().position(|entry| *entry == target)?;
        *encoded_digit = b'0' + position as u8;
    }

    Some(encoded)
}

/// Reverse of [`encrypt_pincode`]; mirrors the server's
/// `char_pincode_decrypt` and is used by the tests.
#[cfg_attr(not(test), allow(dead_code))]
pub fn decrypt_pincode(seed: u32, encoded: &[u8; PINCODE_LENGTH]) -> Option<[u8; PINCODE_LENGTH]> {
    if !encoded.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let table = pincode_permutation(seed);
    let mut digits = [b'0'; PINCODE_LENGTH];

    for (digit, encoded_digit) in digits.iter_mut().zip(encoded) {
        let position = usize::from(encoded_digit - b'0');
        let value = *table.get(position)?;
        *digit = b'0' + value;
    }

    Some(digits)
}

/// Build the same `tab[10]` permutation as `char_pincode_decrypt`
/// (`char.cpp:2103-2131`).
fn pincode_permutation(seed: u32) -> [u8; 10] {
    let mut table = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut user_seed = seed;

    for index in 1..10usize {
        user_seed = 0x881234u32.wrapping_add(user_seed.wrapping_mul(0x3498));
        let position = (user_seed % (index as u32 + 1)) as usize;

        if index != position {
            table.swap(index, position);
        }
    }

    table
}

#[cfg(test)]
mod tests {
    use super::{decrypt_pincode, encrypt_pincode, pincode_permutation};

    #[test]
    fn permutation_is_a_real_permutation() {
        for seed in [0u32, 1, 7, 0x1234, 0xFFFF, 0xDEAD_BEEF] {
            let mut table = pincode_permutation(seed);
            table.sort_unstable();
            assert_eq!(table, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], "seed {seed:#X}");
        }
    }

    #[test]
    fn encryption_round_trips_through_the_server_decryptor() {
        for seed in [0u32, 1, 2, 7, 42, 0x1234, 0xFFFF, 0xDEAD_BEEF] {
            for code in ["0000", "1234", "9876", "2468", "1357", "5555"] {
                let encoded = encrypt_pincode(seed, code).unwrap();
                let decoded = decrypt_pincode(seed, &encoded).unwrap();
                assert_eq!(std::str::from_utf8(&decoded).unwrap(), code, "seed {seed:#X} code {code}");
            }
        }
    }

    #[test]
    fn known_seed_matches_the_server_permutation() {
        // Independent implementation of rAthena `char_pincode_decrypt`
        // (char.cpp:2103-2143) with seed 0x1234 produces the permutation
        // [3, 8, 6, 2, 7, 1, 5, 0, 9, 4], so sending "5309" decodes to "1234".
        assert_eq!(encrypt_pincode(0x1234, "1234"), Some(*b"5309"));
        assert_eq!(decrypt_pincode(0x1234, b"5309"), Some(*b"1234"));
    }

    #[test]
    fn non_digit_or_wrong_length_codes_are_rejected() {
        assert_eq!(encrypt_pincode(1, "123"), None);
        assert_eq!(encrypt_pincode(1, "12345"), None);
        assert_eq!(encrypt_pincode(1, "12a4"), None);
        assert_eq!(encrypt_pincode(1, " 123"), None);
    }
}
