use std::path::PathBuf;

pub fn resolve_session_root() -> PathBuf {
    if let Ok(root) = std::env::var("VOXPROOF_SESSION_ROOT") {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_app_data_session_root()
}

fn default_app_data_session_root() -> PathBuf {
    directories::ProjectDirs::from("com", "VoxProof", "VoxProof")
        .map(|dirs| dirs.data_local_dir().join("sessions"))
        .unwrap_or_else(|| PathBuf::from("sessions"))
}
