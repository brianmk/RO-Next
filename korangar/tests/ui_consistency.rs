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
