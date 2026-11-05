//! Pushes files to a master in batches.

use super::Client;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// How often a syncer drains its queue when no interval is given.
pub const DEFAULT_SYNC_INTERVAL: Duration = Duration::from_secs(30);

/// Pushes files to a master in batches.
///
/// It uploads and nothing else. It never removes a local file, because a
/// watcher pointed at a directory a person saves things into is exactly where
/// that would be most surprising.
pub struct Syncer {
    client: Client,
    interval: Duration,
    max_backoff: Duration,
    pending: Mutex<HashMap<PathBuf, String>>,
    current: Mutex<Duration>,
}

impl Syncer {
    pub fn new(client: Client, interval: Duration) -> Self {
        let interval = if interval.is_zero() {
            DEFAULT_SYNC_INTERVAL
        } else {
            interval
        };
        Self {
            client,
            interval,
            max_backoff: interval * 16,
            pending: Mutex::new(HashMap::new()),
            current: Mutex::new(interval),
        }
    }

    /// Adds a path and the digest already computed for it.
    pub fn enqueue(&self, md5: &str, path: &Path) {
        self.pending
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), md5.to_string());
    }

    /// Returns how many paths are waiting.
    pub fn pending(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    /// Returns how long the caller should wait before draining again.
    pub fn next_interval(&self, last_round_ok: bool) -> Duration {
        let mut current = self.current.lock().unwrap();
        if last_round_ok {
            *current = self.interval;
            return *current;
        }
        *current *= 2;
        if *current > self.max_backoff {
            *current = self.max_backoff;
        }
        *current
    }

    /// Sends the queued files.
    pub fn sync(&self) -> Result {
        let batch: HashMap<PathBuf, String> = {
            let mut pending = self.pending.lock().unwrap();
            std::mem::take(&mut *pending)
        };
        let mut result = Result::default();
        if batch.is_empty() {
            return result;
        }

        let mut first_of: HashMap<String, PathBuf> = HashMap::new();
        for (path, digest) in &batch {
            first_of
                .entry(digest.clone())
                .or_insert_with(|| path.clone());
        }

        for (digest, path) in first_of {
            match self.client.check(&digest) {
                Ok(true) => {
                    result.already_present += files_with_digest(&batch, &digest);
                }
                Ok(false) => match self.client.ingest(&path, &digest) {
                    Ok(()) => result.uploaded += files_with_digest(&batch, &digest),
                    Err(_) => {
                        result.failed += 1;
                        self.enqueue(&digest, &path);
                    }
                },
                Err(_) => {
                    result.failed += 1;
                    self.enqueue(&digest, &path);
                }
            }
        }
        result
    }
}

/// Counts the queued paths that carry a digest.
fn files_with_digest(batch: &HashMap<PathBuf, String>, digest: &str) -> usize {
    batch
        .values()
        .filter(|value| value.as_str() == digest)
        .count()
}

/// Counts what one round did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Result {
    pub uploaded: usize,
    pub already_present: usize,
    pub failed: usize,
}

impl Result {
    pub fn ok(&self) -> bool {
        self.failed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_is_capped() {
        let syncer = Syncer::new(Client::new("http://127.0.0.1:1"), Duration::from_secs(1));
        assert_eq!(syncer.next_interval(true), Duration::from_secs(1));
        assert_eq!(syncer.next_interval(false), Duration::from_secs(2));
        assert_eq!(syncer.next_interval(false), Duration::from_secs(4));
        assert_eq!(syncer.next_interval(true), Duration::from_secs(1));
    }

    #[test]
    fn the_queue_counts_paths() {
        let syncer = Syncer::new(Client::new("http://127.0.0.1:1"), Duration::from_secs(1));
        syncer.enqueue("abc", Path::new("/a"));
        syncer.enqueue("abc", Path::new("/b"));
        assert_eq!(syncer.pending(), 2);
    }
}
