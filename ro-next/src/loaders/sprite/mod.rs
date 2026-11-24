use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use image::RgbaImage;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::sprite::{Palette, PaletteColor, RgbaImageData, SpriteData};
use ragnarok_formats::version::GenericFormatMetadata;
#[cfg(feature = "debug")]
use ro_next_container::CacheStatistics;
use ro_next_container::{Cacheable, SimpleCache};
#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, Timer, print_debug};
use ro_next_interface::element::StateElement;
use ro_next_loaders::FileLoader;
use rust_state::RustState;

use super::{FALLBACK_SPRITE_FILE, TextureLoader};
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;
use crate::loaders::color::premultiply_alpha;
use crate::loaders::error::LoadError;

const MAX_CACHE_COUNT: u32 = 4096;
const MAX_CACHE_SIZE: usize = 256 << 20;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct Sprite {
    pub palette_size: usize,
    #[hidden_element]
    pub textures: Vec<Arc<Texture>>,
    #[cfg(feature = "debug")]
    sprite_data: SpriteData,
}

impl Cacheable for Sprite {
    fn size(&self) -> usize {
        self.textures.iter().map(|t| t.get_byte_size()).sum()
    }
}

pub struct SpriteLoader {
    game_file_loader: Arc<GameFileLoader>,
    texture_loader: Arc<TextureLoader>,
    cache: Mutex<SimpleCache<String, Arc<Sprite>>>,
}

impl SpriteLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>, texture_loader: Arc<TextureLoader>) -> Self {
        Self {
            game_file_loader,
            texture_loader,
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    #[cfg(feature = "debug")]
    pub fn cache_statistics(&self) -> CacheStatistics {
        self.cache.lock().unwrap().statistics()
    }

    /// Tests if a sprite file (path relative to `data\sprite\`, including the
    /// extension) exists in the game archives.
    pub fn file_exists(&self, path: &str) -> bool {
        self.game_file_loader.file_exists(&format!("data\\sprite\\{path}").to_lowercase())
    }

    /// Builds the cache key for a sprite. The same sprite file can be rendered
    /// with different external palettes (hair and body colors), so the palette
    /// has to be part of the key.
    fn cache_key(path: &str, palette_path: Option<&str>) -> String {
        match palette_path {
            Some(palette_path) => format!("{path}\u{1}{palette_path}"),
            None => path.to_owned(),
        }
    }

    /// Loads a `.pal` palette file (256 RGBA entries) that replaces the palette
    /// embedded in a sprite. Returns `None` if the file is missing or
    /// malformed; the caller then keeps the embedded palette.
    fn load_palette(&self, path: &str) -> Option<Palette> {
        let bytes = match self.game_file_loader.get(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                crate::logging::warn_once(
                    module_path!(),
                    path,
                    format_args!("failed to load sprite palette `{path}`: {error:?}; keeping the embedded palette"),
                );

                return None;
            }
        };

        let palette = Palette::from_bytes(&mut ByteReader::without_metadata(&bytes));

        if let Err(error) = &palette {
            crate::logging::warn_once(
                module_path!(),
                path,
                format_args!("malformed sprite palette `{path}`: {error:?}; keeping the embedded palette"),
            );
        }

        palette.ok()
    }

    pub fn load(&self, path: &str, palette_path: Option<&str>) -> Result<Arc<Sprite>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}", path.magenta()));

        let bytes = match self.game_file_loader.get(&format!("data\\sprite\\{path}")) {
            Ok(bytes) => bytes,
            Err(error) => {
                // Never recurse into the fallback if the fallback itself is
                // missing: that used to overflow the stack.
                if path == FALLBACK_SPRITE_FILE {
                    return Err(LoadError::File(error));
                }

                crate::logging::warn_once(
                    module_path!(),
                    path,
                    format_args!("failed to load sprite `data\\sprite\\{path}`: {error:?}; using the fallback sprite"),
                );

                return self.get_or_load(FALLBACK_SPRITE_FILE, None);
            }
        };
        let mut byte_reader: ByteReader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);

        let sprite_data = match SpriteData::from_bytes(&mut byte_reader) {
            Ok(sprite_data) => sprite_data,
            Err(error) => {
                if path == FALLBACK_SPRITE_FILE {
                    return Err(LoadError::Conversion(error));
                }

                crate::logging::warn_once(
                    module_path!(),
                    path,
                    format_args!("malformed sprite `data\\sprite\\{path}`: {error:?}; using the fallback sprite"),
                );

                return self.get_or_load(FALLBACK_SPRITE_FILE, None);
            }
        };

        #[cfg(feature = "debug")]
        let cloned_sprite_data = sprite_data.clone();

        // Sprites normally embed their palette, but some files do not. Fall back
        // to the default palette instead of panicking while decoding the palette
        // based image data.
        let embedded_palette = sprite_data.palette.clone().unwrap_or_else(|| {
            crate::logging::warn_once(
                module_path!(),
                path,
                format_args!("sprite `data\\sprite\\{path}` has no embedded palette; using the default palette"),
            );

            Palette::default()
        });

        // An explicit palette file (hair/body color) replaces the embedded one.
        let palette = match palette_path {
            Some(palette_path) => self.load_palette(palette_path).unwrap_or(embedded_palette),
            None => embedded_palette,
        };

        let rgba_images: Vec<RgbaImageData> = sprite_data
            .rgba_image_data
            .iter()
            .map(|image_data| {
                // Revert the rows, the image is flipped upside down
                // Convert the pixel from ABGR format to RGBA format
                let width = image_data.width;
                let data = image_data
                    .data
                    .chunks_exact(4 * width as usize)
                    .rev()
                    .flat_map(|pixels| {
                        pixels
                            .chunks_exact(4)
                            .flat_map(|pixel| [pixel[3], pixel[2], pixel[1], pixel[0]])
                            .collect::<Vec<u8>>()
                    })
                    .collect();

                RgbaImageData {
                    width: image_data.width,
                    height: image_data.height,
                    data,
                }
            })
            .collect();

        // TODO: Move this to an extension trait in `ro_next_loaders`.
        pub fn color_bytes(palette: &PaletteColor, index: u8) -> [u8; 4] {
            let alpha = match index {
                0 => 0,
                _ => 255,
            };

            [palette.red, palette.green, palette.blue, alpha]
        }

        let palette_images = sprite_data.palette_image_data.iter().map(|image_data| {
            // Decode palette image data if necessary
            let data: Vec<u8> = image_data
                .data
                .0
                .iter()
                .flat_map(|palette_index| color_bytes(&palette.colors[*palette_index as usize], *palette_index))
                .collect();

            RgbaImageData {
                width: image_data.width,
                height: image_data.height,
                data,
            }
        });
        let palette_size = palette_images.len();

        let textures = palette_images
            .chain(rgba_images)
            .map(|mut image_data| {
                premultiply_alpha(&mut image_data.data);

                let image = RgbaImage::from_raw(image_data.width as u32, image_data.height as u32, image_data.data)
                    .unwrap_or_else(|| RgbaImage::new(1, 1));

                self.texture_loader.create_color(path, image, false)
            })
            .collect();

        let sprite = Arc::new(Sprite {
            palette_size,
            textures,
            #[cfg(feature = "debug")]
            sprite_data: cloned_sprite_data,
        });

        let cache_key = Self::cache_key(path, palette_path);
        let _result = self.cache.lock().unwrap().insert(cache_key, sprite.clone());

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] sprite could not be added to cache. Path: '{}': {:?}",
                "error".red(),
                &path,
                error
            );
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&self, path: &str, palette_path: Option<&str>) -> Option<Arc<Sprite>> {
        self.cache.lock().unwrap().get(&Self::cache_key(path, palette_path)).cloned()
    }

    pub fn get_or_load(&self, path: &str, palette_path: Option<&str>) -> Result<Arc<Sprite>, LoadError> {
        let Some(sprite) = self.get(path, palette_path) else {
            return self.load(path, palette_path);
        };

        Ok(sprite)
    }
}
