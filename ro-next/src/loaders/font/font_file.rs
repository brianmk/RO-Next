use std::sync::Arc;

use cgmath::Point2;
use cosmic_text::FontSystem;
use cosmic_text::fontdb::{ID, Source};
#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, Timer};
use ro_next_loaders::FileLoader;
use ttf_parser::Face;

use crate::loaders::GameFileLoader;
use crate::loaders::font::GlyphCoordinate;
use crate::loaders::font::coverage::{FontCoverageBuilder, face_codepoints};
use crate::loaders::font::slug::{SlugFontData, SlugGlyphBounds};
use crate::loaders::rectangle::Rectangle;

const FONT_FOLDER_PATH: &str = "data\\font";

pub(crate) type FaceGlyphs = Arc<Vec<Option<GlyphCoordinate>>>;

pub(crate) struct FontFile {
    pub(crate) faces: Vec<(ID, FaceGlyphs)>,
}

impl FontFile {
    pub(crate) fn new(
        name: &str,
        game_file_loader: &GameFileLoader,
        font_system: &mut FontSystem,
        slug_data: &mut SlugFontData,
        coverage: &mut FontCoverageBuilder,
    ) -> Option<Self> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load font: {}", name.magenta()));

        let ttf_file_path = format!("{}\\{}.ttf", FONT_FOLDER_PATH, name);

        let Ok(font_data) = game_file_loader.get(&ttf_file_path) else {
            crate::logging::warn_once(
                module_path!(),
                &ttf_file_path,
                format_args!("failed to load font file `{ttf_file_path}`; text using this font falls back to the next loaded font"),
            );
            return None;
        };

        let ids = font_system.db_mut().load_font_source(Source::Binary(Arc::new(font_data)));

        let faces: Vec<(ID, FaceGlyphs)> = ids
            .iter()
            .filter_map(|&id| {
                // Collect the cmap of the face as well, so the text layout can
                // replace characters that no loaded font can render instead of
                // drawing a `.notdef` box for them.
                let (glyphs, codepoints) = font_system.db().with_face_data(id, |data, index| {
                    let face = Face::parse(data, index).ok()?;
                    Some((slug_data.add_face(&face), face_codepoints(&face)))
                })??;

                coverage.add_face(codepoints);

                let glyphs = glyphs.into_iter().map(|bounds| bounds.map(glyph_coordinate)).collect();

                Some((id, Arc::new(glyphs)))
            })
            .collect();

        if faces.is_empty() {
            crate::logging::warn_once(
                module_path!(),
                &ttf_file_path,
                format_args!("failed to parse font file `{ttf_file_path}`; text using this font falls back to the next loaded font"),
            );
            return None;
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Some(Self { faces })
    }
}

/// Converts the em space bounds of a glyph with the y-axis pointing up into
/// the placement of the glyph relative to its origin with the y-axis pointing
/// down.
fn glyph_coordinate(bounds: SlugGlyphBounds) -> GlyphCoordinate {
    GlyphCoordinate {
        em_coordinate: Rectangle::new(Point2::new(bounds.x_min, bounds.y_max), Point2::new(bounds.x_max, bounds.y_min)),
        glyph_index: bounds.glyph_index,
        width: bounds.x_max - bounds.x_min,
        height: bounds.y_max - bounds.y_min,
        offset_top: -bounds.y_max,
        offset_left: bounds.x_min,
    }
}
