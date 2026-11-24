# How to use a different font

It is possible to use a different font in RO-Next. This may be needed, if you want to support languages that are not
included in the distributed font file. Currently, we use "Noto Sans", which includes all glyphs for the Latin, Cyrillic,
and Greek alphabets. If you need to support a different alphabet, then you need to use a different font (for example
"Noto Sans Japanese"). We support having fallback fonts, so if for example the primary font doesn't have a specific
glyph, the fallback font is tried. Multiple fallback fonts are supported.

1. Copy the font file into the `archive/data/font` folder.
2. Add the name of the font file without the `.ttf` extension to the font list passed to `FontLoader::new` in the
   `ro-next/src/lib.rs` file.

## Shipped fonts

| file | family | licence | purpose |
| --- | --- | --- | --- |
| `NotoSans.ttf` | Noto Sans Regular | SIL OFL 1.1 (`OFL.txt`) | primary font: Latin, Greek, Cyrillic, punctuation, digits |
| `NanumGothic.ttf` | NanumGothic Regular (version 3.020) | SIL OFL 1.1 (`NanumGothic-OFL.txt`) | fallback font: Hangul syllables/jamo and the KS X 1001 symbols used by the official 2019 NPC scripts (math operators, arrows, geometric shapes, misc symbols, box drawing, CJK punctuation) |

`NanumGothic.ttf` is byte-identical to `NanumGothic-Regular.ttf` from the
[Naver Nanum fonts](https://hangeul.naver.com/font) (SIL Open Font License 1.1, copyright © 2011 NHN Corporation,
designed by Sandoll Communications). It is unmodified and is redistributed under its reserved font name.

The 2019 server sends free text (player chat, NPC dialogue) and the official quest scripts use KS X 1001 symbols
(`▲ ▼ ⊂ ∑ ∀ ∠ ■ ☆ ♡ ☎ …`), none of which the primary Noto Sans has. A fallback font is the only way to render
player-typed Korean; the client additionally replaces characters that no loaded font can render at the text
layout boundary so a missing glyph can never turn into a `.notdef` box (see `loaders/font/coverage.rs`).

The fonts are force-added to git (`git add -f`) because `.gitignore` excludes `ro-next/archive/data/font/*`.
