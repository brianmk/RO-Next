//! Protocol name-length constants.
//!
//! These are **wire contract** values, not UI preferences: they have to match
//! the fixed-length fields of the Ragnarok Online packet layouts (rAthena
//! `common/pc.hpp` / `char.hpp`). They used to be duplicated as magic numbers
//! in the interface windows, `ro-next/src/lib.rs` and the networking layer;
//! this module is the single documented home.
//!
//! | constant | value | wire shape |
//! | --- | --- | --- |
//! | [`NAME_LENGTH`] | 24 | every fixed `name[24]` field (`#[length(24)]` strings and `[u8; 24]` arrays: character, party, friend and account names) |
//! | [`MIN_NAME_LENGTH`] | 4 | minimum character name length enforced by the character server |
//! | [`PASSWORD_LENGTH`] | 24 | password string of [`LoginServerLoginPacket`] (0x0064), `#[length(24)]` |
//! | [`USER_ID_LENGTH`] | 24 | 24-byte login user id ([`UserId`]) |
//!
//! Do not change these to "improve" the client: a different value changes the
//! packet layout and desynchronizes the connection. The values only live here
//! so the windows, the client loop and the networking layer cannot drift
//! apart.
//!
//! [`LoginServerLoginPacket`]: crate::LoginServerLoginPacket
//! [`UserId`]: crate::UserId

/// Length of the fixed `name[24]` wire field.
///
/// This is the length of character, party, friend and account names in the
/// packet layouts (`#[length(24)]` strings and `[u8; 24]` arrays).
pub const NAME_LENGTH: usize = 24;

/// Minimum character name length accepted by the (rAthena) character server.
///
/// The character creation and friend list windows use it for client-side
/// validation; the server is still the final authority.
pub const MIN_NAME_LENGTH: usize = 4;

/// Length of the fixed 24-byte password field of the login packet.
pub const PASSWORD_LENGTH: usize = 24;

/// Length of the fixed 24-byte login user id field.
pub const USER_ID_LENGTH: usize = 24;

#[cfg(test)]
mod tests {
    use super::{MIN_NAME_LENGTH, NAME_LENGTH, PASSWORD_LENGTH, USER_ID_LENGTH};

    #[test]
    fn protocol_name_lengths_match_the_wire_layout() {
        // These values are part of the packet contract; they must not be
        // "cleaned up" without a matching PACKETVER change.
        assert_eq!(NAME_LENGTH, 24);
        assert_eq!(MIN_NAME_LENGTH, 4);
        assert_eq!(PASSWORD_LENGTH, 24);
        assert_eq!(USER_ID_LENGTH, 24);
    }

    #[test]
    fn name_lengths_are_consistent() {
        assert!(MIN_NAME_LENGTH < NAME_LENGTH);
        assert_eq!(
            PASSWORD_LENGTH, NAME_LENGTH,
            "both are fixed 24-byte strings in the login packet"
        );
    }
}
