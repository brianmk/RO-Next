//! Communication between the audio engine and the low-level audio API.

pub(crate) mod cpal;
mod renderer;
pub(crate) mod resources;

pub(crate) use renderer::*;

/// The default kira used by [`AudioManager`](crate::AudioManager)s.
pub(crate) type DefaultBackend = cpal::CpalBackend;

/// A backend that never opens an audio output device and never produces sound.
///
/// It is used for headless/background test clients, where audio output is
/// disabled with the `RO_NEXT_MUTE_AUDIO` environment variable. The audio
/// engine keeps working normally (tracks, sounds, looping and playback state
/// advance at real time), so the rest of the client behaves exactly like it
/// does with an audio device, but no audio ever reaches the operating system.
pub(crate) struct SilentBackend {
    should_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl SilentBackend {
    /// Number of frames that are mixed per iteration, roughly matching a
    /// typical audio callback buffer.
    const BUFFER_SIZE: usize = 512;
    /// The sample rate that the silent mixer runs at.
    const SAMPLE_RATE: u32 = 48_000;
}

impl Backend for SilentBackend {
    type Error = std::convert::Infallible;

    fn setup(_internal_buffer_size: usize) -> Result<(Self, u32), Self::Error> {
        let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        Ok((
            Self {
                should_stop: should_stop.clone(),
            },
            Self::SAMPLE_RATE,
        ))
    }

    fn start(&mut self, mut renderer: Renderer) -> Result<(), Self::Error> {
        let should_stop = self.should_stop.clone();

        std::thread::spawn(move || {
            let mut output = vec![0.0f32; Self::BUFFER_SIZE * 2];
            let period = std::time::Duration::from_secs_f64(Self::BUFFER_SIZE as f64 / Self::SAMPLE_RATE as f64);

            while !should_stop.load(std::sync::atomic::Ordering::SeqCst) {
                renderer.on_start_processing();
                renderer.process(&mut output, 2);
                std::thread::sleep(period);
            }
        });

        Ok(())
    }
}

impl Drop for SilentBackend {
    fn drop(&mut self) {
        self.should_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Connects a [`Renderer`] to a lower level audio API.
pub(crate) trait Backend: Sized {
    /// Errors that can occur when using this kira.
    type Error;

    /// Starts the kira and returns itself and the initial sample rate.
    fn setup(internal_buffer_size: usize) -> Result<(Self, u32), Self::Error>;

    /// Sends the renderer to the kira to start audio playback.
    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error>;
}
