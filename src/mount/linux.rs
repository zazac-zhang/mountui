use std::path::Path;
use std::process::Command as StdCommand;

use async_trait::async_trait;

use crate::bookmark::{Bookmark, Protocol};
use crate::mount::{DiskUsage, MountAdapter, MountEntry, MountError, Result};

pub struct LinuxAdapter;

#[async_trait]
impl MountAdapter for LinuxAdapter {
    fn list_mounts(&self) -> Result<Vec<MountEntry>> {
        let content = std::fs::read_to_string("/proc/mounts")?;
        let mut entries = Vec::new();
        for line in content.lines() {
            if let Some(entry) = parse_proc_mounts_line(line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    async fn mount(&self, bookmark: &Bookmark) -> Result<()> {
        let cmd = build_mount_command(bookmark);
        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MountError::MountFailed(stderr.to_string()));
        }
        Ok(())
    }

    async fn unmount(&self, mount_point: &Path) -> Result<()> {
        let output = tokio::process::Command::new("umount")
            .arg(mount_point)
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MountError::UnmountFailed(stderr.to_string()));
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

/// Parse a /proc/mounts line: "device /mount/point fstype options dump pass"
fn parse_proc_mounts_line(line: &str) -> Option<MountEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    Some(MountEntry {
        device: parts[0].to_string(),
        mount_point: parts[1].into(),
        fs_type: parts[2].to_string(),
        options: parts[3].to_string(),
    })
}

fn build_mount_command(bookmark: &Bookmark) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("mount");
    match bookmark.protocol {
        Protocol::Sshfs => {
            let mut sshfs_cmd = tokio::process::Command::new("sshfs");
            let remote = match &bookmark.username {
                Some(user) => format!("{}@{}:{}", user, bookmark.host, bookmark.remote_path),
                None => format!("{}:{}", bookmark.host, bookmark.remote_path),
            };
            sshfs_cmd.arg(&remote).arg(&bookmark.mount_point);
            if let Some(ref opts) = bookmark.options {
                sshfs_cmd.args(["-o", opts]);
            }
            if let Some(ref port) = bookmark.port {
                sshfs_cmd.args(["-p", &port.to_string()]);
            }
            return sshfs_cmd;
        }
        Protocol::Nfs => {
            let remote = format!("{}:{}", bookmark.host, bookmark.remote_path);
            cmd.arg("-t").arg("nfs").arg(&remote).arg(&bookmark.mount_point);
            if let Some(ref opts) = bookmark.options {
                cmd.args(["-o", opts]);
            }
        }
        Protocol::Smb => {
            let remote = format!(
                "//{}/{}",
                bookmark.host,
                bookmark.remote_path.trim_start_matches('/')
            );
            cmd.arg("-t")
                .arg("cifs")
                .arg(&remote)
                .arg(&bookmark.mount_point);
            if let Some(ref user) = bookmark.username {
                cmd.args(["-o", &format!("user={user}")]);
            }
            if let Some(ref opts) = bookmark.options {
                cmd.args(["-o", opts]);
            }
        }
    }
    cmd
}

fn parse_df_output(text: &str) -> Result<DiskUsage> {
    let mut lines = text.lines();
    lines.next(); // skip header
    let data_line =
        lines.next().ok_or_else(|| MountError::Parse("no data line in df output".into()))?;
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
