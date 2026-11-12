//! Runs the command line as a user does.

use onley::Options;
use onley_core::db::{Db, FileRecord};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_with_home(home: &Path, args: &[&str], input: &str) -> Output {
    let mut stdin = Cursor::new(input.as_bytes().to_vec());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let options = Options {
        interactive: false,
        home: Some(home),
        env_master: Some(String::new()),
    };
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    let code = onley::run_with(&args, &mut stdin, &mut stdout, &mut stderr, &options);
    Output {
        code,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    }
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let db = dir.path().join("local.db");
    (dir, db)
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn no_arguments_prints_usage() {
    let (dir, _) = fixture();
    let out = run_with_home(dir.path(), &[], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("Usage: onley"));
}

#[test]
fn an_unknown_subcommand_fails() {
    let (dir, _) = fixture();
    let out = run_with_home(dir.path(), &["nope"], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("unknown subcommand: nope"));
}

#[test]
fn scan_dupes_and_stats() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    write(&root, "a/one.txt", b"same");
    write(&root, "b/two.txt", b"same");
    write(&root, "three.txt", b"other");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();

    let scan = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    assert_eq!(scan.code, 0, "{}", scan.stderr);
    assert!(scan.stdout.contains("Scanning:"));
    assert!(scan.stdout.contains("3 file(s) indexed"));

    let dupes = run_with_home(dir.path(), &["-db", &db_text, "dupes"], "");
    assert_eq!(dupes.code, 0);
    assert!(dupes.stdout.contains("Found 1 duplicate group(s):"));
    assert!(dupes.stdout.contains("one.txt"));
    assert!(dupes.stdout.contains("two.txt"));

    let stats = run_with_home(dir.path(), &["-db", &db_text, "stats"], "");
    assert_eq!(stats.code, 0);
    assert!(stats.stdout.contains("Total indexed: 3"));
    assert!(stats.stdout.contains("Duplicates:    2"));
}

#[test]
fn a_second_scan_skips_unchanged_files() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    write(&root, "a.txt", b"same");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();
    let _ = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    let second = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    assert!(second.stdout.contains("1 unchanged skipped"));
}

#[test]
fn scan_requires_an_existing_directory() {
    let (dir, db) = fixture();
    let db_text = db.to_string_lossy().into_owned();
    let out = run_with_home(
        dir.path(),
        &["-db", &db_text, "scan", "/nonexistent-onley-dir"],
        "",
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("directory not found"));
}

#[test]
fn scan_rejects_a_second_directory() {
    let (dir, db) = fixture();
    let db_text = db.to_string_lossy().into_owned();
    let out = run_with_home(dir.path(), &["-db", &db_text, "scan", "/tmp", "/var"], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("this command takes a single directory"));
}

#[test]
fn clean_all_exists() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    let a = write(&root, "a.txt", b"same");
    let b = write(&root, "b.txt", b"same");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();
    let _ = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    let out = run_with_home(dir.path(), &["-y", "-db", &db_text, "clean-all"], "");
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("Done: 1 deleted, 0 failed."));
    assert!(a.exists());
    assert!(!b.exists());
}

#[test]
fn clean_all_can_be_declined() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    let a = write(&root, "a.txt", b"same");
    let b = write(&root, "b.txt", b"same");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();
    let _ = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    let out = run_with_home(dir.path(), &["-db", &db_text, "clean-all"], "n\n");
    assert_eq!(out.code, 0);
    assert!(out.stdout.contains("Cancelled."));
    assert!(a.exists() && b.exists());
}

#[test]
fn clean_keeps_the_chosen_copy() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    let a = write(&root, "a.txt", b"same");
    let b = write(&root, "b.txt", b"same");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();
    let _ = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    let out = run_with_home(dir.path(), &["-y", "-db", &db_text, "clean"], "1\n");
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("Done: 1 deleted, 0 failed."));
    assert!(a.exists());
    assert!(!b.exists());
}

#[test]
fn clean_without_input_fails_instead_of_cancelling() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    write(&root, "a.txt", b"same");
    write(&root, "b.txt", b"same");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();
    let _ = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    let out = run_with_home(dir.path(), &["-db", &db_text, "clean"], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("nothing was deleted"));
}

#[test]
fn a_quiet_index_reports_no_duplicates() {
    let (dir, db) = fixture();
    let db_text = db.to_string_lossy().into_owned();
    let out = run_with_home(dir.path(), &["-db", &db_text, "dupes"], "");
    assert_eq!(out.code, 0);
    assert!(out.stdout.contains("No duplicate files found."));
}

#[test]
fn config_set_and_show() {
    let (dir, _) = fixture();
    let set = run_with_home(
        dir.path(),
        &["config", "set", "master", "http://m:8080"],
        "",
    );
    assert_eq!(set.code, 0, "{}", set.stderr);
    assert!(set.stdout.contains("master = http://m:8080"));
    let show = run_with_home(dir.path(), &["config", "show"], "");
    assert_eq!(show.code, 0);
    assert!(show.stdout.contains("master: http://m:8080"));
}

#[test]
fn config_rejects_a_bad_url() {
    let (dir, _) = fixture();
    let out = run_with_home(dir.path(), &["config", "set", "master", "no-scheme"], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("needs an http:// or https:// scheme"));
}

#[test]
fn config_rejects_an_unknown_key() {
    let (dir, _) = fixture();
    let out = run_with_home(dir.path(), &["config", "set", "other", "x"], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("unknown setting"));
}

#[test]
fn the_default_index_lives_under_the_home_directory() {
    let (dir, _) = fixture();
    let home = dir.path().join("home");
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    write(&root, "a.txt", b"a");
    let root_text = root.to_string_lossy().into_owned();
    let out = run_with_home(&home, &["scan", &root_text], "");
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(home.join(".onley").join("local.db").exists());
}

#[test]
fn replica_check_needs_a_master() {
    let (dir, db) = fixture();
    let db_text = db.to_string_lossy().into_owned();
    let out = run_with_home(dir.path(), &["-db", &db_text, "replica", "check"], "");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("no master is configured"));
}

#[test]
fn the_database_keeps_the_schema_the_go_version_wrote() {
    let (dir, db) = fixture();
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    let a = write(&root, "a.txt", b"same");
    let b = write(&root, "b.txt", b"same");
    let db_text = db.to_string_lossy().into_owned();
    let root_text = root.to_string_lossy().into_owned();
    let _ = run_with_home(dir.path(), &["-db", &db_text, "scan", &root_text], "");
    let store = Db::open(&db).unwrap();
    let groups = store.duplicates().unwrap();
    assert_eq!(groups.len(), 1);
    let paths: Vec<PathBuf> = groups[0]
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    assert!(paths.contains(&a));
    assert!(paths.contains(&b));
    assert_eq!(groups[0].files[0].md5.len(), 32);
}

#[test]
fn the_go_schema_is_read_directly() {
    let (dir, db) = fixture();
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE files (id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT NOT NULL UNIQUE,
             name TEXT NOT NULL, size INTEGER NOT NULL, md5 TEXT NOT NULL,
             mtime INTEGER NOT NULL DEFAULT 0, scanned_at DATETIME NOT NULL DEFAULT (datetime('now')));
             CREATE INDEX idx_files_md5 ON files(md5);
             INSERT INTO files(path,name,size,md5,mtime) VALUES
             ('/a/one','one',4,'abcd','1'),('/b/two','two',4,'abcd','1');",
        )
        .unwrap();
    }
    let out = run_with_home(dir.path(), &["-db", &db.to_string_lossy(), "dupes"], "");
    assert!(out.stdout.contains("Found 1 duplicate group(s):"));
}

#[test]
fn a_real_binary_runs_and_prints_stats() {
    let (dir, db) = fixture();
    let binary = env!("CARGO_BIN_EXE_onley");
    let output = Command::new(binary)
        .args(["-db", &db.to_string_lossy(), "stats"])
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Total indexed: 0"));
    let _ = FileRecord::new("/x", 1, "y", 0);
}

#[test]
fn replica_check_moves_files_to_the_master() {
    use onley_core::replica::Server;
    use onley_core::scanner;
    use std::sync::Arc;

    let (dir, db) = fixture();
    // The master's own index and store.
    let master_dir = dir.path().join("master");
    std::fs::create_dir_all(&master_dir).unwrap();
    draw(&master_dir, "m.txt", b"shared");
    let master_db = Arc::new(Db::open(&dir.path().join("master.db")).unwrap());
    for event in scanner::scan(
        master_db.clone(),
        master_dir.clone(),
        1,
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
    ) {
        assert!(event.error.is_none());
    }
    let store_dir = dir.path().join("store");
    std::fs::create_dir_all(&store_dir).unwrap();
    let server = Server::new(master_db, store_dir.clone());
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    };
    std::thread::spawn(move || {
        let _ = server.serve(port);
    });

    // The replica's own index.
    let replica_dir = dir.path().join("replica");
    std::fs::create_dir_all(&replica_dir).unwrap();
    draw(&replica_dir, "copy.txt", b"shared");
    draw(&replica_dir, "unique.txt", b"replica-only");
    let replica_db = Db::open(&db).unwrap();
    for event in scanner::scan(
        Arc::new(replica_db),
        replica_dir.clone(),
        1,
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
    ) {
        assert!(event.error.is_none());
    }

    let master = format!("http://127.0.0.1:{port}");
    // The server thread's first call to `serve` compiles on the JIT; wait until it answers.
    let client = onley_core::replica::Client::new(&master);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while client.ping().is_err() {
        assert!(std::time::Instant::now() < deadline, "master did not start");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let out = run_with_home(
        dir.path(),
        &[
            "-db",
            &db.to_string_lossy(),
            "replica",
            "check",
            "-master",
            &master,
        ],
        "y\n",
    );
    assert_eq!(
        out.code, 0,
        "stdout: {}\nstderr: {}",
        out.stdout, out.stderr
    );
    assert!(out.stdout.contains("1 migrated"));
    assert!(!replica_dir.join("copy.txt").exists());
    assert!(!replica_dir.join("unique.txt").exists());
    let stored: Vec<_> = std::fs::read_dir(&store_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();
    assert!(!stored.is_empty());
}

fn draw(dir: &Path, name: &str, bytes: &[u8]) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
}
