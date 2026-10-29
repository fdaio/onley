package selfpath

import (
	"os"
	"path/filepath"
	"testing"
)

func TestResolve(t *testing.T) {
	if got := Resolve(); got == "" {
		t.Fatal("Resolve returned an empty path")
	}
}

func TestSame(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "onley.exe")
	if err := os.WriteFile(path, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if !Same(path, path) {
		t.Error("Same(path, path) = false")
	}
	if Same(path, filepath.Join(dir, "other.exe")) {
		t.Error("Same matched a different file")
	}
	if Same(path, "") || Same("", path) {
		t.Error("Same matched an empty path")
	}
}
