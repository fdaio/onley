//! Talks to a master onley server.

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use ureq::SendBody;

/// What should happen to a file on the replica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The master already has this file; delete it locally.
    DeleteLocal,
    /// The master lacks this file; transfer it to the master.
    Migrate,
}

/// One item in the replica reconciliation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEntry {
    pub path: PathBuf,
    pub md5: String,
    pub size: i64,
    pub action: Action,
}

/// Queries and uploads to a master onley server over HTTP.
#[derive(Clone)]
pub struct Client {
    master_url: String,
    agent: ureq::Agent,
}

impl Client {
    /// Creates a client pointing at `master_url`, for example
    /// `http://master-host:8080`.
    pub fn new(master_url: &str) -> Self {
        let config = ureq::Agent::config_builder()
            // The response timeout bounds a master that does not answer. It
            // does not bound how long an upload body takes.
            .timeout_global(None)
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .http_status_as_error(false)
            .build();
        Self {
            master_url: master_url.trim_end_matches('/').to_string(),
            agent: config.into(),
        }
    }

    /// Returns `Ok` if the master is reachable and healthy.
    pub fn ping(&self) -> Result<(), String> {
        let mut response = self
            .agent
            .get(format!("{}/v1/health", self.master_url))
            .call()
            .map_err(|err| err.to_string())?;
        if response.status().as_u16() != 200 {
            return Err(format!(
                "health check returned {}",
                response.status().as_u16()
            ));
        }
        let _ = response.body_mut().read_to_string();
        Ok(())
    }

    /// Returns whether the master already holds a file with `md5`.
    pub fn check(&self, md5: &str) -> Result<bool, String> {
        let mut response = self
            .agent
            .get(format!("{}/v1/check?md5={md5}", self.master_url))
            .call()
            .map_err(|err| err.to_string())?;
        if response.status().as_u16() != 200 {
            return Err(format!("check returned {}", response.status().as_u16()));
        }
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|err| err.to_string())?;
        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|err| err.to_string())?;
        Ok(value
            .get("found")
            .and_then(|found| found.as_bool())
            .unwrap_or(false))
    }

    /// Uploads the file at `local_path` to the master.
    pub fn ingest(&self, local_path: &Path, md5: &str) -> Result<(), String> {
        let file =
            File::open(local_path).map_err(|err| format!("{}: {err}", local_path.display()))?;
        let name = local_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_owned());
        let boundary = format!("onley-{}-{}", std::process::id(), next_boundary());
        let head = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n",
            name.replace('"', "_")
        );
        let tail = format!(
            "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"md5\"\r\n\r\n{md5}\
             \r\n--{boundary}\r\nContent-Disposition: form-data; name=\"path\"\r\n\r\n{}\
             \r\n--{boundary}--\r\n",
            local_path.display()
        );
        let mut body = Cursor::new(head.into_bytes())
            .chain(file)
            .chain(Cursor::new(tail.into_bytes()));

        let response = self
            .agent
            .post(format!("{}/v1/ingest", self.master_url))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .send(SendBody::from_reader(&mut body))
            .map_err(|err| err.to_string())?;
        let status = response.status().as_u16();
        if status != 200 {
            let mut response = response;
            let text = response.body_mut().read_to_string().unwrap_or_default();
            return Err(format!("ingest returned {status}: {}", text.trim()));
        }
        Ok(())
    }
}

fn next_boundary() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Reads a `file:` URL into a local path.
pub fn file_url_path(raw: &str) -> Option<PathBuf> {
    let url = raw.strip_prefix("file://")?;
    Some(PathBuf::from(percent_decode(url)))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&value[i + 1..i + 3], 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_urls_become_paths() {
        assert_eq!(
            file_url_path("file:///tmp/a%20b"),
            Some(PathBuf::from("/tmp/a b"))
        );
        assert_eq!(file_url_path("https://example.com"), None);
    }
}
