// Package selfpath identifies the running executable.
package selfpath

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

// Resolve returns the running executable's path with symlinks resolved.
func Resolve() string {
	path, err := os.Executable()
	if err != nil {
		return ""
	}
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		return resolved
	}
	return path
}

// Same reports whether a and b name the same file. It handles symlinks and
// the case-insensitive paths Windows returns.
func Same(a, b string) bool {
	if a == "" || b == "" {
		return false
	}
	cleanA, cleanB := filepath.Clean(a), filepath.Clean(b)
	if cleanA == cleanB {
		return true
	}
	if infoA, err := os.Stat(a); err == nil {
		if infoB, err := os.Stat(b); err == nil && os.SameFile(infoA, infoB) {
			return true
		}
	}
	if runtime.GOOS == "windows" {
		return strings.EqualFold(cleanA, cleanB)
	}
	return false
}
