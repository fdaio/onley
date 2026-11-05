//! Reads and writes onley's per-user settings.
//!
//! The settings live next to the index so that both follow the user. Only a
//! value that is fixed per machine belongs here.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

/// The contents of the configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Base URL of the master server, for example `http://host:8080`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub master: String,
}

/// Reads the configuration at `path`.
///
/// A missing file is not an error. A file that exists but cannot be parsed is
/// an error, because a command must not fall back to a value it was never
/// given.
pub fn load(path: &Path) -> io::Result<Config> {
    match fs::read(path) {
        Ok(data) => serde_json::from_slice(&data).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("parse {}: {err}", path.display()),
            )
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(err) => Err(io::Error::new(
            err.kind(),
            format!("read {}: {err}", path.display()),
        )),
    }
}

impl Config {
    /// Writes the configuration to `path`.
    ///
    /// The file is written through a temporary file and renamed, so an
    /// interrupted write cannot leave a half-written file. The mode is 0600.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut data = serde_json::to_vec_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        data.push(b'\n');
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "config.json".to_owned());
        let tmp_path = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let result = (|| {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&tmp_path)?;
            file.write_all(&data)?;
            file.sync_all()?;
            fs::rename(&tmp_path, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load(&dir.path().join("config.json")).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn saves_and_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = Config {
            master: "http://host:8080".into(),
        };
        cfg.save(&path).unwrap();
        assert_eq!(load(&path).unwrap(), cfg);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn a_broken_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"{not json").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, br#"{"master":"http://m:1","future":true}"#).unwrap();
        assert_eq!(load(&path).unwrap().master, "http://m:1");
    }
}
