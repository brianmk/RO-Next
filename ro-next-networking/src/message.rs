#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum MessageColor {
    Rgb { red: u8, green: u8, blue: u8 },
    Broadcast,
    Server,
    Error,
    Information,
}
