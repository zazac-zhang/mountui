# MountUI Design Spec

## Context

Managing remote filesystem mounts (SSHFS/NFS/SMB) on macOS and Linux is tedious: users must remember host addresses, paths, mount commands, and manually create mount points. MountUI is a ratatui-based TUI tool that provides a unified interface for viewing mounts, managing server bookmarks, and performing mount/umount operations quickly.

## Architecture

Async layered component architecture with platform abstraction.

```
┌─────────────────────────────────┐
│         TUI Layer (ratatui)      │
│  ┌──────────┐ ┌───────────────┐ │
│  │ MountList│ │ BookmarkPanel │ │
│  └──────────┘ └───────────────┘ │
├─────────────────────────────────┤
│       App State (tokio mpsc)     │
├─────────────────────────────────┤
│  MountAdapter │ BookmarkStore    │
│  (sys mount)  │ (TOML config)   │
└─────────────────────────────────┘
```

## Project Structure

```
mountui/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, init tokio runtime
│   ├── app.rs               # App state machine, core business logic
│   ├── ui/
│   │   ├── mod.rs            # UI render entry
│   │   ├── mount_list.rs     # Mounted filesystems view
│   │   ├── bookmark_list.rs  # Bookmarks view
│   │   └── mount_point.rs    # Mount point management view
│   ├── mount/
│   │   ├── mod.rs            # MountAdapter trait
│   │   ├── linux.rs          # Linux implementation
│   │   └── macos.rs          # macOS implementation
│   ├── bookmark.rs           # Bookmark data model + TOML persistence
│   └── config.rs             # Config file path management
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | TUI framework + terminal backend |
| `tokio` (full) | Async runtime for non-blocking mount operations |
| `serde` + `toml` | Config serialization/deserialization |
| `directories` | Cross-platform config directory resolution |
| `clap` (derive) | CLI argument parsing (optional: mount a bookmark directly) |
| `thiserror` | Error type definitions |
| `async-trait` | Async trait for MountAdapter |

## Data Models

### Bookmark

```rust
#[derive(Serialize, Deserialize, Clone)]
struct Bookmark {
    name: String,
    protocol: Protocol,
    host: String,
    port: Option<u16>,
    remote_path: String,
    mount_point: String,
    username: Option<String>,
    options: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
enum Protocol {
    Sshfs,
    Nfs,
    Smb,
}
```

### MountEntry (runtime)

```rust
struct MountEntry {
    device: String,
    mount_point: PathBuf,
    fs_type: String,
    options: String,
}

struct DiskUsage {
    total: u64,
    used: u64,
    available: u64,
}
```

### Persistence

Config file location (via `directories` crate):
- macOS: `~/Library/Application Support/mountui/bookmarks.toml`
- Linux: `~/.config/mountui/bookmarks.toml`

Format: TOML array of `[[bookmark]]` tables.

## TUI Layout

```
┌─ MountUI ──── [1:Mounts] [2:Bookmarks] [3:Mount Points] ────┐
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Device          MountPoint    Type     Options         │  │
│  │  ─────────────────────────────────────────────────────  │  │
│  │> /dev/sda1       /             ext4     rw,relatime     │  │
│  │  192.168.1.100:/ /mnt/nas      nfs      rw             │  │
│  │  //nas/share     /mnt/smb      smbfs    rw             │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
│  [m]ount  [u]nmount  [a]dd bookmark  [e]dit  [d]elete  [q]uit│
└───────────────────────────────────────────────────────────────┘
```

Three tab views:

1. **Mounts** — All current system mounts, highlights remote mounts (SSHFS/NFS/SMB), `[u]nmount support
2. **Bookmarks** — All saved bookmarks, `[m]ount quick mount, `[a]dd`/`[e]dit`/`[d]elete` management
3. **Mount Points** — Manage local mount point directories, show disk usage

### Keybindings

- `j/k` or `Up/Down` — Navigate list
- `1/2/3` or `Tab` — Switch views
- `Enter` or action key — Execute operation
- `/` — Search/filter
- `q` — Quit
- Inline form input for add/edit bookmark (not popup dialog)

### Async Mount Feedback

- Mount operations execute in tokio task
- Status bar shows "Mounting {name}..." during operation
- Success/failure reported via status bar toast

## Mount Adapter Layer

```rust
#[async_trait]
trait MountAdapter: Send + Sync {
    fn list_mounts(&self) -> Result<Vec<MountEntry>>;
    async fn mount(&self, bookmark: &Bookmark) -> Result<()>;
    async fn unmount(&self, mount_point: &Path) -> Result<()>;
    fn create_mount_point(&self, path: &Path) -> Result<()>;
    fn disk_usage(&self, path: &Path) -> Result<DiskUsage>;
}
```

### Platform Differences

| Operation | Linux | macOS |
|-----------|-------|-------|
| List mounts | Parse `/proc/mounts` | Parse `mount` command output |
| SSHFS mount | `sshfs user@host:path /mnt/pt` | Same |
| NFS mount | `mount -t nfs host:path /mnt/pt` | `mount_nfs host:path /mnt/pt` |
| SMB mount | `mount -t cifs //host/share /mnt/pt -o user=` | `mount_smbfs //user@host/share /mnt/pt` |
| Unmount | `umount /mnt/pt` | `umount /mnt/pt` or `diskutil unmount` |

### Error Handling

- Mount/umount failure: capture stderr, display in status bar
- Common errors (network unreachable, permission denied): friendly messages
- All errors use `thiserror` error types

## Privilege Handling

Tool does NOT handle privilege escalation internally. Users must:
- Run the tool with `sudo` directly, or
- Configure `/etc/sudoers` with NOPASSWD entries for mount commands

## Verification

1. `cargo build` compiles without warnings
2. `cargo clippy` passes
3. Launch TUI, verify:
   - Mounts tab shows current system mounts
   - Bookmarks tab displays empty state, can add/edit/delete bookmarks
   - Mount Points tab shows directories and disk usage
   - Mount/unmount operations work on target platform
   - Config persists to TOML and reloads on restart
