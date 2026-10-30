package main

import (
	"errors"
	"io/fs"
)

// isPermissionError reports whether err is a file-system permission failure.
func isPermissionError(err error) bool {
	return errors.Is(err, fs.ErrPermission)
}
