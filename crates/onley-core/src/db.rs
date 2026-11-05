//! The SQLite index.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS files (
	id         INTEGER PRIMARY KEY AUTOINCREMENT,
	path       TEXT    NOT NULL UNIQUE,
	name       TEXT    NOT NULL,
	size       INTEGER NOT NULL,
	md5        TEXT    NOT NULL,
	mtime      INTEGER NOT NULL DEFAULT 0,
	scanned_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_files_md5 ON files(md5);
";

/// Adds `mtime` to databases created before the field existed. The statement
/// fails when the column is present, which is not an error here.
const MIGRATION: &str = "ALTER TABLE files ADD COLUMN mtime INTEGER NOT NULL DEFAULT 0";

/// One indexed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    pub id: i64,
    pub path: PathBuf,
    pub name: String,
    pub size: i64,
    pub md5: String,
    pub mtime: i64,
}

impl FileRecord {
    /// A record for a file that is not indexed yet.
    pub fn new(path: impl Into<PathBuf>, size: i64, md5: impl Into<String>, mtime: i64) -> Self {
        let path = path.into();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            id: 0,
            path,
            name,
            size,
            md5: md5.into(),
            mtime,
        }
    }
}

/// One digest and all files that share it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub md5: String,
    pub size: i64,
    pub files: Vec<FileRecord>,
}

/// One digest and every file that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Md5Group {
    pub md5: String,
    pub size: i64,
    pub files: Vec<FileRecord>,
}

/// The index.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Opens or creates the index at `path`.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Opens an empty in-memory index.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn configure(conn: &Connection) -> rusqlite::Result<()> {
        // The pragmas come first. WAL and the schema both need a write lock, so
        // a second process opening the index needs the busy timeout.
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        conn.execute_batch(SCHEMA)?;
        let _ = conn.execute(MIGRATION, []);
        Ok(())
    }

    /// Inserts or replaces the record for its path.
    pub fn upsert(&self, record: &FileRecord) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO files(path, name, size, md5, mtime) VALUES(?,?,?,?,?)
             ON CONFLICT(path) DO UPDATE SET
               name=excluded.name, size=excluded.size, md5=excluded.md5,
               mtime=excluded.mtime, scanned_at=datetime('now')",
            params![
                record.path.to_string_lossy(),
                record.name,
                record.size,
                record.md5,
                record.mtime
            ],
        )?;
        Ok(())
    }

    /// Returns the stored record for `path`.
    pub fn lookup(&self, path: &Path) -> rusqlite::Result<Option<FileRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, path, name, size, md5, mtime FROM files WHERE path = ?",
            params![path.to_string_lossy()],
            row,
        )
        .optional()
    }

    /// Returns every duplicate group, ordered by digest and path.
    pub fn duplicates(&self) -> rusqlite::Result<Vec<DuplicateGroup>> {
        let conn = self.conn.lock().unwrap();
        let mut statement = conn.prepare(
            "SELECT id, path, name, size, md5, mtime FROM files
             WHERE md5 IN (SELECT md5 FROM files GROUP BY md5 HAVING count(*) > 1)
             ORDER BY md5, path",
        )?;
        let rows = statement.query_map([], row)?;
        let mut groups: Vec<DuplicateGroup> = Vec::new();
        for record in rows {
            let record = record?;
            match groups.last_mut() {
                Some(group) if group.md5 == record.md5 => group.files.push(record),
                _ => groups.push(DuplicateGroup {
                    md5: record.md5.clone(),
                    size: record.size,
                    files: vec![record],
                }),
            }
        }
        Ok(groups)
    }

    /// Removes the record for `path`.
    pub fn delete_record(&self, path: &Path) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM files WHERE path = ?",
            params![path.to_string_lossy()],
        )?;
        Ok(())
    }

    /// Returns every indexed file, ordered by path.
    pub fn all_files(&self) -> rusqlite::Result<Vec<FileRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut statement =
            conn.prepare("SELECT id, path, name, size, md5, mtime FROM files ORDER BY path")?;
        statement.query_map([], row)?.collect()
    }

    /// Returns every distinct digest with its files, ordered by digest.
    pub fn md5_groups(&self) -> rusqlite::Result<Vec<Md5Group>> {
        let conn = self.conn.lock().unwrap();
        let mut statement =
            conn.prepare("SELECT id, path, name, size, md5, mtime FROM files ORDER BY md5, path")?;
        let rows = statement.query_map([], row)?;
        let mut groups: Vec<Md5Group> = Vec::new();
        for record in rows {
            let record = record?;
            match groups.last_mut() {
                Some(group) if group.md5 == record.md5 => group.files.push(record),
                _ => groups.push(Md5Group {
                    md5: record.md5.clone(),
                    size: record.size,
                    files: vec![record],
                }),
            }
        }
        Ok(groups)
    }

    /// Returns every file whose digest is `md5`.
    pub fn find_by_md5(&self, md5: &str) -> rusqlite::Result<Vec<FileRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut statement =
            conn.prepare("SELECT id, path, name, size, md5, mtime FROM files WHERE md5 = ?")?;
        statement.query_map(params![md5], row)?.collect()
    }

    /// Returns the number of indexed files and the number with duplicates.
    pub fn stats(&self) -> rusqlite::Result<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        let duplicates: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE md5 IN
             (SELECT md5 FROM files GROUP BY md5 HAVING count(*) > 1)",
            [],
            |row| row.get(0),
        )?;
        Ok((total, duplicates))
    }
}

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRecord> {
    let path: String = row.get(1)?;
    Ok(FileRecord {
        id: row.get(0)?,
        path: PathBuf::from(path),
        name: row.get(2)?,
        size: row.get(3)?,
        md5: row.get(4)?,
        mtime: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(path: &str, size: i64, md5: &str, mtime: i64) -> FileRecord {
        FileRecord::new(path, size, md5, mtime)
    }

    #[test]
    fn upsert_and_lookup() {
        let db = Db::open_in_memory().unwrap();
        let r = record("/tmp/a", 10, "abc", 100);
        db.upsert(&r).unwrap();
        let found = db.lookup(Path::new("/tmp/a")).unwrap().unwrap();
        assert_eq!(found.size, 10);
        assert_eq!(found.md5, "abc");
        assert_eq!(found.mtime, 100);
        assert_eq!(found.name, "a");
        db.upsert(&record("/tmp/a", 20, "def", 200)).unwrap();
        let found = db.lookup(Path::new("/tmp/a")).unwrap().unwrap();
        assert_eq!(found.size, 20);
        assert_eq!(found.md5, "def");
    }

    #[test]
    fn missing_lookup_is_none() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.lookup(Path::new("/tmp/none")).unwrap().is_none());
    }

    #[test]
    fn duplicates_group_and_order() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&record("/b", 5, "one", 0)).unwrap();
        db.upsert(&record("/a", 5, "one", 0)).unwrap();
        db.upsert(&record("/c", 7, "two", 0)).unwrap();
        db.upsert(&record("/d", 7, "two", 0)).unwrap();
        db.upsert(&record("/e", 9, "solo", 0)).unwrap();
        let groups = db.duplicates().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].md5, "one");
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[0].files[0].path, PathBuf::from("/a"));
        assert_eq!(groups[1].md5, "two");
        assert_eq!(groups[1].size, 7);
    }

    #[test]
    fn stats_count_files_and_duplicates() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&record("/a", 1, "x", 0)).unwrap();
        db.upsert(&record("/b", 1, "x", 0)).unwrap();
        db.upsert(&record("/c", 1, "y", 0)).unwrap();
        assert_eq!(db.stats().unwrap(), (3, 2));
    }

    #[test]
    fn md5_groups_include_single_files() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&record("/a", 1, "x", 0)).unwrap();
        db.upsert(&record("/b", 1, "x", 0)).unwrap();
        db.upsert(&record("/c", 1, "y", 0)).unwrap();
        let groups = db.md5_groups().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].md5, "x");
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[1].md5, "y");
        assert_eq!(groups[1].files.len(), 1);
    }

    #[test]
    fn delete_record_removes_only_that_path() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&record("/a", 1, "x", 0)).unwrap();
        db.upsert(&record("/b", 1, "x", 0)).unwrap();
        db.delete_record(Path::new("/a")).unwrap();
        assert!(db.lookup(Path::new("/a")).unwrap().is_none());
        assert!(db.lookup(Path::new("/b")).unwrap().is_some());
    }

    #[test]
    fn all_files_are_ordered_by_path() {
        let db = Db::open_in_memory().unwrap();
        for path in ["/c", "/a", "/b"] {
            db.upsert(&record(path, 1, "x", 0)).unwrap();
        }
        let paths: Vec<PathBuf> = db
            .all_files()
            .unwrap()
            .into_iter()
            .map(|r| r.path)
            .collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c")
            ]
        );
    }

    #[test]
    fn an_existing_database_keeps_its_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("local.db");
        {
            let db = Db::open(&path).unwrap();
            db.upsert(&record("/a", 1, "x", 0)).unwrap();
        }
        let db = Db::open(&path).unwrap();
        assert_eq!(db.stats().unwrap().0, 1);
    }
}
