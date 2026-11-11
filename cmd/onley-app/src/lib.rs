//! Onley's desktop client, written in Rust with NSH.

mod categories;
mod state;
mod theme;
mod ui;

use nsh_core::{
    desktop::{MenuRole, WindowAttributes},
    element::desktop::{MenuItem, main_window, set_application_menu},
    prelude::*,
};
use onley_core::i18n;
pub use state::Config;
use std::sync::Mutex;

/// The config the next app instance reads.
static NEXT_CONFIG: Mutex<Option<Config>> = Mutex::new(None);

/// Gives the next app instance its config.
pub fn set_config(config: Config) {
    *NEXT_CONFIG.lock().unwrap() = Some(config);
}

fn take_config() -> Config {
    NEXT_CONFIG
        .lock()
        .unwrap()
        .take()
        .unwrap_or_else(Config::from_environment)
}

/// The UI set a TSX entry imports; this app has none.
#[native_module("onley/native")]
fn native(_registry: &mut Registry) {}

/// The application.
#[component]
pub fn App() -> impl IntoElement {
    let state = state::State::new(take_config());
    let mut attributes = WindowAttributes::default();
    attributes.title = Some("Onley".to_owned());
    main_window().set(attributes);
    install_menu(&state);
    listen_for_second_instance(&state);
    ui::root(&state)
}

/// Adds the app's own commands to the menu bar.
fn install_menu(state: &state::State) {
    let add = state.clone();
    let scan = state.clone();
    let settings = state.clone();

    let mut items = Vec::new();
    if cfg!(target_os = "macos") {
        items.push(MenuItem::submenu(
            "Onley",
            vec![
                MenuItem::role(MenuRole::About),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Hide),
                MenuItem::role(MenuRole::HideOthers),
                MenuItem::role(MenuRole::ShowAll),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Quit),
            ],
        ));
    }

    let mut file = vec![
        MenuItem::new(state.t(i18n::ADD_FOLDER))
            .shortcut("mod+o")
            .on_select(move || add.open_add_folder()),
        MenuItem::new(state.t(i18n::RUN_GLOBAL_SCAN))
            .shortcut("mod+r")
            .on_select(move || scan.start_scan()),
        MenuItem::separator(),
        MenuItem::new(state.t(i18n::SETTINGS))
            .shortcut("mod+,")
            .on_select(move || settings.settings.set(true)),
    ];
    if !cfg!(target_os = "macos") {
        file.push(MenuItem::separator());
        file.push(MenuItem::role(MenuRole::Quit));
    }
    items.push(MenuItem::submenu(state.t(i18n::FILE_MENU), file));
    items.push(MenuItem::submenu(
        state.t(i18n::EDIT_MENU),
        vec![
            MenuItem::role(MenuRole::Undo),
            MenuItem::role(MenuRole::Redo),
            MenuItem::separator(),
            MenuItem::role(MenuRole::Cut),
            MenuItem::role(MenuRole::Copy),
            MenuItem::role(MenuRole::Paste),
            MenuItem::role(MenuRole::SelectAll),
        ],
    ));
    set_application_menu(items);
}

/// Adds the folders a later launch hands the running instance.
fn listen_for_second_instance(state: &state::State) {
    let state = state.clone();
    let owner = state.owner;
    owner.run(move || {
        let state = state.clone();
        let _task = spawn(async move {
            loop {
                let launch = nsh_core::element::next_second_instance().await;
                for arg in launch.args.iter().skip(1) {
                    let arg_path = std::path::Path::new(arg);
                    let path = if arg_path.is_absolute() {
                        arg_path.to_path_buf()
                    } else {
                        std::path::Path::new(&launch.cwd).join(arg_path)
                    };
                    if path.is_dir() {
                        state.add_folder(&path.to_string_lossy());
                    }
                }
            }
        });
    });
}
