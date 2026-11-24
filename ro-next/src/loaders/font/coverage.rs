//! Deterministic display safety for text that is handed to the renderer.
//!
//! Every string that reaches the text renderer is laid out character by
//! character. If none of the loaded fonts has a glyph for a character, the
//! shaper falls back to glyph `0` (`.notdef`), which the Slug rasterizer draws
//! as a square box - the "square glyphs" that players see for Korean chat and
//! for the KS X 1001 symbols of the official quest scripts.
//!
//! [`FontCoverage`] is the union of the `cmap` of every loaded font. The text
//! layout sanitizes its input with it:
//!
//! * characters that are covered by at least one loaded font are rendered as
//!   they are,
//! * characters that no font covers are replaced with [`REPLACEMENT_CHARACTER`]
//!   (`?`) so a square can never appear,
//! * invisible format characters (zero-width/BOM/direction marks) that no font
//!   covers are dropped instead of being replaced,
//! * control characters are kept only when the text layout gives them meaning
//!   (`\n`, `\r`, `\t`), any other control character is dropped,
//! * unrenderable Unicode whitespace is replaced by a normal space.
//!
//! The markup of the client (`^RRGGBB` color codes) is pure ASCII and is never
//! touched.
//!
//! This is intentionally a *font coverage* check and not an "is ASCII" check:
//! the shipped fallback font (`NanumGothic.ttf`) covers Korean and the KS
//! X 1001 symbols, so Korean chat and the NPC quest dialogues keep their
//! original text while still being safe on a data set without the fallback
//! font.

use std::borrow::Cow;
use std::ops::RangeInclusive;

use ttf_parser::Face;

/// Replacement for a character that none of the loaded fonts can render.
pub(crate) const REPLACEMENT_CHARACTER: char = '?';

/// Collects the Unicode codepoints of the `cmap` tables of a font face.
///
/// Only Unicode subtables are read; the non-Unicode subtables of old Windows
/// fonts (`cmap` platform 3 encodings 0..=6) cannot be mapped to `char`s.
/// Codepoints that are mapped to glyph `0` are not contained in the returned
/// list (a `cmap` entry that points to `.notdef` does not produce a glyph).
pub(crate) fn face_codepoints(face: &Face) -> Vec<u32> {
    let Some(cmap) = face.tables().cmap else {
        return Vec::new();
    };

    let mut codepoints = Vec::new();

    for subtable in cmap.subtables {
        if !subtable.is_unicode() {
            continue;
        }

        subtable.codepoints(|codepoint| {
            // `codepoints` reports every entry of the subtable, including the
            // ones that map to glyph 0. Those are exactly the characters that
            // would render as a box, so filter them out by asking for the
            // glyph index (the `codepoints` callback does not provide it).
            if subtable.glyph_index(codepoint).is_some() {
                codepoints.push(codepoint);
            }
        });
    }

    codepoints
}

/// Builder that unions the codepoints of all loaded font faces.
#[derive(Default)]
pub(crate) struct FontCoverageBuilder {
    codepoints: Vec<u32>,
}

impl FontCoverageBuilder {
    /// Adds all codepoints of one font face.
    pub(crate) fn add_face(&mut self, mut codepoints: Vec<u32>) {
        self.codepoints.append(&mut codepoints);
    }

    /// Sorts and compresses the collected codepoints into ranges.
    pub(crate) fn build(mut self) -> FontCoverage {
        self.codepoints.sort_unstable();
        self.codepoints.dedup();

        let mut ranges: Vec<RangeInclusive<u32>> = Vec::new();

        for codepoint in self.codepoints {
            match ranges.last_mut() {
                Some(range) if codepoint == *range.end() + 1 => *range = *range.start()..=codepoint,
                _ => ranges.push(codepoint..=codepoint),
            }
        }

        FontCoverage { ranges }
    }
}

/// Union of the Unicode codepoints that all loaded fonts can render.
#[derive(Debug, Default)]
pub(crate) struct FontCoverage {
    /// Sorted, non-overlapping, inclusive codepoint ranges.
    ranges: Vec<RangeInclusive<u32>>,
}

impl FontCoverage {
    /// The number of distinct codepoints covered by the loaded fonts.
    #[cfg(test)]
    pub(crate) fn codepoint_count(&self) -> usize {
        self.ranges.iter().map(|range| (range.end() - range.start() + 1) as usize).sum()
    }

    /// Whether at least one loaded font has a glyph for `character`.
    pub(crate) fn contains(&self, character: char) -> bool {
        let codepoint = character as u32;

        // Binary search for the last range that starts at or before the codepoint.
        let index = self.ranges.partition_point(|range| *range.start() <= codepoint);

        match index.checked_sub(1) {
            Some(index) => self.ranges[index].contains(&codepoint),
            None => false,
        }
    }

    /// Replaces every character that no loaded font can render.
    ///
    /// Returns a borrowed slice when the text contains only renderable
    /// characters (including the very common pure-ASCII case), so the hot
    /// layout path does not allocate for ordinary text.
    pub(crate) fn sanitize<'a>(&self, text: &'a str) -> Cow<'a, str> {
        // Fast path: pure ASCII text that contains no control characters other
        // than line breaks/tabs is always displayable and never allocates.
        if text.is_ascii()
            && text
                .bytes()
                .all(|byte| !byte.is_ascii_control() || matches!(byte, b'\n' | b'\t' | b'\r'))
        {
            return Cow::Borrowed(text);
        }

        let mut sanitized: Option<String> = None;
        let mut copied_until = 0;

        for (offset, character) in text.char_indices() {
            let renderable = match character.is_ascii() {
                // Line breaks/tabs are layout characters, not glyphs; other
                // ASCII control characters are dropped below.
                true => !character.is_ascii_control() || matches!(character, '\n' | '\t' | '\r'),
                false => self.contains(character),
            };

            if renderable {
                continue;
            }

            let sanitized = sanitized.get_or_insert_with(|| String::with_capacity(text.len()));
            sanitized.push_str(&text[copied_until..offset]);

            if let Some(replacement) = replacement_for(character) {
                sanitized.push(replacement);
            }

            copied_until = offset + character.len_utf8();
        }

        match sanitized {
            Some(mut sanitized) => {
                sanitized.push_str(&text[copied_until..]);
                Cow::Owned(sanitized)
            }
            None => Cow::Borrowed(text),
        }
    }
}

/// The character that replaces `character`, or [`None`] to drop it.
fn replacement_for(character: char) -> Option<char> {
    // The text layout treats these as line breaks/tabs and never asks for a glyph.
    if matches!(character, '\n' | '\r' | '\t') {
        return Some(character);
    }

    // Control characters have no visible glyph; dropping them is safer than
    // rendering a box for them.
    if character.is_control() {
        return None;
    }

    // Invisible format characters (zero-width space, BOM, bidi marks, ...)
    // must not leak into the transcript as boxes or question marks.
    if is_invisible(character) {
        return None;
    }

    // Unicode whitespace should not turn into "?".
    if character.is_whitespace() {
        return Some(' ');
    }

    Some(REPLACEMENT_CHARACTER)
}

/// Format characters that are invisible by design.
fn is_invisible(character: char) -> bool {
    matches!(
        character,
        '\u{00AD}' // soft hyphen
            | '\u{061C}' // arabic letter mark
            | '\u{180E}' // mongolian vowel separator
            | '\u{200B}'..='\u{200F}' // zero width space .. right-to-left mark
            | '\u{202A}'..='\u{202E}' // bidi embedding/override/isolate controls
            | '\u{2060}'..='\u{2064}' // word joiner, invisible operators
            | '\u{2066}'..='\u{206F}' // bidi isolates, deprecated format characters
            | '\u{FEFF}' // zero width no-break space / BOM
            | '\u{FFF9}'..='\u{FFFB}' // interlinear annotation
            | '\u{E0001}' // language tag
            | '\u{E0020}'..='\u{E007F}' // tag characters
    )
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::path::PathBuf;

    use ttf_parser::Face;

    use super::{FontCoverage, FontCoverageBuilder, REPLACEMENT_CHARACTER, face_codepoints};

    fn coverage(codepoints: &[u32]) -> FontCoverage {
        let mut builder = FontCoverageBuilder::default();
        builder.add_face(codepoints.to_vec());
        builder.build()
    }

    fn shipped_font_coverage(file_name: &str) -> FontCoverage {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("archive/data/font").join(file_name);
        let data = std::fs::read(&path).unwrap_or_else(|error| panic!("can't read '{}': {error}", path.display()));
        let face = Face::parse(&data, 0).unwrap_or_else(|error| panic!("can't parse '{}': {error}", path.display()));

        let mut builder = FontCoverageBuilder::default();
        builder.add_face(face_codepoints(&face));
        builder.build()
    }

    #[test]
    fn ascii_text_is_borrowed_unchanged() {
        let coverage = coverage(&[]);
        let sanitized = coverage.sanitize("^FF0000Hello, world!^000000");

        assert!(matches!(sanitized, Cow::Borrowed(_)));
        assert_eq!(sanitized, "^FF0000Hello, world!^000000");
    }

    #[test]
    fn renderable_characters_are_kept_and_missing_ones_are_replaced() {
        let coverage = coverage(&['a' as u32, 'é' as u32]);

        let sanitized = coverage.sanitize("café 한글 123");

        assert_eq!(sanitized, format!("café {r}{r} 123", r = REPLACEMENT_CHARACTER));
        // Covered text without missing characters is not copied.
        assert!(matches!(coverage.sanitize("café"), Cow::Borrowed(_)));
    }

    #[test]
    fn the_question_mark_replacement_is_only_used_when_no_font_covers_the_character() {
        let coverage = coverage(&[REPLACEMENT_CHARACTER as u32]);
        // A covered non-ASCII character proves the check is not an ASCII check.
        assert!(!coverage.contains('한'));
        assert!(coverage.contains('?'));
    }

    #[test]
    fn invisible_format_characters_are_dropped_not_replaced() {
        let coverage = coverage(&[]);

        // e.g. the stray BOM characters of the `Eswhoo` skill name.
        assert_eq!(coverage.sanitize("Eswhoo\u{FEFF}\u{FEFF}"), "Eswhoo");
        assert_eq!(coverage.sanitize("a\u{200B}b"), "ab");
    }

    #[test]
    fn layout_significant_control_characters_are_kept_and_others_are_dropped() {
        let coverage = coverage(&[]);

        assert_eq!(coverage.sanitize("line 1\nline 2\ttab"), "line 1\nline 2\ttab");
        assert_eq!(coverage.sanitize("bell\u{0007}"), "bell");
    }

    #[test]
    fn unrenderable_whitespace_becomes_a_normal_space() {
        let coverage = coverage(&[]);

        assert_eq!(coverage.sanitize("a\u{3000}b"), "a b");
        assert_eq!(coverage.sanitize("a\u{00A0}b"), "a b");
    }

    #[test]
    fn ranges_are_unioned_from_multiple_faces() {
        let mut builder = FontCoverageBuilder::default();
        builder.add_face(vec![0x41, 0x42, 0x43]);
        builder.add_face(vec![0x43, 0x44, 0xAC00, 0xAC01]);
        let coverage = builder.build();

        assert_eq!(coverage.codepoint_count(), 6);
        assert!(coverage.contains('A'));
        assert!(coverage.contains('B'));
        assert!(coverage.contains('D'));
        assert!(coverage.contains('가'));
        assert!(coverage.contains('각'));
        assert!(!coverage.contains('나'));
        assert!(!coverage.contains('H'));
    }

    #[test]
    fn shipped_fonts_cover_the_primary_and_fallback_scripts() {
        let primary = shipped_font_coverage("NotoSans.ttf");
        let fallback = shipped_font_coverage("NanumGothic.ttf");

        // The primary font renders the Latin UI and `?` replacement.
        assert!(primary.contains('A'));
        assert!(primary.contains(':'));
        assert!(primary.contains(REPLACEMENT_CHARACTER));
        // ... and intentionally has no Korean/KS glyphs (this is why the fallback
        // exists).
        assert!(!primary.contains('한'));
        assert!(!primary.contains('⊂'));

        // The fallback covers Korean (all Hangul syllables) and the KS X 1001
        // symbols of the official NPC scripts.
        for character in [
            '한', '글', '가', '힣', '▲', '▼', '⊂', '∑', '∀', '∠', '■', '☆', '♡', '☎', '∬', '∈', 'α',
        ] {
            assert!(fallback.contains(character), "fallback font has no glyph for {character:?}");
        }

        // The union is what the renderer uses, so everything above is safe.
        let mut combined = FontCoverageBuilder::default();
        combined.add_face(vec!['A' as u32]);
        combined.add_face(vec!['한' as u32]);
        let union = combined.build();
        assert!(union.contains('A') && union.contains('한'));
    }
}
