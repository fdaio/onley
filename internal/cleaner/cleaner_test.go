package cleaner

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
	"time"

	"onley/internal/db"
)

func openDB(t *testing.T) *db.DB {
	t.Helper()
	store, err := db.Open(":memory:")
	if err != nil {
		t.Fatalf("db.Open: %v", err)
	}
	t.Cleanup(func() { store.Close() })
	return store
}

func writeIndexed(t *testing.T, store *db.DB, path, content string) Target {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	info, err := os.Stat(path)
	if err != nil {
		t.Fatal(err)
	}
	record := db.FileRecord{
		Path: path, Name: filepath.Base(path), Size: info.Size(),
		MD5: "d41d8cd98f00b204e9800998ecf8427e", Mtime: info.ModTime().Unix(),
	}
	if err := store.Upsert(record); err != nil {
		t.Fatal(err)
	}
	return Target{Path: path, Size: record.Size, Mtime: record.Mtime}
}

func TestVerify(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	path := filepath.Join(dir, "file.txt")
	target := writeIndexed(t, store, path, "content")
	if err := Verify(target); err != nil {
		t.Fatalf("Verify: %v", err)
	}
	if err := os.Remove(path); err != nil {
		t.Fatal(err)
	}
	if err := Verify(target); !errors.Is(err, os.ErrNotExist) {
		t.Errorf("Verify missing = %v, want os.ErrNotExist", err)
	}
}

func TestDeleteKeepsUntargetedFiles(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	keepPath := filepath.Join(dir, "keep.txt")
	deletePath := filepath.Join(dir, "delete.txt")
	if err := os.WriteFile(keepPath, []byte("keep"), 0o644); err != nil {
		t.Fatal(err)
	}
	target := writeIndexed(t, store, deletePath, "delete")

	results := Delete(store, []Target{target}, false)
	if len(results) != 1 || !results[0].Deleted || results[0].Err != nil {
		t.Fatalf("results = %+v", results)
	}
	if _, err := os.Stat(deletePath); !os.IsNotExist(err) {
		t.Errorf("deleted file still exists: %v", err)
	}
	if _, err := os.Stat(keepPath); err != nil {
		t.Errorf("untargeted file is gone: %v", err)
	}
	if rec, err := store.Lookup(deletePath); err != nil || rec != nil {
		t.Errorf("record still present: %v, %v", rec, err)
	}
}

func TestDeleteVerifyRefusesChangedFile(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	path := filepath.Join(dir, "changed.txt")
	target := writeIndexed(t, store, path, "first")
	if err := os.WriteFile(path, []byte("second content"), 0o644); err != nil {
		t.Fatal(err)
	}
	later := time.Now().Add(2 * time.Second)
	if err := os.Chtimes(path, later, later); err != nil {
		t.Fatal(err)
	}

	results := Delete(store, []Target{target}, true)
	if len(results) != 1 || results[0].Err == nil || results[0].Deleted {
		t.Fatalf("results = %+v", results)
	}
	if _, err := os.Stat(path); err != nil {
		t.Errorf("changed file was deleted: %v", err)
	}
	if rec, err := store.Lookup(path); err != nil || rec == nil {
		t.Errorf("record missing after refusal: %v, %v", rec, err)
	}
}

func TestDeleteVerifyCleansRecordOfMissingFile(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	path := filepath.Join(dir, "gone.txt")
	target := writeIndexed(t, store, path, "gone")
	if err := os.Remove(path); err != nil {
		t.Fatal(err)
	}

	results := Delete(store, []Target{target}, true)
	if len(results) != 1 || !results[0].Missing || results[0].Deleted {
		t.Fatalf("results = %+v", results)
	}
	if rec, err := store.Lookup(path); err != nil || rec != nil {
		t.Errorf("record still present: %v, %v", rec, err)
	}
}

func TestDeleteVerifyRefusesLegacyRecordWithoutMtime(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	path := filepath.Join(dir, "legacy.txt")
	target := writeIndexed(t, store, path, "legacy")
	target.Mtime = 0

	results := Delete(store, []Target{target}, true)
	if len(results) != 1 || results[0].Err == nil || results[0].Deleted {
		t.Fatalf("results = %+v", results)
	}
	if _, err := os.Stat(path); err != nil {
		t.Errorf("legacy file was deleted: %v", err)
	}
}

func TestDeleteRefusesRunningExecutable(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	path := filepath.Join(dir, "onley.exe")
	target := writeIndexed(t, store, path, "running")

	results := deleteWithProtect(store, []Target{target}, true, path)
	if len(results) != 1 || !errors.Is(results[0].Err, ErrSelfDelete) || results[0].Deleted {
		t.Fatalf("results = %+v", results)
	}
	if _, err := os.Stat(path); err != nil {
		t.Errorf("running executable was deleted: %v", err)
	}
	if rec, err := store.Lookup(path); err != nil || rec == nil {
		t.Errorf("record missing after refusal: %v, %v", rec, err)
	}
}

func TestDeleteContinuesAfterFailure(t *testing.T) {
	store := openDB(t)
	dir := t.TempDir()
	missing := Target{Path: filepath.Join(dir, "missing.txt")}
	path := filepath.Join(dir, "ok.txt")
	ok := writeIndexed(t, store, path, "ok")

	results := Delete(store, []Target{missing, ok}, true)
	if len(results) != 2 {
		t.Fatalf("results = %+v", results)
	}
	if !results[0].Missing {
		t.Errorf("missing result = %+v", results[0])
	}
	if !results[1].Deleted || results[1].Err != nil {
		t.Errorf("second result = %+v", results[1])
	}
}
