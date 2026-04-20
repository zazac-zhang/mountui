use serde::{Deserialize, Serialize};

/// Supported remote filesystem protocols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Sshfs,
    Nfs,
    Smb,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Sshfs => write!(f, "sshfs"),
            Protocol::Nfs => write!(f, "nfs"),
            Protocol::Smb => write!(f, "smb"),
        }
    }
}

/// A saved bookmark for a remote mount target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    pub remote_path: String,
    pub mount_point: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub options: Option<String>,
}

/// Wrapper for TOML serialization: bookmarks stored as `[[bookmark]]` array.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BookmarkFile {
    pub bookmark: Vec<Bookmark>,
}

/// Load bookmarks from a TOML file.
pub fn load_bookmarks(path: &std::path::Path) -> Result<Vec<Bookmark>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)?;
    let file: BookmarkFile = toml::from_str(&content)?;
    Ok(file.bookmark)
}

/// Save bookmarks to a TOML file.
pub fn save_bookmarks(
    path: &std::path::Path,
    bookmarks: &[Bookmark],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = BookmarkFile {
        bookmark: bookmarks.to_vec(),
    };
    let content = toml::to_string_pretty(&file)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}
