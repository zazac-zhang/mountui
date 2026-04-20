# MountUI

TUI filesystem mount manager for macOS and Linux.

## Build & Run

```bash
cargo build
cargo run
```

## Project Structure

```
src/
├── main.rs          # Entry point, tokio runtime, terminal setup
├── app.rs           # App state machine, event handling, business logic
├── bookmark.rs      # Bookmark model and TOML persistence
├── config.rs        # Config directory and file paths
├── mount/
│   ├── mod.rs       # MountAdapter trait and common types
│   ├── macos.rs     # macOS mount/umount implementation
│   └── linux.rs     # Linux mount/umount implementation
└── ui/
    ├── mod.rs       # Layout, tabs, status/help bars
    ├── mount_list.rs     # Mounts tab (Tab 1)
    ├── bookmark_list.rs  # Bookmarks tab (Tab 2)
    └── mount_point.rs    # Mount Points tab (Tab 3)
```

## Architecture

- **UI**: ratatui + crossterm, renders every 100ms event poll cycle
- **State**: `App` struct holds all state; modes: Normal / Search / Form
- **Async**: tokio runtime for mount/umount; results via `std::sync::mpsc`
- **Platform**: `MountAdapter` trait with macOS and Linux implementations
- **Config**: Bookmarks stored in TOML (`~/Library/Application Support/mountui/` or `~/.config/mountui/`)

## Key Bindings

| Key | Action |
|-----|--------|
| `1-3` / `F1-F3` / `Tab` | Switch tabs |
| `j/k` / Arrow keys | Navigate |
| `/` | Search |
| `q` | Quit |

### Tab-specific

- **Mounts**: `u` Unmount, `r` Refresh
- **Bookmarks**: `m` Mount, `d` Delete, `a` Add, `e` Edit
- **Mount Points**: `x` Remove empty unmounted mount point
