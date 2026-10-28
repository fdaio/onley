// Package cleaner removes files and their index records.
package cleaner

import (
	"errors"
	"fmt"
	"os"

	"onley/internal/db"
)

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

// Delete removes each target and then its record. Verification, when set,
// refuses a file whose size or modification time no longer matches the index,
// and drops the record of a file that is already gone. A failure on one target
// does not stop the others.
func Delete(store *db.DB, targets []Target, verify bool) []Result {
	results := make([]Result, 0, len(targets))
	for _, target := range targets {
		r := Result{Path: target.Path}

		if verify {
			info, err := os.Lstat(target.Path)
			switch {
			case errors.Is(err, os.ErrNotExist):
				r.Missing = true
				r.IndexErr = store.DeleteRecord(target.Path)
				results = append(results, r)
				continue
			case err != nil:
				r.Err = fmt.Errorf("stat: %w", err)
				results = append(results, r)
				continue
			case !info.Mode().IsRegular():
				r.Err = errors.New("not a regular file")
				results = append(results, r)
				continue
			case target.Mtime == 0:
				r.Err = errors.New("index has no modification time; rescan first")
				results = append(results, r)
				continue
			case info.Size() != target.Size || info.ModTime().Unix() != target.Mtime:
				r.Err = errors.New("changed since scan")
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
