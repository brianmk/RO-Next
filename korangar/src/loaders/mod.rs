mod action;
mod animation;
mod archive;
mod color;
pub mod diagnostics;
mod rectangle;

mod r#async;
mod effect;
pub mod error;
mod font;
mod gamefile;
mod imf;
mod map;
mod minimap_marker;
mod model;
mod server;
mod smoothing;
mod sprite;
mod texture;
mod video;

pub use self::action::*;
pub use self::animation::*;
pub use self::r#async::*;
pub use self::diagnostics::{
    AssetFailureSeverity, TransientAssetNoticeLimiter, asset_failure_log_key, asset_failure_severity, transient_asset_failure_message,
};
pub use self::effect::EffectLoader;
pub use self::font::{
    FontLoader, FontSize, GLYPH_PADDING, GlyphInstruction, OverflowBehavior, Scaling, TEXT_SHADOW_RADIUS, take_layout_cache_statistics,
};
pub use self::gamefile::*;
pub(crate) use self::imf::Imf;
pub use self::map::{GAT_TILE_SIZE, MapLoader};
pub use self::minimap_marker::{MARKER_DIRECTION_COUNT, generate_direction_markers};
pub use self::model::*;
pub use self::server::{ClientInfo, ClientInfoPathExt, PacketVersion, Service, ServiceId, load_client_info};
pub use self::smoothing::{smooth_ground_normals, smooth_model_normals};
pub use self::sprite::*;
pub use self::texture::{ImageType, TextureLoader, TextureSetBuilder, TextureSetTexture};
pub use self::video::VideoLoader;

pub const FALLBACK_BMP_FILE: &str = "missing.bmp";
pub const FALLBACK_JPEG_FILE: &str = "missing.jpg";
pub const FALLBACK_PNG_FILE: &str = "missing.png";
pub const FALLBACK_TGA_FILE: &str = "missing.tga";
pub const FALLBACK_MODEL_FILE: &str = "missing.rsm";
pub const FALLBACK_SPRITE_FILE: &str = "npc\\missing.spr";
pub const FALLBACK_ACTIONS_FILE: &str = "npc\\missing.act";

/// Some assets in the kRO game files are prefixed with an ASCII marker before
/// the actual payload starts. The official client recognizes these markers and
/// skips them, but they are not part of the underlying file format. We
/// therefore have to strip them before handing the data to a decoder, otherwise
/// the files can't be parsed at all.
///
/// The two markers we know about are:
/// * `11111111` (followed by a newline) in front of effect textures, i.e. most
///   `.bmp` and `.tga` files inside `data\texture\effect`.
/// * `22222222` (followed by a newline) in front of `.str` effect definition
///   files.
///
/// Both markers are eight bytes long, so a file that starts with the marker is
/// always at least that long.
pub fn strip_texture_magic_prefix(bytes: &mut Vec<u8>) {
    strip_magic_prefix(bytes, b"11111111");
}

/// Strips the `22222222` marker in front of `.str` effect files. See
/// [`strip_texture_magic_prefix`].
pub fn strip_effect_magic_prefix(bytes: &mut Vec<u8>) {
    strip_magic_prefix(bytes, b"22222222");
}

fn strip_magic_prefix(bytes: &mut Vec<u8>, marker: &[u8; 8]) {
    if !bytes.starts_with(marker) {
        return;
    }

    bytes.drain(..marker.len());

    if bytes.starts_with(b"\r\n") {
        bytes.drain(..2);
    } else if bytes.starts_with(b"\n") {
        bytes.drain(..1);
    }
}

#[cfg(test)]
mod tests {
    use super::{strip_effect_magic_prefix, strip_texture_magic_prefix};

    fn data(payload: &[u8]) -> Vec<u8> {
        payload.to_vec()
    }

    #[test]
    fn strips_texture_prefix() {
        let mut bytes = data(b"11111111\nBMpayload");
        strip_texture_magic_prefix(&mut bytes);
        assert_eq!(bytes, b"BMpayload");
    }

    #[test]
    fn strips_texture_prefix_with_crlf() {
        let mut bytes = data(b"11111111\r\nTGApayload");
        strip_texture_magic_prefix(&mut bytes);
        assert_eq!(bytes, b"TGApayload");
    }

    #[test]
    fn strips_texture_prefix_without_newline() {
        let mut bytes = data(b"11111111BMpayload");
        strip_texture_magic_prefix(&mut bytes);
        assert_eq!(bytes, b"BMpayload");
    }

    #[test]
    fn strips_effect_prefix() {
        let mut bytes = data(b"22222222\nSTRMpayload");
        strip_effect_magic_prefix(&mut bytes);
        assert_eq!(bytes, b"STRMpayload");
    }

    #[test]
    fn leaves_regular_payload_alone() {
        let payload = b"BMregular bitmap without marker";
        let mut bytes = data(payload);
        strip_texture_magic_prefix(&mut bytes);
        strip_effect_magic_prefix(&mut bytes);
        assert_eq!(bytes, payload);
    }

    #[test]
    fn does_not_panic_on_partial_or_empty_marker() {
        for candidate in [&b""[..], &b"111"[..], &b"1111111"[..], &b"11111111"[..], &b"2222222"[..]] {
            let original = candidate.to_vec();
            let mut bytes = candidate.to_vec();
            strip_texture_magic_prefix(&mut bytes);
            strip_effect_magic_prefix(&mut bytes);

            if original == b"11111111" {
                // A bare marker is fully stripped.
                assert!(bytes.is_empty());
            } else {
                assert_eq!(bytes, original);
            }
        }
    }
}
