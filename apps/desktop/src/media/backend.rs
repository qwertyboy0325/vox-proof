use std::path::{Path, PathBuf};

use egui::ColorImage;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod stub;

#[cfg(target_os = "macos")]
pub type DefaultBackend = macos::AvFoundationBackend;
#[cfg(not(target_os = "macos"))]
pub type DefaultBackend = stub::UnavailableBackend;

pub fn default_backend() -> DefaultBackend {
    DefaultBackend::default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    OpenFailed(String),
    Unavailable(String),
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed(message) | Self::Unavailable(message) => f.write_str(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaSnapshot {
    pub path: Option<PathBuf>,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub volume: f32,
    pub has_video: bool,
    pub error: Option<String>,
}

impl Default for MediaSnapshot {
    fn default() -> Self {
        Self {
            path: None,
            playing: false,
            position_ms: 0,
            duration_ms: None,
            volume: 1.0,
            has_video: false,
            error: None,
        }
    }
}

pub trait MediaBackend {
    fn attach(&mut self, path: &Path) -> Result<(), MediaError>;
    fn detach(&mut self);
    fn play(&mut self);
    fn pause(&mut self);
    fn seek_ms(&mut self, ms: u64);
    fn set_volume(&mut self, volume: f32);
    fn snapshot(&self) -> MediaSnapshot;
    fn take_video_frame(&mut self) -> Option<ColorImage>;
}

#[derive(Debug)]
pub struct FakeMediaBackend {
    snapshot: MediaSnapshot,
    pub fail_next_attach: bool,
}

impl FakeMediaBackend {
    pub fn with_duration(duration_ms: u64) -> Self {
        Self {
            snapshot: MediaSnapshot {
                duration_ms: Some(duration_ms),
                ..MediaSnapshot::default()
            },
            fail_next_attach: false,
        }
    }
}

impl Default for FakeMediaBackend {
    fn default() -> Self {
        Self::with_duration(10_000)
    }
}

impl MediaBackend for FakeMediaBackend {
    fn attach(&mut self, path: &Path) -> Result<(), MediaError> {
        if self.fail_next_attach {
            self.fail_next_attach = false;
            self.snapshot.path = None;
            self.snapshot.playing = false;
            self.snapshot.error = Some("Could not open the selected media file.".to_owned());
            return Err(MediaError::OpenFailed(
                "Could not open the selected media file.".to_owned(),
            ));
        }
        let duration_ms = self.snapshot.duration_ms.or(Some(10_000));
        self.snapshot = MediaSnapshot {
            path: Some(path.to_path_buf()),
            playing: false,
            position_ms: 0,
            duration_ms,
            volume: self.snapshot.volume,
            has_video: path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "mp4" | "mov" | "m4v" | "avi"
                    )
                }),
            error: None,
        };
        Ok(())
    }

    fn detach(&mut self) {
        let volume = self.snapshot.volume;
        let duration_ms = self.snapshot.duration_ms;
        self.snapshot = MediaSnapshot {
            volume,
            duration_ms,
            ..MediaSnapshot::default()
        };
    }

    fn play(&mut self) {
        if self.snapshot.path.is_some() {
            self.snapshot.playing = true;
        }
    }

    fn pause(&mut self) {
        self.snapshot.playing = false;
    }

    fn seek_ms(&mut self, ms: u64) {
        if self.snapshot.path.is_some() {
            self.snapshot.position_ms = super::clamp_seek_ms(ms, self.snapshot.duration_ms);
        }
    }

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
