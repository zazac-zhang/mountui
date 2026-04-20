use std::path::{Path, PathBuf};

/// Resolve the config directory for mountui, creating it if needed.
pub fn config_dir() -> Option<PathBuf> {
    let proj = directories::ProjectDirs::from("", "", "mountui")?;
    let dir = proj.config_dir().to_path_buf();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).ok()?;
    }
    Some(dir)
}

/// Returns the path to the bookmarks TOML file.
pub fn bookmarks_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("bookmarks.toml"))
}

/// Ensures a mount point directory exists.
pub fn ensure_mount_point(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}
