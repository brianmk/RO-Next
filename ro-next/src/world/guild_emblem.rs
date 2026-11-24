//! Guild emblem support.
//!
//! Classic Ragnarok Online clients request a guild's 24x24 emblem image from
//! the map server (`CZ_REQ_GUILD_EMBLEM_IMG`, `0x0151`) and receive a BMP file
//! (`ZC_GUILD_EMBLEM_IMG`, `0x0152`). This module decodes that BMP (including
//! the classic magenta `#FF00FF` transparency marker) and caches the resulting
//! texture per guild id.

use std::io::Read;
use std::sync::Arc;

use flate2::read::ZlibDecoder;
use hashbrown::{HashMap, HashSet};
use image::{Rgba, RgbaImage};

use crate::graphics::Texture;

/// Color that the classic client treats as transparent in guild emblems.
const TRANSPARENT_COLOR: [u8; 3] = [0xFF, 0x00, 0xFF];

/// A loaded guild emblem texture.
pub struct GuildEmblem {
    /// Decoded 24x24 emblem image.
    pub texture: Arc<Texture>,
}

/// Cache of guild emblems, keyed by guild id.
///
/// Requests are rate-limited: each guild is requested at most once per map
/// session. Emblems survive guild name updates (the guild id does not change),
/// the cache is cleared when the map connection is dropped.
#[derive(Default)]
pub struct GuildEmblemCache {
    emblems: HashMap<u32, GuildEmblem>,
    requested: HashSet<u32>,
}

impl GuildEmblemCache {
    pub fn emblem(&self, guild_id: u32) -> Option<&GuildEmblem> {
        self.emblems.get(&guild_id)
    }

    /// Returns `true` if an emblem request should be sent for the guild id.
    ///
    /// Each guild is only requested once, even if the request fails or the
    /// guild has no emblem. A new `MapServerConnected`/reconnect clears the
    /// cache and therefore also the rate limit.
    pub fn request_if_needed(&mut self, guild_id: u32) -> bool {
        if self.emblems.contains_key(&guild_id) {
            return false;
        }

        self.requested.insert(guild_id)
    }

    pub fn insert(&mut self, guild_id: u32, emblem: GuildEmblem) {
        self.emblems.insert(guild_id, emblem);
    }

    pub fn clear(&mut self) {
        self.emblems.clear();
        self.requested.clear();
    }
}

/// Decodes a guild emblem image (classic 24x24 BMP, 8-bit palette or 24-bit).
///
/// Some servers/proxies send the BMP zlib-compressed, so that is tried as a
/// fallback. The classic magenta transparency marker is converted to alpha 0.
pub fn decode_emblem(data: &[u8]) -> Result<RgbaImage, String> {
    if data.is_empty() {
        return Err("empty guild emblem data".to_string());
    }

    let image = match decode_bmp(data) {
        Ok(image) => image,
        Err(error) => {
            let mut decompressed = Vec::new();
            ZlibDecoder::new(data).read_to_end(&mut decompressed).map_err(|_| error)?;
            decode_bmp(&decompressed)?
        }
    };

    Ok(make_transparency_transparent(image))
}

fn decode_bmp(data: &[u8]) -> Result<RgbaImage, String> {
    image::load_from_memory_with_format(data, image::ImageFormat::Bmp)
        .map(|image| image.to_rgba8())
        .map_err(|error| format!("invalid guild emblem BMP: {error}"))
}

fn make_transparency_transparent(mut image: RgbaImage) -> RgbaImage {
    for pixel in image.pixels_mut() {
        let [red, green, blue, _alpha] = pixel.0;

        if [red, green, blue] == TRANSPARENT_COLOR {
            *pixel = Rgba([TRANSPARENT_COLOR[0], TRANSPARENT_COLOR[1], TRANSPARENT_COLOR[2], 0]);
        }
    }

    image
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{GuildEmblemCache, decode_emblem, make_transparency_transparent};

    /// The red "Domination" guild emblem committed server-side
    /// (`sql-files/population_engine/llm_guild_emblem_red.sql`).
    const RED_EMBLEM_BMP: &[u8] = include_bytes!("test_data/guild_emblem_red.bmp");

    #[test]
    fn decodes_red_guild_emblem_bmp() {
        let image = decode_emblem(RED_EMBLEM_BMP).expect("failed to decode the red emblem BMP");

        assert_eq!(image.dimensions(), (24, 24), "guild emblems are 24x24");

        // The committed Domination emblem is a red field with a dark red border.
        let center = image.get_pixel(12, 12);
        assert!(
            center[0] > 150 && center[1] < 80 && center[2] < 80,
            "expected a red pixel, got {center:?}"
        );

        let border = image.get_pixel(0, 0);
        assert!(
            border[0] > 80 && border[0] < center[0] && border[1] == 0,
            "expected a dark red border, got {border:?}"
        );
    }

    #[test]
    fn decodes_zlib_compressed_guild_emblem_bmp() {
        use std::io::Write;

        use flate2::Compression;
        use flate2::write::ZlibEncoder;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(RED_EMBLEM_BMP).unwrap();
        let compressed = encoder.finish().unwrap();

        let image = decode_emblem(&compressed).expect("failed to decode the zlib compressed emblem");
        assert_eq!(image.dimensions(), (24, 24));
    }

    #[test]
    fn rejects_empty_and_garbage_emblem_data() {
        assert!(decode_emblem(&[]).is_err());
        assert!(decode_emblem(b"not an emblem").is_err());
    }

    #[test]
    fn converts_the_classic_magenta_marker_to_transparency() {
        let mut image = RgbaImage::new(2, 1);
        image.put_pixel(0, 0, Rgba([0xFF, 0x00, 0xFF, 0xFF]));
        image.put_pixel(1, 0, Rgba([0xDC, 0x14, 0x14, 0xFF]));

        let image = make_transparency_transparent(image);

        assert_eq!(image.get_pixel(0, 0)[3], 0, "magenta must become transparent");
        assert_eq!(image.get_pixel(1, 0), &Rgba([0xDC, 0x14, 0x14, 0xFF]), "red must stay opaque");
    }

    #[test]
    fn requests_each_guild_emblem_only_once() {
        let mut cache = GuildEmblemCache::default();

        assert!(cache.request_if_needed(1));
        assert!(!cache.request_if_needed(1), "the same guild must not be requested twice");
        assert!(cache.request_if_needed(2));

        cache.clear();
        assert!(cache.request_if_needed(1), "a new map session may request the emblem again");
    }
}
