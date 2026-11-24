use std::collections::HashMap;

#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, print_debug};
use ro_next_interface::window::{Anchor, AnchorPoint};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use super::WindowClass;
use crate::graphics::{ScreenPosition, ScreenSize};
use crate::state::ClientState;

#[derive(Serialize, Deserialize)]
pub struct WindowState {
    pub anchor: Anchor<ClientState>,
    pub size: ScreenSize,
}

impl WindowState {
    pub fn new(anchor: Anchor<ClientState>, size: ScreenSize) -> Self {
        Self { anchor, size }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct WindowCache {
    entries: HashMap<WindowClass, WindowState>,
}

impl WindowCache {
    // Since `WindowClass` has some variants with debug features enabled, we use a
    // differen file to store the window cache. This avoids failing to load and
    // thereby wiping the previous window cache when switching between debug and
    // non-debug builds.
    #[cfg(not(feature = "debug"))]
    const FILE_NAME: &'static str = "client/window_cache.ron";
    #[cfg(feature = "debug")]
    const FILE_NAME: &'static str = "client/window_cache_debug.ron";

    /// Give windows that need a specific initial position a sensible default.
    ///
    /// This is only used the first time a window is opened (afterwards the
    /// position the user dragged it to is loaded from the cache file).
    fn register_default_window_positions(&mut self) {
        // The buff bar sits at the bottom center, above the hotbar.
        self.entries.entry(WindowClass::BuffBar).or_insert_with(|| {
            WindowState::new(
                Anchor::new(AnchorPoint::BottomCenter, ScreenPosition { left: -180.0, top: -150.0 }),
                ScreenSize { width: 360.0, height: 0.0 },
            )
        });

        // The spirit sphere counter sits directly above the buff bar, so
        // filling/consuming the five Champion spheres is visible without
        // moving the eyes away from the character. The window is only open
        // while the player has spheres.
        self.entries.entry(WindowClass::SpiritSpheres).or_insert_with(|| {
            WindowState::new(
                Anchor::new(AnchorPoint::BottomCenter, ScreenPosition { left: -42.0, top: -172.0 }),
                ScreenSize { width: 83.0, height: 19.0 },
            )
        });

        // Whisper conversations open in the middle of the screen (the official
        // client also places them freely; the user can drag them afterwards).
        self.entries.entry(WindowClass::Whisper).or_insert_with(|| {
            WindowState::new(
                Anchor::new(AnchorPoint::Center, ScreenPosition { left: 0.0, top: 0.0 }),
                ScreenSize {
                    width: 360.0,
                    height: 320.0,
                },
            )
        });

        // The notification overlay is positioned by the client on every screen
        // resize, but a default entry keeps the cache complete.
        self.entries.entry(WindowClass::NotificationHud).or_insert_with(|| {
            WindowState::new(
                Anchor::new(AnchorPoint::TopCenter, ScreenPosition { left: 0.0, top: 12.0 }),
                ScreenSize { width: 520.0, height: 0.0 },
            )
        });

        // The minimap belongs in the top right corner, like in the original client.
        self.entries.entry(WindowClass::Minimap).or_insert_with(|| {
            WindowState::new(
                // `TopRight` offsets are measured from the right screen edge to the
                // left window edge, so the fixed window width (210) and the margin have
                // to be subtracted to keep the whole window on screen.
                Anchor::new(AnchorPoint::TopRight, ScreenPosition {
                    left: -(210.0 + 12.0),
                    top: 12.0,
                }),
                ScreenSize {
                    width: 0.0,
                    height: f32::MAX,
                },
            )
        });
    }

    fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading window cache from {}", Self::FILE_NAME.magenta());

        crate::logging::log_debug!("settings: loading window cache from {}", Self::FILE_NAME);

        match std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
        {
            Some(entries) => {
                let cache = Self { entries };
                crate::logging::log_debug!("settings: loaded window cache with {} entries", cache.entries.len());
                Some(cache)
            }
            None => {
                crate::logging::log_warn!(
                    "settings: could not load window cache from {} (using defaults)",
                    Self::FILE_NAME
                );
                None
            }
        }
    }

    fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving window cache to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(&self.entries, PrettyConfig::new()).unwrap();
        std::fs::write(Self::FILE_NAME, data).expect("unable to write file");
    }
}

impl ro_next_interface::application::WindowCache<ClientState> for WindowCache {
    fn create() -> Self {
        let mut cache = Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to load window cache from {}. creating empty cache",
                Self::FILE_NAME.magenta()
            );

            Default::default()
        });

        cache.register_default_window_positions();

        cache
    }

    fn get_window_state(&self, class: WindowClass) -> Option<(Anchor<ClientState>, ScreenSize)> {
        self.entries.get(&class).map(|entry| (entry.anchor, entry.size))
    }

    fn register_window(&mut self, class: WindowClass, anchor: Anchor<ClientState>, size: ScreenSize) {
        if let Some(entry) = self.entries.get_mut(&class) {
            entry.anchor = anchor;
            entry.size = size;
        } else {
            let entry = WindowState::new(anchor, size);
            self.entries.insert(class, entry);
        }
    }

    fn update_anchor(&mut self, class: WindowClass, anchor: Anchor<ClientState>) {
        if let Some(entry) = self.entries.get_mut(&class) {
            entry.anchor = anchor;
        }
    }

    fn update_size(&mut self, class: WindowClass, size: ScreenSize) {
        if let Some(entry) = self.entries.get_mut(&class) {
            entry.size = size;
        }
    }
}

impl Drop for WindowCache {
    fn drop(&mut self) {
        self.save();
    }
}
