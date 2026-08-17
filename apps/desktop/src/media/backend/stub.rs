use std::path::Path;

use egui::ColorImage;

use super::{MediaBackend, MediaError, MediaSnapshot};

#[derive(Debug, Default)]
pub struct UnavailableBackend {
    snapshot: MediaSnapshot,
}

impl MediaBackend for UnavailableBackend {
    fn attach(&mut self, path: &Path) -> Result<(), MediaError> {
        let message =
            "Media preview uses the macOS media engine in this slice. Review can continue without it."
                .to_owned();
        self.snapshot.path = Some(path.to_path_buf());
        self.snapshot.playing = false;
        self.snapshot.position_ms = 0;
        self.snapshot.has_video = false;
        self.snapshot.error = Some(message.clone());
        Err(MediaError::Unavailable(message))
    }

    fn detach(&mut self) {
        self.snapshot = MediaSnapshot {
            volume: self.snapshot.volume,
            ..MediaSnapshot::default()
        };
    }

    fn play(&mut self) {}

    fn pause(&mut self) {}

    fn seek_ms(&mut self, _ms: u64) {}

    fn set_volume(&mut self, volume: f32) {
        self.snapshot.volume = volume.clamp(0.0, 1.0);
    }

    fn snapshot(&self) -> MediaSnapshot {
        self.snapshot.clone()
    }

    fn take_video_frame(&mut self) -> Option<ColorImage> {
        None
    }
}
