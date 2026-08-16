use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DESKTOP_PRESENTATION_FILENAME: &str = "desktop-presentation.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopPresentation {
    pub source_display_name: String,
}

impl DesktopPresentation {
    pub fn path_for_session(session_root: &Path, session_id: &str) -> PathBuf {
        session_root.join(session_id).join(DESKTOP_PRESENTATION_FILENAME)
    }

    pub fn write_best_effort(session_root: &Path, session_id: &str, source_display_name: &str) {
        let trimmed = source_display_name.trim();
        if trimmed.is_empty() {
            return;
        }
        let session_dir = session_root.join(session_id);
        if fs::create_dir_all(&session_dir).is_err() {
            return;
        }
        let path = session_dir.join(DESKTOP_PRESENTATION_FILENAME);
        let payload = Self {
            source_display_name: trimmed.to_owned(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&payload) {
            let _ = fs::write(path, json);
        }
    }

    pub fn read_source_display_name(session_root: &Path, session_id: &str) -> Option<String> {
        let path = Self::path_for_session(session_root, session_id);
        let text = fs::read_to_string(path).ok()?;
        let payload: Self = serde_json::from_str(&text).ok()?;
        let trimmed = payload.source_display_name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_and_read_round_trip() {
        let temp = tempdir().unwrap();
        DesktopPresentation::write_best_effort(temp.path(), "session-a", "episode-03.srt");
        assert_eq!(
            DesktopPresentation::read_source_display_name(temp.path(), "session-a").as_deref(),
            Some("episode-03.srt")
        );
    }

    #[test]
    fn missing_or_invalid_sidecar_fails_soft() {
        let temp = tempdir().unwrap();
        assert!(DesktopPresentation::read_source_display_name(temp.path(), "missing").is_none());
        let path = temp.path().join("broken").join(DESKTOP_PRESENTATION_FILENAME);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "{ not json").unwrap();
        assert!(DesktopPresentation::read_source_display_name(temp.path(), "broken").is_none());
    }
}
