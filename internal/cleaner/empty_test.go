package cleaner

import (
	"os"
	"path/filepath"
	"testing"
)

// emptied makes dir below root and returns the file it held, already gone, so
// the caller can pass it as a deleted path.
func emptied(t *testing.T, dir string) string {
	t.Helper()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(dir, "gone.bin")
	if err := os.WriteFile(path, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(path); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestRemoveEmptyParentsRemovesEmptiedFolder(t *testing.T) {
	root := t.TempDir()
	sub := filepath.Join(root, "sub")
	deleted := emptied(t, sub)

	removed := RemoveEmptyParents([]string{deleted}, []string{root})
	if len(removed) != 1 || removed[0] != sub {
		t.Fatalf("removed = %v, want [%s]", removed, sub)
	}
	if _, err := os.Stat(sub); !os.IsNotExist(err) {
		t.Errorf("folder still exists: %v", err)
	}
	if _, err := os.Stat(root); err != nil {
		t.Errorf("scan root was removed: %v", err)
	}
}

func TestRemoveEmptyParentsKeepsFolderWithFiles(t *testing.T) {
	root := t.TempDir()
	sub := filepath.Join(root, "sub")
	deleted := emptied(t, sub)
	if err := os.WriteFile(filepath.Join(sub, "stays.bin"), []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}

	if removed := RemoveEmptyParents([]string{deleted}, []string{root}); len(removed) != 0 {
		t.Errorf("removed = %v, want none", removed)
	}
	if _, err := os.Stat(sub); err != nil {
		t.Errorf("folder with files was removed: %v", err)
	}
}

func TestRemoveEmptyParentsKeepsPreExistingEmptyFolder(t *testing.T) {
	root := t.TempDir()
	empty := filepath.Join(root, "empty")
	if err := os.MkdirAll(empty, 0o755); err != nil {
		t.Fatal(err)
	}
	deleted := emptied(t, filepath.Join(root, "sub"))

	RemoveEmptyParents([]string{deleted}, []string{root})
	if _, err := os.Stat(empty); err != nil {
		t.Errorf("a folder that was already empty was removed: %v", err)
	}
}

func TestRemoveEmptyParentsStopsAtRoot(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "gone.bin")
	if err := os.WriteFile(path, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(path); err != nil {
		t.Fatal(err)
	}

	if removed := RemoveEmptyParents([]string{path}, []string{root}); len(removed) != 0 {
		t.Errorf("removed = %v, want none", removed)
	}
	if _, err := os.Stat(root); err != nil {
		t.Errorf("scan root was removed: %v", err)
	}
}

func TestRemoveEmptyParentsCollapsesNestedFolders(t *testing.T) {
	root := t.TempDir()
	deep := filepath.Join(root, "a", "b")
	deleted := emptied(t, deep)

	removed := RemoveEmptyParents([]string{deleted}, []string{root})
	if len(removed) != 2 {
		t.Fatalf("removed = %v, want both folders", removed)
	}
	if _, err := os.Stat(filepath.Join(root, "a")); !os.IsNotExist(err) {
		t.Errorf("nested folder still exists: %v", err)
	}
	if _, err := os.Stat(root); err != nil {
		t.Errorf("scan root was removed: %v", err)
	}
}

func TestRemoveEmptyParentsLeavesFoldersOutsideRoots(t *testing.T) {
	root := t.TempDir()
	outside := filepath.Join(t.TempDir(), "outside")
	deleted := emptied(t, outside)

	if removed := RemoveEmptyParents([]string{deleted}, []string{root}); len(removed) != 0 {
		t.Errorf("removed = %v, want none", removed)
	}
	if _, err := os.Stat(outside); err != nil {
		t.Errorf("folder outside the roots was removed: %v", err)
	}
}

func TestRemoveEmptyParentsDoesNotFollowSymlinks(t *testing.T) {
	root := t.TempDir()
	target := filepath.Join(t.TempDir(), "target")
	sub := filepath.Join(target, "sub")
	deleted := emptied(t, sub)
	link := filepath.Join(root, "link")
	if err := os.Symlink(target, link); err != nil {
		t.Skipf("symlinks unavailable: %v", err)
	}

	reached := filepath.Join(link, "sub", filepath.Base(deleted))
	if removed := RemoveEmptyParents([]string{reached}, []string{root}); len(removed) != 0 {
		t.Errorf("removed = %v, want none", removed)
	}
	if _, err := os.Stat(sub); err != nil {
		t.Errorf("folder behind a symlink was removed: %v", err)
	}
}
