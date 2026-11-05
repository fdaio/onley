//! Walks a directory tree and indexes every regular file.

use crate::db::{Db, FileRecord};
use md5::{Digest, Md5};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// The buffer a scan reads at a time. Each read is reported so a large file
/// moves the speed display before it finishes.
const HASH_CHUNK: usize = 4 << 20;

/// One notable event during a scan.
#[derive(Debug)]
pub struct Progress {
    pub path: PathBuf,
    /// Which worker reported it (0-based).
    pub worker_id: usize,
    /// Files finished so far. Set when `done` is true.
    pub current: usize,
    /// Workers hashing when the file finished.
    pub active: usize,
    /// Bytes read since the previous event for this file.
    pub bytes: i64,
    /// False while a worker is hashing, true when the file is settled.
    pub done: bool,
    /// True when the file was unchanged and skipped.
    pub skipped: bool,
    pub error: Option<io::Error>,
}

struct Job {
    path: PathBuf,
    size: i64,
    mtime: i64,
}

enum Result {
    Error {
        path: PathBuf,
        worker_id: usize,
        error: io::Error,
    },
    Started {
        path: PathBuf,
        worker_id: usize,
    },
    Read {
        path: PathBuf,
        worker_id: usize,
        bytes: i64,
    },
    Skipped {
        path: PathBuf,
        worker_id: usize,
    },
    Hashed {
        path: PathBuf,
        worker_id: usize,
        active: usize,
        record: FileRecord,
    },
}

/// Walks `root` and indexes every regular file using a pool of workers.
///
/// Files whose size and modification time match the stored record are
/// skipped. A cancelled scan keeps the records it already wrote.
pub fn scan(
    db: Arc<Db>,
    root: PathBuf,
    workers: usize,
    cancel: Arc<AtomicBool>,
) -> Receiver<Progress> {
    let workers = workers.max(1);
    let (out_tx, out_rx) = mpsc::sync_channel(workers);
    thread::spawn(move || run(db, root, workers, cancel, out_tx));
    out_rx
}

/// Indexes one regular file unless the stored record already matches.
///
/// Returns the digest it wrote, or `None` when the file was unchanged.
pub fn index_file(path: &Path, db: &Db) -> io::Result<Option<String>> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::other(format!(
            "not a regular file: {}",
            path.display()
        )));
    }
    let size = metadata.len() as i64;
    let mtime = mtime_of(&metadata);
    if unchanged(db, path, size, mtime)? {
        return Ok(None);
    }
    let md5 = hash_file(path, &mut |_| true)?;
    db.upsert(&FileRecord::new(path, size, &md5, mtime))
        .map_err(io::Error::other)?;
    Ok(Some(md5))
}

fn run(
    db: Arc<Db>,
    root: PathBuf,
    workers: usize,
    cancel: Arc<AtomicBool>,
    out: SyncSender<Progress>,
) {
    let (job_tx, job_rx) = mpsc::sync_channel::<Job>(workers * 2);
    let (result_tx, result_rx) = mpsc::channel::<Result>();
    let job_rx = Arc::new(std::sync::Mutex::new(job_rx));

    let walker_cancel = cancel.clone();
    let walker_results = result_tx.clone();
    let walker = thread::spawn(move || {
        for entry in WalkDir::new(&root).follow_links(false) {
            if walker_cancel.load(Ordering::Relaxed) {
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    let path = err.path().unwrap_or(&root).to_path_buf();
                    let _ = walker_results.send(Result::Error {
                        path,
                        worker_id: 0,
                        error: io::Error::other(err.to_string()),
                    });
                    continue;
                }
            };
            let file_type = entry.file_type();
            if !file_type.is_file() {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) => {
                    let _ = walker_results.send(Result::Error {
                        path: entry.path().to_path_buf(),
                        worker_id: 0,
                        error: io::Error::other(err.to_string()),
                    });
                    continue;
                }
            };
            let job = Job {
                path: entry.path().to_path_buf(),
                size: metadata.len() as i64,
                mtime: mtime_of(&metadata),
            };
            let mut pending = job;
            loop {
                if walker_cancel.load(Ordering::Relaxed) {
                    return;
                }
                match job_tx.try_send(pending) {
                    Ok(()) => break,
                    Err(std::sync::mpsc::TrySendError::Full(job)) => {
                        pending = job;
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
                }
            }
        }
    });

    let active = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for worker_id in 0..workers {
        let job_rx = job_rx.clone();
        let result_tx = result_tx.clone();
        let db = db.clone();
        let cancel = cancel.clone();
        let active = active.clone();
        handles.push(thread::spawn(move || {
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                let job = match job_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(std::time::Duration::from_millis(50))
                {
                    Ok(job) => job,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                };
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                match unchanged(&db, &job.path, job.size, job.mtime) {
                    Ok(true) => {
                        if result_tx
                            .send(Result::Skipped {
                                path: job.path,
                                worker_id,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    Ok(false) => {
                        if result_tx
                            .send(Result::Started {
                                path: job.path.clone(),
                                worker_id,
                            })
                            .is_err()
                        {
                            return;
                        }
                        let n = active.fetch_add(1, Ordering::Relaxed) + 1;
                        let hash_path = job.path.clone();
                        let hash_cancel = cancel.clone();
                        let mut on_read = |bytes: i64| {
                            if hash_cancel.load(Ordering::Relaxed) {
                                return false;
                            }
                            result_tx
                                .send(Result::Read {
                                    path: hash_path.clone(),
                                    worker_id,
                                    bytes,
                                })
                                .is_ok()
                        };
                        let md5 = hash_file(&job.path, &mut on_read);
                        active.fetch_sub(1, Ordering::Relaxed);
                        match md5 {
                            Ok(md5) => {
                                if result_tx
                                    .send(Result::Hashed {
                                        path: job.path.clone(),
                                        worker_id,
                                        active: n,
                                        record: FileRecord::new(
                                            &job.path, job.size, md5, job.mtime,
                                        ),
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Err(error) if cancel.load(Ordering::Relaxed) => {
                                let _ = error;
                                return;
                            }
                            Err(error) => {
                                if result_tx
                                    .send(Result::Error {
                                        path: job.path,
                                        worker_id,
                                        error,
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                            }
                        }
                    }
                    Err(error) => {
                        if result_tx
                            .send(Result::Error {
                                path: job.path,
                                worker_id,
                                error,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        }));
    }
    drop(result_tx);
    drop(job_rx);

    let mut current = 0usize;
    for result in result_rx {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let progress = match result {
            Result::Error {
                path,
                worker_id,
                error,
            } => Progress {
                path,
                worker_id,
                current,
                active: active.load(Ordering::Relaxed),
                bytes: 0,
                done: true,
                skipped: false,
                error: Some(error),
            },
            Result::Started { path, worker_id } => Progress {
                path,
                worker_id,
                current,
                active: active.load(Ordering::Relaxed),
                bytes: 0,
                done: false,
                skipped: false,
                error: None,
            },
            Result::Read {
                path,
                worker_id,
                bytes,
            } => Progress {
                path,
                worker_id,
                current,
                active: active.load(Ordering::Relaxed),
                bytes,
                done: false,
                skipped: false,
                error: None,
            },
            Result::Skipped { path, worker_id } => {
                current += 1;
                Progress {
                    path,
                    worker_id,
                    current,
                    active: active.load(Ordering::Relaxed),
                    bytes: 0,
                    done: true,
                    skipped: true,
                    error: None,
                }
            }
            Result::Hashed {
                path,
                worker_id,
                active: n,
                record,
            } => {
                if let Err(error) = db.upsert(&record) {
                    Progress {
                        path,
                        worker_id,
                        current,
                        active: n,
                        bytes: 0,
                        done: true,
                        skipped: false,
                        error: Some(io::Error::other(error)),
                    }
                } else {
                    current += 1;
                    Progress {
                        path,
                        worker_id,
                        current,
                        active: n,
                        bytes: 0,
                        done: true,
                        skipped: false,
                        error: None,
                    }
                }
            }
        };
        if out.send(progress).is_err() {
            return;
        }
    }

    let _ = walker.join();
    for handle in handles {
        let _ = handle.join();
    }
}

fn unchanged(db: &Db, path: &Path, size: i64, mtime: i64) -> io::Result<bool> {
    let existing = db.lookup(path).map_err(io::Error::other)?;
    Ok(existing.is_some_and(|record| record.size == size && record.mtime == mtime))
}

fn mtime_of(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Hashes `path`, reporting each read to `on_read`.
///
/// `on_read` returning false stops the read.
pub fn hash_file(path: &Path, on_read: &mut impl FnMut(i64) -> bool) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();
    let mut buffer = vec![0u8; HASH_CHUNK];
    loop {
        let n = file.read(&mut buffer)?;
        if n > 0 {
            hasher.update(&buffer[..n]);
            if !on_read(n as i64) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "scan cancelled"));
            }
        }
        if n == 0 {
            break;
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn collect(rx: Receiver<Progress>) -> Vec<Progress> {
        rx.into_iter().collect()
    }

    #[test]
    fn indexes_every_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/b.txt"), b"hello").unwrap();
        let db = Arc::new(Db::open_in_memory().unwrap());
        let progress = collect(scan(
            db.clone(),
            dir.path().to_path_buf(),
            2,
            Arc::new(AtomicBool::new(false)),
        ));
        let done: Vec<_> = progress
            .iter()
            .filter(|p| p.done && p.error.is_none())
            .collect();
        assert_eq!(done.len(), 2);
        assert_eq!(db.stats().unwrap().0, 2);
        let groups = db.duplicates().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
    }

    #[test]
    fn a_second_scan_skips_unchanged_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        let db = Arc::new(Db::open_in_memory().unwrap());
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = collect(scan(
            db.clone(),
            dir.path().to_path_buf(),
            1,
            cancel.clone(),
        ));
        let progress = collect(scan(db.clone(), dir.path().to_path_buf(), 1, cancel));
        assert!(progress.iter().any(|p| p.done && p.skipped));
        assert_eq!(db.stats().unwrap().0, 1);
    }

    #[test]
    fn a_changed_file_is_rehashed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, b"hello").unwrap();
        let db = Arc::new(Db::open_in_memory().unwrap());
        let cancel = Arc::new(AtomicBool::new(false));
        let _ = collect(scan(
            db.clone(),
            dir.path().to_path_buf(),
            1,
            cancel.clone(),
        ));
        fs::write(&path, b"a different length").unwrap();
        let _ = collect(scan(db.clone(), dir.path().to_path_buf(), 1, cancel));
        let record = db.lookup(&path).unwrap().unwrap();
        assert_eq!(
            record.md5,
            format!("{:x}", Md5::digest(b"a different length"))
        );
    }

    #[test]
    fn a_missing_root_reports_an_error() {
        let db = Arc::new(Db::open_in_memory().unwrap());
        let progress = collect(scan(
            db,
            PathBuf::from("/nonexistent-onley-test-root"),
            1,
            Arc::new(AtomicBool::new(false)),
        ));
        assert!(progress.iter().any(|p| p.error.is_some()));
    }

    #[test]
    fn index_file_skips_an_unchanged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, b"hello").unwrap();
        let db = Db::open_in_memory().unwrap();
        assert_eq!(
            index_file(&path, &db).unwrap(),
            Some(format!("{:x}", Md5::digest(b"hello")))
        );
        assert_eq!(index_file(&path, &db).unwrap(), None);
    }

    #[test]
    fn a_cancelled_scan_stops_early() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            fs::write(dir.path().join(format!("{i}.txt")), b"same").unwrap();
        }
        let db = Arc::new(Db::open_in_memory().unwrap());
        let cancel = Arc::new(AtomicBool::new(true));
        let progress = collect(scan(db.clone(), dir.path().to_path_buf(), 2, cancel));
        assert!(progress.len() < 20);
    }

    #[test]
    fn hashing_reports_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        fs::write(&path, vec![7u8; HASH_CHUNK + 10]).unwrap();
        let mut total = 0;
        let digest = hash_file(&path, &mut |n| {
            total += n;
            true
        })
        .unwrap();
        assert_eq!(total as usize, HASH_CHUNK + 10);
        assert_eq!(digest.len(), 32);
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;

    #[test]
    fn cancelling_stops_a_large_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.bin");
        std::fs::write(&path, vec![3u8; 8 << 20]).unwrap();
        let db = Arc::new(Db::open_in_memory().unwrap());
        let cancel = Arc::new(AtomicBool::new(false));
        let rx = scan(db, dir.path().to_path_buf(), 1, cancel.clone());
        std::thread::sleep(std::time::Duration::from_millis(1));
        cancel.store(true, Ordering::Relaxed);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err()
            {
                return;
            }
        }
        panic!("the scan did not stop after cancellation");
    }
}
