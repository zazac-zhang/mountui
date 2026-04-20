use std::path::Path;
use std::process::Command as StdCommand;

use async_trait::async_trait;

use crate::bookmark::{Bookmark, Protocol};
use crate::mount::{DiskUsage, MountAdapter, MountEntry, MountError, Result};

pub struct MacOsAdapter;

#[async_trait]
impl MountAdapter for MacOsAdapter {
    fn list_mounts(&self) -> Result<Vec<MountEntry>> {
        let output = StdCommand::new("mount").output()?;
        if !output.status.success() {
            return Err(MountError::CommandFailed("mount command failed".into()));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        for line in text.lines() {
            if let Some(entry) = parse_mount_line(line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    async fn mount(&self, bookmark: &Bookmark) -> Result<()> {
        let mut cmd = build_mount_command(bookmark);
        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MountError::MountFailed(stderr.to_string()));
        }
        Ok(())
    }

    async fn unmount(&self, mount_point: &Path) -> Result<()> {
        let mp = mount_point.to_string_lossy().to_string();
        let output = tokio::process::Command::new("umount")
            .arg(&mp)
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Try diskutil as fallback on macOS
            let output2 = tokio::process::Command::new("diskutil")
                .args(["unmount", &mp])
                .output()
                .await?;
            if !output2.status.success() {
                let stderr2 = String::from_utf8_lossy(&output2.stderr);
                return Err(MountError::UnmountFailed(format!("{stderr}\n{stderr2}")));
            }
        }
        Ok(())
    }

    fn create_mount_point(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }
        Ok(())
    }

    fn disk_usage(&self, path: &Path) -> Result<DiskUsage> {
        let output = StdCommand::new("df")
            .env("LC_ALL", "C")
            .args(["-k", &path.to_string_lossy()])
            .output()?;
        if !output.status.success() {
            return Err(MountError::CommandFailed("df command failed".into()));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        parse_df_output(&text)
    }
}

/// Parse a macOS mount line: "device on /mount/point (fstype, options)"
fn parse_mount_line(line: &str) -> Option<MountEntry> {
    let (device, rest) = line.split_once(" on ")?;
    let (mount_point, rest) = rest.split_once(" (")?;
    let rest = rest.strip_suffix(')').unwrap_or(rest);
    let fs_type = rest
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    Some(MountEntry {
        device: device.to_string(),
        mount_point: mount_point.into(),
        fs_type,
        options: rest.to_string(),
    })
}

fn build_mount_command(bookmark: &Bookmark) -> tokio::process::Command {
    match bookmark.protocol {
        Protocol::Sshfs => {
            let mut sshfs_cmd = tokio::process::Command::new("sshfs");
            let remote = format_remote_path(bookmark);
            sshfs_cmd.arg(&remote);
            sshfs_cmd.arg(&bookmark.mount_point);
            if let Some(ref opts) = bookmark.options {
                sshfs_cmd.args(["-o", opts]);
            }
            if let Some(ref port) = bookmark.port {
                sshfs_cmd.args(["-p", &port.to_string()]);
            }
            sshfs_cmd
        }
        Protocol::Nfs => {
            let mut cmd = tokio::process::Command::new("mount");
            let remote = format!("{}:{}", bookmark.host, bookmark.remote_path);
            cmd.arg("-t")
                .arg("nfs")
                .arg(&remote)
                .arg(&bookmark.mount_point);
            if let Some(ref opts) = bookmark.options {
                cmd.args(["-o", opts]);
            }
            cmd
        }
        Protocol::Smb => {
            let mut cmd = tokio::process::Command::new("mount");
            let remote = format_smb_remote(bookmark);
            cmd.arg("-t")
                .arg("smbfs")
                .arg(&remote)
                .arg(&bookmark.mount_point);
            if let Some(ref opts) = bookmark.options {
                cmd.args(["-o", opts]);
            }
            cmd
        }
    }
}

fn format_remote_path(bookmark: &Bookmark) -> String {
    match &bookmark.username {
        Some(user) => format!("{}@{}:{}", user, bookmark.host, bookmark.remote_path),
        None => format!("{}:{}", bookmark.host, bookmark.remote_path),
    }
}

fn format_smb_remote(bookmark: &Bookmark) -> String {
    match &bookmark.username {
        Some(user) => format!(
            "//{}@{}/{}",
            user,
            bookmark.host,
            bookmark.remote_path.trim_start_matches('/')
        ),
        None => format!(
            "//{}/{}",
            bookmark.host,
            bookmark.remote_path.trim_start_matches('/')
        ),
    }
}

fn parse_df_output(text: &str) -> Result<DiskUsage> {
    let mut lines = text.lines();
    lines.next(); // skip header
    let data_line = lines
        .next()
        .ok_or_else(|| MountError::Parse("no data line in df output".into()))?;
    let fields: Vec<&str> = data_line.split_whitespace().collect();
    if fields.len() < 4 {
        return Err(MountError::Parse("unexpected df output format".into()));
    }
    let total: u64 = fields[1]
        .parse::<u64>()
        .map_err(|_| MountError::Parse("invalid total".into()))?
        * 1024;
    let used: u64 = fields[2]
        .parse::<u64>()
        .map_err(|_| MountError::Parse("invalid used".into()))?
        * 1024;
    let available: u64 = fields[3]
        .parse::<u64>()
        .map_err(|_| MountError::Parse("invalid available".into()))?
        * 1024;
    Ok(DiskUsage {
        total,
        used,
        available,
    })
}
