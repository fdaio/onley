//! Runs the phone app on this Mac, for the desktop preview.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
fn main() -> impl std::process::Termination {
    use nsh_platform_macos as platform;
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<onley_mobile::App>()),
        platform::AppOptions {
            title: "Onley Photos".to_owned(),
            app_id: "io.github.laotree.onley.photos".to_owned(),
            size: (390, 844),
            ..Default::default()
        },
    )
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Onley Photos runs on iOS; use `nsh package --target ios-simulator`.");
}
