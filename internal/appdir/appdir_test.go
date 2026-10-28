package appdir

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestDirCreatesPrivateDirectory(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("USERPROFILE", home)

	dir, err := Dir()
	if err != nil {
		t.Fatalf("Dir: %v", err)
	}
	if want := filepath.Join(home, DirName); dir != want {
		t.Fatalf("Dir = %q, want %q", dir, want)
	}
	info, err := os.Stat(dir)
	if err != nil {
		t.Fatalf("stat: %v", err)
	}
	if runtime.GOOS != "windows" {
		if got := info.Mode().Perm(); got != 0o700 {
			t.Errorf("mode = %o, want 700", got)
		}
	}
}

func TestDBAndConfigPaths(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("USERPROFILE", home)

	db, err := DBPath()
	if err != nil {
		t.Fatalf("DBPath: %v", err)
	}
	if want := filepath.Join(home, DirName, DBFileName); db != want {
		t.Errorf("DBPath = %q, want %q", db, want)
	}

	cfg, err := ConfigPath()
	if err != nil {
		t.Fatalf("ConfigPath: %v", err)
	}
	if want := filepath.Join(home, DirName, ConfigFileName); cfg != want {
		t.Errorf("ConfigPath = %q, want %q", cfg, want)
	}
}

func TestDirIsIdempotent(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("USERPROFILE", home)

	first, err := Dir()
	if err != nil {
		t.Fatal(err)
	}
	second, err := Dir()
	if err != nil {
		t.Fatal(err)
	}
	if first != second {
		t.Errorf("Dir changed from %q to %q", first, second)
	}
}
