package main

import (
	"context"
	"fmt"
	"net/url"
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

func surfaceFrame(v *nsgotest.View) (x, y, width, height float32) {
	b := v.Frame
	for p := v.Parent; p != nil; p = p.Parent {
		b.Origin.X += p.Frame.Origin.X - p.Offset.X
		b.Origin.Y += p.Frame.Origin.Y - p.Offset.Y
	}
	return float32(b.Origin.X), float32(b.Origin.Y), float32(b.Size.Width), float32(b.Size.Height)
}

func startAppWithStorage(t *testing.T, store *db.DB, dir string, storage map[string]string) *nsgotest.App {
	t.Helper()
	cfg := Config{
		Store:      store,
		DBPath:     filepath.Join(t.TempDir(), "index.db"),
		Home:       t.TempDir(),
		DefaultDir: dir,
		Workers:    2,
	}
	return nsgotest.Start(t, Root(cfg), nsgotest.Size(1080, 720), nsgotest.Storage(storage))
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
	if got := app.Find("id:group-summary").Text; got != "0" {
		t.Errorf("Chinese summary = %q, want 0\n%s", got, app.Dump())
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
	if !app.Exists("text:LOCAL ONLY") {
		t.Fatalf("English header missing\n%s", app.Dump())
	}
	app.Press("id:language-toggle")
	app.WaitFor("text:仅本地运行", 2*time.Second)
	if got := app.Find("id:group-summary").Text; got != "0" {
		t.Errorf("Chinese summary = %q, want 0\n%s", got, app.Dump())
	}
	app.Screenshot("chinese")
	app.Press("id:language-toggle")
	app.WaitFor("text:LOCAL ONLY", 2*time.Second)
}

func TestEmptyState(t *testing.T) {
	store := newStore(t)
	app := startApp(t, store, t.TempDir(), nil)

	if !app.Exists("id:empty-state") {
		t.Fatalf("empty state missing\n%s", app.Dump())
	}
	if got := app.Find("id:group-summary").Text; got != "0" {
		t.Errorf("groups = %q, want 0", got)
	}
	if got := app.Find("id:duplicate-summary").Text; got != "0 B" {
		t.Errorf("summary = %q, want 0 B", got)
	}
	if !app.Exists("text:NO DUPLICATES YET") {
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
	app.WaitUntil(10*time.Second, func() bool { return app.Find("id:group-summary").Text == "2" })
	if got := app.Find("id:indexed-files").Text; got != "4" {
		t.Errorf("indexed files = %q, want 4", got)
	}
	if summary := app.Find("id:duplicate-summary").Text; !strings.Contains(summary, "8.0 KB") {
		t.Errorf("summary = %q, want 8.0 KB", summary)
	}
}

func TestCategoryTabsFilter(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, "photo.jpg", strings.Repeat("p", 4096))
	writeFile(t, dir, "photo copy.jpg", strings.Repeat("p", 4096))
	writeFile(t, dir, "clip.mov", strings.Repeat("v", 4096))
	writeFile(t, dir, "clip copy.mov", strings.Repeat("v", 4096))
	writeFile(t, dir, "report.pdf", strings.Repeat("d", 4096))
	writeFile(t, dir, "report copy.pdf", strings.Repeat("d", 4096))
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	for _, label := range []string{"ALL COPIES (03)", "IMAGES (01)", "VIDEOS (01)", "DOCUMENTS (01)"} {
		if !app.Exists("text:" + label) {
			t.Errorf("tab %q missing\n%s", label, app.Dump())
		}
	}
	app.Press("id:tab:images")
	app.WaitUntil(2*time.Second, func() bool { return !app.Exists("id:keep:" + filepath.Join(dir, "clip.mov")) })
	if !app.Exists("id:keep:" + filepath.Join(dir, "photo.jpg")) {
		t.Errorf("image group missing after filter\n%s", app.Dump())
	}
	app.Press("id:tab:all")
	app.WaitUntil(2*time.Second, func() bool { return app.Exists("id:keep:" + filepath.Join(dir, "clip.mov")) })
}

func TestSearchFilters(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, "alpha.bin", strings.Repeat("a", 4096))
	writeFile(t, dir, "alpha copy.bin", strings.Repeat("a", 4096))
	writeFile(t, dir, "beta.bin", strings.Repeat("b", 4096))
	writeFile(t, dir, "beta copy.bin", strings.Repeat("b", 4096))
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:tab:search")
	app.WaitFor("id:search-input", 2*time.Second)
	app.Type("id:search-input", "alpha\n")
	app.WaitUntil(2*time.Second, func() bool { return !app.Exists("id:keep:" + filepath.Join(dir, "beta.bin")) })
	if !app.Exists("id:keep:" + filepath.Join(dir, "alpha.bin")) {
		t.Errorf("alpha group missing after search\n%s", app.Dump())
	}
}

func TestKeepByPath(t *testing.T) {
	dir := t.TempDir()
	z := writeFile(t, dir, "z.bin", strings.Repeat("a", 4096))
	a := writeFile(t, dir, "a.bin", strings.Repeat("a", 4096))
	setMtime(t, a, 100)
	setMtime(t, z, 200)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	app.Press("id:policy-path")
	app.WaitUntil(2*time.Second, func() bool { return app.Find("id:keep:" + a).Props.Selected })
	if app.Find("id:keep:" + z).Props.Selected {
		t.Errorf("z.bin is still kept after keep-by-path")
	}
}

func TestDropFolder(t *testing.T) {
	first := t.TempDir()
	second := t.TempDir()
	app := startApp(t, newStore(t), first, nil)
	uri := (&url.URL{Scheme: "file", Path: second}).String()
	app.DropFiles("id:drop-zone", uri)
	if !app.Exists("id:folder:" + second) {
		t.Fatalf("dropped folder missing: %s\n%s", uri, app.Dump())
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
	app.WaitUntil(10*time.Second, func() bool { return app.Find("id:group-summary").Text == "1" })

	if got := app.Find("id:indexed-files").Text; got != "3" {
		t.Errorf("indexed files = %q, want 3", got)
	}
	if got := app.Find("id:duplicate-summary").Text; !strings.Contains(got, "4.0 KB") {
		t.Errorf("summary = %q, want 4.0 KB", got)
	}

	app.Press("id:keep:" + keep)
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
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
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "0" })
	if got := app.Find("id:duplicate-summary").Text; got != "0 B" {
		t.Errorf("summary = %q, want 0 B", got)
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
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
	if err := os.Remove(keep); err != nil {
		t.Fatal(err)
	}
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:Cannot keep", 5*time.Second)
	if _, err := os.Stat(other); err != nil {
		t.Errorf("other copy was deleted after the kept copy vanished: %v", err)
	}
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "1" })
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
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:changed since scan", 10*time.Second)

	if _, err := os.Stat(changed); err != nil {
		t.Errorf("changed file was deleted: %v", err)
	}
	if got := app.Find("id:group-summary").Text; got != "1" {
		t.Errorf("groups = %q, want 1", got)
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
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
	app.Press("id:delete-files")
	app.Press("id:delete-confirm")
	app.WaitFor("text:1 was already gone", 10*time.Second)
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "0" })
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
	app.WaitFor("text:REVIEW & DELETE (2 FILES)", 2*time.Second)
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
	app.WaitUntil(5*time.Second, func() bool { return app.Find("id:group-summary").Text == "0" })
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
	app.Press("id:policy-oldest")
	app.WaitFor("text:REVIEW & DELETE (2 FILES)", 2*time.Second)
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
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
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

func TestFileRowsShowFolder(t *testing.T) {
	first := t.TempDir()
	second := t.TempDir()
	content := strings.Repeat("a", 4096)
	one := writeFile(t, first, "photo.jpg", content)
	two := writeFile(t, second, "photo.jpg", content)
	store := newStore(t)
	scanInto(t, store, first)
	scanInto(t, store, second)

	app := startApp(t, store, first, nil)
	for _, path := range []string{one, two} {
		row := app.Find("id:file-dir:" + path)
		if row == nil {
			t.Fatalf("path label missing for %s\n%s", path, app.Dump())
		}
		if got := row.Children[0].Text; got != path {
			t.Errorf("path for %s = %q", path, got)
		}
	}
}

func TestFileRowsShowPathForDistinctNames(t *testing.T) {
	dir := t.TempDir()
	content := strings.Repeat("a", 4096)
	one := writeFile(t, dir, "a.bin", content)
	two := writeFile(t, dir, "b.bin", content)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	for _, path := range []string{one, two} {
		if !app.Exists("id:file-dir:" + path) {
			t.Errorf("path label missing for distinct names\n%s", app.Dump())
		}
	}
}

func TestAutoKeepHighlightMatchesChoices(t *testing.T) {
	dir := t.TempDir()
	old := writeFile(t, dir, "a1.bin", strings.Repeat("a", 4096))
	newest := writeFile(t, dir, "a2.bin", strings.Repeat("a", 4096))
	setMtime(t, old, 100)
	setMtime(t, newest, 200)
	store := newStore(t)
	scanInto(t, store, dir)

	app := startAppWithStorage(t, store, dir, map[string]string{"keep_policy": "newest"})
	if !app.Find("id:policy-newest").Props.Selected {
		t.Fatalf("newest is not applied on load\n%s", app.Dump())
	}
	app.Press("id:policy-newest")
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
	if !app.Find("id:policy-newest").Props.Selected {
		t.Errorf("newest is not highlighted after it is applied\n%s", app.Dump())
	}

	app.Press("id:keep:" + old)
	app.WaitUntil(2*time.Second, func() bool {
		return !app.Find("id:policy-newest").Props.Selected &&
			!app.Find("id:policy-oldest").Props.Selected
	})
}

func TestRemoveFolder(t *testing.T) {
	first := t.TempDir()
	second := t.TempDir()
	app := startApp(t, newStore(t), first, nil)
	addFolderViaDialog(t, app, second)

	app.Press("id:remove-folder:" + second)
	app.WaitForGone("id:folder:"+second, 2*time.Second)
	if !app.Exists("id:folder:" + first) {
		t.Fatalf("first folder disappeared\n%s", app.Dump())
	}

	app.Press("id:remove-folder:" + first)
	app.WaitForGone("id:folder:"+first, 2*time.Second)
	if !app.Find("id:scan-button").Props.Disabled {
		t.Errorf("rescan is enabled with no folders")
	}
	if app.Find("id:add-folder").Props.Disabled {
		t.Errorf("add folder is disabled with no folders")
	}
}

func TestClearChoices(t *testing.T) {
	dir := t.TempDir()
	keep := writeFile(t, dir, "a.bin", strings.Repeat("a", 4096))
	writeFile(t, dir, "b.bin", strings.Repeat("a", 4096))
	store := newStore(t)
	scanInto(t, store, dir)

	app := startApp(t, store, dir, nil)
	if app.Find("id:clear-choices").Props.Disabled {
		t.Errorf("clear choices is disabled on load")
	}
	app.Press("id:clear-choices")
	app.WaitUntil(2*time.Second, func() bool {
		return app.Find("id:delete-files").Props.Disabled &&
			app.Find("id:delete-files").Props.Label == "REVIEW & DELETE (0 FILES)"
	})
	if app.Find("id:keep:" + keep).Props.Selected {
		t.Errorf("keep choice is still selected after clear")
	}

	app.Press("id:keep:" + keep)
	app.WaitFor("text:REVIEW & DELETE (1 FILE)", 2*time.Second)
	if app.Find("id:clear-choices").Props.Disabled {
		t.Errorf("clear choices is disabled with a choice")
	}
}

func TestScrollToLastGroup(t *testing.T) {
	dir := t.TempDir()
	for group := 0; group < 10; group++ {
		content := strings.Repeat(string(rune('a'+group)), 4096)
		for copyIndex := 0; copyIndex < 3; copyIndex++ {
			sub := filepath.Join(dir, fmt.Sprintf("g%02d-%d", group, copyIndex))
			if err := os.Mkdir(sub, 0o755); err != nil {
				t.Fatal(err)
			}
			writeFile(t, sub, "photo.jpg", content)
		}
	}
	store := newStore(t)
	scanInto(t, store, dir)
	groups, err := store.Duplicates()
	if err != nil {
		t.Fatal(err)
	}
	if len(groups) != 10 {
		t.Fatalf("groups = %d, want 10", len(groups))
	}
	lastGroup := groups[len(groups)-1]
	lastFile := lastGroup.Files[len(lastGroup.Files)-1].Path

	app := startApp(t, store, dir, nil)
	list := app.Find("id:duplicate-list")
	if list == nil {
		t.Fatalf("duplicate list missing\n%s", app.Dump())
	}
	if list.Content.Height <= list.Frame.Size.Height {
		t.Fatalf("list content %v fits viewport %v; want a scrollable list", list.Content.Height, list.Frame.Size.Height)
	}
	if list.Offset.Y > 0 {
		app.Scroll("id:duplicate-list", -list.Offset.Y)
	}
	app.WaitUntil(2*time.Second, func() bool { return app.Find("id:duplicate-list").Offset.Y == 0 })

	app.Scroll("id:duplicate-list", 100000)
	list = app.Find("id:duplicate-list")
	if list.Offset.Y <= 0 {
		t.Errorf("list did not scroll down: offset %v", list.Offset.Y)
	}

	if !app.Exists("id:keep:" + lastFile) {
		t.Fatalf("last file row missing after scroll\n%s", app.Dump())
	}
	row := app.Find("id:keep:" + lastFile)
	_, listTop, _, listHeight := surfaceFrame(list)
	listBottom := listTop + listHeight
	_, rowTop, _, rowHeight := surfaceFrame(row)
	if rowTop < listTop || rowTop+rowHeight > listBottom {
		t.Errorf("last file row at %v..%v is outside list %v..%v", rowTop, rowTop+rowHeight, listTop, listBottom)
	}

	bottom := app.Find("id:bottom-bar")
	if bottom == nil {
		t.Fatalf("bottom bar missing\n%s", app.Dump())
	}
	_, bottomTop, _, bottomHeight := surfaceFrame(bottom)
	if bottomTop+bottomHeight > 720 {
		t.Errorf("bottom bar at %v..%v is outside the window", bottomTop, bottomTop+bottomHeight)
	}

	beforeUp := list.Offset.Y
	app.Scroll("id:duplicate-list", -100000)
	list = app.Find("id:duplicate-list")
	if list.Offset.Y >= beforeUp {
		t.Errorf("list offset = %v after scrolling up, want less than %v", list.Offset.Y, beforeUp)
	}
}

func TestNativeScreenshot(t *testing.T) {
	home := t.TempDir()
	downloads := filepath.Join(home, "Downloads")
	archive := filepath.Join(downloads, "archive")
	versions := filepath.Join(downloads, "versions")
	for _, dir := range []string{archive, versions} {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	team := writeFile(t, downloads, "team-photo.jpg", strings.Repeat("photo", 4096))
	teamCopy := writeFile(t, versions, "team-photo copy.jpg", strings.Repeat("photo", 4096))
	report := writeFile(t, downloads, "annual-report.pdf", strings.Repeat("report", 4096))
	reportCopy := writeFile(t, archive, "annual-report copy.pdf", strings.Repeat("report", 4096))
	setMtime(t, team, 100)
	setMtime(t, report, 100)
	setMtime(t, teamCopy, 200)
	setMtime(t, reportCopy, 200)
	store := newStore(t)
	scanInto(t, store, downloads)

	cfg := Config{
		Store:      store,
		DBPath:     filepath.Join(t.TempDir(), "index.db"),
		Home:       home,
		DefaultDir: downloads,
		Workers:    2,
	}
	app := nsgotest.Start(t, Root(cfg), nsgotest.Size(1080, 720))
	app.Screenshot("onley-main")
}
