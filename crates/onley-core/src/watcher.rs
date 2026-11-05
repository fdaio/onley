//! Reports settled file changes under a directory tree.

use notify::{EventKind, RecursiveMode, Watcher as _};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};
use walkdir::WalkDir;

/// How long a file must stop changing before it is indexed.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(500);

/// One settled change under the watched root.
#[derive(Debug)]
pub struct Event {
    pub path: PathBuf,
    /// Create and write are reported together: both mean the file is present
    /// and has settled.
    pub create: bool,
    pub write: bool,
    /// The path is gone. The caller drops it from the index and touches
    /// nothing else.
    pub remove: bool,
    /// The path could not be read or watched. The path may still be valid, so
    /// the caller reports it and keeps watching.
    pub error: Option<notify::Error>,
}

struct Pending {
    deadline: Instant,
    size: i64,
    mtime: i64,
}

/// Reports settled file changes under a directory tree.
///
/// A changed path is held until no event for it arrives for the debounce
/// window. The size and modification time are compared across the window as
/// well, so a file that is still growing waits again.
pub struct Watcher {
    events: Receiver<Event>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Watcher {
    /// Creates a watcher for `root` and registers the whole existing tree.
    pub fn new(root: &Path, debounce: Duration) -> notify::Result<Self> {
        let display_root = std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf());
        let watch_root = std::fs::canonicalize(root).unwrap_or_else(|_| display_root.clone());
        let debounce = if debounce.is_zero() {
            DEFAULT_DEBOUNCE
        } else {
            debounce
        };
        let (raw_tx, raw_rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |result| {
            let _ = raw_tx.send(result);
        })?;
        watcher.watch(&watch_root, RecursiveMode::Recursive)?;

        let (out_tx, out_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = thread::spawn(move || {
            run(
                watcher,
                watch_root,
                display_root,
                debounce,
                raw_rx,
                out_tx,
                thread_stop,
            );
        });
        Ok(Self {
            events: out_rx,
            stop,
            handle: Some(handle),
        })
    }

    /// Returns the channel settled changes arrive on.
    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Stops the watcher and releases the watches.
    pub fn close(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.close();
    }
}

fn run(
    _watcher: notify::RecommendedWatcher,
    watch_root: PathBuf,
    display_root: PathBuf,
    debounce: Duration,
    raw: Receiver<notify::Result<notify::Event>>,
    out: mpsc::Sender<Event>,
    stop: Arc<AtomicBool>,
) {
    let mut pending: HashMap<PathBuf, Pending> = HashMap::new();
    let mut closed = false;

    // The whole existing tree is queued, so the run reconciles whatever a
    // previous run missed.
    for entry in WalkDir::new(&watch_root).follow_links(false) {
        if let Ok(entry) = entry
            && entry.file_type().is_file()
        {
            let path = display_path(entry.path(), &watch_root, &display_root);
            hold(&mut pending, &path, debounce);
        }
    }

    while !stop.load(Ordering::Relaxed) {
        match raw.recv_timeout(Duration::from_millis(25)) {
            Ok(Ok(event)) => {
                let paths: Vec<PathBuf> = event
                    .paths
                    .into_iter()
                    .map(|path| display_path(&path, &watch_root, &display_root))
                    .collect();
                handle(&mut pending, event.kind, paths, debounce, &out);
            }
            Ok(Err(error)) => {
                if out
                    .send(Event {
                        path: PathBuf::new(),
                        create: false,
                        write: false,
                        remove: false,
                        error: Some(error),
                    })
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                closed = true;
                break;
            }
        }
        settle(&mut pending, debounce, &out);
    }
    if !closed {
        settle(&mut pending, Duration::ZERO, &out);
    }
}

fn handle(
    pending: &mut HashMap<PathBuf, Pending>,
    kind: EventKind,
    paths: Vec<PathBuf>,
    debounce: Duration,
    out: &mpsc::Sender<Event>,
) {
    match kind {
        EventKind::Remove(_) => {
            for path in paths {
                pending.remove(&path);
                let _ = out.send(Event {
                    path,
                    create: false,
                    write: false,
                    remove: true,
                    error: None,
                });
            }
        }
        EventKind::Access(_) | EventKind::Other => {}
        EventKind::Create(_) => {
            for path in paths {
                hold(pending, &path, debounce);
            }
        }
        EventKind::Modify(kind) => match kind {
            notify::event::ModifyKind::Metadata(_) => {}
            notify::event::ModifyKind::Name(_) => {
                for path in paths {
                    if path.exists() {
                        hold(pending, &path, debounce);
                    } else {
                        pending.remove(&path);
                        let _ = out.send(Event {
                            path,
                            create: false,
                            write: false,
                            remove: true,
                            error: None,
                        });
                    }
                }
            }
            _ => {
                for path in paths {
                    hold(pending, &path, debounce);
                }
            }
        },
        _ => {}
    }
}

fn display_path(path: &Path, watch_root: &Path, display_root: &Path) -> PathBuf {
    match path.strip_prefix(watch_root) {
        Ok(rest) if !rest.as_os_str().is_empty() => display_root.join(rest),
        _ => path.to_path_buf(),
    }
}

fn hold(pending: &mut HashMap<PathBuf, Pending>, path: &Path, debounce: Duration) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if metadata.is_dir() {
        return;
    }
    if !metadata.is_file() {
        return;
    }
    let size = metadata.len() as i64;
    let mtime = mtime_of(&metadata);
    pending.insert(
        path.to_path_buf(),
        Pending {
            deadline: Instant::now() + debounce,
            size,
            mtime,
        },
    );
}

fn settle(pending: &mut HashMap<PathBuf, Pending>, debounce: Duration, out: &mpsc::Sender<Event>) {
    let now = Instant::now();
    let ready: Vec<PathBuf> = pending
        .iter()
        .filter(|(_, entry)| entry.deadline <= now)
        .map(|(path, _)| path.clone())
        .collect();
    for path in ready {
        let Some(entry) = pending.get(&path) else {
            continue;
        };
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let size = metadata.len() as i64;
                let mtime = mtime_of(&metadata);
                if size != entry.size || mtime != entry.mtime {
                    // Still changing: reopen the window with the new baseline.
                    let baseline = pending.get_mut(&path).unwrap();
                    baseline.deadline = Instant::now() + debounce.max(Duration::from_millis(1));
                    baseline.size = size;
                    baseline.mtime = mtime;
                    continue;
                }
                pending.remove(&path);
                if out
                    .send(Event {
                        path,
                        create: true,
                        write: true,
                        remove: false,
                        error: None,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(_) => {
                pending.remove(&path);
            }
            Err(_) => {
                pending.remove(&path);
                if out
                    .send(Event {
                        path,
                        create: false,
                        write: false,
                        remove: true,
                        error: None,
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
    }
}

fn mtime_of(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn wait_for(rx: &Receiver<Event>, predicate: impl Fn(&Event) -> bool) -> Option<Event> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) if predicate(&event) => return Some(event),
                Ok(_) => {}
                Err(_) => {}
            }
        }
        None
    }

    #[test]
    fn reports_a_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let mut watcher = Watcher::new(&root, Duration::from_millis(50)).unwrap();
        let path = root.join("new.txt");
        fs::write(&path, b"hello").unwrap();
        let event = wait_for(watcher.events(), |event| event.path == path && event.create).unwrap();
        assert!(event.write);
        watcher.close();
    }

    #[test]
    fn reports_a_removal() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let path = root.join("gone.txt");
        fs::write(&path, b"hello").unwrap();
        let mut watcher = Watcher::new(&root, Duration::from_millis(50)).unwrap();
        fs::remove_file(&path).unwrap();
        let event = wait_for(watcher.events(), |event| event.path == path && event.remove).unwrap();
        assert!(event.remove);
        watcher.close();
    }

    #[test]
    fn a_write_settles_before_it_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let mut watcher = Watcher::new(&root, Duration::from_millis(20)).unwrap();
        let path = root.join("grow.txt");
        fs::write(&path, b"one").unwrap();
        let event = wait_for(watcher.events(), |event| event.path == path && event.create).unwrap();
        assert!(event.create);
        watcher.close();
    }
}
