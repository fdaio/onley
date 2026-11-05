//! The desktop client's state, without a view.

use crate::categories::{self, Category};
use nsh_core::prelude::*;
use nsh_core::storage::Storage;
use onley_core::{
    appmodel::{self, KeepPolicy, SpeedSample, Summary},
    cleaner,
    db::Db,
    db::DuplicateGroup,
    format, i18n, selfpath,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const RECENT_LIMIT: usize = 5;
const RECENT_KEY: &str = "recent_folders";

/// What the app needs to run and what tests replace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub db_path: Option<PathBuf>,
    pub open_error: Option<String>,
    pub workers: usize,
    pub home: PathBuf,
    pub default_dir: PathBuf,
    pub self_path: Option<PathBuf>,
}

impl Config {
    /// Reads the environment, including `ONLEY_DB` and `ONLEY_HOME` when set.
    pub fn from_environment() -> Self {
        let (db_path, open_error) = match std::env::var_os("ONLEY_DB").map(PathBuf::from) {
            Some(path) => (Some(path), None),
            None => match onley_core::appdir::db_path() {
                Ok(path) => (Some(path), None),
                Err(err) => (None, Some(err.to_string())),
            },
        };
        let mut config = Self::detect(db_path, open_error);
        if let Some(home) = std::env::var_os("ONLEY_HOME").map(PathBuf::from) {
            config.home = home.clone();
            config.default_dir = home;
        }
        config
    }

    /// Reads the environment for the paths the app uses.
    pub fn detect(db_path: Option<PathBuf>, open_error: Option<String>) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let downloads = home.join("Downloads");
        let default_dir = if downloads.is_dir() {
            downloads
        } else {
            home.clone()
        };
        Self {
            db_path,
            open_error,
            workers: std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).max(1))
                .unwrap_or(1),
            home,
            default_dir,
            self_path: selfpath::resolve(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathStatus {
    #[default]
    Unchecked,
    Usable,
    Missing,
    File,
    Denied,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathInfo {
    pub status: PathStatus,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanProgress {
    pub done: usize,
    pub skipped: usize,
    pub errors: usize,
    pub permission_errors: usize,
    pub hashed: i64,
    pub speed: f64,
    pub elapsed: Duration,
    pub last_error: String,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            done: 0,
            skipped: 0,
            errors: 0,
            permission_errors: 0,
            hashed: 0,
            speed: 0.0,
            elapsed: Duration::ZERO,
            last_error: String::new(),
        }
    }
}

struct DeleteOutcome {
    deleted: usize,
    freed: i64,
    missing: usize,
    failed: usize,
    first_error: String,
    removed_folders: usize,
}

struct DeleteFailure {
    message: String,
}

#[derive(Clone)]
pub struct State {
    pub cfg: Config,
    pub db: Option<Arc<Db>>,

    pub groups: Signal<Vec<DuplicateGroup>>,
    pub scoped: Signal<Vec<DuplicateGroup>>,
    pub visible: Signal<Vec<DuplicateGroup>>,
    pub summary: Signal<Summary>,
    pub indexed: Signal<usize>,
    pub scan_dirs: Signal<Vec<String>>,
    pub scan_path: Signal<String>,
    pub adding_folder: Signal<bool>,
    pub path_state: Signal<PathInfo>,
    pub recent: Signal<Vec<String>>,
    pub settings: Signal<bool>,
    pub filters: Signal<bool>,
    pub keep: Signal<HashMap<String, String>>,
    pub keep_policy: Signal<KeepPolicy>,
    pub category: Signal<Category>,
    pub query: Signal<String>,
    pub active_root: Signal<String>,
    pub scanning: Signal<bool>,
    pub busy: Signal<bool>,
    pub progress: Signal<ScanProgress>,
    pub message: Signal<String>,
    pub note: Signal<String>,
    pub note_success: Signal<bool>,
    pub confirm: Signal<bool>,
    pub lang: Signal<i18n::Lang>,
    pub cancel: Rc<RefCell<Option<Arc<AtomicBool>>>>,
    pub owner: Owner,
}

impl State {
    /// Builds the state and loads the index.
    pub fn new(cfg: Config) -> Self {
        let mut open_error = cfg.open_error.clone();
        let db = match &cfg.db_path {
            Some(path) => match Db::open(path) {
                Ok(db) => Some(Arc::new(db)),
                Err(err) => {
                    open_error = Some(err.to_string());
                    None
                }
            },
            None => None,
        };

        let mut lang = i18n::detect(&locale().to_string());
        if let Some(saved) = Storage::get_item("language")
            && let Some(saved) = i18n::parse(&saved)
        {
            lang = saved;
        }
        let _ = open_error;
        let mut policy = KeepPolicy::default();
        if let Some(saved) = Storage::get_item("keep_policy")
            && let Some(saved) = i18n_parse_policy(&saved)
        {
            policy = saved;
        }
        let recent = load_recent();
        let default_root = cfg.default_dir.to_string_lossy().into_owned();

        let state = Self {
            cfg,
            db,
            groups: signal(Vec::new()),
            scoped: signal(Vec::new()),
            visible: signal(Vec::new()),
            summary: signal(Summary::default()),
            indexed: signal(0),
            scan_dirs: signal(vec![default_root.clone()]),
            scan_path: signal(String::new()),
            adding_folder: signal(false),
            path_state: signal(PathInfo::default()),
            recent: signal(recent),
            settings: signal(false),
            filters: signal(false),
            keep: signal(HashMap::new()),
            keep_policy: signal(policy),
            category: signal(Category::All),
            query: signal(String::new()),
            active_root: signal(default_root),
            scanning: signal(false),
            busy: signal(false),
            progress: signal(ScanProgress::default()),
            message: signal(String::new()),
            note: signal(String::new()),
            note_success: signal(false),
            confirm: signal(false),
            lang: signal(lang),
            cancel: Rc::new(RefCell::new(None)),
            owner: Owner::current(),
        };
        state.reload();
        state
    }

    /// Runs `future` under the component's owner, so it is not cancelled
    /// when the handler that started it returns.
    pub fn spawn_task(&self, future: impl std::future::Future<Output = ()> + 'static) {
        let owner = self.owner;
        owner.run(|| {
            let _spawned = spawn(future);
        });
    }

    /// Reads the index and applies it.
    pub fn reload(&self) {
        let Some(db) = &self.db else {
            self.message.set(self.t(i18n::INDEX_UNAVAILABLE));
            return;
        };
        match load(db) {
            Ok((groups, total)) => self.apply(groups, total),
            Err(err) => self.show_index_error(&err),
        }
    }

    fn show_index_error(&self, err: &str) {
        if err.contains("permission") || err.contains("denied") {
            self.message.set(self.t(i18n::PERMISSION_INDEX));
        } else {
            self.message.set(self.tf(i18n::CANNOT_READ_INDEX, &[&err]));
        }
    }

    /// Applies a fresh index.
    pub fn apply(&self, groups: Vec<DuplicateGroup>, total: usize) {
        self.groups.set(groups);
        self.indexed.set(total);
        self.refresh_visible();
        self.apply_keep_policy(self.keep_policy.get());
    }

    pub fn t(&self, key: &'static str) -> String {
        i18n::t(self.lang.get(), key).to_string()
    }

    pub fn tf(&self, key: &'static str, args: &[&dyn std::fmt::Display]) -> String {
        i18n::f(self.lang.get(), key, args)
    }

    pub fn file_count(&self, n: usize) -> String {
        i18n::file_count(self.lang.get(), n)
    }

    pub fn set_language(&self, lang: i18n::Lang) {
        self.lang.set(lang);
        let _ = Storage::set_item("language", lang.as_str());
        self.set_note("", false);
        self.message.set(String::new());
    }

    pub fn set_note(&self, text: impl Into<String>, success: bool) {
        self.note.set(text.into());
        self.note_success.set(success);
    }

    /// Applies the category, folder and search filters.
    pub fn refresh_visible(&self) {
        let groups = self.groups.get();
        let roots = self.scan_dirs.get();
        let scoped: Vec<DuplicateGroup> = groups
            .into_iter()
            .filter(|group| !group.files.is_empty() && categories::group_in_roots(group, &roots))
            .collect();
        self.summary.set(appmodel::summarize(&scoped));
        self.scoped.set(scoped.clone());

        let category = self.category.get();
        let query = self.query.get().trim().to_lowercase();
        let visible: Vec<DuplicateGroup> = scoped
            .into_iter()
            .filter(|group| {
                if category != Category::All
                    && categories::file_category(&group.files[0].name) != category
                {
                    return false;
                }
                if !query.is_empty() && !categories::group_matches(group, &query) {
                    return false;
                }
                true
            })
            .collect();
        self.visible.set(visible);
    }

    pub fn set_category(&self, category: Category) {
        self.category.set(category);
        self.refresh_visible();
    }

    pub fn set_query(&self, query: impl Into<String>) {
        self.query.set(query.into());
        self.refresh_visible();
    }

    pub fn set_active_root(&self, root: impl Into<String>) {
        self.active_root.set(root.into());
        self.refresh_visible();
    }

    /// Counts duplicate groups per category.
    pub fn category_counts(&self) -> HashMap<Category, usize> {
        let mut counts: HashMap<Category, usize> = HashMap::new();
        let roots = self.scan_dirs.get();
        for group in self.groups.get() {
            if group.files.is_empty() || !categories::group_in_roots(&group, &roots) {
                continue;
            }
            *counts.entry(Category::All).or_default() += 1;
            *counts
                .entry(categories::file_category(&group.files[0].name))
                .or_default() += 1;
        }
        counts
    }

    pub fn plan(&self) -> appmodel::Plan {
        appmodel::build_plan(
            &self.scoped.get(),
            &self.keep.get(),
            self.cfg.self_path.as_deref(),
        )
    }

    pub fn set_keep(&self, md5: &str, path: &str) {
        for group in self.groups.get() {
            if group.md5 != md5 {
                continue;
            }
            if let Some(protected) = appmodel::protected_file(&group, self.cfg.self_path.as_deref())
                && !selfpath::same(&protected.path, Path::new(path))
            {
                return;
            }
            break;
        }
        let mut next = self.keep.get();
        next.insert(md5.to_string(), path.to_string());
        self.keep.set(next);
    }

    /// Stores the policy and selects the keep copy in every group.
    ///
    /// A delete rejects the change because it acts on the current choices.
    /// A scan accepts it, and the policy applies again when the scan ends.
    pub fn apply_keep_policy(&self, policy: KeepPolicy) {
        if self.busy.get() {
            return;
        }
        if self.keep_policy.get() != policy {
            self.keep_policy.set(policy);
        }
        let _ = Storage::set_item("keep_policy", policy.as_str());
        let mut next = HashMap::new();
        for group in self.groups.get() {
            if let Some(protected) = appmodel::protected_file(&group, self.cfg.self_path.as_deref())
            {
                next.insert(
                    group.md5.clone(),
                    protected.path.to_string_lossy().into_owned(),
                );
                continue;
            }
            if let Some(chosen) = appmodel::choose_keep(&group, policy) {
                next.insert(
                    group.md5.clone(),
                    chosen.path.to_string_lossy().into_owned(),
                );
            }
        }
        self.keep.set(next);
    }

    pub fn open_add_folder(&self) {
        if self.scanning.get() || self.busy.get() {
            return;
        }
        self.scan_path.set(String::new());
        self.message.set(String::new());
        self.path_state.set(PathInfo::default());
        self.adding_folder.set(true);
    }

    pub fn close_add_folder(&self) {
        self.adding_folder.set(false);
        self.scan_path.set(String::new());
        self.message.set(String::new());
        self.path_state.set(PathInfo::default());
    }

    pub fn confirm_add_folder(&self) {
        if self.scan_path.get().trim().is_empty() {
            return;
        }
        if self.add_folder(&self.scan_path.get()) {
            self.adding_folder.set(false);
        }
    }

    /// Refreshes the index without blocking the UI.
    pub fn refresh_async(&self) {
        let Some(db) = self.db.clone() else {
            return;
        };
        let state = self.clone();
        self.spawn_task(async move {
            let handle = std::thread::spawn(move || load(&db));
            let loaded = join(handle).await;
            match loaded {
                Some(Ok((groups, total))) => state.apply(groups, total),
                Some(Err(err)) => state.show_index_error(&err),
                None => state.set_note("", false),
            }
        });
    }

    /// Normalises and adds a folder, merging it with the selected tree.
    pub fn add_folder(&self, raw: &str) -> bool {
        if self.scanning.get() || self.busy.get() {
            return false;
        }
        let (abs, info) = self.probe_path(raw);
        let path = match info.status {
            PathStatus::Unchecked => return false,
            PathStatus::Usable => abs,
            PathStatus::Denied => {
                self.message.set(self.tf(i18n::PERMISSION_FOLDER, &[&abs]));
                return false;
            }
            PathStatus::File => {
                self.message.set(self.tf(i18n::NOT_A_FOLDER, &[&abs]));
                return false;
            }
            _ => {
                self.message
                    .set(self.tf(i18n::CANNOT_SCAN_FOLDER, &[&info.error]));
                return false;
            }
        };
        let mut roots = self.scan_dirs.get();
        roots.push(path.clone());
        self.scan_dirs.set(appmodel::merge_roots(&roots));
        self.remember_recent(&path);
        self.refresh_visible();
        self.scan_path.set(String::new());
        self.message.set(String::new());
        true
    }

    /// Reports what `raw` names without changing the selected folders.
    pub fn probe_path(&self, raw: &str) -> (String, PathInfo) {
        let path = self.expand(raw);
        if path.is_empty() {
            return (String::new(), PathInfo::default());
        }
        let mut path = PathBuf::from(path);
        if path.is_relative() {
            path = self.cfg.home.join(path);
        }
        let abs = std::path::absolute(&path).unwrap_or(path);
        let display = abs.to_string_lossy().into_owned();
        match std::fs::metadata(&abs) {
            Ok(metadata) if metadata.is_dir() => (
                display,
                PathInfo {
                    status: PathStatus::Usable,
                    error: String::new(),
                },
            ),
            Ok(_) => (
                display,
                PathInfo {
                    status: PathStatus::File,
                    error: String::new(),
                },
            ),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => (
                display,
                PathInfo {
                    status: PathStatus::Denied,
                    error: err.to_string(),
                },
            ),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
                display,
                PathInfo {
                    status: PathStatus::Missing,
                    error: err.to_string(),
                },
            ),
            Err(err) => (
                display,
                PathInfo {
                    status: PathStatus::Unreadable,
                    error: err.to_string(),
                },
            ),
        }
    }

    pub fn path_hint(&self) -> String {
        let info = self.path_state.get();
        match info.status {
            PathStatus::Usable => self.t(i18n::PATH_USABLE),
            PathStatus::Missing => self.t(i18n::PATH_MISSING),
            PathStatus::File => self.t(i18n::PATH_FILE),
            PathStatus::Denied => self.t(i18n::PATH_DENIED),
            PathStatus::Unreadable => self.tf(i18n::CANNOT_SCAN_FOLDER, &[&info.error]),
            PathStatus::Unchecked => String::new(),
        }
    }

    /// Replaces the typed path and checks it.
    pub fn set_scan_path(&self, raw: impl Into<String>) {
        let raw = raw.into();
        self.scan_path.set(raw.clone());
        self.message.set(String::new());
        self.update_path_check(&raw);
    }

    /// Checks a typed path now.
    pub fn update_path_check(&self, raw: &str) {
        if raw.trim().is_empty() {
            self.path_state.set(PathInfo::default());
            return;
        }
        let (_, info) = self.probe_path(raw);
        self.path_state.set(info);
    }

    /// Fills the field from the clipboard: a copied folder first, then text.
    pub fn paste_path(&self) {
        let state = self.clone();
        self.spawn_task(async move {
            let files = clipboard_files().await;
            for uri in files {
                if let Some(path) = onley_core::replica::file_url_path(&uri)
                    && path.is_dir()
                {
                    state.set_scan_path(path.to_string_lossy().into_owned());
                    return;
                }
            }
            let text = clipboard_string().await;
            let text = text.trim().to_string();
            if text.is_empty() {
                state.message.set(state.t(i18n::CLIPBOARD_EMPTY));
                return;
            }
            state.set_scan_path(text);
        });
    }

    /// Moves a path to the front of the remembered folders.
    pub fn remember_recent(&self, path: &str) {
        let mut next = vec![path.to_string()];
        for old in self.recent.get() {
            if old != path && next.len() < RECENT_LIMIT {
                next.push(old);
            }
        }
        self.recent.set(next.clone());
        if let Ok(json) = serde_json::to_string(&next) {
            let _ = Storage::set_item(RECENT_KEY, json);
        }
    }

    pub fn remove_folder(&self, path: &str) {
        if self.scanning.get() || self.busy.get() {
            return;
        }
        let roots: Vec<String> = self
            .scan_dirs
            .get()
            .into_iter()
            .filter(|root| root != path)
            .collect();
        self.scan_dirs.set(roots);
        if self.active_root.get() == path {
            self.set_active_root("");
        }
        self.refresh_visible();
    }

    pub fn clear_keep(&self) {
        if self.scanning.get() || self.busy.get() {
            return;
        }
        self.keep.set(HashMap::new());
    }

    /// Starts a scan of every selected root.
    pub fn start_scan(&self) {
        if self.db.is_none() || self.scanning.get() || self.busy.get() {
            return;
        }
        let draft = self.scan_path.get().trim().to_string();
        if !draft.is_empty() && !self.add_folder(&draft) {
            return;
        }
        let roots = appmodel::merge_roots(&self.scan_dirs.get());
        if roots.is_empty() {
            self.message.set(self.t(i18n::ADD_FOLDER_FIRST));
            return;
        }
        for root in &roots {
            match std::fs::metadata(root) {
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                    self.message.set(self.tf(i18n::PERMISSION_FOLDER, &[root]));
                    return;
                }
                Err(err) => {
                    self.message
                        .set(self.tf(i18n::CANNOT_SCAN_FOLDER, &[&err.to_string()]));
                    return;
                }
                Ok(metadata) if !metadata.is_dir() => {
                    self.message.set(self.tf(i18n::NOT_A_FOLDER, &[root]));
                    return;
                }
                Ok(_) => {}
            }
        }
        self.scan_dirs.set(roots.clone());
        self.refresh_visible();
        self.keep.set(HashMap::new());
        self.confirm.set(false);
        self.progress.set(ScanProgress::default());
        self.scanning.set(true);
        self.message.set(String::new());
        self.set_note("", false);

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel.replace(Some(cancel.clone()));
        let state = self.clone();
        self.spawn_task(async move {
            state.run_scan(roots, cancel).await;
        });
    }

    async fn run_scan(&self, roots: Vec<String>, cancel: Arc<AtomicBool>) {
        enum Message {
            Progress(onley_core::scanner::Progress),
            Finished,
        }
        let Some(db) = self.db.clone() else {
            return;
        };
        let workers = self.cfg.workers.max(1);
        let (tx, rx) = std::sync::mpsc::channel::<Message>();
        let scan_cancel = cancel.clone();
        let handle = std::thread::spawn(move || {
            for root in &roots {
                if scan_cancel.load(Ordering::Relaxed) {
                    break;
                }
                let progress = onley_core::scanner::scan(
                    db.clone(),
                    PathBuf::from(root),
                    workers,
                    scan_cancel.clone(),
                );
                for event in progress {
                    if tx.send(Message::Progress(event)).is_err() {
                        return;
                    }
                }
                if scan_cancel.load(Ordering::Relaxed) {
                    break;
                }
            }
            let _ = tx.send(Message::Finished);
        });

        let started = Instant::now();
        let mut progress = ScanProgress::default();
        let mut hashed: i64 = 0;
        let mut samples: VecDeque<SpeedSample> = VecDeque::new();
        let mut finished = false;
        let mut last_draw = Instant::now() - Duration::from_secs(1);

        loop {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::Progress(event) => {
                        if let Some(error) = event.error {
                            progress.errors += 1;
                            progress.last_error = error.to_string();
                            if error.kind() == std::io::ErrorKind::PermissionDenied {
                                progress.permission_errors += 1;
                            }
                        } else if event.done {
                            progress.done += 1;
                            if event.skipped {
                                progress.skipped += 1;
                            }
                        }
                        if event.bytes > 0 {
                            hashed += event.bytes;
                            let now = Instant::now();
                            samples.push_back(SpeedSample {
                                at: now,
                                bytes: event.bytes,
                            });
                            let cutoff = now.checked_sub(Duration::from_secs(2)).unwrap_or(now);
                            while samples.front().is_some_and(|sample| sample.at < cutoff) {
                                samples.pop_front();
                            }
                        }
                    }
                    Message::Finished => finished = true,
                }
            }

            let now = Instant::now();
            if now.duration_since(last_draw) >= Duration::from_millis(200) || finished {
                last_draw = now;
                progress.hashed = hashed;
                progress.speed = appmodel::rate(
                    &samples.iter().copied().collect::<Vec<_>>(),
                    now,
                    Duration::from_secs(1),
                );
                progress.elapsed = now - started;
                let snapshot = progress.clone();
                self.progress.set(snapshot);
            }

            if finished && rx.try_recv().is_err() {
                break;
            }
            timer(Duration::from_millis(50)).await;
        }
        let _ = handle.join();

        let cancelled = cancel.load(Ordering::Relaxed);
        self.scanning.set(false);
        self.cancel.replace(None);
        self.progress.set(progress.clone());
        if cancelled {
            self.set_note(self.t(i18n::SCAN_CANCELED), false);
        } else {
            self.set_note(
                self.tf(i18n::SCAN_FINISHED, &[&progress.done, &progress.skipped]),
                false,
            );
        }
        if progress.permission_errors > 0 {
            let mut message = self.tf(i18n::PERMISSION_READ_ERRORS, &[&progress.permission_errors]);
            let other = progress.errors.saturating_sub(progress.permission_errors);
            if other > 0 {
                message.push(' ');
                message.push_str(&self.tf(i18n::READ_ERRORS, &[&other]));
            }
            self.message.set(message);
        } else if progress.errors > 0 {
            let mut message = self.tf(i18n::READ_ERRORS, &[&progress.errors]);
            if !progress.last_error.is_empty() {
                message.push_str(": ");
                message.push_str(&progress.last_error);
            }
            self.message.set(message);
        }
        self.refresh_async();
    }

    pub fn cancel_scan(&self) {
        if let Some(cancel) = self.cancel.borrow().as_ref() {
            cancel.store(true, Ordering::Relaxed);
            self.set_note(self.t(i18n::CANCELING), false);
        }
    }

    pub fn open_confirm(&self) {
        if self.db.is_none() || self.scanning.get() || self.busy.get() {
            return;
        }
        if self.plan().targets.is_empty() {
            return;
        }
        self.confirm.set(true);
    }

    /// Deletes the planned files after the confirmation.
    pub fn confirm_delete(&self) {
        let plan = self.plan();
        let Some(db) = self.db.clone() else {
            self.confirm.set(false);
            return;
        };
        if plan.targets.is_empty() {
            self.confirm.set(false);
            return;
        }
        self.confirm.set(false);
        self.busy.set(true);
        self.message.set(String::new());
        self.set_note("", false);

        let targets: Vec<cleaner::Target> = plan
            .targets
            .iter()
            .map(|file| cleaner::Target {
                path: file.path.clone(),
                size: file.size,
                mtime: file.mtime,
            })
            .collect();
        let keeps: Vec<cleaner::Target> = plan
            .keeps
            .iter()
            .map(|file| cleaner::Target {
                path: file.path.clone(),
                size: file.size,
                mtime: file.mtime,
            })
            .collect();
        let roots = self.scan_dirs.get();
        let protect = self.cfg.self_path.clone();
        let self_message = self.t(i18n::CANNOT_DELETE_SELF);
        let permission_format = self.t(i18n::PERMISSION_DELETE);
        let index_cleanup = self.t(i18n::INDEX_CLEANUP_FAILED);
        let permission_index = self.t(i18n::PERMISSION_INDEX);
        let cancel_keep = self.t(i18n::DELETE_CANCELED_KEEP);
        let cannot_keep = self.t(i18n::CANNOT_KEEP);
        let state = self.clone();

        self.spawn_task(async move {
            let sizes: Vec<i64> = targets.iter().map(|target| target.size).collect();
            let handle = std::thread::spawn(move || -> std::result::Result<DeleteOutcome, DeleteFailure> {
                for keep in &keeps {
                    if let Err(err) = cleaner::verify(keep) {
                        return Err(DeleteFailure {
                            message: if matches!(err, cleaner::DeleteError::Io(ref io) if io.kind() == std::io::ErrorKind::PermissionDenied)
                            {
                                permission_format_for(&permission_format, &keep.path)
                            } else {
                                format!("{}: {err}", keep.path.display())
                            },
                        });
                    }
                }
                let results = cleaner::delete_with_protect(
                    &db,
                    &targets,
                    true,
                    protect.as_deref(),
                );
                let mut deleted = 0usize;
                let mut missing = 0usize;
                let mut failed = 0usize;
                let mut freed = 0i64;
                let mut deleted_paths = Vec::new();
                let mut first_error = String::new();
                for (i, result) in results.iter().enumerate() {
                    if result.deleted {
                        deleted += 1;
                        freed += sizes.get(i).copied().unwrap_or(0);
                        deleted_paths.push(result.path.clone());
                    }
                    if result.missing {
                        missing += 1;
                    }
                    if let Some(error) = &result.error {
                        failed += 1;
                        if first_error.is_empty() {
                            first_error = match error {
                                cleaner::DeleteError::SelfDelete => {
                                    format!("{}: {self_message}", result.path.display())
                                }
                                cleaner::DeleteError::Io(io)
                                    if io.kind() == std::io::ErrorKind::PermissionDenied =>
                                {
                                    permission_format_for(&permission_format, &result.path)
                                }
                                other => format!("{}: {other}", result.path.display()),
                            };
                        }
                    }
                    if let Some(error) = &result.index_error {
                        failed += 1;
                        if first_error.is_empty() {
                            first_error = if error.to_string().contains("permission") {
                                format!("{}: {permission_index}", result.path.display())
                            } else {
                                format!(
                                    "{}: {index_cleanup}: {error}",
                                    result.path.display()
                                )
                            };
                        }
                    }
                }
                let removed_folders =
                    cleaner::remove_empty_parents(&deleted_paths, &roots).len();
                Ok(DeleteOutcome {
                    deleted,
                    freed,
                    missing,
                    failed,
                    first_error,
                    removed_folders,
                })
            });

            let outcome = join(handle).await;
            state.busy.set(false);
            match outcome {
                Some(Ok(outcome)) => {
                    let mut note =
                        state.tf(i18n::DELETED, &[&state.file_count(outcome.deleted), &format::bytes(outcome.freed)]);
                    if outcome.removed_folders > 0 {
                        note.push(' ');
                        note.push_str(
                            &state.tf(i18n::REMOVED_FOLDERS, &[&outcome.removed_folders]),
                        );
                    }
                    if outcome.missing > 0 {
                        note.push(' ');
                        note.push_str(&state.tf(i18n::ALREADY_GONE, &[&outcome.missing]));
                    }
                    if outcome.failed > 0 {
                        note.push(' ');
                        note.push_str(&state.tf(i18n::FAILED, &[&outcome.failed]));
                        state.message.set(outcome.first_error.clone());
                    }
                    state.set_note(note, outcome.deleted > 0);
                }
                Some(Err(failure)) => {
                    state.set_note(cancel_keep, false);
                    let message = if failure.message.contains("permission") {
                        failure.message
                    } else {
                        let _ = cannot_keep;
                        failure.message
                    };
                    state.message.set(message);
                }
                None => {
                    state.set_note(cancel_keep, false);
                }
            }
            state.refresh_async();
        });
    }

    /// Expands `~` and `~/` to the home directory.
    pub fn expand(&self, path: &str) -> String {
        let path = path.trim();
        if path == "~" {
            return self.cfg.home.to_string_lossy().into_owned();
        }
        if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
            return self.cfg.home.join(rest).to_string_lossy().into_owned();
        }
        path.to_string()
    }
}

/// Waits for a blocking thread without blocking the engine thread.
async fn join<T: Send + 'static>(handle: std::thread::JoinHandle<T>) -> Option<T> {
    loop {
        if handle.is_finished() {
            return handle.join().ok();
        }
        timer(Duration::from_millis(25)).await;
    }
}

fn permission_format_for(format: &str, path: &Path) -> String {
    format.replace("{}", &path.display().to_string())
}

fn i18n_parse_policy(value: &str) -> Option<KeepPolicy> {
    let (policy, ok) = appmodel::parse_keep_policy(value);
    ok.then_some(policy)
}

fn load(db: &Db) -> Result<(Vec<DuplicateGroup>, usize), String> {
    let groups = db.duplicates().map_err(|err| err.to_string())?;
    let (total, _) = db.stats().map_err(|err| err.to_string())?;
    Ok((groups, total as usize))
}

fn load_recent() -> Vec<String> {
    let Some(raw) = Storage::get_item(RECENT_KEY) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Replaces the home directory with `~` for display.
pub fn display_path(path: &str, home: &Path) -> String {
    if home.as_os_str().is_empty() {
        return path.to_string();
    }
    let home_text = home.to_string_lossy();
    if path == home_text {
        return "~".to_string();
    }
    if let Some(rest) = path.strip_prefix(home_text.as_ref())
        && rest.starts_with(std::path::MAIN_SEPARATOR)
    {
        return format!("~{rest}");
    }
    path.to_string()
}
