mod color_span_iterator;
mod coverage;
mod font_file;
mod layout_key;
mod slug;

use std::hash::Hash;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use cgmath::{Point2, Vector2};
use cosmic_text::fontdb::ID;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight, fontdb};
use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_container::CacheStatistics;
use korangar_container::{Cacheable, SimpleCache};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
use korangar_interface::application::TextLayouter;
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::{ElementDisplay, StateElement};
use rust_state::RustState;
use serde::{Deserialize, Serialize};
use wgpu::{Device, Queue};

use self::color_span_iterator::ColorSpanIterator;
use self::coverage::{FontCoverage, FontCoverageBuilder};
use self::slug::SlugFontData;
use super::GameFileLoader;
use crate::graphics::{Color, ScreenSize, SlugFont};
use crate::loaders::font::font_file::{FaceGlyphs, FontFile};
use crate::loaders::font::layout_key::{LayoutKey, LayoutKeyRef};
use crate::loaders::rectangle::Rectangle;
use crate::state::ClientState;

/// The padding in pixels that every glyph quad needs for the rasterizer to
/// cover every pixel that the glyph outline touches.
pub const GLYPH_PADDING: f32 = 0.5;
/// The radius of the dark shadow around in-game text in em units. Must match
/// `TEXT_SHADOW_RADIUS` in the postprocessing shader module.
pub const TEXT_SHADOW_RADIUS: f32 = 0.08;

/// Profiling counters of the text layout cache (visible with
/// `KORANGAR_PERF_LOG=1`).
static LAYOUT_CACHE_CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAYOUT_CACHE_HITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAYOUT_CACHE_MISSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Number of text layout calls, cache hits and actual shaping operations since
/// the last call to [`take_layout_cache_statistics`].
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutCacheStatistics {
    pub calls: u64,
    pub hits: u64,
    pub misses: u64,
}

/// Take and reset the text layout cache statistics.
pub fn take_layout_cache_statistics() -> LayoutCacheStatistics {
    use std::sync::atomic::Ordering::Relaxed;

    LayoutCacheStatistics {
        calls: LAYOUT_CACHE_CALLS.swap(0, Relaxed),
        hits: LAYOUT_CACHE_HITS.swap(0, Relaxed),
        misses: LAYOUT_CACHE_MISSES.swap(0, Relaxed),
    }
}

const MAX_CACHE_COUNT: u32 = 2048;
const MAX_CACHE_SIZE: usize = 32 << 20;

struct CachedLayout {
    glyphs: Vec<GlyphInstruction>,
    size: Vector2<f32>,
}

impl Cacheable for CachedLayout {
    fn size(&self) -> usize {
        size_of::<Vector2<f32>>() +
        size_of::<GlyphInstruction>() * self.glyphs.len() +
        // Approximation for the text that is saved as a key in the lookup table and the FIFO.
        2 * self.glyphs.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontSize(pub f32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, RustState, StateElement)]
pub enum OverflowBehavior {
    Shrink,
    LineBreak,
}

impl korangar_interface::application::FontSize for FontSize {
    fn scaled(&self, scaling: f32) -> Self {
        Self(self.0 * scaling)
    }
}

impl ElementDisplay for FontSize {
    fn element_display(&self) -> String {
        format!("^000001ₐ^000000{}", self.0.element_display())
    }
}

impl std::ops::Mul<f32> for FontSize {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self(self.0 * rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scaling(f32);

impl Scaling {
    // TODO: Likely remove this function
    pub fn get_factor(&self) -> f32 {
        self.0
    }
}

impl DropDownItem<Scaling> for Scaling {
    fn text(&self) -> &str {
        match self.0 {
            0.5 => "50%",
            0.6 => "60%",
            0.7 => "70%",
            0.75 => "75%",
            0.8 => "80%",
            0.9 => "90%",
            1.0 => "100%",
            1.1 => "110%",
            1.2 => "120%",
            1.25 => "125%",
            1.3 => "130%",
            1.4 => "140%",
            1.5 => "150%",
            1.6 => "160%",
            1.7 => "170%",
            1.75 => "175%",
            1.8 => "180%",
            1.9 => "190%",
            2.0 => "200%",
            _ => "Custom",
        }
    }

    fn value(&self) -> Scaling {
        *self
    }
}

impl ElementDisplay for Scaling {
    fn element_display(&self) -> String {
        format!("^000001a^000000{}", self.0.element_display())
    }
}

impl Scaling {
    /// All scaling options that are offered in the interface settings window.
    pub const OPTIONS: [Scaling; 19] = [
        Scaling::new(0.5),
        Scaling::new(0.6),
        Scaling::new(0.7),
        Scaling::new(0.75),
        Scaling::new(0.8),
        Scaling::new(0.9),
        Scaling::new(1.0),
        Scaling::new(1.1),
        Scaling::new(1.2),
        Scaling::new(1.25),
        Scaling::new(1.3),
        Scaling::new(1.4),
        Scaling::new(1.5),
        Scaling::new(1.6),
        Scaling::new(1.7),
        Scaling::new(1.75),
        Scaling::new(1.8),
        Scaling::new(1.9),
        Scaling::new(2.0),
    ];

    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    /// Returns the closest scaling option for the given factor. This is used
    /// to apply the `KORANGAR_UI_SCALE` environment variable and the DPI
    /// based default without producing a value that is not part of the
    /// settings options.
    pub fn closest(factor: f32) -> Self {
        Self::OPTIONS
            .into_iter()
            .min_by(|left, right| {
                let left = (left.0 - factor).abs();
                let right = (right.0 - factor).abs();
                left.total_cmp(&right)
            })
            .unwrap_or(Self::new(1.0))
    }
}

#[derive(Copy, Clone)]
pub struct GlyphInstruction {
    pub position: Rectangle<f32>,
    pub em_coordinate: Rectangle<f32>,
    pub glyph_index: u32,
    pub color: Color,
}

impl GlyphInstruction {
    /// Grows the glyph by `padding` pixels on every side. The em space
    /// coordinates grow by the same amount, so the glyph keeps its size and
    /// position on screen. The Slug shader needs at least [`GLYPH_PADDING`]
    /// to cover every pixel that the outline touches.
    pub fn dilated(&self, padding: f32, font_size: FontSize) -> Self {
        let em_padding = padding / font_size.0;

        Self {
            position: Rectangle::new(
                Point2::new(self.position.min.x - padding, self.position.min.y - padding),
                Point2::new(self.position.max.x + padding, self.position.max.y + padding),
            ),
            em_coordinate: Rectangle::new(
                Point2::new(self.em_coordinate.min.x - em_padding, self.em_coordinate.min.y + em_padding),
                Point2::new(self.em_coordinate.max.x + em_padding, self.em_coordinate.max.y - em_padding),
            ),
            glyph_index: self.glyph_index,
            color: self.color,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct GlyphCoordinate {
    pub(crate) em_coordinate: Rectangle<f32>,
    pub(crate) glyph_index: u32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) offset_top: f32,
    pub(crate) offset_left: f32,
}

pub struct FontLoader {
    font_system: Mutex<FontSystem>,
    primary_font_family: String,
    /// Characters that at least one of the loaded fonts can render. Used to
    /// replace unrenderable characters before they can become `.notdef` boxes.
    coverage: FontCoverage,
    slug_font: SlugFont,
    glyph_cache: HashMap<ID, FaceGlyphs>,
    layout_cache: Mutex<SimpleCache<LayoutKey, CachedLayout>>,
}

impl FontLoader {
    pub fn new(fonts: &[String], game_file_loader: &GameFileLoader, device: &Device, queue: &Queue) -> Self {
        assert_ne!(fonts.len(), 0, "no font defined");

        let mut font_system = FontSystem::new_with_locale_and_db(Self::system_locale(), fontdb::Database::new());
        let mut glyph_cache = HashMap::new();
        let mut slug_data = SlugFontData::default();
        let mut coverage_builder = FontCoverageBuilder::default();

        let fonts: Vec<FontFile> = fonts
            .iter()
            .filter_map(|font_name| {
                FontFile::new(
                    font_name,
                    game_file_loader,
                    &mut font_system,
                    &mut slug_data,
                    &mut coverage_builder,
                )
            })
            .collect();

        let primary_font_family = Self::extract_primary_font_family(&font_system, &fonts);
        let coverage = coverage_builder.build();

        // A missing fallback font used to be a silent failure that only showed
        // up as square glyphs in Korean chat/NPC dialogue. Make it visible once.
        if !coverage.contains('한') {
            crate::logging::log_warn!(
                "no loaded font covers Korean/Hangul; Korean text will be shown as '?' (is archive/data/font/NanumGothic.ttf missing?)"
            );
        }

        for (id, glyphs) in fonts.into_iter().flat_map(|font| font.faces) {
            glyph_cache.insert(id, glyphs);
            let _ = font_system.get_font(id, Weight::NORMAL);
        }

        slug_data.ensure_not_empty();

        #[cfg(feature = "debug")]
        print_debug!(
            "slug font data: {} curve entries, {} band entries, {} glyphs ({} bytes)",
            slug_data.curves.len().magenta(),
            slug_data.bands.len().magenta(),
            slug_data.glyphs.len().magenta(),
            (size_of_val(slug_data.curves.as_slice()) + size_of_val(slug_data.bands.as_slice()) + size_of_val(slug_data.glyphs.as_slice()))
                .magenta(),
        );

        let slug_font = SlugFont::new(device, queue, &slug_data.curves, &slug_data.bands, &slug_data.glyphs);

        let layout_cache = SimpleCache::new(
            NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
            NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
        );

        Self {
            font_system: Mutex::new(font_system),
            primary_font_family,
            coverage,
            slug_font,
            glyph_cache,
            layout_cache: Mutex::new(layout_cache),
        }
    }

    #[cfg(feature = "debug")]
    pub fn cache_statistics(&self) -> CacheStatistics {
        self.layout_cache.lock().unwrap().statistics()
    }

    fn system_locale() -> String {
        sys_locale::get_locale().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to get system locale, falling back to en-US", "warning".yellow());
            "en-US".to_string()
        })
    }

    fn extract_primary_font_family(font_system: &FontSystem, fonts: &[FontFile]) -> String {
        let primary_font_id = fonts
            .first()
            .and_then(|font| font.faces.first())
            .map(|(id, _)| *id)
            .expect("no primary font ID found");

        font_system
            .db()
            .face(primary_font_id)
            .and_then(|face| face.families.first().map(|(family, _)| family.clone()))
            .expect("primary font has no family name")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_text_dimensions(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
        line_height_scale: f32,
        available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        // Avoid floating point inaccuracy leading to a line break when shrinking.
        const THRESHOLD: f32 = 1.0;

        let layout_width = match overflow_behavior {
            OverflowBehavior::Shrink => None,
            OverflowBehavior::LineBreak => Some(available_width),
        };

        let mut size = self.layout_text(
            text,
            default_color,
            highlight_color,
            font_size,
            line_height_scale,
            layout_width,
            None,
        );

        if let OverflowBehavior::Shrink = overflow_behavior {
            let scaling_factor = (available_width / (size.x + THRESHOLD)).min(1.0);

            font_size = FontSize(font_size.0 * scaling_factor);
            size *= scaling_factor;
        }

        (
            ScreenSize {
                width: size.x,
                height: size.y,
            },
            font_size,
        )
    }

    /// Writes the text layout for the given text into the `glyphs` buffer and
    /// returns the size of the text in pixels.
    ///
    /// Does not clear the glyph buffer before writing into it.
    #[allow(clippy::too_many_arguments)]
    pub fn layout_text(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        font_size: FontSize,
        line_height_scale: f32,
        available_width: Option<f32>,
        glyphs: Option<&mut Vec<GlyphInstruction>>,
    ) -> Vector2<f32> {
        // Replace characters that none of the loaded fonts can render before they
        // reach the shaper: missing glyphs would become `.notdef` boxes. This is
        // the single display-safety gate of the renderer; ASCII text and text
        // whose glyphs are all covered take the borrowed fast path.
        let sanitized = self.coverage.sanitize(text);
        let text = sanitized.as_ref();

        // TODO: NHA We currently call get_text_dimensions() with different
        //       "available_width" and "overflow_behavior" then when we call
        //       layout_text() for with the resulting available_width.
        //       This results us caching two instead of one layout.
        let key = LayoutKeyRef {
            text,
            default_color: default_color.into(),
            highlight_color: highlight_color.into(),
            font_size,
            line_height_scale,
            layout_width: available_width.unwrap_or(f32::INFINITY),
        };

        LAYOUT_CACHE_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if let Some(layout) = self.layout_cache.lock().unwrap().get_with(&key, |k| k == &key) {
            LAYOUT_CACHE_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if let Some(glyphs) = glyphs {
                glyphs.extend(layout.glyphs.iter().copied())
            }
            return layout.size;
        }

        LAYOUT_CACHE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let (size, rendered_glyphs) = self.render_layout(
            text,
            default_color,
            highlight_color,
            font_size,
            line_height_scale,
            available_width,
        );

        if let Some(glyphs) = glyphs {
            glyphs.extend(rendered_glyphs.iter().copied());
        }

        let _result = self.layout_cache.lock().unwrap().insert(key.to_owned(), CachedLayout {
            glyphs: rendered_glyphs,
            size,
        });

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] layout could not be added to cache. Text: '{}': {:?}",
                "error".red(),
                text,
                error
            );
        }

        size
    }

    // TODO: NHA cosmic_text could help us to render text in boxes.
    //       But that would need us to re-evaluate on how we render test in general.
    //       We also don't really use the "line_height_scale", which would provide
    //       an easy way to handle "line height".
    fn render_layout(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        font_size: FontSize,
        line_height_scale: f32,
        available_width: Option<f32>,
    ) -> (Vector2<f32>, Vec<GlyphInstruction>) {
        let mut text_width = 0f32;
        let mut text_height = 0f32;

        let metrics = Metrics::relative(font_size.0, line_height_scale);
        let attributes = Attrs::new().family(Family::Name(&self.primary_font_family));

        // We try to hold the mutex lock as short as possible.
        let buffer = {
            let mut font_system = self.font_system.lock().unwrap();
            let mut buffer = Buffer::new(&mut font_system, metrics);

            buffer.set_size(&mut font_system, available_width, None);
            buffer.set_rich_text(
                &mut font_system,
                ColorSpanIterator::new(text, default_color, highlight_color, attributes.clone()),
                &attributes,
                Shaping::Advanced,
                None,
            );

            buffer
        };

        let mut rendered_glyphs = Vec::new();

        for run in buffer.layout_runs() {
            text_width = text_width.max(run.line_w);
            text_height += run.line_height;

            for layout_glyph in run.glyphs.iter() {
                let physical_glyph = layout_glyph.physical((0.0, 0.0), 1.0);

                let Some(glyph_coordinate) = self.glyph_cache.get(&layout_glyph.font_id).and_then(|font| {
                    font.get(layout_glyph.glyph_id as usize).copied().flatten().map(|mut glyph| {
                        glyph.width *= font_size.0;
                        glyph.height *= font_size.0;
                        glyph.offset_left *= font_size.0;
                        glyph.offset_top *= font_size.0;
                        glyph
                    })
                }) else {
                    continue;
                };

                let x = physical_glyph.x as f32 + glyph_coordinate.offset_left;
                let y = run.line_y + physical_glyph.y as f32 + glyph_coordinate.offset_top;
                let width = glyph_coordinate.width;
                let height = glyph_coordinate.height;

                let position = Rectangle::new(Point2::new(x, y), Point2::new(x + width, y + height));
                let color = layout_glyph.color_opt.map(|color| color.into()).unwrap_or(default_color);

                rendered_glyphs.push(GlyphInstruction {
                    position,
                    em_coordinate: glyph_coordinate.em_coordinate,
                    glyph_index: glyph_coordinate.glyph_index,
                    color,
                });
            }
        }

        (Vector2::new(text_width, text_height), rendered_glyphs)
    }

    /// The GPU buffers with the glyph outlines of all fonts.
    pub fn get_slug_font(&self) -> &SlugFont {
        &self.slug_font
    }
}

impl TextLayouter<ClientState> for Arc<FontLoader> {
    fn get_text_dimensions(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        font_size: FontSize,
        available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        let (size, font_size) = self.as_ref().get_text_dimensions(
            text,
            default_color,
            highlight_color,
            font_size,
            1.0,
            available_width,
            overflow_behavior,
        );

        (size, font_size)
    }
}
