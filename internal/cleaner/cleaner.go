// Package cleaner removes files and their index records.
package cleaner

import (
	"errors"
	"fmt"
	"os"

	"onley/internal/db"
	"onley/internal/selfpath"
)

// ErrSelfDelete reports an attempt to delete the running executable.
var ErrSelfDelete = errors.New("cannot delete the running executable")

// Target is one file to delete. Size and Mtime are the values the index holds;
// they are checked when Delete is asked to verify the file.
type Target struct {
	Path  string
	Size  int64
	Mtime int64
}

// Result reports what happened to one target.
type Result struct {
	Path string
	// Deleted is true when the file was removed from disk.
	Deleted bool
	// Missing is true when verification found the file already gone. Its
	// index record is removed, because the record can no longer be used.
	Missing bool
	// Err is a file-system failure; the record is kept.
	Err error
	// IndexErr is a failure to remove the record after the file was
	// deleted.
	IndexErr error
}

// Verify reports whether target still matches the index. It returns an error
// wrapping os.ErrNotExist when the file is gone, and refuses an old record
// without a modification time.
func Verify(target Target) error {
	info, err := os.Lstat(target.Path)
	if err != nil {
		return fmt.Errorf("stat: %w", err)
	}
	if !info.Mode().IsRegular() {
		return errors.New("not a regular file")
	}
	if target.Mtime == 0 {
		return errors.New("index has no modification time; rescan first")
	}
	if info.Size() != target.Size || info.ModTime().Unix() != target.Mtime {
		return errors.New("changed since scan")
	}
	return nil
}

// Delete removes each target and then its record. Verification, when set,
// refuses a file whose size or modification time no longer matches the index,
// and drops the record of a file that is already gone. A failure on one target
// does not stop the others.
func Delete(store *db.DB, targets []Target, verify bool) []Result {
	return deleteWithProtect(store, targets, verify, selfpath.Resolve())
}

// deleteWithProtect is Delete with an explicit executable path to protect.
func deleteWithProtect(store *db.DB, targets []Target, verify bool, protect string) []Result {
	results := make([]Result, 0, len(targets))
	for _, target := range targets {
		r := Result{Path: target.Path}

		if selfpath.Same(target.Path, protect) {
			r.Err = ErrSelfDelete
			results = append(results, r)
			continue
		}

		if verify {
			if err := Verify(target); err != nil {
				if errors.Is(err, os.ErrNotExist) {
					r.Missing = true
					r.IndexErr = store.DeleteRecord(target.Path)
				} else {
					r.Err = err
				}
				results = append(results, r)
				continue
			}
		}

		if err := os.Remove(target.Path); err != nil {
			r.Err = err
			results = append(results, r)
			continue
		}
		r.Deleted = true
		r.IndexErr = store.DeleteRecord(target.Path)
		results = append(results, r)
	}
	return results
}
