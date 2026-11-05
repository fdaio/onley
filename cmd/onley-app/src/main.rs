//! Runs the desktop client on the machine's native backend.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

const APP_ID: &str = "io.github.laotree.onley";

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    onley_app::set_config(onley_app::Config::from_environment());
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<onley_app::App>().app_id(APP_ID)),
        platform::AppOptions {
            title: "Onley".to_owned(),
            app_id: APP_ID.to_owned(),
            size: (1000, 620),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("Run this binary on macOS, Windows or Linux.");
