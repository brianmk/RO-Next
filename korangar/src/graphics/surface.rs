use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use rust_state::RustState;
use wgpu::{Adapter, CurrentSurfaceTexture, Device, PresentMode, SurfaceConfiguration, SurfaceTexture, TextureFormat};

use crate::graphics::{ScreenSize, presentation_watchdog};

#[derive(RustState, Debug, Clone, Copy)]
pub struct PresentModeInfo {
    /// Vsync On (Fast)
    pub supports_mailbox: bool,
    /// Vsync Off
    pub supports_immediate: bool,
}

impl PresentModeInfo {
    pub fn from_adapter(adapter: &Adapter, surface: &wgpu::Surface) -> PresentModeInfo {
        let mut present_mode_info = PresentModeInfo {
            supports_immediate: false,
            supports_mailbox: false,
        };

        surface
            .get_capabilities(adapter)
            .present_modes
            .iter()
            .for_each(|present_mode| match present_mode {
                PresentMode::Mailbox => present_mode_info.supports_mailbox = true,
                PresentMode::Immediate => present_mode_info.supports_immediate = true,
                _ => {}
            });

        present_mode_info
    }
}

pub struct Surface {
    device: Device,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    present_mode_info: PresentModeInfo,
    invalid: bool,
    /// Whether the window is backed by the native Wayland backend.
    is_wayland: bool,
}

impl Surface {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        adapter: &Adapter,
        device: Device,
        surface: wgpu::Surface<'static>,
        window_width: u32,
        window_height: u32,
        triple_buffering: bool,
        vsync: bool,
        is_wayland: bool,
    ) -> Self {
        let window_width = window_width.max(1);
        let window_height = window_height.max(1);

        let mut config = surface.get_default_config(adapter, window_width, window_height).unwrap();

        let surfaces_formats: Vec<TextureFormat> = surface.get_capabilities(adapter).formats;

        #[cfg(feature = "debug")]
        {
            print_debug!("Supported surface formats:");
            for format in &surfaces_formats {
                print_debug!("{:?}", format);
            }
        }

        let present_mode_info = PresentModeInfo::from_adapter(adapter, &surface);

        config.format = surfaces_formats.first().copied().expect("not surface formats found");
        let desired_maximum_frame_latency = desired_maximum_frame_latency(triple_buffering, is_wayland);
        config.desired_maximum_frame_latency = desired_maximum_frame_latency;
        config.present_mode = match vsync {
            false if present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        {
            print_debug!("Surface present mode is {:?}", config.present_mode.magenta());
            print_debug!("Surface format is {:?}", config.format);
        }

        surface.configure(&device, &config);

        println!(
            "[surface] backend={} present_mode={:?} frame_latency={} format={:?} size={}x{}",
            if is_wayland { "wayland" } else { "x11" },
            config.present_mode,
            config.desired_maximum_frame_latency,
            config.format,
            config.width,
            config.height,
        );

        Self {
            device,
            surface,
            config,
            present_mode_info,
            invalid: false,
            is_wayland,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn acquire(&mut self) -> Option<SurfaceTexture> {
        // The watchdog observes how long the acquire blocks. On affected NVIDIA Wayland
        // drivers `get_current_texture` can block forever when the compositor
        // stops releasing swapchain images, and the call cannot be interrupted
        // from Rust.
        presentation_watchdog::begin_acquire();
        presentation_watchdog::simulate_stall_if_configured();

        let result = self.handle_acquire_result(self.surface.get_current_texture());

        presentation_watchdog::end_acquire();

        result
    }

    /// Handles the result of a swapchain image acquisition explicitly.
    ///
    /// `Timeout`/`Occluded` skip the frame, `Outdated`/`Lost`/`Validation`
    /// reconfigure the surface and retry once, everything else is passed
    /// through.
    fn handle_acquire_result(&mut self, result: CurrentSurfaceTexture) -> Option<SurfaceTexture> {
        match result {
            CurrentSurfaceTexture::Success(texture) => Some(texture),
            CurrentSurfaceTexture::Suboptimal(texture) => {
                crate::logging::log_debug!("[surface] suboptimal swapchain image acquired, reconfiguring the surface");
                self.invalid = true;
                Some(texture)
            }
            CurrentSurfaceTexture::Timeout => {
                crate::logging::log_debug!("[surface] swapchain acquire timed out, skipping the frame");
                None
            }
            CurrentSurfaceTexture::Occluded => {
                crate::logging::log_debug!("[surface] swapchain is occluded, skipping the frame");
                None
            }
            CurrentSurfaceTexture::Outdated => {
                crate::logging::log_debug!("[surface] swapchain is outdated, reconfiguring the surface");
                self.reconfigure_and_retry()
            }
            CurrentSurfaceTexture::Lost => {
                crate::logging::log_debug!("[surface] swapchain was lost, recreating it");
                self.reconfigure_and_retry()
            }
            CurrentSurfaceTexture::Validation => {
                crate::logging::log_error!("[surface] swapchain validation error, reconfiguring the surface");
                self.reconfigure_and_retry()
            }
        }
    }

    fn reconfigure_and_retry(&mut self) -> Option<SurfaceTexture> {
        self.surface.configure(&self.device, &self.config);

        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => Some(texture),
            CurrentSurfaceTexture::Suboptimal(texture) => Some(texture),
            CurrentSurfaceTexture::Timeout => {
                crate::logging::log_debug!("[surface] swapchain acquire timed out after reconfiguring, skipping the frame");
                None
            }
            CurrentSurfaceTexture::Occluded => None,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Validation => {
                crate::logging::log_warn!("[surface] the swapchain is still unusable after reconfiguring the surface");
                None
            }
        }
    }

    pub fn invalidate(&mut self) {
        self.invalid = true;
    }

    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn reconfigure(&mut self) {
        self.invalid = false;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        self.config.present_mode = match enabled {
            false if self.present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if self.present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        print_debug!("set surface present mode to {:?}", self.config.present_mode.magenta());

        self.invalidate();
    }

    pub fn set_triple_buffering(&mut self, enabled: bool) {
        self.config.desired_maximum_frame_latency = desired_maximum_frame_latency(enabled, self.is_wayland);
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.config.width = window_size.width as u32;
        self.config.height = window_size.height as u32;
        self.invalidate();
    }

    pub fn present_mode_info(&self) -> PresentModeInfo {
        self.present_mode_info
    }

    pub fn format(&self) -> TextureFormat {
        self.config.format
    }

    pub fn window_size(&self) -> Vector2<usize> {
        Vector2 {
            x: self.config.width as usize,
            y: self.config.height as usize,
        }
    }

    pub fn window_screen_size(&self) -> ScreenSize {
        ScreenSize {
            width: self.config.width as f32,
            height: self.config.height as f32,
        }
    }
}

/// Frame latency of the swapchain.
///
/// Native Wayland uses one image in flight even when triple buffering is
/// enabled: fewer images are held by the compositor, which makes the NVIDIA
/// Wayland WSI deadlock (blocked image acquisition) much less likely.
/// `KORANGAR_FRAME_LATENCY` overrides the value for testing.
fn desired_maximum_frame_latency(triple_buffering: bool, is_wayland: bool) -> u32 {
    if let Some(latency) = std::env::var("KORANGAR_FRAME_LATENCY")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|latency| *latency >= 1)
    {
        return latency;
    }

    match (is_wayland, triple_buffering) {
        (true, _) => 1,
        (false, true) => 2,
        (false, false) => 1,
    }
}
