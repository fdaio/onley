//! Drives the desktop client the way a user does.

use md5::Digest;
use nsh_core::geometry::{Points, Size};
use nsh_core::platform::Locale;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, MemoryStorage};
use onley_app::Config;
use onley_core::db::{Db, FileRecord};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct Fixture {
    dir: tempfile::TempDir,
    db_path: PathBuf,
}

impl Fixture {
    fn empty() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("local.db");
        let _ = Db::open(&db_path).unwrap();
        Self { dir, db_path }
    }

    fn with(files: &[(&str, &[u8], i64)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("local.db");
        {
            let db = Db::open(&db_path).unwrap();
            for (name, bytes, mtime) in files {
                let path = dir.path().join(name);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(&path, bytes).unwrap();
                set_mtime(&path, *mtime);
                let digest: String = md5::Md5::digest(bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                db.upsert(&FileRecord::new(&path, bytes.len() as i64, digest, *mtime))
                    .unwrap();
            }
        }
        Self { dir, db_path }
    }

    fn config(&self) -> Config {
        Config {
            db_path: Some(self.db_path.clone()),
            open_error: None,
            workers: 2,
            home: self.dir.path().to_path_buf(),
            default_dir: self.dir.path().to_path_buf(),
            self_path: None,
        }
    }
}

fn set_mtime(path: &Path, unix: i64) {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(unix.max(0) as u64);
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

struct TestApp {
    _guard: MutexGuard<'static, ()>,
    app: Headless,
}

fn start(config: Config, storage: &[(&str, &str)], locale: &str) -> TestApp {
    start_with_storage(
        config,
        MemoryStorage::new(
            storage
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string())),
        ),
        locale,
    )
}

/// Starts the app on `storage`, so a test can read what it writes.
fn start_with_storage(config: Config, storage: Arc<MemoryStorage>, locale: &str) -> TestApp {
    let guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    onley_app::set_config(config);
    let mut engine = EngineConfig::new(Root::of::<onley_app::App>());
    engine.viewport = Size {
        width: Points(1000.0),
        height: Points(900.0),
    };
    engine.storage = Some(storage);
    if let Some(locale) = Locale::parse(locale) {
        engine.environment.locale = locale;
    }
    let app = Headless::start(engine).expect("the app starts");
    TestApp { _guard: guard, app }
}

fn texts(app: &mut Headless) -> Vec<String> {
    app.screen()
        .texts()
        .iter()
        .map(|text| text.to_string())
        .collect()
}

fn has(app: &mut Headless, needle: &str) -> bool {
    texts(app).iter().any(|text| text.contains(needle))
}

/// Runs frames until the screen shows `needle`.
fn wait_contains(app: &mut Headless, needle: &str) {
    app.wait_until(needle, Duration::from_secs(10), |screen| {
        screen.texts().iter().any(|text| text.contains(needle))
    })
    .unwrap_or_else(|err| panic!("{err}"));
}

/// Runs frames until the group card marks `name` as the copy to keep.
fn wait_keeps(app: &mut Headless, name: &str) {
    let suffix = format!("/{name}");
    app.wait_until(
        &format!("KEEP on {name}"),
        Duration::from_secs(5),
        |screen| {
            let texts = screen.texts();
            let Some(index) = texts.iter().position(|text| text.ends_with(&suffix)) else {
                return false;
            };
            texts.get(index + 1) == Some(&"KEEP")
        },
    )
    .unwrap_or_else(|err| panic!("{err}"));
}

/// Runs frames until the screen no longer shows `needle`.
fn wait_gone(app: &mut Headless, needle: &str) {
    app.wait_until(
        &format!("{needle} gone"),
        Duration::from_secs(10),
        |screen| !screen.texts().iter().any(|text| text.contains(needle)),
    )
    .unwrap_or_else(|err| panic!("{err}"));
}

/// Runs frames until the scan ends and the header settles.
fn wait_for_scan(app: &mut Headless) {
    app.wait_for_gone("id:scan-strip", Duration::from_secs(30))
        .unwrap_or_else(|err| panic!("{err}"));
    app.wait_for("id:delete-files").unwrap();
}

/// Opens settings, applies a keep policy, and closes settings.
fn set_policy(app: &mut Headless, id: &str) {
    let _ = app.wait_for_gone("id:scan-strip", Duration::from_secs(10));
    app.press("id:settings").unwrap();
    if app.wait_for("id:settings-dialog").is_err() {
        // The scan can rebuild the view between the press and the render.
        app.press("id:settings").unwrap();
        app.wait_for("id:settings-dialog").unwrap();
    }
    app.wait_for(&format!("id:{id}")).unwrap();
    app.press(&format!("id:{id}")).unwrap();
    app.press("id:settings-close").unwrap();
    app.wait_for_gone("id:settings-dialog", Duration::from_secs(5))
        .unwrap();
}

#[test]
fn shows_the_chinese_empty_state() {
    let fixture = Fixture::empty();
    let mut app = start(fixture.config(), &[], "zh-CN");
    wait_contains(&mut app.app, "暂无重复文件");
    assert!(has(&mut app.app, "扫描文件夹以建立索引"));
}

#[test]
fn a_stored_language_wins_over_the_system_one() {
    let fixture = Fixture::empty();
    let mut app = start(fixture.config(), &[("language", "en")], "zh-CN");
    wait_contains(&mut app.app, "NO DUPLICATES YET");
}

#[test]
fn settings_switches_the_language() {
    let fixture = Fixture::empty();
    let mut app = start(fixture.config(), &[], "zh-CN");
    app.app.wait_for("id:settings").unwrap();
    app.app.press("id:settings").unwrap();
    app.app.wait_for("id:lang-en").unwrap();
    app.app.press("id:lang-en").unwrap();
    wait_contains(&mut app.app, "NO DUPLICATES YET");
}

#[test]
fn the_empty_state_offers_a_scan() {
    let fixture = Fixture::empty();
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:empty-state").unwrap();
    app.app.wait_for("id:scan-button").unwrap();
    assert!(has(&mut app.app, "NO DUPLICATES YET"));
}

#[test]
fn a_narrow_window_keeps_the_actions_visible() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let fixture = Fixture::empty();
    onley_app::set_config(fixture.config());
    let mut engine = EngineConfig::new(Root::of::<onley_app::App>());
    engine.viewport = Size {
        width: Points(560.0),
        height: Points(800.0),
    };
    let mut app = Headless::start(engine).expect("the app starts");
    app.wait_for("id:search-input").unwrap();
    app.wait_for("id:filters").unwrap();
    app.wait_for("id:settings").unwrap();
    app.wait_for("id:scan-button").unwrap();
}

#[test]
fn scan_finds_duplicates() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("sub/two.txt", b"same-bytes", 100),
        ("three.txt", b"other", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_contains(&mut app.app, "one.txt");
    wait_contains(&mut app.app, "two.txt");
    assert!(has(&mut app.app, "ORIGINAL COPY"));
}

#[test]
fn category_tabs_filter_groups() {
    let fixture = Fixture::with(&[
        ("a.jpg", b"image-bytes", 100),
        ("b.jpg", b"image-bytes", 100),
        ("a.pdf", b"doc-bytes", 100),
        ("b.pdf", b"doc-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_for_scan(&mut app.app);
    wait_contains(&mut app.app, "a.jpg");
    assert!(has(&mut app.app, "a.pdf"));

    app.app.press("id:filters").unwrap();
    app.app.wait_for("id:tab:images").unwrap();
    app.app.press("id:tab:images").unwrap();
    app.app.press("id:filters-close").unwrap();
    wait_contains(&mut app.app, "a.jpg");
    assert!(!has(&mut app.app, "a.pdf"));

    app.app.press("id:filters").unwrap();
    app.app.press("id:tab:documents").unwrap();
    app.app.press("id:filters-close").unwrap();
    wait_contains(&mut app.app, "a.pdf");
    assert!(!has(&mut app.app, "a.jpg"));
}

#[test]
fn search_filters_by_name() {
    let fixture = Fixture::with(&[
        ("alpha.jpg", b"image-bytes", 100),
        ("beta.jpg", b"image-bytes", 100),
        ("gamma.pdf", b"doc-bytes", 100),
        ("delta.pdf", b"doc-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:search-input").unwrap();
    app.app.type_text("id:search-input", "gamma").unwrap();
    wait_gone(&mut app.app, "alpha.jpg");
    wait_contains(&mut app.app, "gamma.pdf");
}

#[test]
fn keep_policy_selects_one_copy_per_group() {
    let fixture = Fixture::with(&[
        ("old.txt", b"same-bytes", 100),
        ("new.txt", b"same-bytes", 300),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_for_scan(&mut app.app);
    wait_contains(&mut app.app, "old.txt");

    app.app.press("id:settings").unwrap();
    app.app.wait_for("id:policy-newest").unwrap();
    app.app.press("id:policy-newest").unwrap();
    wait_contains(&mut app.app, "(1 FILE)");
    app.app.press("id:policy-oldest").unwrap();
    wait_contains(&mut app.app, "(1 FILE)");

    app.app.press("id:clear-choices").unwrap();
    wait_contains(&mut app.app, "(0 FILES)");
}

#[test]
fn a_policy_chosen_while_scanning_applies_when_the_scan_ends() {
    let fixture = Fixture::with(&[
        ("old.txt", b"same-bytes", 100),
        ("new.txt", b"same-bytes", 300),
    ]);
    let big = std::fs::File::create(fixture.dir.path().join("big.bin")).unwrap();
    big.set_len(1 << 31).unwrap();
    let storage = MemoryStorage::new(std::iter::empty());
    let mut app = start_with_storage(fixture.config(), storage.clone(), "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    app.app.wait_for("id:scan-strip").unwrap();

    app.app.press("id:settings").unwrap();
    app.app.wait_for("id:policy-newest").unwrap();
    assert!(app.app.screen().enabled("id:policy-newest").unwrap());
    app.app.press("id:policy-newest").unwrap();
    assert!(app.app.screen().matches("id:scan-strip").unwrap());
    wait_keeps(&mut app.app, "new.txt");
    assert_eq!(
        storage.entries().get("keep_policy").map(String::as_str),
        Some("newest")
    );

    app.app.press("id:settings-close").unwrap();
    let _ = app.app.press("id:cancel-scan");
    app.app
        .wait_for_gone("id:scan-strip", Duration::from_secs(5))
        .unwrap();
    wait_keeps(&mut app.app, "new.txt");
}

#[test]
fn remove_folder_hides_its_groups() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_for_scan(&mut app.app);
    wait_contains(&mut app.app, "one.txt");

    let root = fixture.dir.path().to_string_lossy().into_owned();
    app.app.press("id:filters").unwrap();
    app.app
        .wait_for(&format!("id:remove-folder:{root}"))
        .unwrap();
    app.app.press(&format!("id:remove-folder:{root}")).unwrap();
    app.app.press("id:filters-close").unwrap();
    wait_contains(&mut app.app, "NO DUPLICATES YET");
}

#[test]
fn adding_a_folder_merges_a_child_into_its_parent() {
    let fixture = Fixture::empty();
    let parent = fixture.dir.path().join("parent");
    let child = parent.join("child");
    std::fs::create_dir_all(&child).unwrap();
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:filters").unwrap();

    for path in [&parent, &child] {
        app.app.press("id:filters").unwrap();
        app.app.wait_for("id:add-folder").unwrap();
        app.app.press("id:add-folder").unwrap();
        app.app.wait_for("id:scan-path").unwrap();
        app.app
            .type_text("id:scan-path", &path.to_string_lossy())
            .unwrap();
        app.app.wait_for("id:add-folder-confirm").unwrap();
        app.app.press("id:add-folder-confirm").unwrap();
        app.app
            .wait_for_gone("id:add-folder-dialog", Duration::from_secs(5))
            .unwrap();
    }
    app.app.press("id:filters").unwrap();
    wait_contains(&mut app.app, "(01)");
    app.app.press("id:filters-close").unwrap();
}

#[test]
fn delete_removes_the_extra_copy_and_the_empty_folder() {
    let fixture = Fixture::with(&[
        ("a-keep/one.txt", b"same-bytes", 100),
        ("z-gone/two.txt", b"same-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_contains(&mut app.app, "one.txt");
    set_policy(&mut app.app, "policy-path");
    wait_contains(&mut app.app, "(1 FILE)");
    app.app.press("id:delete-files").unwrap();
    app.app.wait_for("id:confirm-dialog").unwrap();
    app.app.press("id:delete-confirm").unwrap();
    wait_contains(&mut app.app, "Deleted 1 file");

    assert!(fixture.dir.path().join("a-keep/one.txt").exists());
    assert!(!fixture.dir.path().join("z-gone/two.txt").exists());
    assert!(!fixture.dir.path().join("z-gone").exists());
}

#[test]
fn delete_stops_when_the_copy_to_keep_is_gone() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_contains(&mut app.app, "one.txt");
    set_policy(&mut app.app, "policy-path");
    wait_contains(&mut app.app, "(1 FILE)");

    std::fs::remove_file(fixture.dir.path().join("one.txt")).unwrap();
    app.app.press("id:delete-files").unwrap();
    app.app.wait_for("id:confirm-dialog").unwrap();
    app.app.press("id:delete-confirm").unwrap();
    wait_contains(&mut app.app, "Delete canceled");
    assert!(fixture.dir.path().join("two.txt").exists());
}

#[test]
fn delete_skips_a_file_that_changed() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_contains(&mut app.app, "one.txt");
    set_policy(&mut app.app, "policy-path");
    wait_contains(&mut app.app, "(1 FILE)");

    std::fs::write(fixture.dir.path().join("two.txt"), b"changed and longer").unwrap();
    app.app.press("id:delete-files").unwrap();
    app.app.wait_for("id:confirm-dialog").unwrap();
    app.app.press("id:delete-confirm").unwrap();
    wait_contains(&mut app.app, "1 failed");
    assert!(fixture.dir.path().join("two.txt").exists());
}

#[test]
fn delete_reports_a_copy_that_is_already_gone() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_contains(&mut app.app, "one.txt");
    set_policy(&mut app.app, "policy-path");
    wait_contains(&mut app.app, "(1 FILE)");

    std::fs::remove_file(fixture.dir.path().join("two.txt")).unwrap();
    app.app.press("id:delete-files").unwrap();
    app.app.wait_for("id:confirm-dialog").unwrap();
    app.app.press("id:delete-confirm").unwrap();
    wait_contains(&mut app.app, "already gone");
}

#[test]
fn the_running_executable_is_never_deleted() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let mut config = fixture.config();
    config.self_path = Some(fixture.dir.path().join("one.txt"));
    let mut app = start(config, &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    wait_contains(&mut app.app, "one.txt");
    set_policy(&mut app.app, "policy-newest");
    wait_contains(&mut app.app, "1 FILE");
    app.app.press("id:delete-files").unwrap();
    app.app.wait_for("id:confirm-dialog").unwrap();
    app.app.press("id:delete-confirm").unwrap();
    wait_contains(&mut app.app, "Deleted 1 file");
    assert!(fixture.dir.path().join("one.txt").exists());
    assert!(!fixture.dir.path().join("two.txt").exists());
}

#[test]
fn the_scan_strip_shows_progress_and_can_be_canceled() {
    let fixture = Fixture::empty();
    let big = std::fs::File::create(fixture.dir.path().join("big.bin")).unwrap();
    big.set_len(1 << 30).unwrap();
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:scan-button").unwrap();
    app.app.press("id:scan-button").unwrap();
    app.app.wait_for("id:scan-strip").unwrap();
    app.app.wait_for("id:scan-speed").unwrap();
    if app.app.press("id:cancel-scan").is_ok() {
        wait_contains(&mut app.app, "Scan canceled");
    }
}

#[test]
fn the_list_stays_visible_while_scanning() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let big = std::fs::File::create(fixture.dir.path().join("big.bin")).unwrap();
    big.set_len(1 << 30).unwrap();
    let mut app = start(fixture.config(), &[], "en-US");
    wait_contains(&mut app.app, "one.txt");

    app.app.press("id:scan-button").unwrap();
    app.app.wait_for("id:scan-strip").unwrap();
    assert!(has(&mut app.app, "one.txt"));
    let _ = app.app.press("id:cancel-scan");
}

#[test]
fn the_index_is_shared_with_the_command_line() {
    let fixture = Fixture::with(&[
        ("one.txt", b"same-bytes", 100),
        ("two.txt", b"same-bytes", 100),
    ]);
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:indexed-files").unwrap();
    assert!(has(&mut app.app, "2"));
    assert!(has(&mut app.app, "1"));
}

#[test]
fn a_missing_folder_is_not_added() {
    let fixture = Fixture::empty();
    let mut app = start(fixture.config(), &[], "en-US");
    app.app.wait_for("id:filters").unwrap();
    app.app.press("id:filters").unwrap();
    app.app.wait_for("id:add-folder").unwrap();
    app.app.press("id:add-folder").unwrap();
    app.app.wait_for("id:scan-path").unwrap();
    app.app
        .type_text("id:scan-path", "/nonexistent-onley-folder")
        .unwrap();
    wait_contains(&mut app.app, "No folder at this path");
    app.app.press("id:add-folder-confirm").unwrap();
    wait_contains(&mut app.app, "Cannot scan that folder");
    assert!(has(&mut app.app, "ADD A FOLDER"));
}
