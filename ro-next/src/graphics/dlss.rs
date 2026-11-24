//! Detection of a usable NVIDIA DLSS (NGX) runtime.
//!
//! DLSS is not implemented in this client: it requires the proprietary NVIDIA
//! NGX SDK, motion vectors, jittered projection matrices and a Vulkan/DLSS
//! integration (the `dlss_wgpu` crate can provide the latter, but the runtime
//! library has to be obtained from NVIDIA and may not be redistributed with
//! this project). The graphics settings still offer a "DLSS" entry so users
//! can see the option, but it is disabled unless a real runtime is present.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Returns whether a native NVIDIA DLSS runtime library can be found.
///
/// This never enables DLSS by itself, it only makes it possible to show a
/// different tooltip when a runtime happens to be installed (for example when
/// a user manually placed `libnvidia-ngx-dlss.so` next to the client).
pub(crate) fn dlss_runtime_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();

    *AVAILABLE.get_or_init(|| {
        let mut search_directories = Vec::new();

        if let Ok(override_path) = std::env::var("RO_NEXT_DLSS_RUNTIME_PATH") {
            search_directories.push(PathBuf::from(override_path));
        }

        if let Ok(executable) = std::env::current_exe()
            && let Some(directory) = executable.parent()
        {
            search_directories.push(directory.to_path_buf());
        }

        search_directories.extend([
            PathBuf::from("/usr/lib64"),
            PathBuf::from("/usr/lib/x86_64-linux-gnu"),
            PathBuf::from("/usr/lib"),
            PathBuf::from("/usr/local/lib"),
        ]);

        search_directories.iter().any(|directory| has_dlss_runtime(directory))
    })
}

fn has_dlss_runtime(directory: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        file_name.starts_with("libnvidia-ngx-dlss.so") || file_name == "nvngx_dlss.dll"
    })
}

/// The reason that is shown in the tooltip of the disabled DLSS entry in the
/// anti-aliasing drop down.
pub(crate) fn dlss_disabled_reason() -> &'static str {
    if dlss_runtime_available() {
        "A DLSS runtime was found, but this build has no NVIDIA NGX SDK integration"
    } else {
        "Requires NVIDIA DLSS runtime/SDK"
    }
}
