//! Locates onley's per-user files.

use std::fs;
use std::io;
use std::path::PathBuf;

/// Directory under the user's home that holds the index and the settings.
pub const DIR_NAME: &str = ".onley";
/// Index file inside [`DIR_NAME`].
pub const DB_FILE_NAME: &str = "local.db";
/// Settings file inside [`DIR_NAME`].
pub const CONFIG_FILE_NAME: &str = "config.json";

/// Returns `~/.onley`, creating it if it does not exist.
///
/// The mode is 0700 because the index lists the paths and digests of the
/// user's files.
pub fn dir() -> io::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "cannot locate the home directory")
    })?;
    dir_in(&home)
}

/// Returns `<home>/.onley`, creating it if it does not exist.
pub fn dir_in(home: &std::path::Path) -> io::Result<PathBuf> {
    let dir = home.join(DIR_NAME);
    fs::create_dir_all(&dir)?;
    set_private(&dir)?;
    Ok(dir)
}

/// Returns the index path under `home`.
pub fn db_path_in(home: &std::path::Path) -> io::Result<PathBuf> {
    Ok(dir_in(home)?.join(DB_FILE_NAME))
}

/// Returns the settings path under `home`.
pub fn config_path_in(home: &std::path::Path) -> io::Result<PathBuf> {
    Ok(dir_in(home)?.join(CONFIG_FILE_NAME))
}

/// Returns the default index path, creating its directory if needed.
pub fn db_path() -> io::Result<PathBuf> {
    Ok(dir()?.join(DB_FILE_NAME))
}

/// Returns the default settings path, creating its directory if needed.
pub fn config_path() -> io::Result<PathBuf> {
    Ok(dir()?.join(CONFIG_FILE_NAME))
}

#[cfg(unix)]
fn set_private(dir: &std::path::Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private(_dir: &std::path::Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_live_under_the_home_directory() {
        let dir = dir().unwrap();
        assert!(dir.ends_with(DIR_NAME));
        assert!(dir.is_dir());
        assert_eq!(db_path().unwrap(), dir.join(DB_FILE_NAME));
        assert_eq!(config_path().unwrap(), dir.join(CONFIG_FILE_NAME));
    }
}
