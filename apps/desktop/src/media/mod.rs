//! Non-authoritative local media preview. Playback never writes review authority.

mod backend;

use std::path::{Path, PathBuf};

use egui::ColorImage;

pub use backend::{FakeMediaBackend, MediaBackend, MediaError, MediaSnapshot, default_backend};

/// Last event-driven cue seek. Repaint must not own playback position.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MediaSeekIntent {
    last_segment: Option<usize>,
}

impl MediaSeekIntent {
    pub fn last_segment(self) -> Option<usize> {
        self.last_segment
    }

    /// Cue click, Seek, queue/keyboard focus, or equivalent user/focus navigation.
    pub fn on_event(&mut self, segment: usize, force: bool) -> bool {
        if !force && self.last_segment == Some(segment) {
            return false;
        }
        self.last_segment = Some(segment);
        true
    }

    /// One-shot initial cue only. Must not overwrite a prior event seek.
    pub fn on_repaint_establish(&mut self, selected_segment: usize) -> bool {
        if self.last_segment.is_some() {
            return false;
        }
        self.last_segment = Some(selected_segment);
        true
    }

    pub fn clear(&mut self) {
        self.last_segment = None;
    }
}

pub struct MediaPreview<B: MediaBackend = backend::DefaultBackend> {
    backend: B,
    pending_seek_ms: Option<u64>,
}

impl Default for MediaPreview {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            pending_seek_ms: None,
        }
    }
}

impl<B: MediaBackend> MediaPreview<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            pending_seek_ms: None,
        }
    }

    pub fn attach(&mut self, path: PathBuf) -> Result<(), MediaError> {
        self.backend.attach(&path)?;
        if let Some(ms) = self.pending_seek_ms {
            self.backend
                .seek_ms(clamp_seek_ms(ms, self.backend.snapshot().duration_ms));
        }
        Ok(())
    }

    pub fn detach(&mut self) {
        self.backend.detach();
        self.pending_seek_ms = None;
    }

    pub fn is_attached(&self) -> bool {
        self.backend.snapshot().path.is_some()
    }

    pub fn play(&mut self) {
        self.backend.play();
    }

    pub fn pause(&mut self) {
        self.backend.pause();
    }

    pub fn toggle_play(&mut self) {
        if self.backend.snapshot().playing {
            self.backend.pause();
        } else {
            self.backend.play();
        }
    }

    pub fn seek_ms(&mut self, ms: u64) {
        let clamped = clamp_seek_ms(ms, self.backend.snapshot().duration_ms);
        self.pending_seek_ms = Some(clamped);
        if self.is_attached() {
            self.backend.seek_ms(clamped);
        }
    }

    pub fn seek_to_cue_start(&mut self, start_ms: u64) {
        self.seek_ms(start_ms);
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.backend.set_volume(volume.clamp(0.0, 1.0));
    }

    pub fn snapshot(&self) -> MediaSnapshot {
        self.backend.snapshot()
    }

    pub fn take_video_frame(&mut self) -> Option<ColorImage> {
        self.backend.take_video_frame()
    }

    pub fn path_display(&self) -> Option<String> {
        self.backend.snapshot().path.as_ref().map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        })
    }
}

pub fn clamp_seek_ms(target_ms: u64, duration_ms: Option<u64>) -> u64 {
    match duration_ms {
        Some(duration) => target_ms.min(duration),
        None => target_ms,
    }
}

pub fn format_media_clock(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let millis = ms % 1000;
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3600;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{millis:03}")
    } else {
        format!("{minutes:02}:{seconds:02}.{millis:03}")
    }
}

pub fn is_supported_media_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4" | "mov" | "m4v" | "avi" | "m4a" | "mp3" | "wav" | "aac" | "caf" | "aiff"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_seek_clamps_to_duration() {
        assert_eq!(clamp_seek_ms(1_500, Some(1_000)), 1_000);
        assert_eq!(clamp_seek_ms(400, Some(1_000)), 400);
        assert_eq!(clamp_seek_ms(400, None), 400);
    }

    #[test]
    fn attach_play_pause_seek_and_detach() {
        let mut preview = MediaPreview::new(FakeMediaBackend::with_duration(5_000));
        preview.attach(PathBuf::from("/tmp/talk.mp4")).unwrap();
        let snap = preview.snapshot();
        assert_eq!(snap.path.as_deref(), Some(Path::new("/tmp/talk.mp4")));
        assert!(!snap.playing);
        preview.play();
        assert!(preview.snapshot().playing);
        preview.pause();
        assert!(!preview.snapshot().playing);
        preview.seek_to_cue_start(1_250);
        assert_eq!(preview.snapshot().position_ms, 1_250);
        preview.detach();
        assert!(!preview.is_attached());
        assert_eq!(preview.snapshot().path, None);
    }

    #[test]
    fn cue_beyond_duration_clamps() {
        let mut preview = MediaPreview::new(FakeMediaBackend::with_duration(800));
        preview.attach(PathBuf::from("/tmp/short.m4a")).unwrap();
        preview.seek_to_cue_start(5_000);
        assert_eq!(preview.snapshot().position_ms, 800);
    }

    #[test]
    fn attach_failure_does_not_keep_media_bound() {
        let mut backend = FakeMediaBackend::with_duration(1_000);
        backend.fail_next_attach = true;
        let mut preview = MediaPreview::new(backend);
        let err = preview
            .attach(PathBuf::from("/tmp/missing.mp4"))
            .unwrap_err();
        assert!(matches!(err, MediaError::OpenFailed(_)));
        assert!(!preview.is_attached());
        assert!(preview.snapshot().error.is_some());
    }

    #[test]
    fn pending_cue_seek_applies_after_attach() {
        let mut preview = MediaPreview::new(FakeMediaBackend::with_duration(9_000));
        preview.seek_to_cue_start(2_000);
        assert!(!preview.is_attached());
        preview.attach(PathBuf::from("/tmp/talk.mp4")).unwrap();
        assert_eq!(preview.snapshot().position_ms, 2_000);
    }

    #[test]
    fn changing_media_replaces_path() {
        let mut preview = MediaPreview::new(FakeMediaBackend::with_duration(3_000));
        preview.attach(PathBuf::from("/tmp/a.mp4")).unwrap();
        preview.attach(PathBuf::from("/tmp/b.mp3")).unwrap();
        assert_eq!(
            preview.snapshot().path.as_deref(),
            Some(Path::new("/tmp/b.mp3"))
        );
    }

    #[test]
    fn explicit_cue_seek_is_not_overwritten_by_repaint_establish() {
        let mut intent = MediaSeekIntent::default();
        assert!(intent.on_event(0, false));
        assert!(intent.on_event(1, true));
        assert!(!intent.on_repaint_establish(0));
        assert!(!intent.on_repaint_establish(1));
        assert_eq!(intent.last_segment(), Some(1));
    }

    #[test]
    fn later_focus_event_may_seek_after_explicit_cue() {
        let mut intent = MediaSeekIntent::default();
        assert!(intent.on_event(1, true));
        assert!(intent.on_event(0, false));
        assert_eq!(intent.last_segment(), Some(0));
        assert!(!intent.on_event(0, false));
    }
}
