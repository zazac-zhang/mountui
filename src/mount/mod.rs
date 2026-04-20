#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use thiserror::Error;

use crate::bookmark::Bookmark;

#[derive(Error, Debug)]
pub enum MountError {
    #[error("Failed to execute command: {0}")]
    CommandFailed(String),
    #[error("Mount failed: {0}")]
    MountFailed(String),
    #[error("Unmount failed: {0}")]
    UnmountFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, MountError>;

/// A single mount entry from the system.
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: PathBuf,
    pub fs_type: String,
    pub options: String,
}

/// Disk usage information for a mount point.
#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

impl DiskUsage {
    pub fn usage_percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.used as f64 / self.total as f64) * 100.0
        }
    }
}

/// Platform abstraction for mount operations.
#[async_trait]
pub trait MountAdapter: Send + Sync {
    /// List all currently mounted filesystems.
    fn list_mounts(&self) -> Result<Vec<MountEntry>>;
    /// Mount a remote filesystem described by a bookmark.
    async fn mount(&self, bookmark: &Bookmark) -> Result<()>;
    /// Unmount the filesystem at the given mount point.
    async fn unmount(&self, mount_point: &Path) -> Result<()>;
    /// Create the mount point directory if it doesn't exist.
    fn create_mount_point(&self, path: &Path) -> Result<()>;
    /// Get disk usage for a mount point.
    fn disk_usage(&self, path: &Path) -> Result<DiskUsage>;
}

/// Returns the appropriate adapter for the current platform.
pub fn platform_adapter() -> Box<dyn MountAdapter> {
    #[cfg(target_os = "macos")]
    {
        Box::new(crate::mount::macos::MacOsAdapter)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(crate::mount::linux::LinuxAdapter)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported platform. Only macOS and Linux are supported.");
    }
}
