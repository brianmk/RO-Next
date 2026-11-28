#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, print_debug};
use ro_next_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub(super) struct GameArchiveList {
    pub archives: Vec<String>,
}

impl Default for GameArchiveList {
    fn default() -> Self {
        Self {
            archives: Self::DEFAULT_FILES.iter().map(ToString::to_string).collect(),
        }
    }
}

/// Locate the RO-Next payload `archive/` folder when the binary is started
/// outside its normal install directory (for example from the repository's
/// `target/release` during development).
fn find_payload_archive() -> Option<String> {
    let mut starts = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        starts.push(current_dir);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        starts.push(directory.to_path_buf());
    }

    for start in starts {
        for ancestor in start.ancestors().take(6) {
            for candidate in [ancestor.join("archive"), ancestor.join("ro-next").join("archive")] {
                if candidate.join("data").join("lua-scaffolding").is_dir() {
                    return Some(candidate.display().to_string());
                }
            }
        }
    }

    None
}

impl GameArchiveList {
    const DEFAULT_FILES: &'static [&'static str] = &["data.grf", "rdata.grf", "archive/"];
    const FILE_NAME: &'static str = "client/game_archives.ron";

    pub(super) fn load() -> Self {
        #[cfg(feature = "debug")]
        print_debug!("loading game archive list from {}", Self::FILE_NAME.magenta());

        if let Some(archives) = std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str::<GameArchiveList>(&data).ok())
        {
            return archives;
        }

        #[cfg(feature = "debug")]
        print_debug!(
            "[{}] failed to load game archive list from {}; trying with default",
            "warning".yellow(),
            Self::FILE_NAME.magenta(),
        );

        // Running the binary straight from the client root (for example
        // `target/release/ro-next` with the working directory set to
        // `CondaRo-client`) has no `client/game_archives.ron`. In that layout
        // the classic data lives in `game/`; use it instead of the bare
        // defaults (`data.grf` in the working directory) so the client still
        // starts without a launcher.
        let mut fallback = GameArchiveList::default();
        if !std::path::Path::new("data.grf").is_file() && std::path::Path::new("game").is_dir() {
            let mut archives = Vec::new();
            for archive in ["data.grf", "CondaRO.grf", "palettes.grf", "rdata.grf"] {
                if std::path::Path::new("game").join(archive).is_file() {
                    archives.push(format!("game/{archive}"));
                }
            }
            if std::path::Path::new("game/archive").is_dir() {
                archives.push("game/archive/".to_string());
            }
            if !archives.is_empty() {
                fallback.archives = archives;
            }
        }

        if let Some(archive) = find_payload_archive() {
            fallback.archives.push(archive);
        }

        fallback
    }
}
