// Package appdir locates onley's per-user files.
package appdir

import (
	"fmt"
	"os"
	"path/filepath"
)

const (
	// DirName is the directory under the user's home that holds the index
	// and the settings.
	DirName = ".onley"
	// DBFileName is the index file inside DirName.
	DBFileName = "local.db"
	// ConfigFileName is the settings file inside DirName.
	ConfigFileName = "config.json"
)

// Dir returns ~/.onley, creating it if it does not exist.
//
// It is 0700 because the index lists the paths and digests of the user's
// files. The directory is what keeps that private; the modes on the files
// inside it do not have to be restrictive as well.
func Dir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("cannot locate the home directory: %w", err)
	}
	dir := filepath.Join(home, DirName)
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return "", fmt.Errorf("cannot create %s: %w", dir, err)
	}
	return dir, nil
}

// DBPath returns the default index path, creating its directory if needed.
func DBPath() (string, error) {
	dir, err := Dir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, DBFileName), nil
}

// ConfigPath returns the default settings path, creating its directory if
// needed.
func ConfigPath() (string, error) {
	dir, err := Dir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, ConfigFileName), nil
}
