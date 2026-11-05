//! The terminal prompts shared by the clean commands.

use crate::db::DuplicateGroup;
use crate::format;
use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Reports that the input ended before the question was answered.
///
/// It is deliberately distinct from answering no: a command that deletes
/// files must not treat silence as a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoInput;

impl std::fmt::Display for NoInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("input ended before the question was answered")
    }
}

impl std::error::Error for NoInput {}

/// Writes all duplicate groups to `w`.
pub fn show_duplicates(groups: &[DuplicateGroup], w: &mut dyn Write) -> std::io::Result<()> {
    if groups.is_empty() {
        let _ = writeln!(w, "No duplicate files found.");
        return Ok(());
    }
    let _ = writeln!(w, "Found {} duplicate group(s):\n", groups.len());
    for (i, group) in groups.iter().enumerate() {
        let _ = writeln!(
            w,
            "── Group {}  MD5: {}  size: {} ──",
            i + 1,
            group.md5,
            format::bytes(group.size)
        );
        for (j, file) in group.files.iter().enumerate() {
            let _ = writeln!(w, "  [{}] {}", j + 1, file.path.display());
        }
        let _ = writeln!(w);
    }
    Ok(())
}

/// Reads one line of input. `None` means the stream ended without an answer.
fn read_answer(reader: &mut dyn BufRead) -> Option<String> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim().to_string()),
        Err(_) if line.trim().is_empty() => None,
        Err(_) => Some(line.trim().to_string()),
    }
}

/// Lets the user pick which duplicates to delete for each group.
///
/// Returns the paths chosen for deletion. It reports [`NoInput`] when the
/// input ends before every group has been offered.
pub fn clean_interactive(
    groups: &[DuplicateGroup],
    w: &mut dyn Write,
    reader: &mut dyn BufRead,
) -> Result<Vec<PathBuf>, NoInput> {
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let mut to_delete = Vec::new();

    let _ = writeln!(
        w,
        "For each group, enter the number(s) to KEEP (others will be deleted)."
    );
    let _ = writeln!(w, "Press Enter to skip a group.");
    let _ = writeln!(w);

    for (i, group) in groups.iter().enumerate() {
        let _ = writeln!(
            w,
            "── Group {}  MD5: {}  size: {} ──",
            i + 1,
            group.md5,
            format::bytes(group.size)
        );
        for (j, file) in group.files.iter().enumerate() {
            let _ = writeln!(w, "  [{}] {}", j + 1, file.path.display());
        }
        let _ = write!(w, "Keep number(s) (e.g. 1 or 1,2; Enter to skip): ");
        let _ = w.flush();

        let Some(line) = read_answer(reader) else {
            return Err(NoInput);
        };
        if line.is_empty() {
            let _ = writeln!(w, "Skipped.");
            let _ = writeln!(w);
            continue;
        }

        let mut keep = std::collections::HashSet::new();
        let mut valid = true;
        for part in line.split(',') {
            let part = part.trim();
            match part.parse::<usize>() {
                Ok(n) if n >= 1 && n <= group.files.len() => {
                    keep.insert(n);
                }
                _ => {
                    let _ = writeln!(w, "  Invalid number {:?}, skipping group.", part);
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            let _ = writeln!(w);
            continue;
        }
        for (j, file) in group.files.iter().enumerate() {
            if !keep.contains(&(j + 1)) {
                to_delete.push(file.path.clone());
            }
        }
        let _ = writeln!(w);
    }
    Ok(to_delete)
}

/// Shows the files to be deleted and asks for confirmation.
pub fn confirm_delete(
    paths: &[PathBuf],
    w: &mut dyn Write,
    reader: &mut dyn BufRead,
) -> Result<bool, NoInput> {
    if paths.is_empty() {
        return Ok(false);
    }
    let _ = writeln!(
        w,
        "The following {} file(s) will be permanently deleted:",
        paths.len()
    );
    for path in paths {
        let _ = writeln!(w, "  - {}", path.display());
    }
    let _ = write!(w, "Confirm deletion? (y/N): ");
    let _ = w.flush();
    let Some(line) = read_answer(reader) else {
        return Err(NoInput);
    };
    Ok(line.eq_ignore_ascii_case("y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::FileRecord;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn group() -> DuplicateGroup {
        DuplicateGroup {
            md5: "0123456789abcdef0123456789abcdef".into(),
            size: 2048,
            files: vec![
                FileRecord::new(PathBuf::from("/a"), 2048, "x", 0),
                FileRecord::new(PathBuf::from("/b"), 2048, "x", 0),
            ],
        }
    }

    #[test]
    fn shows_groups() {
        let mut out = Vec::new();
        show_duplicates(&[group()], &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Found 1 duplicate group(s):"));
        assert!(text.contains("/a"));
        assert!(text.contains("2.0 KB"));
    }

    #[test]
    fn no_duplicates_says_so() {
        let mut out = Vec::new();
        show_duplicates(&[], &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "No duplicate files found.\n"
        );
    }

    #[test]
    fn keeping_one_deletes_the_other() {
        let mut out = Vec::new();
        let mut input = Cursor::new(b"1\n".to_vec());
        let paths = clean_interactive(&[group()], &mut out, &mut input).unwrap();
        assert_eq!(paths, vec![PathBuf::from("/b")]);
    }

    #[test]
    fn an_empty_line_skips() {
        let mut out = Vec::new();
        let mut input = Cursor::new(b"\n".to_vec());
        assert!(
            clean_interactive(&[group()], &mut out, &mut input)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn an_invalid_number_skips_the_group() {
        let mut out = Vec::new();
        let mut input = Cursor::new(b"9\n".to_vec());
        assert!(
            clean_interactive(&[group()], &mut out, &mut input)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn exhausted_input_is_not_a_choice() {
        let mut out = Vec::new();
        let mut input = Cursor::new(Vec::new());
        assert_eq!(
            clean_interactive(&[group()], &mut out, &mut input),
            Err(NoInput)
        );
    }

    #[test]
    fn a_line_without_a_newline_still_answers() {
        let mut out = Vec::new();
        let mut input = Cursor::new(b"1".to_vec());
        assert_eq!(
            clean_interactive(&[group()], &mut out, &mut input).unwrap(),
            vec![PathBuf::from("/b")]
        );
    }

    #[test]
    fn confirmation_accepts_y_only() {
        let mut out = Vec::new();
        let mut input = Cursor::new(b"y\n".to_vec());
        assert!(confirm_delete(&[PathBuf::from("/a")], &mut out, &mut input).unwrap());
        let mut out = Vec::new();
        let mut input = Cursor::new(b"yes\n".to_vec());
        assert!(!confirm_delete(&[PathBuf::from("/a")], &mut out, &mut input).unwrap());
    }

    #[test]
    fn confirmation_without_input_fails() {
        let mut out = Vec::new();
        let mut input = Cursor::new(Vec::new());
        assert_eq!(
            confirm_delete(&[PathBuf::from("/a")], &mut out, &mut input),
            Err(NoInput)
        );
    }
}
