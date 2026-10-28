package main

import (
	"context"
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/oneo-me/nsgo/core"

	"onley/internal/appmodel"
	"onley/internal/cleaner"
	"onley/internal/db"
	"onley/internal/format"
	"onley/internal/scanner"
)

// Config is what the app needs to run and what tests replace.
type Config struct {
	Store      *db.DB
	DBPath     string
	OpenErr    error
	Workers    int
	Home       string
	DefaultDir string
	Scan       func(context.Context, string, *db.DB, int) <-chan scanner.Progress
}

type scanProgress struct {
	Done      int
	Skipped   int
	Errors    int
	Hashed    int64
	Speed     float64
	Elapsed   time.Duration
	LastError string
}

type state struct {
	cfg        Config
	cx         *core.Scope
	home       string
	defaultDir string

	groups   *core.Signal[[]db.DuplicateGroup]
	total    *core.Signal[int]
	summary  *core.Signal[appmodel.Summary]
	scanDirs *core.Signal[[]string]
	scanPath *core.Signal[string]
	keep     *core.Signal[map[string]string]
	scanning *core.Signal[bool]
	busy     *core.Signal[bool]
	progress *core.Signal[scanProgress]
	message  *core.Signal[string]
	note     *core.Signal[string]
	confirm  *core.Signal[bool]
	dragging *core.Signal[bool]
	speedMax *core.Signal[float64]
	needle   *core.AnimatedValue
	pulse    *core.AnimatedValue
	pulseRun *core.AnimationRun
	cancel   context.CancelFunc
}

func newState(cx *core.Scope, cfg Config) *state {
	home := cfg.Home
	if home == "" {
		if found, err := os.UserHomeDir(); err == nil {
			home = found
		}
	}
	if home == "" {
		home = "."
	}
	defaultDir := cfg.DefaultDir
	if defaultDir == "" {
		defaultDir = filepath.Join(home, "Downloads")
		if info, err := os.Stat(defaultDir); err != nil || !info.IsDir() {
			defaultDir = home
		}
	}
	if cfg.Workers < 1 {
		cfg.Workers = 1
	}
	if cfg.Scan == nil {
		cfg.Scan = scanner.ScanContext
	}

	s := &state{
		cfg:        cfg,
		cx:         cx,
		home:       home,
		defaultDir: defaultDir,
		groups:     core.NewSignalFunc(cx, []db.DuplicateGroup{}, nil),
		total:      core.NewSignal(cx, 0),
		summary:    core.NewSignal(cx, appmodel.Summary{}),
		scanDirs:   core.NewSignalFunc(cx, []string{defaultDir}, nil),
		scanPath:   core.NewSignal(cx, ""),
		keep:       core.NewSignalFunc(cx, map[string]string{}, nil),
		scanning:   core.NewSignal(cx, false),
		busy:       core.NewSignal(cx, false),
		progress:   core.NewSignal(cx, scanProgress{}),
		message:    core.NewSignal(cx, ""),
		note:       core.NewSignal(cx, ""),
		confirm:    core.NewSignal(cx, false),
		dragging:   core.NewSignal(cx, false),
		speedMax:   core.NewSignal(cx, 40.0),
		needle:     core.NewAnimatedValue(cx, 0),
		pulse:      core.NewAnimatedValue(cx, 0),
	}

	switch {
	case cfg.OpenErr != nil:
		s.message.Set("Cannot open the index: " + cfg.OpenErr.Error())
	case cfg.Store == nil:
		s.message.Set("The index is not available.")
	default:
		groups, total, err := s.load()
		if err != nil {
			s.message.Set("Cannot read the index: " + err.Error())
		} else {
			s.apply(groups, total)
		}
	}
	return s
}

func (s *state) load() ([]db.DuplicateGroup, int, error) {
	groups, err := s.cfg.Store.Duplicates()
	if err != nil {
		return nil, 0, err
	}
	total, _, err := s.cfg.Store.Stats()
	if err != nil {
		return nil, 0, err
	}
	return groups, total, nil
}

func (s *state) apply(groups []db.DuplicateGroup, total int) {
	s.groups.Set(groups)
	s.total.Set(total)
	s.summary.Set(appmodel.Summarize(groups))
	s.pruneKeep(groups)
}

// pruneKeep drops choices for groups or files that are no longer in the index.
func (s *state) pruneKeep(groups []db.DuplicateGroup) {
	keep := s.keep.Get()
	if len(keep) == 0 {
		return
	}
	next := make(map[string]string, len(keep))
	for _, group := range groups {
		path, ok := keep[group.MD5]
		if !ok {
			continue
		}
		for _, file := range group.Files {
			if file.Path == path {
				next[group.MD5] = path
				break
			}
		}
	}
	if !maps.Equal(keep, next) {
		s.keep.Set(next)
	}
}

func (s *state) refreshAsync(after func()) {
	if s.cfg.Store == nil {
		return
	}
	go func() {
		groups, total, err := s.load()
		s.cx.Post(func() {
			if err != nil {
				s.message.Set("Cannot read the index: " + err.Error())
				return
			}
			s.apply(groups, total)
			if after != nil {
				after()
			}
		})
	}()
}

func (s *state) plan() appmodel.Plan {
	return appmodel.BuildPlan(s.groups.Get(), s.keep.Get())
}

func (s *state) setKeep(md5, path string) {
	next := maps.Clone(s.keep.Get())
	if next == nil {
		next = make(map[string]string)
	}
	next[md5] = path
	s.keep.Set(next)
}

func (s *state) keepFirst() {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	next := make(map[string]string)
	for _, group := range s.groups.Get() {
		if len(group.Files) > 0 {
			next[group.MD5] = group.Files[0].Path
		}
	}
	s.keep.Set(next)
}

func (s *state) clearKeep() {
	s.keep.Set(map[string]string{})
}

// autoKeep selects one file in every group and opens the deletion review. The
// selection is still a plan: nothing is deleted until the dialog is confirmed.
func (s *state) autoKeep() {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	s.keepFirst()
	s.openConfirm()
}

func (s *state) refresh() {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	s.refreshAsync(func() { s.note.Set("Index refreshed.") })
}

// addFolder normalizes and adds a folder, merging it with the selected tree.
func (s *state) addFolder(raw string) bool {
	if s.scanning.Get() || s.busy.Get() {
		return false
	}
	path := s.expand(raw)
	if path == "" {
		return false
	}
	if !filepath.IsAbs(path) {
		path = filepath.Join(s.home, path)
	}
	abs, err := filepath.Abs(path)
	if err != nil {
		s.message.Set("Cannot resolve the scan path: " + err.Error())
		return false
	}
	info, err := os.Stat(abs)
	if err != nil {
		s.message.Set("Cannot scan that folder: " + err.Error())
		return false
	}
	if !info.IsDir() {
		s.message.Set("The scan path is not a folder: " + abs)
		return false
	}
	roots := append([]string(nil), s.scanDirs.Get()...)
	roots = append(roots, abs)
	s.scanDirs.Set(appmodel.MergeRoots(roots))
	s.scanPath.Set("")
	s.message.Set("")
	return true
}

func (s *state) removeFolder(path string) {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	roots := s.scanDirs.Get()
	next := make([]string, 0, len(roots))
	for _, root := range roots {
		if root != path {
			next = append(next, root)
		}
	}
	s.scanDirs.Set(next)
}

func (s *state) scanRootsLabel() string {
	roots := s.scanDirs.Get()
	switch len(roots) {
	case 0:
		return "no folders"
	case 1:
		return displayPath(roots[0], s.home)
	default:
		return fmt.Sprintf("%d folders", len(roots))
	}
}

func (s *state) startScan() {
	if s.cfg.Store == nil || s.scanning.Get() || s.busy.Get() {
		return
	}
	if draft := strings.TrimSpace(s.scanPath.Get()); draft != "" {
		if !s.addFolder(draft) {
			return
		}
	}
	roots := appmodel.MergeRoots(s.scanDirs.Get())
	if len(roots) == 0 {
		s.message.Set("Add a folder to scan.")
		return
	}
	for _, root := range roots {
		info, err := os.Stat(root)
		if err != nil {
			s.message.Set("Cannot scan that folder: " + err.Error())
			return
		}
		if !info.IsDir() {
			s.message.Set("The scan path is not a folder: " + root)
			return
		}
	}
	s.scanDirs.Set(roots)

	ctx, cancel := context.WithCancel(context.Background())
	s.cancel = cancel
	s.keep.Set(map[string]string{})
	s.confirm.Set(false)
	s.progress.Set(scanProgress{})
	s.speedMax.Set(40)
	s.needle.Set(0)
	s.pulse.Set(0)
	if s.pulseRun != nil {
		s.pulseRun.Stop()
	}
	s.pulseRun = s.cx.StartAnimation(core.Loop(core.Sequence(
		core.Timing(s.pulse, 1, core.TimingConfig{Duration: 900 * time.Millisecond}),
		core.Timing(s.pulse, 0, core.TimingConfig{Duration: 900 * time.Millisecond}),
	), 0), nil)
	s.scanning.Set(true)
	s.message.Set("")
	s.note.Set("")
	go s.runScan(ctx, roots)
}

func (s *state) runScan(ctx context.Context, roots []string) {
	var p scanProgress
	var samples []appmodel.SpeedSample
	var hashed int64
	var done int
	started := time.Now()
	lastPost := time.Now()

	post := func(now time.Time) {
		p.Done = done
		p.Hashed = hashed
		p.Speed = appmodel.Rate(samples, now, time.Second)
		p.Elapsed = now.Sub(started)
		if now.Sub(lastPost) < 100*time.Millisecond {
			return
		}
		lastPost = now
		snapshot := p
		s.cx.Post(func() {
			s.progress.Set(snapshot)
			s.animateSpeed(snapshot.Speed)
		})
	}

	for _, root := range roots {
		for ev := range s.cfg.Scan(ctx, root, s.cfg.Store, s.cfg.Workers) {
			now := time.Now()
			if ev.Bytes > 0 {
				hashed += ev.Bytes
				samples = append(samples, appmodel.SpeedSample{At: now, Bytes: ev.Bytes})
				cutoff := now.Add(-2 * time.Second)
				for len(samples) > 0 && samples[0].At.Before(cutoff) {
					samples = samples[1:]
				}
			}
			if ev.Err != nil {
				p.Errors++
				p.LastError = ev.Err.Error()
			} else if ev.Done {
				done++
				if ev.Skipped {
					p.Skipped++
				}
			}
			post(now)
		}
		if ctx.Err() != nil {
			break
		}
	}

	p.Done = done
	p.Hashed = hashed
	p.Speed = appmodel.Rate(samples, time.Now(), time.Second)
	p.Elapsed = time.Since(started)

	cancelled := ctx.Err() != nil
	s.cx.Post(func() {
		if s.pulseRun != nil {
			s.pulseRun.Stop()
			s.pulseRun = nil
		}
		s.pulse.Set(0)
		s.needle.Set(0)
		s.scanning.Set(false)
		s.cancel = nil
		s.progress.Set(p)
		if cancelled {
			s.note.Set("Scan canceled. The partial index is kept.")
		} else {
			s.note.Set(fmt.Sprintf("Scan finished: %d files processed, %d unchanged skipped.", p.Done, p.Skipped))
		}
		if p.Errors > 0 {
			msg := fmt.Sprintf("%d file(s) could not be read", p.Errors)
			if p.LastError != "" {
				msg += ": " + p.LastError
			}
			s.message.Set(msg)
		}
		s.refreshAsync(nil)
	})
}

// animateSpeed moves the gauge needle toward bytesPerSecond. The ceiling grows
// before the needle to keep it inside the dial.
func (s *state) animateSpeed(bytesPerSecond float64) {
	mb := bytesPerSecond / (1024 * 1024)
	ceiling := appmodel.GaugeMax(s.speedMax.Get(), mb)
	if ceiling != s.speedMax.Get() {
		s.speedMax.Set(ceiling)
	}
	percent := 0.0
	if ceiling > 0 {
		percent = mb / ceiling * 100
	}
	percent = max(0, min(100, percent))
	s.cx.StartAnimation(core.Spring(s.needle, percent, core.SpringConfig{Stiffness: 80, Damping: 14}), nil)
}

func (s *state) cancelScan() {
	if s.cancel != nil {
		s.cancel()
		s.note.Set("Canceling the scan…")
	}
}

func (s *state) openConfirm() {
	if s.cfg.Store == nil || s.scanning.Get() || s.busy.Get() {
		return
	}
	if len(s.plan().Targets) == 0 {
		return
	}
	s.confirm.Set(true)
}

func (s *state) confirmDelete() {
	plan := s.plan()
	if s.cfg.Store == nil || len(plan.Targets) == 0 {
		s.confirm.Set(false)
		return
	}
	s.confirm.Set(false)
	s.busy.Set(true)
	s.message.Set("")
	s.note.Set("")

	targets := make([]cleaner.Target, len(plan.Targets))
	for i, file := range plan.Targets {
		targets[i] = cleaner.Target{Path: file.Path, Size: file.Size, Mtime: file.Mtime}
	}
	keeps := make([]cleaner.Target, len(plan.Keeps))
	for i, file := range plan.Keeps {
		keeps[i] = cleaner.Target{Path: file.Path, Size: file.Size, Mtime: file.Mtime}
	}

	go func() {
		for _, keep := range keeps {
			if err := cleaner.Verify(keep); err != nil {
				s.cx.Post(func() {
					s.busy.Set(false)
					s.note.Set("Delete canceled: the copy to keep is no longer available.")
					s.message.Set("Cannot keep " + keep.Path + ": " + err.Error() + ". Review the files again.")
					s.refreshAsync(nil)
				})
				return
			}
		}
		results := cleaner.Delete(s.cfg.Store, targets, true)
		var deleted, failed, missing int
		var freed int64
		firstErr := ""
		for i, result := range results {
			if result.Deleted {
				deleted++
				freed += targets[i].Size
			}
			if result.Missing {
				missing++
			}
			if result.Err != nil {
				failed++
				if firstErr == "" {
					firstErr = result.Path + ": " + result.Err.Error()
				}
			}
			if result.IndexErr != nil {
				failed++
				if firstErr == "" {
					firstErr = result.Path + ": index cleanup failed: " + result.IndexErr.Error()
				}
			}
		}
		s.cx.Post(func() {
			s.busy.Set(false)
			note := fmt.Sprintf("Deleted %s, freed %s.", fileCount(deleted), format.Bytes(freed))
			if missing > 0 {
				note += fmt.Sprintf(" %d was already gone.", missing)
			}
			if failed > 0 {
				note += fmt.Sprintf(" %d failed.", failed)
				s.message.Set(firstErr)
			}
			s.note.Set(note)
			s.refreshAsync(nil)
		})
	}()
}

func (s *state) dropFiles(uris []string) {
	s.dragging.Set(false)
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	added := false
	for _, uri := range uris {
		path, ok := filePath(uri)
		if !ok {
			continue
		}
		if info, err := os.Stat(path); err != nil || !info.IsDir() {
			continue
		}
		if s.addFolder(path) {
			added = true
		}
	}
	if !added {
		s.message.Set("Drop a folder to scan it.")
		return
	}
	s.startScan()
}

func (s *state) expand(path string) string {
	path = strings.TrimSpace(path)
	if path == "~" {
		return s.home
	}
	if strings.HasPrefix(path, "~/") || strings.HasPrefix(path, `~\`) {
		return filepath.Join(s.home, path[2:])
	}
	return path
}
