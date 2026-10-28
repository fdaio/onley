package main

import (
	"net/url"
	"path/filepath"
	"runtime"
)

// filePath converts a file: URL from the platform's drag-and-drop into a local
// path.
func filePath(raw string) (string, bool) {
	u, err := url.Parse(raw)
	if err != nil || u.Scheme != "file" {
		return "", false
	}
	path := u.Path
	if path == "" {
		path = u.Opaque
	}
	if path == "" {
		return "", false
	}
	// Windows file URLs are /C:/dir; the leading slash is not part of the
	// path.
	if runtime.GOOS == "windows" && len(path) >= 3 && path[0] == '/' && path[2] == ':' {
		path = path[1:]
	}
	return filepath.FromSlash(path), true
}
