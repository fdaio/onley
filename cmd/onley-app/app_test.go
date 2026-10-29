package main

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/oneo-me/nsgo/core"
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
	return startAppWithSelf(t, store, dir, scan, "")
}

func startAppWithSelf(t *testing.T, store *db.DB, dir string, scan func(context.Context, string, *db.DB, int) <-chan scanner.Progress, selfPath string) *nsgotest.App {
	t.Helper()
	cfg := Config{
		Store:      store,
		DBPath:     filepath.Join(t.TempDir(), "index.db"),
		Home:       t.TempDir(),
		DefaultDir: dir,
		Workers:    2,
		SelfPath:   selfPath,
		Scan:       scan,
	}
	return nsgotest.Start(t, Root(cfg), nsgotest.Size(1080, 720))
}

func addFolderViaDialog(t *testing.T, app *nsgotest.App, path string) {
	t.Helper()
	app.Press("id:add-folder")
	app.WaitFor("id:add-folder-dialog", 2*time.Second)
	app.Type("id:scan-path", path)
	app.Press("id:add-folder-confirm")
	app.WaitForGone("id:add-folder-dialog", 2*time.Second)
}

func TestChineseLocaleDefault(t *testing.T) {
	cfg := Config{Store: newStore(t), Home: t.TempDir(), DefaultDir: t.TempDir(), Workers: 2}
	app := nsgotest.Start(t, Root(cfg), nsgotest.Size(1080, 720),
		nsgotest.Environment(core.Environment{Locale: "zh-CN"}))
	if !app.Exists("text:仅本地运行") {
		t.Fatalf("Chinese locale did not select Chinese\n%s", app.Dump())
	}
	if !app.Exists("text:0 组重复文件") {
		t.Errorf("Chinese summary missing\n%s", app.Dump())
	}
}

func TestStoredLanguage(t *testing.T) {
	cfg := Config{Store: newStore(t), Home: t.TempDir(), DefaultDir: t.TempDir(), Workers: 2}
	app := nsgotest.Start(t, Root(cfg), nsgotest.Size(1080, 720),
		nsgotest.Storage(map[string]string{"language": "zh"}))
	if !app.Exists("text:仅本地运行") {
		t.Fatalf("stored language did not select Chinese\n%s", app.Dump())
	}
}

func TestLanguageToggle(t *testing.T) {
	app := startApp(t, newStore(t), t.TempDir(), nil)
	if !app.Exists("text:Local only") {
		t.Fatalf("English header missing\n%s", app.Dump())
	}
	app.Press("id:language-toggle")
	app.WaitFor("text:仅本地运行", 2*time.Second)
	if !app.Exists("text:0 组重复文件") {
		t.Errorf("Chinese summary missing\n%s", app.Dump())
	}
	app.Screenshot("chinese")
	app.Press("id:language-toggle")
	app.WaitFor("text:Local only", 2*time.Second)
}

func TestEmptyState(t *testing.T) {
	store := newStore(t)
	app := startApp(t, store, t.TempDir(), nil)

	if !app.Exists("id:empty-state") {
		t.Fatalf("empty state missing\n%s", app.Dump())
	}
	if got := app.Find("id:group-summary").Text; got != "0 duplicate groups" {
		t.Errorf("groups = %q, want 0 duplicate groups", got)
	}
	if got := app.Find("id:duplicate-summary").Text; got != "0 files · 0 B reclaimable" {
		t.Errorf("summary = %q, want 0 files · 0 B reclaimable", got)
	}
	if !app.Exists("text:No duplicates yet") {
		t.Errorf("empty title missing\n%s", app.Dump())
	}
	if app.Exists("text:✓") {
		t.Errorf("empty state still uses the text check glyph\n%s", app.Dump())
	}
}

func TestButtonHoverAndPress(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, "a.bin", strings.Repeat("a", 4096))
	writeFile(t, dir, "b.bin", strings.Repeat("a", 4096))
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	idle := app.Find("id:policy-newest").Style.Background
	app.Hover("id:policy-newest")
	if got := app.Find("id:policy-newest").Style.Background; got == idle {
		t.Errorf("hover background unchanged: %v", got)
	}
	app.PressDown("id:policy-newest")
	if got := app.Find("id:policy-newest").Style.Background; got == idle {
		t.Errorf("pressed background unchanged: %v", got)
	}
	app.PressUp("id:policy-newest")
	app.HoverOut()
	if got := app.Find("id:policy-newest").Style.Background; got != idle {
		t.Errorf("released background = %v, want %v", got, idle)
	}
}

func TestAddFolderMergesParentChild(t *testing.T) {
	parent := t.TempDir()
	child := filepath.Join(parent, "child")
	if err := os.Mkdir(child, 0o755); err != nil {
		t.Fatal(err)
	}
	sibling := t.TempDir()

	app := startApp(t, newStore(t), parent, nil)
	if !app.Exists("id:folder:" + parent) {
		t.Fatalf("initial folder missing\n%s", app.Dump())
	}

	addFolderViaDialog(t, app, child)
	if !app.Exists("id:folder:"+parent) || app.Exists("id:folder:"+child) {
		t.Fatalf("child was not merged into its parent\n%s", app.Dump())
	}

	addFolderViaDialog(t, app, sibling)
	if !app.Exists("id:folder:"+parent) || !app.Exists("id:folder:"+sibling) {
		t.Fatalf("sibling folder missing\n%s", app.Dump())
	}
}

func TestScanMultipleRoots(t *testing.T) {
	first := t.TempDir()
	second := t.TempDir()
	writeFile(t, first, "a1.bin", strings.Repeat("a", 4096))
	writeFile(t, first, "a2.bin", strings.Repeat("a", 4096))
	writeFile(t, second, "b1.bin", strings.Repeat("b", 4096))
	writeFile(t, second, "b2.bin", strings.Repeat("b", 4096))

	app := startApp(t, newStore(t), first, nil)
	addFolderViaDialog(t, app, second)
	app.Press("id:scan-button")
	app.WaitFor("text:2 duplicate groups", 10*time.Second)
	summary := app.Find("id:duplicate-summary").Text
	if !strings.Contains(summary, "4 files") || !strings.Contains(summary, "8.0 KB") {
		t.Errorf("summary = %q, want 4 files and 8.0 KB", summary)
	}
}

func TestScanRejectsMissingFolder(t *testing.T) {
	store := newStore(t)
	app := startApp(t, store, t.TempDir(), nil)
	missing := filepath.Join(t.TempDir(), "not-there")

	app.Press("id:add-folder")
	app.WaitFor("id:add-folder-dialog", 2*time.Second)
	app.Type("id:scan-path", missing)
	app.Press("id:add-folder-confirm")
	app.WaitFor("id:add-folder-error", 2*time.Second)
	if got := app.Find("id:add-folder-error").Text; !strings.Contains(got, "Cannot scan that folder") {
		t.Errorf("error = %q, want a missing-folder error", got)
	}
	app.Press("id:cancel-add-folder")
	app.WaitForGone("id:add-folder-dialog", 2*time.Second)

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
	app.Press("id:scan-button")
	app.WaitFor("text:1 duplicate group", 10*time.Second)

	if got := app.Find("id:duplicate-summary").Text; !strings.Contains(got, "2 files") || !strings.Contains(got, "4.0 KB") {
		t.Errorf("summary = %q, want 3 files and 4.0 KB", got)
	}

	app.Press("id:keep:" + keep)
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	app.Press("id:delete-files")
	if !app.Exists("text:Delete 1 file and free 4.0 KB?") {
		t.Fatalf("confirmation missing\n%s", app.Dump())
	}
	app.Press("id:cancel-delete")
	app.WaitForGone("id:confirm-dialog", 2*time.Second)
	if _, err := os.Stat(remove); err != nil {
		t.Fatalf("cancel deleted a file: %v", err)
	}

	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:Deleted 1 file, freed 4.0 KB.", 10*time.Second)
	if _, err := os.Stat(remove); !os.IsNotExist(err) {
		t.Errorf("selected file still exists: %v", err)
	}
	if _, err := os.Stat(keep); err != nil {
		t.Errorf("kept file is gone: %v", err)
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "0 duplicate groups" })
	if got := app.Find("id:duplicate-summary").Text; got != "0 files · 0 B reclaimable" {
		t.Errorf("summary = %q, want 0 files · 0 B reclaimable", got)
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
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	if err := os.Remove(keep); err != nil {
		t.Fatal(err)
	}
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:Cannot keep", 5*time.Second)
	if _, err := os.Stat(other); err != nil {
		t.Errorf("other copy was deleted after the kept copy vanished: %v", err)
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "1 duplicate group" })
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
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:changed since scan", 10*time.Second)

	if _, err := os.Stat(changed); err != nil {
		t.Errorf("changed file was deleted: %v", err)
	}
	if got := app.Find("id:group-summary").Text; got != "1 duplicate group" {
		t.Errorf("groups = %q, want 1 duplicate group", got)
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
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:1 was already gone", 10*time.Second)
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "0 duplicate groups" })
}

func setMtime(t *testing.T, path string, unix int64) {
	t.Helper()
	tm := time.Unix(unix, 0)
	if err := os.Chtimes(path, tm, tm); err != nil {
		t.Fatal(err)
	}
}

func TestAutoKeepNewestAcrossGroups(t *testing.T) {
	dir := t.TempDir()
	oldA := writeFile(t, dir, "a1.bin", strings.Repeat("a", 4096))
	newA := writeFile(t, dir, "a2.bin", strings.Repeat("a", 4096))
	oldB := writeFile(t, dir, "b1.bin", strings.Repeat("b", 4096))
	newB := writeFile(t, dir, "b2.bin", strings.Repeat("b", 4096))
	setMtime(t, oldA, 100)
	setMtime(t, oldB, 100)
	setMtime(t, newA, 200)
	setMtime(t, newB, 200)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:policy-newest")
	app.WaitFor("text:Delete 2 files", 2*time.Second)
	app.Press("id:delete-files")
	if !app.Exists("text:Delete 2 files and free 8.0 KB?") {
		t.Fatalf("confirmation missing\n%s", app.Dump())
	}
	app.Press("id:cancel-delete")
	app.WaitForGone("id:confirm-dialog", 2*time.Second)
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:Deleted 2 files, freed 8.0 KB.", 10*time.Second)
	if _, err := os.Stat(newA); err != nil {
		t.Errorf("newest A is gone: %v", err)
	}
	if _, err := os.Stat(newB); err != nil {
		t.Errorf("newest B is gone: %v", err)
	}
	for _, path := range []string{oldA, oldB} {
		if _, err := os.Stat(path); !os.IsNotExist(err) {
			t.Errorf("old copy %s still exists: %v", path, err)
		}
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "0 duplicate groups" })
}

func TestAutoKeepOldestAcrossGroups(t *testing.T) {
	dir := t.TempDir()
	oldA := writeFile(t, dir, "a1.bin", strings.Repeat("a", 4096))
	newA := writeFile(t, dir, "a2.bin", strings.Repeat("a", 4096))
	oldB := writeFile(t, dir, "b1.bin", strings.Repeat("b", 4096))
	newB := writeFile(t, dir, "b2.bin", strings.Repeat("b", 4096))
	setMtime(t, oldA, 100)
	setMtime(t, oldB, 100)
	setMtime(t, newA, 200)
	setMtime(t, newB, 200)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:keep:" + oldA)
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	app.Press("id:policy-oldest")
	app.WaitFor("text:Delete 2 files", 2*time.Second)
	if got, ok := app.Storage().GetItem("keep_policy"); !ok || got != "oldest" {
		t.Errorf("stored keep policy = %q, %v; want oldest", got, ok)
	}
	app.Press("id:delete-files")
	if !app.Exists("text:Delete 2 files and free 8.0 KB?") {
		t.Fatalf("confirmation missing\n%s", app.Dump())
	}
	app.Press("id:delete-confirm")
	app.WaitFor("text:Deleted 2 files, freed 8.0 KB.", 10*time.Second)
	if _, err := os.Stat(oldA); err != nil {
		t.Errorf("oldest A is gone: %v", err)
	}
	if _, err := os.Stat(oldB); err != nil {
		t.Errorf("oldest B is gone: %v", err)
	}
	for _, path := range []string{newA, newB} {
		if _, err := os.Stat(path); !os.IsNotExist(err) {
			t.Errorf("new copy %s still exists: %v", path, err)
		}
	}
}

func TestAutoKeepKeepsRunningExecutable(t *testing.T) {
	dir := t.TempDir()
	self := writeFile(t, dir, "Onley.exe", strings.Repeat("a", 4096))
	other := writeFile(t, dir, "Onley copy.exe", strings.Repeat("a", 4096))
	setMtime(t, self, 100)
	setMtime(t, other, 200)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startAppWithSelf(t, store, dir, nil, self)
	if !app.Exists("text:Running") {
		t.Fatalf("running badge missing\n%s", app.Dump())
	}
	if !app.Find("id:keep:" + other).Props.Disabled {
		t.Errorf("other copy is selectable in a group with the running executable")
	}
	if app.Find("id:keep:" + self).Props.Disabled {
		t.Errorf("running executable is not selectable")
	}
	if nsgotest.Target() != "" {
		app.Screenshot("running-executable")
	}

	app.Press("id:policy-newest")
	if !app.Find("id:keep:" + self).Props.Selected {
		t.Fatalf("running executable was not kept\n%s", app.Dump())
	}
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	app.Press("id:delete-files")
	if !app.Exists("text:Delete 1 file and free 4.0 KB?") {
		t.Fatalf("confirmation missing\n%s", app.Dump())
	}
	app.Press("id:delete-confirm")
	app.WaitFor("text:Deleted 1 file, freed 4.0 KB.", 10*time.Second)
	if _, err := os.Stat(self); err != nil {
		t.Errorf("running executable is gone: %v", err)
	}
	if _, err := os.Stat(other); !os.IsNotExist(err) {
		t.Errorf("other copy still exists: %v", err)
	}
}

func TestScanShowsSpeedGauge(t *testing.T) {
	dir := t.TempDir()
	scan := func(ctx context.Context, _ string, _ *db.DB, _ int) <-chan scanner.Progress {
		ch := make(chan scanner.Progress)
		go func() {
			defer close(ch)
			for {
				select {
				case <-ctx.Done():
					return
				case <-time.After(60 * time.Millisecond):
					select {
					case ch <- scanner.Progress{Bytes: 1 << 20}:
					case <-ctx.Done():
						return
					}
				}
			}
		}()
		return ch
	}
	app := startApp(t, newStore(t), dir, scan)
	app.Press("id:scan-button")
	app.WaitFor("id:speed-gauge", 2*time.Second)
	app.WaitFor("id:scan-speed", 2*time.Second)
	app.WaitUntil(3*time.Second, func() bool { return app.Find("id:scan-speed").Text != "0 B/s" })
	if speed := app.Find("id:scan-speed").Text; !strings.Contains(speed, "MB/s") {
		t.Errorf("speed = %q, want MB/s", speed)
	}
	if !app.Exists("text:Finished files are saved. Scan again to resume.") {
		t.Errorf("resume hint missing\n%s", app.Dump())
	}
	app.Screenshot("scanning")
	app.Press("id:cancel-scan")
	app.WaitFor("text:Scan canceled. The partial index is kept.", 5*time.Second)
	if app.Exists("id:speed-gauge") {
		t.Errorf("speed gauge still visible after cancel")
	}
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
	app.WaitFor("text:Finished files are saved. Scan again to resume.", 2*time.Second)
	app.Press("id:cancel-scan")
	app.WaitFor("text:Scan canceled. The partial index is kept.", 5*time.Second)
	if app.Exists("id:cancel-scan") {
		t.Errorf("scan controls still visible after cancel")
	}
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
	app.WaitFor("text:Delete 1 file", 2*time.Second)
	app.Screenshot("onley-main")
}
