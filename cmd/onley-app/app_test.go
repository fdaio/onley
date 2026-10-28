package main

import (
	"context"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/oneo-me/nsgo/nsgotest"

	"onley/internal/db"
	"onley/internal/scanner"
)

func TestMain(m *testing.M) {
	nsgotest.Main(m)
}

func newStore(t *testing.T) *db.DB {
	t.Helper()
	store, err := db.Open(filepath.Join(t.TempDir(), "index.db"))
	if err != nil {
		t.Fatalf("db.Open: %v", err)
	}
	t.Cleanup(func() { store.Close() })
	return store
}

func writeFile(t *testing.T, dir, name, content string) string {
	t.Helper()
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

func scanInto(t *testing.T, store *db.DB, dir string) {
	t.Helper()
	for range scanner.Scan(dir, store, 2) {
	}
}

func startApp(t *testing.T, store *db.DB, dir string, scan func(context.Context, string, *db.DB, int) <-chan scanner.Progress) *nsgotest.App {
	t.Helper()
	cfg := Config{
		Store:      store,
		DBPath:     filepath.Join(t.TempDir(), "index.db"),
		Home:       t.TempDir(),
		DefaultDir: dir,
		Workers:    2,
		Scan:       scan,
	}
	return nsgotest.Start(t, Root(cfg), nsgotest.Size(1080, 720))
}

func TestEmptyState(t *testing.T) {
	store := newStore(t)
	app := startApp(t, store, t.TempDir(), nil)

	if !app.Exists("id:empty-state") {
		t.Fatalf("empty state missing\n%s", app.Dump())
	}
	if got := app.Find("id:stat-total").Text; got != "0 files" {
		t.Errorf("total = %q, want 0 files", got)
	}
	if !app.Exists("text:No duplicates yet") {
		t.Errorf("empty title missing\n%s", app.Dump())
	}
}

func TestScanRejectsMissingFolder(t *testing.T) {
	store := newStore(t)
	app := startApp(t, store, t.TempDir(), nil)
	missing := filepath.Join(t.TempDir(), "not-there")
	app.Clear("id:scan-path")
	app.Type("id:scan-path", missing)
	app.Press("id:scan-button")
	app.WaitFor("text:Cannot scan that folder", 2*time.Second)
	if !app.Exists("id:empty-state") {
		t.Errorf("empty state disappeared after a rejected scan")
	}
}

func TestScanSelectCancelThenDelete(t *testing.T) {
	dir := t.TempDir()
	content := strings.Repeat("a", 4096)
	keep := writeFile(t, dir, "a.bin", content)
	remove := writeFile(t, dir, "b.bin", content)
	writeFile(t, dir, "c.bin", "unique")

	store := newStore(t)
	app := startApp(t, store, dir, nil)
	app.Clear("id:scan-path")
	app.Type("id:scan-path", dir)
	app.Press("id:scan-button")
	app.WaitFor("text:1 group · 2 duplicate files", 10*time.Second)

	if got := app.Find("id:stat-total").Text; got != "3 files" {
		t.Errorf("total = %q, want 3 files", got)
	}
	if got := app.Find("id:stat-reclaim").Text; got != "4.0 KB" {
		t.Errorf("reclaimable = %q, want 4.0 KB", got)
	}

	app.Press("id:keep:" + keep)
	app.WaitFor("text:1 file selected", 2*time.Second)
	app.Press("id:review-delete")
	if !app.Exists("text:Delete 1 file and free 4.0 KB?") {
		t.Fatalf("confirmation missing\n%s", app.Dump())
	}
	app.Press("id:cancel-delete")
	app.WaitForGone("id:confirm-dialog", 2*time.Second)
	if _, err := os.Stat(remove); err != nil {
		t.Fatalf("cancel deleted a file: %v", err)
	}

	app.Press("id:review-delete")
	app.Press("id:delete-confirm")
	app.WaitFor("text:Deleted 1 file, freed 4.0 KB.", 10*time.Second)
	if _, err := os.Stat(remove); !os.IsNotExist(err) {
		t.Errorf("selected file still exists: %v", err)
	}
	if _, err := os.Stat(keep); err != nil {
		t.Errorf("kept file is gone: %v", err)
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:stat-total").Text == "2 files" })
	if got := app.Find("id:stat-groups").Text; got != "0 groups" {
		t.Errorf("groups = %q, want 0 groups", got)
	}
}

func TestDeleteAbortsWhenKeptFileIsGone(t *testing.T) {
	dir := t.TempDir()
	content := strings.Repeat("a", 4096)
	keep := writeFile(t, dir, "a.bin", content)
	other := writeFile(t, dir, "b.bin", content)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:keep:" + keep)
	app.WaitFor("text:1 file selected", 2*time.Second)
	if err := os.Remove(keep); err != nil {
		t.Fatal(err)
	}
	app.Press("id:review-delete")
	app.Press("id:delete-confirm")
	app.WaitFor("text:Cannot keep", 5*time.Second)
	if _, err := os.Stat(other); err != nil {
		t.Errorf("other copy was deleted after the kept copy vanished: %v", err)
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:stat-groups").Text == "1 group" })
}

func TestDeleteRefusesChangedFile(t *testing.T) {
	dir := t.TempDir()
	content := strings.Repeat("a", 4096)
	keep := writeFile(t, dir, "a.bin", content)
	changed := writeFile(t, dir, "b.bin", content)
	store := newStore(t)
	scanInto(t, store, dir)

	if err := os.WriteFile(changed, []byte(strings.Repeat("b", 5000)), 0o644); err != nil {
		t.Fatal(err)
	}
	later := time.Now().Add(2 * time.Second)
	if err := os.Chtimes(changed, later, later); err != nil {
		t.Fatal(err)
	}

	app := startApp(t, store, dir, nil)
	app.Press("id:keep:" + keep)
	app.WaitFor("text:1 file selected", 2*time.Second)
	app.Press("id:review-delete")
	app.Press("id:delete-confirm")
	app.WaitFor("text:changed since scan", 10*time.Second)

	if _, err := os.Stat(changed); err != nil {
		t.Errorf("changed file was deleted: %v", err)
	}
	if got := app.Find("id:stat-groups").Text; got != "1 group" {
		t.Errorf("groups = %q, want 1 group", got)
	}
}

func TestDeleteCleansRecordOfMissingFile(t *testing.T) {
	dir := t.TempDir()
	content := strings.Repeat("a", 4096)
	keep := writeFile(t, dir, "a.bin", content)
	gone := writeFile(t, dir, "b.bin", content)
	writeFile(t, dir, "c.bin", "unique")
	store := newStore(t)
	scanInto(t, store, dir)
	if err := os.Remove(gone); err != nil {
		t.Fatal(err)
	}

	app := startApp(t, store, dir, nil)
	app.Press("id:keep:" + keep)
	app.WaitFor("text:1 file selected", 2*time.Second)
	app.Press("id:review-delete")
	app.Press("id:delete-confirm")
	app.WaitFor("text:1 was already gone", 10*time.Second)
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:stat-total").Text == "2 files" })
	if got := app.Find("id:stat-groups").Text; got != "0 groups" {
		t.Errorf("groups = %q, want 0 groups", got)
	}
}

func TestKeepFirstAcrossGroups(t *testing.T) {
	dir := t.TempDir()
	firstA := writeFile(t, dir, "a1.bin", strings.Repeat("a", 4096))
	secondA := writeFile(t, dir, "a2.bin", strings.Repeat("a", 4096))
	firstB := writeFile(t, dir, "b1.bin", strings.Repeat("b", 4096))
	secondB := writeFile(t, dir, "b2.bin", strings.Repeat("b", 4096))
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:keep-first")
	app.WaitFor("text:2 files selected", 2*time.Second)
	app.Press("id:review-delete")
	if !app.Exists("text:Delete 2 files and free 8.0 KB?") {
		t.Fatalf("confirmation missing\n%s", app.Dump())
	}
	app.Press("id:delete-confirm")
	app.WaitFor("text:Deleted 2 files, freed 8.0 KB.", 10*time.Second)
	if _, err := os.Stat(firstA); err != nil {
		t.Errorf("first file of group A is gone: %v", err)
	}
	if _, err := os.Stat(firstB); err != nil {
		t.Errorf("first file of group B is gone: %v", err)
	}
	for _, path := range []string{secondA, secondB} {
		if _, err := os.Stat(path); !os.IsNotExist(err) {
			t.Errorf("second copy %s still exists: %v", path, err)
		}
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:stat-groups").Text == "0 groups" })
}

func TestCancelScan(t *testing.T) {
	dir := t.TempDir()
	store := newStore(t)
	scan := func(ctx context.Context, _ string, _ *db.DB, _ int) <-chan scanner.Progress {
		ch := make(chan scanner.Progress)
		go func() {
			defer close(ch)
			n := 0
			for {
				select {
				case <-ctx.Done():
					return
				case <-time.After(20 * time.Millisecond):
					n++
					select {
					case ch <- scanner.Progress{Done: true, Current: n}:
					case <-ctx.Done():
						return
					}
				}
			}
		}()
		return ch
	}
	app := startApp(t, store, dir, scan)
	app.Press("id:scan-button")
	app.WaitFor("id:cancel-scan", 2*time.Second)
	app.Press("id:cancel-scan")
	app.WaitFor("text:Scan canceled. The partial index is kept.", 5*time.Second)
	if app.Exists("id:cancel-scan") {
		t.Errorf("scan controls still visible after cancel")
	}
}

func TestDropFolderStartsScan(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, "a.bin", "same")
	writeFile(t, dir, "b.bin", "same")
	store := newStore(t)
	app := startApp(t, store, t.TempDir(), nil)

	uri := (&url.URL{Scheme: "file", Path: dir}).String()
	app.DropFiles("id:drop-zone", uri)
	app.WaitFor("text:1 group · 2 duplicate files", 10*time.Second)
}

func TestNativeScreenshot(t *testing.T) {
	dir := t.TempDir()
	keep := writeFile(t, dir, "a.bin", strings.Repeat("a", 4096))
	writeFile(t, dir, "b.bin", strings.Repeat("a", 4096))
	writeFile(t, dir, "c.bin", "unique")
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:keep:" + keep)
	app.WaitFor("text:1 file selected", 2*time.Second)
	app.Screenshot("onley-main")
}
