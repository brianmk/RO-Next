//! Consistency tests for the interface: hardcoded style literals and
//! hardcoded user-facing strings.
//!
//! These are source scans, not rendering tests. They exist because the two
//! failure modes are invisible in the type system and in the golden screenshot
//! matrix (a new window can be perfectly rendered and still bypass the design
//! tokens and the localization system).
//!
//! A failure here means: use the design tokens (`client_theme().tokens()...`)
//! or add a localization entry instead of a literal. If a file is a debug-only
//! window or legitimately needs a literal, add it to one of the allowlists
//! below *and* explain why in the comment next to it.

use std::fs;
use std::path::{Path, PathBuf};

/// Files that are only compiled with the `debug` feature and are never part of
/// a release build, so they are not part of the user-facing interface.
const DEBUG_ONLY_FILES: &[&str] = &[
    "windows/commands.rs",
    "windows/frame_inspector.rs",
    "windows/maps.rs",
    "windows/packet_inspector.rs",
    "windows/profiler.rs",
    "windows/render_options.rs",
    "windows/theme_inspector.rs",
    "windows/cache.rs",
];

/// User-facing words that must not appear as a literal button text/label in a
/// non-debug window. Add a localization key instead.
const FORBIDDEN_UI_WORDS: &[&str] = &[
    // Context menu / dialogs.
    "\"Whisper\"",
    "\"Add Friend\"",
    "\"Invite to Party\"",
    "\"Trade\"",
    "\"View Info\"",
    "\"Drop\"",
    "\"Choose a warp destination\"",
    // Item info window.
    "\"Card slots\"",
    "\"Random options\"",
    "\"Description\"",
    "\"Category\"",
    "\"Identified\"",
    "\"Required level\"",
    "\"Bonuses\"",
    "\"Unknown card\"",
    "\"Empty card slot\"",
    "\"Unavailable card slot\"",
    "\"Total:\"",
    // Skill description window.
    "\"Tip: right-click",
    "\"not learned (max",
    "\"SP cost\"",
    "\"Upgradable\"",
    "\"Ground target\"",
    "\"Self cast\"",
    // Graphics settings window.
    "\"Resolution\"",
    "\"Borderless fullscreen\"",
    "\"Lighting mode\"",
    "\"Triple buffering\"",
    "\"Enable VSYNC\"",
    "\"Max FPS\"",
    "\"Background FPS\"",
    "\"Texture filtering\"",
    "\"Multisampling\"",
    "\"Supersampling\"",
    "\"Anti-aliasing\"",
    "\"Shadow method\"",
    "\"Shadow detail\"",
    "\"Shadow resolution\"",
    "\"Sample Distribution Shadow Maps\"",
    "\"High quality interface\"",
    // Other windows.
    "\"Close the storage window\"",
    "\"Deleting a character is permanent\"",
    // Equipment stats panel and item actions.
    "\"EQUIPMENT\"",
    "\"CHARACTER\"",
    "\"Weapon attack\"",
    "\"Armor defense\"",
    "\"Refine\"",
    "\"Equipped weight\"",
    "\"Item bonuses\"",
    "\"Magic attack\"",
    "\"Magic defense\"",
    "\"Attack speed\"",
    "\"Hit\"",
    "\"Flee\"",
    "\"Critical\"",
    "\"Weight\"",
    "\"Use\"",
    "\"Equip\"",
    "\"Use / Equip\"",
    "\"Store 1\"",
    "\"Take out 1\"",
    "\"Store all\"",
    "\"Take out all\"",
    "\"Talk to a Kafra\"",
    "\"Enter a party name first\"",
    "\"You have to be in a party\"",
    "\"You are not in a party\"",
    "\"Not implemented yet\"",
    "\"This item cannot be used or equipped\"",
    "\"Enter the account e-mail address first\"",
    "\"This setting is not supported\"",
    "\"You have no open party invitation\"",
    "\"You are invited to join the party\"",
    "\"Spheres\"",
    "\"Lv {} / Jlv {}\"",
    // Debug-only menu entries that live in the non-debug menu window.
    "\"Render options\"",
    "\"Map inspector\"",
    "\"Client state inspector\"",
    "\"Maps\"",
    "\"Commands\"",
    "\"Theme inspector\"",
    "\"Profiler\"",
    "\"Packet inspector\"",
    "\"Cache statistics\"",
    "\"Buy\"",
    "\"Sell\"",
    "\"Zeny\"",
    "\"Cart\"",
    "\"Storage\"",
    "\"Equipment\"",
    "\"Select Character\"",
    "\"Delete character\"",
    "\"Party\"",
    "\"Party invitation\"",
    "\"Friend request\"",
    "\"Graphics Settings\"",
    "\"Cancel\"",
    "\"Close\"",
    "\"Delete\"",
    "\"Switch\"",
    "\"Accept\"",
    "\"Reject\"",
];

fn source_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interface")
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).expect("interface directory must exist") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

fn relative_to_interface(path: &Path) -> String {
    path.strip_prefix(source_directory())
        .expect("path is inside the interface directory")
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_debug_only(file: &str) -> bool {
    DEBUG_ONLY_FILES.iter().any(|entry| *entry == file)
}

/// Non-comment code lines of a file.
fn code_lines(content: &str) -> impl Iterator<Item = &str> {
    content.lines().filter(|line| !line.trim_start().starts_with("//"))
}

#[test]
fn no_hardcoded_window_titles_in_user_facing_windows() {
    let mut violations = Vec::new();

    for path in rust_files(&source_directory()) {
        let file = relative_to_interface(&path);
        if is_debug_only(&file) {
            continue;
        }
        let content = fs::read_to_string(&path).expect("source file is readable");
        for (line_number, line) in code_lines(&content).enumerate() {
            let trimmed = line.trim();
            // `title: "..."` in a `window!` macro is a window title.
            if (trimmed.starts_with("title: \"") || trimmed.starts_with("title: format!(\"")) && !trimmed.contains("localization()") {
                violations.push(format!("{file}:{}: {trimmed}", line_number + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "hardcoded window titles found, use a localization entry instead:\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_hardcoded_button_words_in_user_facing_windows() {
    let mut violations = Vec::new();

    for path in rust_files(&source_directory()) {
        let file = relative_to_interface(&path);
        if is_debug_only(&file) {
            continue;
        }
        let content = fs::read_to_string(&path).expect("source file is readable");
        for (line_number, line) in code_lines(&content).enumerate() {
            if line.contains("localization()") {
                continue;
            }
            for word in FORBIDDEN_UI_WORDS {
                if line.contains(word) {
                    violations.push(format!("{file}:{}: {word} in `{}`", line_number + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "hardcoded user-facing strings found, add them to `Localization` instead:\n{}",
        violations.join("\n")
    );
}

/// Count the direct color constructors (`Color::rgb_u8`, `Color::rgba_u8`,
/// `Color::monochrome_u8`) in the interface. Named constants like
/// `Color::WHITE` and `Color::TRANSPARENT` are tints/pass-through values and
/// are not counted.
fn count_color_literals() -> usize {
    let mut count = 0;
    for path in rust_files(&source_directory()) {
        let content = fs::read_to_string(&path).expect("source file is readable");
        count += code_lines(&content)
            .filter(|line| line.contains("Color::rgb_u8") || line.contains("Color::rgba_u8") || line.contains("Color::monochrome_u8"))
            .count();
    }
    count
}

fn count_font_size_literals() -> usize {
    let mut count = 0;
    for path in rust_files(&source_directory()) {
        let content = fs::read_to_string(&path).expect("source file is readable");
        count += code_lines(&content).filter(|line| line.contains("FontSize(")).count();
    }
    count
}

#[derive(serde::Deserialize)]
struct TokenBudget {
    max_color_literals: usize,
    max_font_size_literals: usize,
}

fn budget() -> TokenBudget {
    let budget_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/ui-token-budget.ron");
    ron::from_str(&fs::read_to_string(&budget_path).expect("token budget file must exist")).expect("token budget file must be valid RON")
}

#[test]
fn hardcoded_style_literals_do_not_exceed_the_token_budget() {
    let budget = budget();
    let max_colors = budget.max_color_literals;
    let max_fonts = budget.max_font_size_literals;

    let colors = count_color_literals();
    let fonts = count_font_size_literals();

    assert!(
        colors <= max_colors,
        "the interface has {colors} hardcoded color literals, but the token budget is {max_colors}. Use the design tokens \
         (`client_theme().tokens().color()...`) instead of adding literals."
    );
    assert!(
        fonts <= max_fonts,
        "the interface has {fonts} FontSize sites, but the token budget is {max_fonts}. Use the typography tokens \
         (`client_theme().tokens().typography()...`) instead of adding literals."
    );
}

// ---------------------------------------------------------------------------
// Extended localization lint (Phase 5).
//
// The checks above only scan `src/interface`. The settings/drop-down labels
// live next to their enums (`graphics/settings.rs`, `settings/*.rs`,
// `loaders/font/mod.rs`, ...) and the localization fallbacks (`fn text`) can
// silently drift from the localization defaults. The checks below scan the
// whole client source tree.
// ---------------------------------------------------------------------------

/// Source files that legitimately contain the UI words: the localization
/// defaults and language files are the *target* of the lint, not a violation.
fn is_localization_source(file: &str) -> bool {
    file.starts_with("state/localization/")
}

/// Root of the client crate sources (`korangar/src`).
fn client_source_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every Rust file of the client crate, relative to `korangar/src` with `/`
/// separators.
fn client_source_files() -> Vec<(String, PathBuf)> {
    let root = client_source_directory();
    rust_files(&root)
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .expect("path is inside the client source directory")
                .to_string_lossy()
                .replace('\\', "/");
            (relative, path)
        })
        .collect()
}

/// Files that contain the forbidden words as *data* instead of as UI labels:
///
/// * `input/key_names.rs` maps the winit `KeyCode` names of the keybinding data
///   (`Delete` is the physical Delete key, not a button label),
/// * the legacy item-description parser matches the Korean/English field labels
///   that occur inside item descriptions, it does not render them.
const NON_UI_LITERAL_FILES: &[&str] = &["input/key_names.rs", "world/library/item_stats_fallback.rs"];

/// The source with `#[cfg(test)]`-gated items blanked out, keeping the line
/// numbers intact.
///
/// Test modules are not user-facing, but production code may follow them (some
/// files keep helper tests in the middle), so the whole file has to be scanned
/// with only the test items removed.
fn production_source(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut output: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    let mut index = 0;

    while index < lines.len() {
        let attribute = lines[index].trim();
        let is_test_gate =
            attribute == "#[cfg(test)]" || (attribute.starts_with("#[cfg(") && attribute.trim_end_matches(']').ends_with("test)"));

        if !is_test_gate {
            index += 1;
            continue;
        }

        // Skip the remaining attributes/comments and find the gated item.
        let mut item = index + 1;
        while item < lines.len() {
            let line = lines[item].trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("#[") || line.starts_with("#![") {
                item += 1;
            } else {
                break;
            }
        }

        if item >= lines.len() {
            break;
        }

        // Find the end of the gated item: a top-level closing brace or, for
        // items without a body, the terminating semicolon.
        let mut depth = 0i32;
        let mut opened = false;
        let mut end = item;
        'item_end: for (line_index, line) in lines.iter().enumerate().skip(item) {
            for character in line.chars() {
                match character {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth -= 1,
                    ';' if !opened => {
                        end = line_index;
                        break 'item_end;
                    }
                    _ => {}
                }
            }
            if opened && depth == 0 {
                end = line_index;
                break;
            }
        }

        for line in output.iter_mut().take(end + 1).skip(index) {
            line.clear();
        }
        index = end + 1;
    }

    output.join("\n")
}

/// `true` for files that are only compiled with the `debug` feature (existing
/// allowlist), that are the localization source itself, or that contain the
/// words as data rather than as UI labels.
fn is_scan_exempt(file: &str) -> bool {
    let interface_relative = file.strip_prefix("interface/").unwrap_or(file);
    is_debug_only(interface_relative) || is_localization_source(file) || NON_UI_LITERAL_FILES.contains(&file)
}

/// User-facing UI words must not appear as string literals outside the
/// `src/interface` windows either.
///
/// This catches labels that were moved next to their settings enum instead of
/// into `Localization` (the `fn text` / `fn text_key` pair below catches the
/// drop-down labels, this scan catches every other literal). The localization
/// source (`state/localization`) is exempt because it *defines* the literals.
#[test]
fn no_hardcoded_ui_words_outside_the_interface() {
    let mut violations = Vec::new();

    for (file, path) in client_source_files() {
        if file.starts_with("interface/") || is_scan_exempt(&file) {
            continue;
        }

        let content = fs::read_to_string(&path).expect("source file is readable");
        let production = production_source(&content);
        for (line_number, line) in code_lines(&production).enumerate() {
            for word in FORBIDDEN_UI_WORDS {
                if line.contains(word) {
                    violations.push(format!("{file}:{}: {word} in `{}`", line_number + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "hardcoded user-facing strings found outside the interface, add them to `Localization` instead:\n{}",
        violations.join("\n")
    );
}

/// Extracts the string literals of a source fragment (good enough for the
/// label literals used by `fn text`: no raw strings, no string continuation).
fn string_literals(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut literals = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }

        let mut literal = String::new();
        index += 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' if index + 1 < bytes.len() => {
                    index += 2;
                    continue;
                }
                b'"' => {
                    index += 1;
                    break;
                }
                byte => {
                    literal.push(byte as char);
                    index += 1;
                }
            }
        }

        if !literal.is_empty() {
            literals.push(literal);
        }
    }

    literals
}

/// Finds the `fn text(&self)` body of an `impl` block (index range of the
/// method including its closing brace) when the block has one.
fn text_method_body(block: &str) -> Option<String> {
    let start = block.find("fn text(&self)")?;
    let body = &block[start..];
    let mut depth = 0i32;
    let mut seen_open = false;
    let mut end = body.len();

    for (index, character) in body.char_indices() {
        match character {
            '{' => {
                depth += 1;
                seen_open = true;
            }
            '}' => {
                depth -= 1;
                if seen_open && depth == 0 {
                    end = index + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    seen_open.then(|| body[..end].to_owned())
}

/// Split the source into top-level `impl` blocks (rustfmt puts the closing
/// brace at column 0, nested blocks are indented).
fn impl_blocks(content: &str) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks = Vec::new();
    let mut start = None;

    for (index, line) in lines.iter().enumerate() {
        if start.is_none() && line.starts_with("impl ") {
            start = Some(index);
        }
        if let Some(block_start) = start
            && *line == "}"
        {
            blocks.push(lines[block_start..=index].join("\n"));
            start = None;
        }
    }

    blocks
}

/// Localization source that every `fn text` fallback literal has to exist in.
fn localization_source() -> String {
    let path = client_source_directory().join("state/localization/texts.rs");
    fs::read_to_string(&path).expect("the localization texts source has to exist")
}

/// Drop-down options have to be localizable: every `fn text(&self)` that
/// returns a literal must be paired with a `fn text_key(&self)` in the same
/// impl, and the fallback literal must exist in the localization defaults so
/// the untranslated fallback and the translated label cannot drift apart.
///
/// Implementations without literals (for example the server `Service` display
/// name, which is data from `sclientinfo.xml`) are dynamic and stay as they
/// are; the language selection returns the native language names on purpose.
#[test]
fn drop_down_labels_are_localized() {
    let localization = localization_source();
    let mut violations = Vec::new();

    for (file, path) in client_source_files() {
        if is_scan_exempt(&file) {
            continue;
        }

        let content = fs::read_to_string(&path).expect("source file is readable");
        for block in impl_blocks(&production_source(&content)) {
            let Some(body) = text_method_body(&block) else {
                continue;
            };

            let literals = string_literals(&body);
            if literals.is_empty() {
                // Dynamic label (data-driven text, no literal).
                continue;
            }

            if !block.contains("fn text_key(&self)") {
                violations.push(format!(
                    "{file}: `fn text(&self)` returns a hardcoded literal without a `fn text_key(&self)` localization key"
                ));
                continue;
            }

            for literal in literals {
                if !localization.contains(&format!("\"{literal}\"")) {
                    violations.push(format!(
                        "{file}: drop-down fallback literal \"{literal}\" is missing from `state/localization/texts.rs`; add the label to \
                         `Localization`/`DropdownTexts`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "unlocalized drop-down labels found:\n{}",
        violations.join("\n")
    );
}
