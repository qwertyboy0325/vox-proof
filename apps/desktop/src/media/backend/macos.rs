use std::path::{Path, PathBuf};

use egui::ColorImage;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_av_foundation::{
    AVMediaTypeVideo, AVPlayer, AVPlayerItemStatus, AVPlayerItemVideoOutput,
};
use objc2_core_foundation::CFString;
use objc2_core_media::{CMTime, kCMTimeZero};
use objc2_core_video::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight,
    CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelBufferPixelFormatTypeKey,
    kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use super::{MediaBackend, MediaError, MediaSnapshot};

const MAX_PREVIEW_WIDTH: usize = 640;

pub struct AvFoundationBackend {
    player: Option<Retained<AVPlayer>>,
    video_output: Option<Retained<AVPlayerItemVideoOutput>>,
    path: Option<PathBuf>,
    volume: f32,
    error: Option<String>,
    last_has_video: bool,
}

impl Default for AvFoundationBackend {
    fn default() -> Self {
        Self {
            player: None,
            video_output: None,
            path: None,
            volume: 1.0,
            error: None,
            last_has_video: false,
        }
    }
}

impl AvFoundationBackend {
    fn main_thread() -> Result<MainThreadMarker, MediaError> {
        MainThreadMarker::new().ok_or_else(|| {
            MediaError::OpenFailed("Media preview must run on the main thread.".to_owned())
        })
    }

    fn refresh_error_from_item(&mut self) {
        let Some(player) = &self.player else {
            return;
        };
        // SAFETY: player was created on the main thread and is only used there.
        let item = unsafe { player.currentItem() };
        let Some(item) = item else {
            return;
        };
        let status = unsafe { item.status() };
        if status == AVPlayerItemStatus::Failed {
            let message = unsafe { item.error() }
                .map(|error| error.localizedDescription().to_string())
                .unwrap_or_else(|| "Could not play the selected media file.".to_owned());
            self.error = Some(message);
            self.last_has_video = false;
        }
    }

    fn has_video_track(&self) -> bool {
        let Some(player) = &self.player else {
            return self.last_has_video;
        };
        let Some(item) = (unsafe { player.currentItem() }) else {
            return self.last_has_video;
        };
        if unsafe { item.status() } != AVPlayerItemStatus::ReadyToPlay {
            return self.last_has_video;
        }
        let tracks = unsafe { item.tracks() };
        for track in tracks.iter() {
            let Some(asset_track) = (unsafe { track.assetTrack() }) else {
                continue;
            };
            let media_type = unsafe { asset_track.mediaType() };
            if let Some(video) = unsafe { AVMediaTypeVideo } {
                if &*media_type == video {
                    return true;
                }
            }
        }
        false
    }
}

impl MediaBackend for AvFoundationBackend {
    fn attach(&mut self, path: &Path) -> Result<(), MediaError> {
        self.detach();
        let mtm = Self::main_thread()?;
        let Some(path_str) = path.to_str() else {
            let message = "Could not open the selected media file.".to_owned();
            self.error = Some(message.clone());
            return Err(MediaError::OpenFailed(message));
        };
        let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
        // SAFETY: AVPlayer creation and output wiring run on the main thread.
        let player = unsafe { AVPlayer::playerWithURL(&url, mtm) };
        unsafe { player.setVolume(self.volume) };

        let output = unsafe {
            let allocated = AVPlayerItemVideoOutput::alloc();
            AVPlayerItemVideoOutput::initWithPixelBufferAttributes(
                allocated,
                Some(&bgra_attributes()),
            )
        };
        unsafe { output.setSuppressesPlayerRendering(true) };
        if let Some(item) = unsafe { player.currentItem() } {
            unsafe { item.addOutput(&output) };
        }

        self.player = Some(player);
        self.video_output = Some(output);
        self.path = Some(path.to_path_buf());
        self.error = None;
        self.last_has_video = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp4" | "mov" | "m4v" | "avi"
                )
            });
        self.refresh_error_from_item();
        if let Some(message) = &self.error {
            let message = message.clone();
            self.detach();
            self.error = Some(message.clone());
            return Err(MediaError::OpenFailed(message));
        }
        Ok(())
    }

    fn detach(&mut self) {
        if let Some(player) = self.player.take() {
            unsafe { player.pause() };
        }
        self.video_output = None;
        self.path = None;
        self.error = None;
        self.last_has_video = false;
    }

    fn play(&mut self) {
        let Some(player) = &self.player else {
            return;
        };
        unsafe { player.play() };
        self.refresh_error_from_item();
    }

    fn pause(&mut self) {
        let Some(player) = &self.player else {
            return;
        };
        unsafe { player.pause() };
    }

    fn seek_ms(&mut self, ms: u64) {
        let Some(player) = &self.player else {
            return;
        };
        let time = cue_time(ms);
        // Cue navigation is evidence-facing: request the exact cue boundary rather
        // than AVPlayer's default efficiency-oriented seek tolerance.
        unsafe { player.seekToTime_toleranceBefore_toleranceAfter(time, kCMTimeZero, kCMTimeZero) };
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = &self.player {
            unsafe { player.setVolume(self.volume) };
        }
    }

    fn snapshot(&self) -> MediaSnapshot {
        let mut snapshot = MediaSnapshot {
            path: self.path.clone(),
            playing: false,
            position_ms: 0,
            duration_ms: None,
            volume: self.volume,
            has_video: self.last_has_video,
            error: self.error.clone(),
        };
        let Some(player) = &self.player else {
            return snapshot;
        };
        snapshot.playing = unsafe { player.rate() } != 0.0;
        let current = unsafe { player.currentTime() };
        let seconds = unsafe { current.seconds() };
        if seconds.is_finite() && seconds >= 0.0 {
            snapshot.position_ms = (seconds * 1000.0).round() as u64;
        }
        if let Some(item) = unsafe { player.currentItem() } {
            let duration = unsafe { item.duration() };
            let duration_seconds = unsafe { duration.seconds() };
            if duration_seconds.is_finite() && duration_seconds >= 0.0 {
                snapshot.duration_ms = Some((duration_seconds * 1000.0).round() as u64);
            }
            if unsafe { item.status() } == AVPlayerItemStatus::Failed {
                snapshot.error = unsafe { item.error() }
                    .map(|error| error.localizedDescription().to_string())
                    .or(snapshot.error);
            }
        }
        snapshot.has_video = self.has_video_track();
        snapshot
    }

    fn take_video_frame(&mut self) -> Option<ColorImage> {
        self.refresh_error_from_item();
        let player = self.player.as_ref()?;
        let output = self.video_output.as_ref()?;
        let item_time = unsafe { player.currentTime() };
        if !unsafe { output.hasNewPixelBufferForItemTime(item_time) } {
            return None;
        }
        let buffer = unsafe {
            output.copyPixelBufferForItemTime_itemTimeForDisplay(item_time, std::ptr::null_mut())
        }?;
        color_image_from_pixel_buffer(&buffer)
    }
}

fn cue_time(ms: u64) -> CMTime {
    let value = i64::try_from(ms).unwrap_or(i64::MAX);
    // Millisecond cue starts are exactly representable at this timescale.
    unsafe { CMTime::new(value, 1_000) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_time_preserves_millisecond_boundaries() {
        let time = cue_time(1_250);
        assert_eq!(unsafe { time.seconds() }, 1.25);
    }
}

fn bgra_attributes() -> Retained<NSDictionary<NSString, AnyObject>> {
    let key = unsafe { cfstring_as_nsstring(kCVPixelBufferPixelFormatTypeKey) };
    let value = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
    let value: Retained<AnyObject> = unsafe { Retained::cast_unchecked(value) };
    NSDictionary::from_slices(&[key], &[&*value])
}

unsafe fn cfstring_as_nsstring(cf: &'static CFString) -> &'static NSString {
    unsafe { &*(cf as *const CFString).cast::<NSString>() }
}

fn color_image_from_pixel_buffer(buffer: &objc2_core_video::CVPixelBuffer) -> Option<ColorImage> {
    let format = CVPixelBufferGetPixelFormatType(buffer);
    if format != kCVPixelFormatType_32BGRA {
        return None;
    }
    let lock = unsafe { CVPixelBufferLockBaseAddress(buffer, CVPixelBufferLockFlags::ReadOnly) };
    if lock != kCVReturnSuccess {
        return None;
    }
    let image = (|| {
        let width = CVPixelBufferGetWidth(buffer);
        let height = CVPixelBufferGetHeight(buffer);
        let stride = CVPixelBufferGetBytesPerRow(buffer);
        if width == 0 || height == 0 {
            return None;
        }
        let base = CVPixelBufferGetBaseAddress(buffer);
        if base.is_null() {
            return None;
        }
        let src = unsafe { std::slice::from_raw_parts(base as *const u8, stride * height) };
        Some(bgra_to_color_image(src, width, height, stride))
    })();
    let _ = unsafe { CVPixelBufferUnlockBaseAddress(buffer, CVPixelBufferLockFlags::ReadOnly) };
    image
}

fn bgra_to_color_image(src: &[u8], width: usize, height: usize, stride: usize) -> ColorImage {
    let scale = if width > MAX_PREVIEW_WIDTH {
        MAX_PREVIEW_WIDTH as f32 / width as f32
    } else {
        1.0
    };
    let out_w = ((width as f32) * scale).round().max(1.0) as usize;
    let out_h = ((height as f32) * scale).round().max(1.0) as usize;
    let mut pixels = Vec::with_capacity(out_w * out_h);
    for y in 0..out_h {
        let src_y = (y as f32 / scale).floor() as usize;
        let row = &src[src_y * stride..];
        for x in 0..out_w {
            let src_x = (x as f32 / scale).floor() as usize;
            let i = src_x * 4;
            let b = row[i];
            let g = row[i + 1];
            let r = row[i + 2];
            let a = row[i + 3];
            pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
        }
    }
    ColorImage::new([out_w, out_h], pixels)
}
