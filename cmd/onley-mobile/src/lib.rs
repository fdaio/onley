//! Onley's phone client: find duplicate photos in the system library.

mod i18n;
mod model;
mod scan;
pub mod source;
mod state;
mod ui;

use nsh_core::prelude::*;

pub use state::set_library;

/// The UI set a TSX entry imports; this app has none.
#[native_module("onley/photos")]
fn native(_registry: &mut Registry) {}

/// The application.
#[component]
pub fn App() -> impl IntoElement {
    let state = state::State::new();
    ui::root(&state)
}
