//! Serves the master API to replicas.

use crate::db::{Db, FileRecord};
use axum::{
    Json,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use md5::{Digest, Md5};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where ingested files are written.
#[derive(Clone)]
pub struct Server {
    db: Arc<Db>,
    store_dir: PathBuf,
}

impl Server {
    pub fn new(db: Arc<Db>, store_dir: impl Into<PathBuf>) -> Self {
        Self {
            db,
            store_dir: store_dir.into(),
        }
    }

    /// Starts the HTTP server and blocks until it stops.
    pub fn serve(self, port: u16) -> io::Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async move {
            let state = Arc::new(self);
            let app = axum::Router::new()
                .route("/v1/health", get(health))
                .route("/v1/check", get(check))
                .route("/v1/ingest", post(ingest))
                .layer(DefaultBodyLimit::disable())
                .with_state(state);
            let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
            axum::serve(listener, app).await.map_err(io::Error::other)
        })
    }
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}

async fn check(
    State(server): State<Arc<Server>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let Some(md5) = params.get("md5") else {
        return (StatusCode::BAD_REQUEST, "missing md5 parameter").into_response();
    };
    match server.db.find_by_md5(md5) {
        Ok(files) => {
            let paths: Vec<String> = files
                .iter()
                .map(|file| file.path.to_string_lossy().into_owned())
                .collect();
            let mut body = json!({ "found": !files.is_empty() });
            if !paths.is_empty() {
                body["paths"] = json!(paths);
            }
            Json(body).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

async fn ingest(State(server): State<Arc<Server>>, mut multipart: Multipart) -> Response {
    let mut md5 = String::new();
    let mut uploaded: Option<Uploaded> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(err) => {
                return (StatusCode::BAD_REQUEST, format!("parse form: {err}")).into_response();
            }
        };
        match field.name() {
            Some("file") => {
                let filename = field
                    .file_name()
                    .map(safe_filename)
                    .unwrap_or_else(|| "file".to_owned());
                match save_field(server.store_dir.clone(), &filename, field).await {
                    Ok(saved) => uploaded = Some(saved),
                    Err(err) => {
                        let _ = std::fs::remove_file(&err.1);
                        return (StatusCode::INTERNAL_SERVER_ERROR, err.0).into_response();
                    }
                }
            }
            Some("md5") => md5 = field.text().await.unwrap_or_default(),
            Some(_) => {
                let _ = field.bytes().await;
            }
            None => {}
        }
    }

    let Some(uploaded) = uploaded else {
        return (
            StatusCode::BAD_REQUEST,
            "missing file field: no file part in the request",
        )
            .into_response();
    };

    if !is_md5_hex(&md5) {
        let _ = std::fs::remove_file(&uploaded.temp);
        return (
            StatusCode::BAD_REQUEST,
            "missing or invalid md5: want 32 lowercase hex characters, as produced by the replica",
        )
            .into_response();
    }

    if uploaded.digest != md5 {
        let _ = std::fs::remove_file(&uploaded.temp);
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "md5 mismatch: the upload declares {md5} but hashes to {}, so it is not the file the replica indexed",
                uploaded.digest
            ),
        )
            .into_response();
    }

    let dest_dir = server.store_dir.join(&md5[..2]).join(&md5[2..]);
    if let Err(err) = std::fs::create_dir_all(&dest_dir) {
        let _ = std::fs::remove_file(&uploaded.temp);
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("mkdir: {err}")).into_response();
    }
    let dest = dest_dir.join(&uploaded.filename);
    if let Err(err) = std::fs::rename(&uploaded.temp, &dest) {
        let _ = std::fs::remove_file(&uploaded.temp);
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("create: {err}")).into_response();
    }

    let record = FileRecord::new(&dest, uploaded.size, &md5, 0);
    if let Err(err) = server.db.upsert(&record) {
        let _ = std::fs::remove_file(&dest);
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("index: {err}")).into_response();
    }

    Json(json!({ "ok": true, "path": dest.to_string_lossy() })).into_response()
}

struct Uploaded {
    temp: PathBuf,
    filename: String,
    size: i64,
    digest: String,
}

/// Streams one part to a temporary file and hashes what arrived.
async fn save_field(
    dir: PathBuf,
    filename: &str,
    mut field: axum::extract::multipart::Field<'_>,
) -> Result<Uploaded, (String, PathBuf)> {
    let temp = dir.join(format!(
        ".onley-upload-{}-{}",
        std::process::id(),
        next_counter()
    ));
    let mut file = match std::fs::File::create(&temp) {
        Ok(file) => file,
        Err(err) => return Err((format!("create: {err}"), temp)),
    };
    let mut hasher = Md5::new();
    let mut size = 0i64;
    loop {
        match field.chunk().await {
            Ok(Some(chunk)) => {
                hasher.update(&chunk);
                size += chunk.len() as i64;
                if let Err(err) = file.write_all(&chunk) {
                    return Err((format!("write: {err}"), temp));
                }
            }
            Ok(None) => break,
            Err(err) => return Err((format!("write: {err}"), temp)),
        }
    }
    if let Err(err) = file.sync_all() {
        return Err((format!("write: {err}"), temp));
    }
    drop(file);
    Ok(Uploaded {
        temp,
        filename: filename.to_owned(),
        size,
        digest: crate::hex_digest(hasher.finalize().as_slice()),
    })
}

/// Keeps the last path component, so a name cannot write outside the store.
fn safe_filename(name: &str) -> String {
    Path::new(name)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned())
}

/// Reports whether `value` is a 32-character lowercase hex digest.
fn is_md5_hex(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn next_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_hex_accepts_lowercase_only() {
        assert!(is_md5_hex("0123456789abcdef0123456789abcdef"));
        assert!(!is_md5_hex("0123456789ABCDEF0123456789abcdef"));
        assert!(!is_md5_hex("short"));
    }

    #[test]
    fn file_names_lose_their_directories() {
        assert_eq!(safe_filename("book.pdf"), "book.pdf");
        assert_eq!(safe_filename("../../etc/passwd"), "passwd");
        assert_eq!(safe_filename("/tmp/a"), "a");
    }
}
