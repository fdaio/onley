//! Onley's desktop client, written in Rust with NSH.

mod categories;
mod gauge;
mod state;
mod theme;
mod ui;

use nsh_core::{element::desktop::set_title, prelude::*};
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
    set_title("Onley");
    ui::root(&state)
}
