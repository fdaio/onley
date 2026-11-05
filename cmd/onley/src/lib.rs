//! The onley command line.

use onley_core::{appdir, cleaner, config, db::Db, replica, scanner, ui, watcher};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

const LEGACY_DB_FILE: &str = "onley.db";
const MASTER_ENV_VAR: &str = "ONLEY_MASTER";

/// Options the tests replace.
#[derive(Default)]
pub struct Options<'a> {
    /// Whether standard output is a terminal someone is watching.
    pub interactive: bool,
    /// The home directory to use instead of the system's.
    pub home: Option<&'a Path>,
    /// The master override to use instead of `ONLEY_MASTER`.
    pub env_master: Option<String>,
}

/// Runs the command line and returns its exit code.
pub fn run(
    args: &[String],
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    run_with(args, stdin, stdout, stderr, &Options::default())
}

/// Runs the command line with explicit options.
pub fn run_with(
    args: &[String],
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    options: &Options<'_>,
) -> i32 {
    let mut db_path: Option<PathBuf> = None;
    let mut yes = false;
    let mut workers = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    let mut rest: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            rest.extend_from_slice(&args[i + 1..]);
            break;
        }
        if arg == "-" || !arg.starts_with('-') {
            rest.extend_from_slice(&args[i..]);
            break;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        match name {
            "-db" | "--db" => {
                let value = match inline {
                    Some(value) => value,
                    None => {
                        i += 1;
                        match args.get(i) {
                            Some(value) => value.clone(),
                            None => {
                                let _ = writeln!(stderr, "flag needs an argument: -db");
                                usage(stderr);
                                return 1;
                            }
                        }
                    }
                };
                db_path = Some(PathBuf::from(value));
            }
            "-y" | "--y" | "-yes" | "--yes" => {
                yes = inline.map(|v| v != "false").unwrap_or(true);
            }
            "-workers" | "--workers" => {
                let value = match inline {
                    Some(value) => value,
                    None => {
                        i += 1;
                        match args.get(i) {
                            Some(value) => value.clone(),
                            None => {
                                let _ = writeln!(stderr, "flag needs an argument: -workers");
                                usage(stderr);
                                return 1;
                            }
                        }
                    }
                };
                match value.parse::<usize>() {
                    Ok(n) => workers = n,
                    Err(_) => {
                        let _ = writeln!(
                            stderr,
                            "invalid value {:?} for flag -workers: parse error",
                            value
                        );
                        usage(stderr);
                        return 1;
                    }
                }
            }
            "-h" | "--help" | "-help" => {
                usage(stderr);
                return 1;
            }
            _ => {
                let _ = writeln!(stderr, "flag provided but not defined: {name}");
                usage(stderr);
                return 1;
            }
        }
        i += 1;
    }

    if rest.is_empty() {
        usage(stderr);
        return 1;
    }
    workers = workers.max(1);

    let db_path = match db_path {
        Some(path) => path,
        None => match default_db_path(options) {
            Ok(path) => path,
            Err(err) => {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
        },
    };

    match rest[0].as_str() {
        "scan" => {
            if rest.len() < 2 {
                let _ = writeln!(stderr, "error: scan requires a directory argument");
                return 1;
            }
            if let Err(err) = reject_extra_dirs(&rest[2..]) {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
            cmd_scan(
                &db_path,
                &rest[1],
                workers,
                stdout,
                stderr,
                options.interactive,
            )
        }
        "watch" => {
            if rest.len() < 2 {
                let _ = writeln!(stderr, "error: watch requires a directory argument");
                return 1;
            }
            cmd_watch(&db_path, &rest[1], &rest[2..], stdout, stderr, options)
        }
        "dupes" => {
            if let Err(err) = reject_extra_dirs(&rest[1..]) {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
            cmd_dupes(&db_path, stdout, stderr)
        }
        "clean" => {
            if let Err(err) = reject_extra_args(&rest[1..]) {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
            cmd_clean(&db_path, yes, stdin, stdout, stderr)
        }
        "clean-all" => {
            if let Err(err) = reject_extra_args(&rest[1..]) {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
            cmd_clean_all(&db_path, yes, stdin, stdout, stderr)
        }
        "stats" => {
            if let Err(err) = reject_extra_args(&rest[1..]) {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
            cmd_stats(&db_path, stdout, stderr)
        }
        "config" => {
            if rest.len() < 2 {
                let _ = writeln!(
                    stderr,
                    "error: config requires a subcommand, e.g.: config set master <url>"
                );
                return 1;
            }
            cmd_config(&rest[1], &rest[2..], stdout, stderr, options)
        }
        "serve" => cmd_serve(&db_path, &rest[1..], stdout, stderr),
        "replica" => {
            if rest.len() < 2 {
                let _ = writeln!(
                    stderr,
                    "error: replica requires a subcommand, e.g.: replica check"
                );
                return 1;
            }
            match rest[1].as_str() {
                "check" => {
                    cmd_replica_check(&db_path, yes, &rest[2..], stdin, stdout, stderr, options)
                }
                other => {
                    let _ = writeln!(stderr, "unknown replica subcommand: {other}");
                    1
                }
            }
        }
        other => {
            let _ = writeln!(stderr, "unknown subcommand: {other}");
            usage(stderr);
            1
        }
    }
}

/// The entry point of the binary.
pub fn main_entry() -> i32 {
    use std::io::IsTerminal;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    let options = Options {
        interactive: std::io::stdout().is_terminal(),
        home: None,
        env_master: None,
    };
    run_with(&args, &mut stdin, &mut stdout, &mut stderr, &options)
}

fn usage(stderr: &mut dyn Write) {
    let _ = writeln!(stderr, "Usage: onley [options] <subcommand> [args]\n");
    let _ = writeln!(stderr, "Subcommands:");
    let _ = writeln!(stderr, "  scan <dir>           index files in a directory");
    let _ = writeln!(
        stderr,
        "  watch <dir>          keep the index up to date as files change"
    );
    let _ = writeln!(stderr, "  dupes                list all duplicate files");
    let _ = writeln!(
        stderr,
        "  clean                interactively remove duplicates"
    );
    let _ = writeln!(
        stderr,
        "  clean-all            keep first file per group, delete the rest (with confirmation)"
    );
    let _ = writeln!(stderr, "  stats                show index statistics");
    let _ = writeln!(stderr, "  serve                start master HTTP server");
    let _ = writeln!(
        stderr,
        "  replica check        compare with master and apply plan"
    );
    let _ = writeln!(
        stderr,
        "  config <sub>         store or display settings (config set master <url>)\n"
    );
    let _ = writeln!(stderr, "Options:");
    let _ = writeln!(stderr, "  -db string");
    let _ = writeln!(
        stderr,
        "        SQLite database path (default: ~/{}/{})",
        appdir::DIR_NAME,
        appdir::DB_FILE_NAME
    );
    let _ = writeln!(stderr, "  -workers int");
    let _ = writeln!(stderr, "        number of concurrent workers");
    let _ = writeln!(stderr, "  -y\tassume yes for confirmations");
    let _ = writeln!(stderr, "  -yes");
    let _ = writeln!(stderr, "        same as -y");
}

fn reject_extra_args(extra: &[String]) -> Result<(), String> {
    if extra.is_empty() {
        return Ok(());
    }
    let quoted: Vec<String> = extra.iter().map(|arg| format!("{arg:?}")).collect();
    let looks_like_flag = extra.iter().any(|arg| arg.starts_with('-') && arg != "-");
    let err = if quoted.len() == 1 {
        format!("unexpected argument {}", quoted[0])
    } else {
        format!("unexpected arguments {}", quoted.join(", "))
    };
    if looks_like_flag {
        return Err(format!(
            "{err}; options go before the subcommand, e.g.: onley -y clean-all"
        ));
    }
    Err(err)
}

fn reject_extra_dirs(extra: &[String]) -> Result<(), String> {
    if extra.is_empty() {
        return Ok(());
    }
    let quoted: Vec<String> = extra.iter().map(|arg| format!("{arg:?}")).collect();
    if quoted.len() == 1 {
        return Err(format!(
            "unexpected argument {}; this command takes a single directory",
            quoted[0]
        ));
    }
    Err(format!(
        "unexpected arguments {}; this command takes a single directory",
        quoted.join(", ")
    ))
}

fn report_no_input(err: &ui::NoInput, stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "error: {err}; nothing was deleted");
    let _ = writeln!(
        stderr,
        "hint: run this from a terminal, or feed the answer on stdin, e.g. echo y |"
    );
    1
}

fn open_db(path: &Path, stderr: &mut dyn Write) -> Option<Arc<Db>> {
    match Db::open(path) {
        Ok(db) => Some(Arc::new(db)),
        Err(err) => {
            let _ = writeln!(stderr, "failed to open database: {err}");
            None
        }
    }
}

fn home_of(options: &Options<'_>) -> Option<PathBuf> {
    options.home.map(Path::to_path_buf).or_else(dirs::home_dir)
}

fn default_db_path(options: &Options<'_>) -> Result<PathBuf, String> {
    let Some(home) = home_of(options) else {
        return Err("cannot locate the home directory, pass -db to choose an index".to_string());
    };
    let target =
        appdir::db_path_in(&home).map_err(|err| format!("{err}, pass -db to choose an index"))?;
    adopt_legacy_db(&target);
    Ok(target)
}

fn default_config_path(options: &Options<'_>) -> Result<PathBuf, String> {
    let Some(home) = home_of(options) else {
        return Err("cannot locate the home directory".to_string());
    };
    appdir::config_path_in(&home).map_err(|err| err.to_string())
}

fn adopt_legacy_db(target: &Path) {
    if target.exists() {
        return;
    }
    let legacy = Path::new(LEGACY_DB_FILE);
    if !legacy.exists() {
        return;
    }
    match copy_file(legacy, target) {
        Ok(true) => {
            let _ = writeln!(
                std::io::stderr(),
                "Copied {LEGACY_DB_FILE} to {}. Pass -db {LEGACY_DB_FILE} to keep using the old index.",
                target.display()
            );
        }
        Ok(false) => {}
        Err(err) => {
            let _ = writeln!(
                std::io::stderr(),
                "warning: could not copy {LEGACY_DB_FILE} to {}: {err}",
                target.display()
            );
        }
    }
}

/// Copies `src` to `dst` without overwriting, and reports whether it did.
fn copy_file(src: &Path, dst: &Path) -> std::io::Result<bool> {
    let mut input = std::fs::File::open(src)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(dst) {
        Ok(mut output) => {
            std::io::copy(&mut input, &mut output)?;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(err),
    }
}

fn cmd_scan(
    db_path: &Path,
    dir: &str,
    workers: usize,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    interactive: bool,
) -> i32 {
    let Ok(abs_dir) = std::path::absolute(dir) else {
        let _ = writeln!(stderr, "failed to resolve path: {dir}");
        return 1;
    };
    if let Err(err) = std::fs::metadata(&abs_dir) {
        let _ = writeln!(stderr, "directory not found: {err}");
        return 1;
    }

    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };

    let _ = writeln!(stdout, "Scanning: {}", abs_dir.display());

    let mut worker_files = vec![String::new(); workers];
    let mut err_count = 0usize;
    let mut skipped_count = 0usize;
    let mut current = 0usize;
    let start = Instant::now();
    let mut drawn = false;

    let redraw =
        |stdout: &mut dyn Write, worker_files: &[String], current: usize, drawn: &mut bool| {
            if !interactive {
                return;
            }
            if *drawn {
                let _ = write!(stdout, "\x1b[{}A", workers + 1);
            }
            for (i, file) in worker_files.iter().enumerate() {
                if file.is_empty() {
                    let _ = write!(stdout, "\x1b[2K\r  worker {:2}: —\n", i);
                } else {
                    let _ = write!(
                        stdout,
                        "\x1b[2K\r  worker {:2}: {}\n",
                        i,
                        truncate(file, 60)
                    );
                }
            }
            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                current as f64 / elapsed
            } else {
                0.0
            };
            let _ = write!(
                stdout,
                "\x1b[2K\r  processed {:<6} | {:.1}/s\n",
                current, speed
            );
            *drawn = true;
        };

    redraw(stdout, &worker_files, current, &mut drawn);

    let cancel = Arc::new(AtomicBool::new(false));
    let rx = scanner::scan(db.clone(), abs_dir, workers, cancel);
    for progress in rx {
        if let Some(error) = progress.error {
            let _ = writeln!(stderr, "  warning: {error}");
            err_count += 1;
            continue;
        }
        if !progress.done {
            if progress.worker_id < worker_files.len() {
                worker_files[progress.worker_id] = progress.path.display().to_string();
            }
        } else {
            if progress.worker_id < worker_files.len() {
                worker_files[progress.worker_id].clear();
            }
            current = progress.current;
            if progress.skipped {
                skipped_count += 1;
            }
        }
        redraw(stdout, &worker_files, current, &mut drawn);
    }

    if let Ok((total, dups)) = db.stats() {
        let _ = writeln!(
            stdout,
            "Done: {total} file(s) indexed ({skipped_count} unchanged skipped), {dups} with duplicates."
        );
    }
    if err_count > 0 {
        let _ = writeln!(stdout, "  ({err_count} file(s) skipped due to errors)");
    }
    0
}

fn truncate(value: &str, n: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= n {
        return value.to_string();
    }
    let start = chars.len() - (n - 3);
    format!("...{}", chars[start..].iter().collect::<String>())
}

fn cmd_watch(
    db_path: &Path,
    dir: &str,
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    options: &Options<'_>,
) -> i32 {
    let mut debounce = watcher::DEFAULT_DEBOUNCE;
    let mut interval = replica::DEFAULT_SYNC_INTERVAL;
    let mut master_flag: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        let take = |i: &mut usize| -> Option<String> {
            if let Some(value) = inline.clone() {
                return Some(value);
            }
            *i += 1;
            args.get(*i).cloned()
        };
        match name {
            "-debounce" | "--debounce" => {
                match take(&mut i).and_then(|v| parse_duration(&v).ok()) {
                    Some(value) => debounce = value,
                    None => {
                        let _ = writeln!(stderr, "invalid value for flag -debounce");
                        return 1;
                    }
                }
            }
            "-interval" | "--interval" => {
                match take(&mut i).and_then(|v| parse_duration(&v).ok()) {
                    Some(value) => interval = value,
                    None => {
                        let _ = writeln!(stderr, "invalid value for flag -interval");
                        return 1;
                    }
                }
            }
            "-master" | "--master" => match take(&mut i) {
                Some(value) => master_flag = Some(value),
                None => {
                    let _ = writeln!(stderr, "flag needs an argument: -master");
                    return 1;
                }
            },
            other => {
                let _ = writeln!(stderr, "flag provided but not defined: {other}");
                return 1;
            }
        }
        i += 1;
    }

    let Ok(abs_dir) = std::path::absolute(dir) else {
        let _ = writeln!(stderr, "failed to resolve path: {dir}");
        return 1;
    };
    let metadata = match std::fs::metadata(&abs_dir) {
        Ok(metadata) => metadata,
        Err(err) => {
            let _ = writeln!(stderr, "directory not found: {err}");
            return 1;
        }
    };
    if !metadata.is_dir() {
        let _ = writeln!(stderr, "not a directory: {}", abs_dir.display());
        return 1;
    }

    let mut watcher = match watcher::Watcher::new(&abs_dir, debounce) {
        Ok(watcher) => watcher,
        Err(err) => {
            let _ = writeln!(stderr, "failed to watch {}: {err}", abs_dir.display());
            return 1;
        }
    };

    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        let _ = ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed));
    }

    let _ = writeln!(
        stdout,
        "Watching {} (debounce {}). Press Ctrl-C to stop.",
        abs_dir.display(),
        format_duration(debounce)
    );

    let (syncer, master) =
        match watch_syncer(master_flag.as_deref(), interval, stdout, stderr, options) {
            Ok(value) => value,
            Err(code) => return code,
        };
    match (&syncer, &master) {
        (None, _) => {
            let _ = writeln!(stdout, "No master configured; indexing only.");
        }
        (Some(_), Some(master)) => {
            let _ = writeln!(
                stdout,
                "Uploading new files to {master} every {}. Nothing is deleted from this machine.",
                format_duration(interval)
            );
        }
        _ => {}
    }

    let (sync_tx, sync_rx) = mpsc::channel::<(bool, String)>();
    let sync_done = Arc::new(AtomicBool::new(false));
    let sync_thread = syncer.as_ref().map(|syncer| {
        let syncer = syncer.clone();
        let stop = stop.clone();
        let done = sync_done.clone();
        std::thread::spawn(move || {
            run_sync_loop(syncer, stop, sync_tx, interval);
            done.store(true, Ordering::Relaxed);
        })
    });

    let mut indexed = 0usize;
    let mut removed = 0usize;
    let mut failed = 0usize;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match watcher.events().recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if let Some(error) = event.error {
                    let _ = writeln!(stderr, "  warning: {error}");
                    failed += 1;
                    continue;
                }
                if event.remove {
                    if let Err(err) = db.delete_record(&event.path) {
                        let _ = writeln!(
                            stderr,
                            "  index cleanup failed {}: {err}",
                            event.path.display()
                        );
                        failed += 1;
                        continue;
                    }
                    removed += 1;
                    let _ = writeln!(stdout, "  removed  {}", event.path.display());
                    continue;
                }
                match scanner::index_file(&event.path, &db) {
                    Ok(digest) => {
                        if let (Some(syncer), Some(digest)) = (&syncer, digest) {
                            syncer.enqueue(&digest, &event.path);
                        }
                        indexed += 1;
                        let _ = writeln!(stdout, "  indexed  {}", event.path.display());
                    }
                    Err(err) => {
                        let _ = writeln!(stderr, "  index failed {}: {err}", event.path.display());
                        failed += 1;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        while let Ok((to_stderr, line)) = sync_rx.try_recv() {
            if to_stderr {
                let _ = writeln!(stderr, "{line}");
            } else {
                let _ = writeln!(stdout, "{line}");
            }
        }
    }

    watcher.close();

    if let Some(thread) = sync_thread {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sync_done.load(Ordering::Relaxed) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        if !sync_done.load(Ordering::Relaxed) {
            let _ = writeln!(
                stderr,
                "warning: the final upload round did not finish in time"
            );
        }
        let _ = thread.join();
    }
    while let Ok((to_stderr, line)) = sync_rx.try_recv() {
        if to_stderr {
            let _ = writeln!(stderr, "{line}");
        } else {
            let _ = writeln!(stdout, "{line}");
        }
    }

    match db.stats() {
        Ok((total, dups)) => {
            let _ = writeln!(
                stdout,
                "\nStopped: {indexed} indexed, {removed} removed, {failed} failed. Index holds {total} file(s), {dups} with duplicates."
            );
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "  stats failed on shutdown: {err}");
            1
        }
    }
}

fn watch_syncer(
    flag_value: Option<&str>,
    interval: Duration,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    options: &Options<'_>,
) -> Result<(Option<Arc<replica::Syncer>>, Option<String>), i32> {
    let cfg_path = match default_config_path(options) {
        Ok(path) => path,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return Err(1);
        }
    };
    let cfg = match config::load(&cfg_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return Err(1);
        }
    };
    let (master, source) = resolve_master(flag_value, &cfg, options);
    if master.is_empty() {
        return Ok((None, None));
    }
    let _ = writeln!(stdout, "Master: {master} (from {source})");
    Ok((
        Some(Arc::new(replica::Syncer::new(
            replica::Client::new(&master),
            interval,
        ))),
        Some(master),
    ))
}

fn run_sync_loop(
    syncer: Arc<replica::Syncer>,
    stop: Arc<AtomicBool>,
    out: mpsc::Sender<(bool, String)>,
    _interval: Duration,
) {
    let mut wait = syncer.next_interval(true);
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        if syncer.pending() == 0 {
            continue;
        }
        let result = syncer.sync();
        if result.uploaded > 0 || result.already_present > 0 {
            let _ = out.send((
                false,
                format!(
                    "  synced  {} uploaded, {} already on the master, {} failed",
                    result.uploaded, result.already_present, result.failed
                ),
            ));
        }
        if result.failed > 0 {
            let _ = out.send((
                true,
                format!(
                    "  {} file(s) could not be pushed; they will be retried",
                    result.failed
                ),
            ));
        }
        wait = syncer.next_interval(result.ok());
    }
}

fn cmd_dupes(db_path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };
    match db.duplicates() {
        Ok(groups) => {
            let _ = ui::show_duplicates(&groups, stdout);
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "query failed: {err}");
            1
        }
    }
}

fn cmd_clean(
    db_path: &Path,
    assume_yes: bool,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };
    let groups = match db.duplicates() {
        Ok(groups) => groups,
        Err(err) => {
            let _ = writeln!(stderr, "query failed: {err}");
            return 1;
        }
    };
    if groups.is_empty() {
        let _ = writeln!(stdout, "No duplicate files found.");
        return 0;
    }
    let _ = ui::show_duplicates(&groups, stdout);
    let to_delete = match ui::clean_interactive(&groups, stdout, stdin) {
        Ok(paths) => paths,
        Err(err) => return report_no_input(&err, stderr),
    };
    if to_delete.is_empty() {
        let _ = writeln!(stdout, "No files selected, exiting.");
        return 0;
    }
    if !assume_yes {
        match ui::confirm_delete(&to_delete, stdout, stdin) {
            Ok(true) => {}
            Ok(false) => {
                let _ = writeln!(stdout, "Cancelled.");
                return 0;
            }
            Err(err) => return report_no_input(&err, stderr),
        }
    }
    let (deleted, failed) = delete_paths(&db, &to_delete, stderr);
    let _ = writeln!(stdout, "Done: {deleted} deleted, {failed} failed.");
    0
}

fn cmd_clean_all(
    db_path: &Path,
    assume_yes: bool,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };
    let groups = match db.duplicates() {
        Ok(groups) => groups,
        Err(err) => {
            let _ = writeln!(stderr, "query failed: {err}");
            return 1;
        }
    };
    if groups.is_empty() {
        let _ = writeln!(stdout, "No duplicate files found.");
        return 0;
    }

    let mut to_delete: Vec<PathBuf> = Vec::new();
    for group in &groups {
        for file in group.files.iter().skip(1) {
            to_delete.push(file.path.clone());
        }
    }

    let _ = writeln!(
        stdout,
        "{} duplicate group(s): keeping the first file in each, deleting {} other(s):\n",
        groups.len(),
        to_delete.len()
    );
    for group in &groups {
        let _ = writeln!(stdout, "  keep:   {}", group.files[0].path.display());
        for file in group.files.iter().skip(1) {
            let _ = writeln!(stdout, "  delete: {}", file.path.display());
        }
        let _ = writeln!(stdout);
    }

    if !assume_yes {
        match ui::confirm_delete(&to_delete, stdout, stdin) {
            Ok(true) => {}
            Ok(false) => {
                let _ = writeln!(stdout, "Cancelled.");
                return 0;
            }
            Err(err) => return report_no_input(&err, stderr),
        }
    }

    let (deleted, failed) = delete_paths(&db, &to_delete, stderr);
    let _ = writeln!(stdout, "Done: {deleted} deleted, {failed} failed.");
    0
}

fn delete_paths(db: &Db, paths: &[PathBuf], stderr: &mut dyn Write) -> (usize, usize) {
    let targets: Vec<cleaner::Target> = paths.iter().map(cleaner::Target::new).collect();
    let mut deleted = 0usize;
    let mut failed = 0usize;
    for result in cleaner::delete(db, &targets, false) {
        if let Some(error) = result.error {
            let _ = writeln!(stderr, "  delete failed {}: {error}", result.path.display());
            failed += 1;
            continue;
        }
        if let Some(error) = result.index_error {
            let _ = writeln!(
                stderr,
                "  index cleanup failed {}: {error}",
                result.path.display()
            );
        }
        deleted += 1;
    }
    (deleted, failed)
}

fn cmd_stats(db_path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };
    match db.stats() {
        Ok((total, dups)) => {
            let _ = writeln!(stdout, "Total indexed: {total}");
            let _ = writeln!(stdout, "Duplicates:    {dups}");
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "stats query failed: {err}");
            1
        }
    }
}

fn cmd_config(
    action: &str,
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    options: &Options<'_>,
) -> i32 {
    let path = match default_config_path(options) {
        Ok(path) => path,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return 1;
        }
    };
    match action {
        "show" => config_show(&path, stdout, stderr, options),
        "set" => {
            if args.len() < 2 {
                let _ = writeln!(
                    stderr,
                    "error: config set requires a key and a value, e.g.: config set master http://host:8080"
                );
                return 1;
            }
            if let Err(err) = reject_extra_args(&args[2..]) {
                let _ = writeln!(stderr, "error: {err}");
                return 1;
            }
            config_set(&path, &args[0], &args[1], stdout, stderr)
        }
        other => {
            let _ = writeln!(stderr, "unknown config subcommand: {other}");
            1
        }
    }
}

fn config_show(
    path: &Path,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    options: &Options<'_>,
) -> i32 {
    let cfg = match config::load(path) {
        Ok(cfg) => cfg,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return 1;
        }
    };
    let _ = writeln!(stdout, "{}", path.display());
    if cfg.master.is_empty() {
        let _ = writeln!(stdout, "  master: (not set)");
    } else {
        let _ = writeln!(stdout, "  master: {}", cfg.master);
    }
    if let Some(value) = env_master(options)
        && !value.is_empty()
    {
        let _ = writeln!(stdout, "  {MASTER_ENV_VAR} overrides master: {value}");
    }
    0
}

fn config_set(
    path: &Path,
    key: &str,
    value: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if key != "master" {
        let _ = writeln!(
            stderr,
            "error: unknown setting {key:?}; only \"master\" can be set"
        );
        return 1;
    }
    if let Err(err) = validate_master_url(value) {
        let _ = writeln!(stderr, "error: {err}");
        return 1;
    }
    let mut cfg = match config::load(path) {
        Ok(cfg) => cfg,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return 1;
        }
    };
    cfg.master = value.to_string();
    if let Err(err) = cfg.save(path) {
        let _ = writeln!(stderr, "error: {err}");
        return 1;
    }
    let _ = writeln!(stdout, "master = {value}");
    let _ = writeln!(stdout, "Written to {}", path.display());
    0
}

fn validate_master_url(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("master URL must not be empty".to_string());
    }
    if !raw.starts_with("http://") && !raw.starts_with("https://") {
        return Err(format!(
            "master URL {raw:?} needs an http:// or https:// scheme"
        ));
    }
    let url = url::Url::parse(raw)
        .map_err(|err| format!("master URL {raw:?} is not a valid URL: {err}"))?;
    if url.host_str().unwrap_or("").is_empty() {
        return Err(format!("master URL {raw:?} has no host"));
    }
    Ok(())
}

fn env_master(options: &Options<'_>) -> Option<String> {
    options
        .env_master
        .clone()
        .or_else(|| std::env::var(MASTER_ENV_VAR).ok())
}

fn resolve_master(
    flag_value: Option<&str>,
    cfg: &config::Config,
    options: &Options<'_>,
) -> (String, &'static str) {
    if let Some(value) = flag_value
        && !value.is_empty()
    {
        return (value.to_string(), "flag");
    }
    if let Some(value) = env_master(options)
        && !value.is_empty()
    {
        return (value, MASTER_ENV_VAR);
    }
    if !cfg.master.is_empty() {
        return (cfg.master.clone(), "config");
    }
    (String::new(), "")
}

fn cmd_serve(
    db_path: &Path,
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let mut port = 8080u16;
    let mut store_dir = PathBuf::from("onley-store");
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        let take = |i: &mut usize| -> Option<String> {
            if let Some(value) = inline.clone() {
                return Some(value);
            }
            *i += 1;
            args.get(*i).cloned()
        };
        match name {
            "-port" | "--port" => match take(&mut i).and_then(|value| value.parse::<u16>().ok()) {
                Some(value) => port = value,
                None => {
                    let _ = writeln!(stderr, "invalid value for flag -port");
                    return 1;
                }
            },
            "-store" | "--store" => match take(&mut i) {
                Some(value) => store_dir = PathBuf::from(value),
                None => {
                    let _ = writeln!(stderr, "flag needs an argument: -store");
                    return 1;
                }
            },
            other => {
                let _ = writeln!(stderr, "flag provided but not defined: {other}");
                return 1;
            }
        }
        i += 1;
    }

    if let Err(err) = std::fs::create_dir_all(&store_dir) {
        let _ = writeln!(stderr, "failed to create store directory: {err}");
        return 1;
    }
    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };
    let _ = writeln!(
        stdout,
        "onley master listening on :{port}, store: {}",
        store_dir.display()
    );
    match replica::Server::new(db, store_dir).serve(port) {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "server error: {err}");
            1
        }
    }
}

fn cmd_replica_check(
    db_path: &Path,
    assume_yes: bool,
    args: &[String],
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    options: &Options<'_>,
) -> i32 {
    let mut master_flag: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        match name {
            "-master" | "--master" => {
                let value = match inline {
                    Some(value) => Some(value),
                    None => {
                        i += 1;
                        args.get(i).cloned()
                    }
                };
                match value {
                    Some(value) => master_flag = Some(value),
                    None => {
                        let _ = writeln!(stderr, "flag needs an argument: -master");
                        return 1;
                    }
                }
            }
            other => {
                let _ = writeln!(stderr, "flag provided but not defined: {other}");
                return 1;
            }
        }
        i += 1;
    }

    let cfg_path = match default_config_path(options) {
        Ok(path) => path,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return 1;
        }
    };
    let cfg = match config::load(&cfg_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return 1;
        }
    };
    let (master, source) = resolve_master(master_flag.as_deref(), &cfg, options);
    if master.is_empty() {
        let _ = writeln!(stderr, "error: no master is configured");
        let _ = writeln!(
            stderr,
            "hint: pass -master <url>, set {MASTER_ENV_VAR}, or run: onley config set master <url>"
        );
        return 1;
    }
    let _ = writeln!(stdout, "Master: {master} (from {source})");

    let client = replica::Client::new(&master);
    if let Err(err) = client.ping() {
        let _ = writeln!(stderr, "cannot reach master {master}: {err}");
        return 1;
    }

    let Some(db) = open_db(db_path, stderr) else {
        return 1;
    };

    let groups = match db.md5_groups() {
        Ok(groups) => groups,
        Err(err) => {
            let _ = writeln!(stderr, "failed to read local index: {err}");
            return 1;
        }
    };
    if groups.is_empty() {
        let _ = writeln!(stdout, "Local index is empty; run scan first.");
        return 0;
    }
    let file_count: usize = groups.iter().map(|group| group.files.len()).sum();
    let _ = writeln!(
        stdout,
        "Comparing {file_count} file(s) in {} distinct digest(s) with master...",
        groups.len()
    );

    let mut to_delete: Vec<replica::PlanEntry> = Vec::new();
    let mut to_migrate: Vec<replica::PlanEntry> = Vec::new();
    let mut query_failed = 0usize;

    for (i, group) in groups.iter().enumerate() {
        match client.check(&group.md5) {
            Ok(found) => {
                for file in &group.files {
                    let entry = replica::PlanEntry {
                        path: file.path.clone(),
                        md5: file.md5.clone(),
                        size: file.size,
                        action: if found {
                            replica::Action::DeleteLocal
                        } else {
                            replica::Action::Migrate
                        },
                    };
                    if found {
                        to_delete.push(entry);
                    } else {
                        to_migrate.push(entry);
                    }
                }
            }
            Err(err) => {
                let _ = writeln!(
                    stderr,
                    "  query failed for {} file(s) with digest {}: {err}",
                    group.files.len(),
                    group.md5
                );
                query_failed += group.files.len();
            }
        }
        let _ = write!(stdout, "\r  progress: {}/{}", i + 1, groups.len());
    }
    let _ = writeln!(stdout);

    if query_failed > 0 {
        let _ = writeln!(
            stdout,
            "  ({query_failed} file(s) skipped due to query errors)"
        );
    }
    if to_delete.is_empty() && to_migrate.is_empty() {
        let _ = writeln!(stdout, "Nothing to do.");
        return 0;
    }

    if !to_delete.is_empty() {
        let _ = writeln!(
            stdout,
            "\nDelete locally (already on master, {} file(s)):",
            to_delete.len()
        );
        for entry in &to_delete {
            let _ = writeln!(stdout, "  {}", entry.path.display());
        }
    }
    if !to_migrate.is_empty() {
        let _ = writeln!(
            stdout,
            "\nMigrate to master (not on master, {} file(s)):",
            to_migrate.len()
        );
        for entry in &to_migrate {
            let _ = writeln!(stdout, "  {}", entry.path.display());
        }
    }

    if !assume_yes {
        let _ = write!(stdout, "\nProceed with the above? [y/N] ");
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => return report_no_input(&ui::NoInput, stderr),
            Ok(_) => {}
            Err(_) if line.trim().is_empty() => return report_no_input(&ui::NoInput, stderr),
            Err(_) => {}
        }
        if line.trim().to_lowercase() != "y" {
            let _ = writeln!(stdout, "Cancelled.");
            return 0;
        }
    }

    let mut skipped = 0usize;
    if assume_yes && !to_delete.is_empty() {
        skipped = to_delete.len();
        warn_skipped_deletes(skipped, stderr);
    }

    let mut deleted = 0usize;
    let mut migrated = 0usize;
    let mut failed = 0usize;

    if skipped == 0 {
        for entry in &to_delete {
            if let Err(err) = std::fs::remove_file(&entry.path) {
                let _ = writeln!(stderr, "  delete failed {}: {err}", entry.path.display());
                failed += 1;
                continue;
            }
            let _ = db.delete_record(&entry.path);
            deleted += 1;
        }
    }

    for entry in &to_migrate {
        if let Err(err) = client.ingest(&entry.path, &entry.md5) {
            let _ = writeln!(stderr, "  migrate failed {}: {err}", entry.path.display());
            failed += 1;
            continue;
        }
        if let Err(err) = std::fs::remove_file(&entry.path) {
            let _ = writeln!(
                stderr,
                "  delete local after migrate failed {}: {err}",
                entry.path.display()
            );
        }
        let _ = db.delete_record(&entry.path);
        migrated += 1;
    }

    let _ = writeln!(
        stdout,
        "\nDone: {deleted} deleted, {migrated} migrated, {failed} failed."
    );
    if skipped > 0 {
        return 1;
    }
    0
}

fn warn_skipped_deletes(skipped: usize, stderr: &mut dyn Write) {
    let _ = writeln!(
        stderr,
        "\nwarning: {skipped} file(s) are already on the master and were left in place."
    );
    let _ = writeln!(
        stderr,
        "         -y does not delete a local copy the master already has, because nothing"
    );
    let _ = writeln!(
        stderr,
        "         would be left anywhere if that turned out to be wrong."
    );
    let _ = writeln!(
        stderr,
        "         The list above is what is waiting for a decision: delete them yourself,"
    );
    let _ = writeln!(
        stderr,
        "         or drop -y and confirm the plan interactively."
    );
}

/// Parses a Go-style duration such as `500ms`, `30s` or `1m30s`.
fn parse_duration(value: &str) -> Result<Duration, String> {
    if value == "0" {
        return Ok(Duration::ZERO);
    }
    let mut rest = value;
    let mut total = Duration::ZERO;
    while !rest.is_empty() {
        let number_end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        if number_end == 0 {
            return Err(format!("invalid duration {value:?}"));
        }
        let number: f64 = rest[..number_end]
            .parse()
            .map_err(|_| format!("invalid duration {value:?}"))?;
        rest = &rest[number_end..];
        let unit_end = rest
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(rest.len());
        let unit = &rest[..unit_end];
        rest = &rest[unit_end..];
        let seconds = match unit {
            "ns" => number / 1e9,
            "us" | "µs" => number / 1e6,
            "ms" => number / 1e3,
            "s" => number,
            "m" => number * 60.0,
            "h" => number * 3600.0,
            "" => return Err(format!("missing unit in duration {value:?}")),
            _ => return Err(format!("unknown unit {unit:?} in duration {value:?}")),
        };
        total += Duration::from_secs_f64(seconds);
    }
    Ok(total)
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        return format!("{millis}ms");
    }
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours}h{minutes}m{seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m{seconds}s")
    } else {
        format!("{seconds}s")
    }
}
