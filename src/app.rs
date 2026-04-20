use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::bookmark::{self, Bookmark, Protocol};
use crate::mount::{self, DiskUsage, MountEntry};

/// Result from an async mount/umount operation.
pub enum AsyncResult {
    Mount {
        name: String,
        result: Result<(), mount::MountError>,
    },
    Unmount {
        mount_point: PathBuf,
        result: Result<(), mount::MountError>,
    },
    RefreshMounts(Vec<MountEntry>),
}

/// Active tab in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Mounts,
    Bookmarks,
    MountPoints,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Mounts, Tab::Bookmarks, Tab::MountPoints];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Mounts => "Mounts",
            Tab::Bookmarks => "Bookmarks",
            Tab::MountPoints => "Mount Points",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Mounts => 0,
            Tab::Bookmarks => 1,
            Tab::MountPoints => 2,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Tab::Mounts,
            1 => Tab::Bookmarks,
            _ => Tab::MountPoints,
        }
    }
}

/// Application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Form,
}

/// Status bar message.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub kind: StatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Error,
}

/// Form fields for adding/editing a bookmark.
#[derive(Debug, Clone)]
pub struct FormData {
    pub fields: [String; FormField::COUNT],
    pub cursor_field: usize,
    pub editing: bool,                 // true = edit existing, false = add new
    pub original_name: Option<String>, // for edit mode
}

impl FormData {
    pub fn new_for_add() -> Self {
        Self {
            fields: [
                String::new(),       // name
                "sshfs".to_string(), // protocol (default)
                String::new(),       // host
                String::new(),       // port
                String::new(),       // remote_path
                String::new(),       // mount_point
                String::new(),       // username
                String::new(),       // options
            ],
            cursor_field: 0,
            editing: false,
            original_name: None,
        }
    }

    pub fn new_for_edit(bookmark: &Bookmark) -> Self {
        Self {
            fields: [
                bookmark.name.clone(),
                bookmark.protocol.to_string(),
                bookmark.host.clone(),
                bookmark.port.map(|p| p.to_string()).unwrap_or_default(),
                bookmark.remote_path.clone(),
                bookmark.mount_point.clone(),
                bookmark.username.clone().unwrap_or_default(),
                bookmark.options.clone().unwrap_or_default(),
            ],
            cursor_field: 0,
            editing: true,
            original_name: Some(bookmark.name.clone()),
        }
    }

    pub fn to_bookmark(&self) -> Option<Bookmark> {
        let name = self.fields[FormField::Name as usize].trim().to_string();
        let host = self.fields[FormField::Host as usize].trim().to_string();
        let mount_point = self.fields[FormField::MountPoint as usize]
            .trim()
            .to_string();
        if name.is_empty() || host.is_empty() || mount_point.is_empty() {
            return None;
        }
        let protocol = match self.fields[FormField::Protocol as usize].trim() {
            "nfs" => Protocol::Nfs,
            "smb" => Protocol::Smb,
            _ => Protocol::Sshfs,
        };
        let port = self.fields[FormField::Port as usize]
            .trim()
            .parse::<u16>()
            .ok();
        let remote_path = self.fields[FormField::RemotePath as usize]
            .trim()
            .to_string();
        let username = {
            let s = self.fields[FormField::Username as usize].trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        };
        let options = {
            let s = self.fields[FormField::Options as usize].trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        };
        Some(Bookmark {
            name,
            protocol,
            host,
            port,
            remote_path,
            mount_point,
            username,
            options,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FormField {
    Name = 0,
    Protocol = 1,
    Host = 2,
    Port = 3,
    RemotePath = 4,
    MountPoint = 5,
    Username = 6,
    Options = 7,
}

impl FormField {
    pub const COUNT: usize = 8;
    pub const LABELS: [&str; Self::COUNT] = [
        "Name",
        "Protocol (sshfs/nfs/smb)",
        "Host",
        "Port",
        "Remote Path",
        "Mount Point",
        "Username",
        "Options",
    ];
}

/// The core application state.
pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub running: bool,
    pub mounts: Vec<MountEntry>,
    pub bookmarks: Vec<Bookmark>,
    pub mount_points: Vec<PathBuf>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub search_query: String,
    pub status: Option<StatusMessage>,
    pub mount_pending: bool,
    pub form: Option<FormData>,
    pub disk_usage_cache: HashMap<PathBuf, (DiskUsage, Instant)>,
    bookmarks_path: PathBuf,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let bookmarks_path =
            crate::config::bookmarks_path().unwrap_or_else(|| PathBuf::from("bookmarks.toml"));

        let bookmarks = bookmark::load_bookmarks(&bookmarks_path).unwrap_or_default();

        let adapter = mount::platform_adapter();
        let mounts = adapter.list_mounts().unwrap_or_default();

        // Collect mount points from both current mounts and bookmarks
        let mut mount_points: Vec<PathBuf> = mounts.iter().map(|m| m.mount_point.clone()).collect();
        for b in &bookmarks {
            let mp = PathBuf::from(&b.mount_point);
            if !mount_points.contains(&mp) {
                mount_points.push(mp);
            }
        }

        Self {
            tab: Tab::Mounts,
            mode: Mode::Normal,
            running: true,
            mounts,
            bookmarks,
            mount_points,
            cursor: 0,
            scroll_offset: 0,
            search_query: String::new(),
            status: None,
            mount_pending: false,
            form: None,
            disk_usage_cache: HashMap::new(),
            bookmarks_path,
        }
    }

    pub fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        tx: &std::sync::mpsc::Sender<AsyncResult>,
    ) {
        use crossterm::event::KeyEventKind;
        if key.kind != KeyEventKind::Press {
            return;
        }

        match self.mode {
            Mode::Normal => self.handle_normal_mode(key.code, tx),
            Mode::Search => self.handle_search_mode(key.code),
            Mode::Form => self.handle_form_mode(key.code),
        }
    }

    fn handle_normal_mode(
        &mut self,
        code: crossterm::event::KeyCode,
        tx: &std::sync::mpsc::Sender<AsyncResult>,
    ) {
        use crossterm::event::KeyCode;
        match code {
            KeyCode::Char('q') => {
                self.running = false;
            }
            KeyCode::Char('1') | KeyCode::F(1) => self.switch_tab(Tab::Mounts),
            KeyCode::Char('2') | KeyCode::F(2) => self.switch_tab(Tab::Bookmarks),
            KeyCode::Char('3') | KeyCode::F(3) => self.switch_tab(Tab::MountPoints),
            KeyCode::Tab => {
                let next = (self.tab.index() + 1) % Tab::ALL.len();
                self.switch_tab(Tab::from_index(next));
            }
            KeyCode::BackTab => {
                let prev = if self.tab.index() == 0 {
                    Tab::ALL.len() - 1
                } else {
                    self.tab.index() - 1
                };
                self.switch_tab(Tab::from_index(prev));
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(-1),
            KeyCode::Char('G') | KeyCode::End => self.move_cursor_end(),
            KeyCode::Char('g') | KeyCode::Home => self.move_cursor_home(),
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.search_query.clear();
            }
            KeyCode::Char('r') => self.refresh_mounts_async(tx),
            KeyCode::Char('u') => self.handle_unmount(tx),
            KeyCode::Char('m') => self.handle_mount(tx),
            KeyCode::Char('d') => self.handle_delete(),
            KeyCode::Char('a') => {
                self.form = Some(FormData::new_for_add());
                self.mode = Mode::Form;
            }
            KeyCode::Char('e') => {
                if self.tab == Tab::Bookmarks
                    && let Some(bm) = self.get_bookmark_at_cursor()
                {
                    self.form = Some(FormData::new_for_edit(&bm));
                    self.mode = Mode::Form;
                }
            }
            KeyCode::Char('x') => {
                if self.tab == Tab::MountPoints {
                    self.handle_remove_mount_point();
                }
            }
            _ => {}
        }
    }

    fn handle_search_mode(&mut self, code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_query.clear();
                self.cursor = 0;
                self.scroll_offset = 0;
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.cursor = 0;
                self.scroll_offset = 0;
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.cursor = 0;
                self.scroll_offset = 0;
            }
            _ => {}
        }
    }

    fn handle_form_mode(&mut self, code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match code {
            KeyCode::Esc => {
                self.form = None;
                self.mode = Mode::Normal;
            }
            KeyCode::Tab => {
                if let Some(ref mut form) = self.form {
                    form.cursor_field = (form.cursor_field + 1) % FormField::COUNT;
                }
            }
            KeyCode::BackTab => {
                if let Some(ref mut form) = self.form {
                    form.cursor_field = if form.cursor_field == 0 {
                        FormField::COUNT - 1
                    } else {
                        form.cursor_field - 1
                    };
                }
            }
            KeyCode::Enter => {
                self.submit_form();
            }
            KeyCode::Backspace => {
                if let Some(ref mut form) = self.form {
                    form.fields[form.cursor_field].pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut form) = self.form {
                    form.fields[form.cursor_field].push(c);
                }
            }
            _ => {}
        }
    }

    fn submit_form(&mut self) {
        let form_data = self.form.take();
        let form_data = match form_data {
            Some(f) => f,
            None => return,
        };

        let bm = match form_data.to_bookmark() {
            Some(b) => b,
            None => {
                self.set_status(
                    "Name, Host, and Mount Point are required",
                    StatusKind::Error,
                );
                self.form = Some(form_data);
                return;
            }
        };

        if form_data.editing {
            // Replace existing bookmark
            if let Some(pos) = self
                .bookmarks
                .iter()
                .position(|b| Some(&b.name) == form_data.original_name.as_ref())
            {
                self.bookmarks[pos] = bm;
            }
        } else {
            // Check for duplicate name
            if self.bookmarks.iter().any(|b| b.name == bm.name) {
                self.set_status(
                    &format!("Bookmark '{}' already exists", bm.name),
                    StatusKind::Error,
                );
                self.form = Some(form_data);
                return;
            }
            self.bookmarks.push(bm);
        }

        if let Err(e) = bookmark::save_bookmarks(&self.bookmarks_path, &self.bookmarks) {
            self.set_status(&format!("Failed to save: {e}"), StatusKind::Error);
        } else {
            let action = if form_data.editing {
                "Updated"
            } else {
                "Added"
            };
            self.set_status(
                &format!(
                    "{} bookmark '{}'",
                    action,
                    self.bookmarks.last().map(|b| b.name.as_str()).unwrap_or("")
                ),
                StatusKind::Success,
            );
        }
        self.mode = Mode::Normal;
        self.tab = Tab::Bookmarks;
    }

    fn switch_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.cursor = 0;
        self.scroll_offset = 0;
        if tab == Tab::Mounts {
            self.handle_refresh_mounts();
        }
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.filtered_items_count();
        if len == 0 {
            return;
        }
        let new = self.cursor as i32 + delta;
        self.cursor = if new < 0 {
            len - 1
        } else if new as usize >= len {
            0
        } else {
            new as usize
        };
    }

    fn move_cursor_end(&mut self) {
        let len = self.filtered_items_count();
        if len > 0 {
            self.cursor = len - 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    pub fn refresh_mounts_async(&self, tx: &std::sync::mpsc::Sender<AsyncResult>) {
        let tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let adapter = mount::platform_adapter();
            let mounts = adapter.list_mounts().unwrap_or_default();
            let _ = tx.send(AsyncResult::RefreshMounts(mounts));
        });
    }

    fn handle_refresh_mounts(&mut self) {
        let adapter = mount::platform_adapter();
        self.mounts = adapter.list_mounts().unwrap_or_default();
    }

    fn handle_unmount(&mut self, tx: &std::sync::mpsc::Sender<AsyncResult>) {
        if self.mount_pending {
            self.set_status("Operation already in progress", StatusKind::Info);
            return;
        }
        let entry = {
            let filtered = self.filtered_mounts();
            filtered.get(self.cursor).cloned()
        };
        if let Some(entry) = entry {
            let mp = entry.mount_point.clone();
            self.mount_pending = true;
            self.set_status(&format!("Unmounting {}...", mp.display()), StatusKind::Info);
            let tx = tx.clone();
            tokio::spawn(async move {
                let adapter = mount::platform_adapter();
                let result = adapter.unmount(&mp).await;
                let _ = tx.send(AsyncResult::Unmount {
                    mount_point: mp,
                    result,
                });
            });
        }
    }

    fn handle_mount(&mut self, tx: &std::sync::mpsc::Sender<AsyncResult>) {
        if self.mount_pending {
            self.set_status("Mount operation already in progress", StatusKind::Info);
            return;
        }
        let bm = match self.get_bookmark_at_cursor() {
            Some(b) => b,
            None => return,
        };
        let name = bm.name.clone();
        self.mount_pending = true;
        self.set_status(&format!("Mounting {name}..."), StatusKind::Info);
        let tx = tx.clone();
        tokio::spawn(async move {
            let adapter = mount::platform_adapter();
            if let Err(e) = adapter.create_mount_point(std::path::Path::new(&bm.mount_point)) {
                let _ = tx.send(AsyncResult::Mount {
                    name,
                    result: Err(e),
                });
                return;
            }
            let result = adapter.mount(&bm).await;
            let _ = tx.send(AsyncResult::Mount { name, result });
        });
    }

    pub fn handle_async_result(&mut self, result: AsyncResult) {
        match result {
            AsyncResult::Mount { name, result } => {
                self.mount_pending = false;
                match result {
                    Ok(()) => {
                        self.set_status(&format!("Mounted {name}"), StatusKind::Success);
                        self.handle_refresh_mounts();
                    }
                    Err(e) => {
                        self.set_status(&format!("Failed to mount {name}: {e}"), StatusKind::Error);
                    }
                }
            }
            AsyncResult::Unmount {
                mount_point,
                result,
            } => {
                self.mount_pending = false;
                match result {
                    Ok(()) => {
                        self.set_status(
                            &format!("Unmounted {}", mount_point.display()),
                            StatusKind::Success,
                        );
                        self.handle_refresh_mounts();
                    }
                    Err(e) => {
                        self.set_status(&format!("Failed to unmount: {e}"), StatusKind::Error);
                    }
                }
            }
            AsyncResult::RefreshMounts(mounts) => {
                self.mounts = mounts;
            }
        }
    }

    fn handle_delete(&mut self) {
        if self.tab != Tab::Bookmarks {
            return;
        }
        let name = {
            let filtered = self.filtered_bookmarks();
            filtered.get(self.cursor).map(|bm| bm.name.clone())
        };
        if let Some(name) = name
            && let Some(pos) = self.bookmarks.iter().position(|b| b.name == name)
        {
            self.bookmarks.remove(pos);
            if let Err(e) = bookmark::save_bookmarks(&self.bookmarks_path, &self.bookmarks) {
                self.set_status(&format!("Failed to save: {e}"), StatusKind::Error);
            } else {
                self.set_status(&format!("Deleted bookmark '{name}'"), StatusKind::Success);
            }
            if self.cursor > 0 && self.cursor >= self.bookmarks.len() {
                self.cursor = self.cursor.saturating_sub(1);
            }
        }
    }

    fn handle_remove_mount_point(&mut self) {
        let filtered = self.filtered_mount_point_paths();
        let path = match filtered.get(self.cursor) {
            Some(p) => p.clone(),
            None => return,
        };

        if self.mounts.iter().any(|m| m.mount_point == path) {
            self.set_status("Cannot remove: mount point is in use", StatusKind::Error);
            return;
        }

        if !path.exists() {
            self.set_status("Directory does not exist", StatusKind::Error);
            return;
        }

        match std::fs::read_dir(&path) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    self.set_status("Cannot remove: directory is not empty", StatusKind::Error);
                    return;
                }
            }
            Err(e) => {
                self.set_status(&format!("Cannot read directory: {e}"), StatusKind::Error);
                return;
            }
        }

        match std::fs::remove_dir(&path) {
            Ok(()) => {
                self.mount_points.retain(|p| p != &path);
                self.set_status(
                    &format!("Removed mount point: {}", path.display()),
                    StatusKind::Success,
                );
                // filtered had len >= 1 (contained path), now len >= 0
                let new_count = filtered.len().saturating_sub(1);
                if self.cursor > 0 && self.cursor >= new_count {
                    self.cursor = self.cursor.saturating_sub(1);
                }
            }
            Err(e) => {
                self.set_status(&format!("Failed to remove: {e}"), StatusKind::Error);
            }
        }
    }

    pub fn set_status(&mut self, text: &str, kind: StatusKind) {
        self.status = Some(StatusMessage {
            text: text.to_string(),
            kind,
        });
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    /// Get a cloned bookmark at the current cursor position (filtered).
    fn get_bookmark_at_cursor(&self) -> Option<Bookmark> {
        self.filtered_bookmarks().get(self.cursor).cloned().cloned()
    }

    /// Get disk usage for a path, with a 30-second cache.
    pub fn get_disk_usage(&mut self, path: &PathBuf) -> Option<DiskUsage> {
        const TTL: std::time::Duration = std::time::Duration::from_secs(30);

        if let Some((usage, ts)) = self.disk_usage_cache.get(path)
            && ts.elapsed() < TTL
        {
            return Some(usage.clone());
        }

        let adapter = mount::platform_adapter();
        match adapter.disk_usage(path) {
            Ok(usage) => {
                self.disk_usage_cache
                    .insert(path.clone(), (usage.clone(), Instant::now()));
                Some(usage)
            }
            Err(_) => None,
        }
    }

    // --- Filtering ---

    pub fn filtered_mounts(&self) -> Vec<&MountEntry> {
        if self.search_query.is_empty() {
            self.mounts.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.mounts
                .iter()
                .filter(|m| {
                    m.device.to_lowercase().contains(&q)
                        || m.mount_point.to_string_lossy().to_lowercase().contains(&q)
                        || m.fs_type.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_bookmarks(&self) -> Vec<&Bookmark> {
        if self.search_query.is_empty() {
            self.bookmarks.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.bookmarks
                .iter()
                .filter(|b| {
                    b.name.to_lowercase().contains(&q)
                        || b.host.to_lowercase().contains(&q)
                        || b.protocol.to_string().to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_mount_point_paths(&self) -> Vec<PathBuf> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for m in &self.mounts {
            if seen.insert(m.mount_point.clone()) {
                result.push(m.mount_point.clone());
            }
        }
        for b in &self.bookmarks {
            let mp = PathBuf::from(&b.mount_point);
            if seen.insert(mp.clone()) {
                result.push(mp);
            }
        }
        for p in &self.mount_points {
            if seen.insert(p.clone()) {
                result.push(p.clone());
            }
        }

        if self.search_query.is_empty() {
            result
        } else {
            let q = self.search_query.to_lowercase();
            result
                .into_iter()
                .filter(|p| p.to_string_lossy().to_lowercase().contains(&q))
                .collect()
        }
    }

    fn filtered_items_count(&self) -> usize {
        match self.tab {
            Tab::Mounts => self.filtered_mounts().len(),
            Tab::Bookmarks => self.filtered_bookmarks().len(),
            Tab::MountPoints => self.filtered_mount_point_paths().len(),
        }
    }

    /// Check if a bookmark's mount_point is currently mounted.
    pub fn is_bookmark_mounted(&self, bookmark: &Bookmark) -> bool {
        self.mounts
            .iter()
            .any(|m| m.mount_point.to_string_lossy() == bookmark.mount_point)
    }
}
