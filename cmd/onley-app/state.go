package main

import (
	"context"
	"errors"
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
	"onley/internal/i18n"
	"onley/internal/scanner"
	"onley/internal/selfpath"
)

// Config is what the app needs to run and what tests replace.
type Config struct {
	Store      *db.DB
	DBPath     string
	OpenErr    error
	Workers    int
	Home       string
	DefaultDir string
	SelfPath   string
	Scan       func(context.Context, string, *db.DB, int) <-chan scanner.Progress
}

type scanProgress struct {
	Done             int
	Skipped          int
	Errors           int
	PermissionErrors int
	Hashed           int64
	Speed            float64
	Elapsed          time.Duration
	LastError        string
}

type state struct {
	cfg        Config
	cx         *core.Scope
	home       string
	defaultDir string
	selfPath   string

	groups       *core.Signal[[]db.DuplicateGroup]
	scoped       *core.Signal[[]db.DuplicateGroup]
	visible      *core.Signal[[]db.DuplicateGroup]
	summary      *core.Signal[appmodel.Summary]
	indexed      *core.Signal[int]
	scanDirs     *core.Signal[[]string]
	scanPath     *core.Signal[string]
	addingFolder *core.Signal[bool]
	keep         *core.Signal[map[string]string]
	keepPolicy   *core.Signal[appmodel.KeepPolicy]
	category     *core.Signal[category]
	query        *core.Signal[string]
	activeRoot   *core.Signal[string]
	scanning     *core.Signal[bool]
	busy         *core.Signal[bool]
	progress     *core.Signal[scanProgress]
	message      *core.Signal[string]
	note         *core.Signal[string]
	noteSuccess  *core.Signal[bool]
	confirm      *core.Signal[bool]
	lang         *core.Signal[i18n.Lang]
	speedMax     *core.Signal[float64]
	needle       *core.AnimatedValue
	pulse        *core.AnimatedValue
	pulseRun     *core.AnimationRun
	cancel       context.CancelFunc
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
	lang := i18n.Detect(cx.Env().Locale())
	if saved, ok := cx.Storage().GetItem("language"); ok {
		if savedLang, ok := i18n.Parse(saved); ok {
			lang = savedLang
		}
	}
	policy := appmodel.KeepOldest
	if saved, ok := cx.Storage().GetItem("keep_policy"); ok {
		if savedPolicy, ok := appmodel.ParseKeepPolicy(saved); ok {
			policy = savedPolicy
		}
	}
	selfPath := cfg.SelfPath
	if selfPath == "" {
		selfPath = selfpath.Resolve()
	}

	s := &state{
		cfg:          cfg,
		cx:           cx,
		home:         home,
		defaultDir:   defaultDir,
		selfPath:     selfPath,
		groups:       core.NewSignalFunc(cx, []db.DuplicateGroup{}, nil),
		scoped:       core.NewSignalFunc(cx, []db.DuplicateGroup{}, nil),
		visible:      core.NewSignalFunc(cx, []db.DuplicateGroup{}, nil),
		summary:      core.NewSignal(cx, appmodel.Summary{}),
		indexed:      core.NewSignal(cx, 0),
		scanDirs:     core.NewSignalFunc(cx, []string{defaultDir}, nil),
		scanPath:     core.NewSignal(cx, ""),
		addingFolder: core.NewSignal(cx, false),
		keep:         core.NewSignalFunc(cx, map[string]string{}, nil),
		keepPolicy:   core.NewSignal(cx, policy),
		category:     core.NewSignal(cx, categoryAll),
		query:        core.NewSignal(cx, ""),
		activeRoot:   core.NewSignal(cx, defaultDir),
		scanning:     core.NewSignal(cx, false),
		busy:         core.NewSignal(cx, false),
		progress:     core.NewSignal(cx, scanProgress{}),
		message:      core.NewSignal(cx, ""),
		note:         core.NewSignal(cx, ""),
		noteSuccess:  core.NewSignal(cx, false),
		confirm:      core.NewSignal(cx, false),
		lang:         core.NewSignal(cx, lang),
		speedMax:     core.NewSignal(cx, 40.0),
		needle:       core.NewAnimatedValue(cx, 0),
		pulse:        core.NewAnimatedValue(cx, 0),
	}

	switch {
	case cfg.OpenErr != nil:
		s.showIndexError(cfg.OpenErr, true)
	case cfg.Store == nil:
		s.message.Set(s.t(i18n.KeyIndexUnavailable))
	default:
		groups, total, err := s.load()
		if err != nil {
			s.showIndexError(err, false)
		} else {
			s.apply(groups, total)
		}
	}
	return s
}

// showIndexError tells the user when the index cannot be opened or read. A
// permission failure gets its own hint, because the raw error is not useful.
func (s *state) showIndexError(err error, opening bool) {
	if isPermissionError(err) {
		s.message.Set(s.t(i18n.KeyPermissionIndex))
		return
	}
	if opening {
		s.message.Set(s.tf(i18n.KeyCannotOpenIndex, err))
		return
	}
	s.message.Set(s.tf(i18n.KeyCannotReadIndex, err))
}

func (s *state) t(key i18n.Key) string {
	return i18n.T(s.lang.Get(), key)
}

func (s *state) tf(key i18n.Key, args ...any) string {
	return i18n.F(s.lang.Get(), key, args...)
}

func (s *state) fileCount(n int) string {
	return i18n.FileCount(s.lang.Get(), n)
}

func (s *state) setLanguage(lang i18n.Lang) {
	s.lang.Set(lang)
	_ = s.cx.Storage().SetItem("language", string(lang))
	s.setNote("", false)
	s.message.Set("")
}

func (s *state) toggleLanguage() {
	s.setLanguage(i18n.Next(s.lang.Get()))
}

func (s *state) setNote(text string, success bool) {
	s.note.Set(text)
	s.noteSuccess.Set(success)
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
	s.indexed.Set(total)
	s.refreshVisible()
	s.applyKeepPolicy(s.keepPolicy.Get())
}

// refreshVisible applies the category, folder and search filters.
func (s *state) refreshVisible() {
	groups := s.groups.Get()
	roots := s.scanDirs.Get()
	scoped := make([]db.DuplicateGroup, 0, len(groups))
	for _, group := range groups {
		if len(group.Files) > 0 && groupInRoots(group, roots) {
			scoped = append(scoped, group)
		}
	}

	s.scoped.Set(scoped)
	s.summary.Set(appmodel.Summarize(scoped))

	cat := s.category.Get()
	query := strings.ToLower(strings.TrimSpace(s.query.Get()))
	out := make([]db.DuplicateGroup, 0, len(scoped))
	for _, group := range scoped {
		if cat != categoryAll && cat != categorySearch && fileCategory(group.Files[0].Name) != cat {
			continue
		}
		if cat == categorySearch && query != "" && !groupMatches(group, query) {
			continue
		}
		out = append(out, group)
	}
	s.visible.Set(out)
}

func (s *state) setCategory(cat category) {
	s.category.Set(cat)
	s.refreshVisible()
}

func (s *state) setQuery(query string) {
	s.query.Set(query)
}

func (s *state) applySearch() {
	s.refreshVisible()
}

func (s *state) setActiveRoot(root string) {
	s.activeRoot.Set(root)
	s.refreshVisible()
}

func (s *state) refreshAsync(after func()) {
	if s.cfg.Store == nil {
		return
	}
	go func() {
		groups, total, err := s.load()
		s.cx.Post(func() {
			if err != nil {
				s.showIndexError(err, false)
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
	return appmodel.BuildPlan(s.scoped.Get(), s.keep.Get(), s.selfPath)
}

func (s *state) setKeep(md5, path string) {
	for _, group := range s.groups.Get() {
		if group.MD5 != md5 {
			continue
		}
		if self, found := appmodel.ProtectedFile(group, s.selfPath); found && !selfpath.Same(path, self.Path) {
			return
		}
		break
	}
	next := maps.Clone(s.keep.Get())
	if next == nil {
		next = make(map[string]string)
	}
	next[md5] = path
	s.keep.Set(next)
}

// applyKeepPolicy stores policy and selects the keep copy in every group.
// Nothing is deleted until the review dialog is confirmed.
func (s *state) applyKeepPolicy(policy appmodel.KeepPolicy) {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	if s.keepPolicy.Get() != policy {
		s.keepPolicy.Set(policy)
	}
	_ = s.cx.Storage().SetItem("keep_policy", policy.String())
	next := make(map[string]string)
	for _, group := range s.groups.Get() {
		if self, found := appmodel.ProtectedFile(group, s.selfPath); found {
			next[group.MD5] = self.Path
			continue
		}
		if len(group.Files) > 0 {
			next[group.MD5] = appmodel.ChooseKeep(group, policy).Path
		}
	}
	s.keep.Set(next)
}

func (s *state) openAddFolder() {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	s.scanPath.Set("")
	s.message.Set("")
	s.addingFolder.Set(true)
}

func (s *state) closeAddFolder() {
	s.addingFolder.Set(false)
	s.scanPath.Set("")
	s.message.Set("")
}

func (s *state) confirmAddFolder() {
	if strings.TrimSpace(s.scanPath.Get()) == "" {
		return
	}
	if s.addFolder(s.scanPath.Get()) {
		s.addingFolder.Set(false)
	}
}

func (s *state) refresh() {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	s.refreshAsync(func() { s.setNote(s.t(i18n.KeyIndexRefreshed), false) })
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
		s.message.Set(s.tf(i18n.KeyCannotResolvePath, err))
		return false
	}
	info, err := os.Stat(abs)
	if err != nil {
		if isPermissionError(err) {
			s.message.Set(s.tf(i18n.KeyPermissionFolder, abs))
		} else {
			s.message.Set(s.tf(i18n.KeyCannotScanFolder, err))
		}
		return false
	}
	if !info.IsDir() {
		s.message.Set(s.tf(i18n.KeyNotAFolder, abs))
		return false
	}
	roots := append([]string(nil), s.scanDirs.Get()...)
	roots = append(roots, abs)
	s.scanDirs.Set(appmodel.MergeRoots(roots))
	s.refreshVisible()
	s.scanPath.Set("")
	s.message.Set("")
	return true
}

// removeFolder drops a folder from the scan roots.
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
	if s.activeRoot.Get() == path {
		s.setActiveRoot("")
	}
	s.refreshVisible()
}

// clearKeep drops every keep choice, so the delete plan becomes empty.
func (s *state) clearKeep() {
	if s.scanning.Get() || s.busy.Get() {
		return
	}
	s.keep.Set(map[string]string{})
}

func (s *state) scanRootsLabel() string {
	roots := s.scanDirs.Get()
	switch len(roots) {
	case 0:
		return s.t(i18n.KeyNoFolders)
	case 1:
		return displayPath(roots[0], s.home)
	default:
		return i18n.FolderCount(s.lang.Get(), len(roots))
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
		s.message.Set(s.t(i18n.KeyAddFolderFirst))
		return
	}
	for _, root := range roots {
		info, err := os.Stat(root)
		if err != nil {
			if isPermissionError(err) {
				s.message.Set(s.tf(i18n.KeyPermissionFolder, root))
			} else {
				s.message.Set(s.tf(i18n.KeyCannotScanFolder, err))
			}
			return
		}
		if !info.IsDir() {
			s.message.Set(s.tf(i18n.KeyNotAFolder, root))
			return
		}
	}
	s.scanDirs.Set(roots)
	s.refreshVisible()

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
		s.pulseRun = nil
	}
	if s.cx.Env().ReduceMotion() {
		// Keep the gauge static when the system asks for less motion.
		s.pulse.Set(1)
	} else {
		s.pulseRun = s.cx.StartAnimation(core.Loop(core.Sequence(
			core.Timing(s.pulse, 1, core.TimingConfig{Duration: 900 * time.Millisecond}),
			core.Timing(s.pulse, 0, core.TimingConfig{Duration: 900 * time.Millisecond}),
		), 0), nil)
	}
	s.scanning.Set(true)
	s.message.Set("")
	s.setNote("", false)
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
				if isPermissionError(ev.Err) {
					p.PermissionErrors++
				}
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
			s.setNote(s.t(i18n.KeyScanCanceled), false)
		} else {
			s.setNote(s.tf(i18n.KeyScanFinished, p.Done, p.Skipped), false)
		}
		if p.PermissionErrors > 0 {
			msg := s.tf(i18n.KeyPermissionReadErrors, p.PermissionErrors)
			if other := p.Errors - p.PermissionErrors; other > 0 {
				msg += " " + s.tf(i18n.KeyReadErrors, other)
			}
			s.message.Set(msg)
		} else if p.Errors > 0 {
			msg := s.tf(i18n.KeyReadErrors, p.Errors)
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
	if s.cx.Env().ReduceMotion() {
		s.needle.Set(percent)
		return
	}
	s.cx.StartAnimation(core.Spring(s.needle, percent, core.SpringConfig{Stiffness: 80, Damping: 14}), nil)
}

func (s *state) cancelScan() {
	if s.cancel != nil {
		s.cancel()
		s.setNote(s.t(i18n.KeyCanceling), false)
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
	s.setNote("", false)

	targets := make([]cleaner.Target, len(plan.Targets))
	for i, file := range plan.Targets {
		targets[i] = cleaner.Target{Path: file.Path, Size: file.Size, Mtime: file.Mtime}
	}
	keeps := make([]cleaner.Target, len(plan.Keeps))
	for i, file := range plan.Keeps {
		keeps[i] = cleaner.Target{Path: file.Path, Size: file.Size, Mtime: file.Mtime}
	}

	// Resolve localized text before the worker starts: signals are not safe to
	// read from the delete goroutine.
	selfDeleteMsg := s.t(i18n.KeyCannotDeleteSelf)
	permissionDeleteFmt := s.t(i18n.KeyPermissionDelete)
	indexCleanupMsg := s.t(i18n.KeyIndexCleanupFailed)
	permissionIndexMsg := s.t(i18n.KeyPermissionIndex)

	go func() {
		for _, keep := range keeps {
			if err := cleaner.Verify(keep); err != nil {
				s.cx.Post(func() {
					s.busy.Set(false)
					s.setNote(s.t(i18n.KeyDeleteCanceledKeep), false)
					if isPermissionError(err) {
						s.message.Set(s.tf(i18n.KeyPermissionKeep, keep.Path))
					} else {
						s.message.Set(s.tf(i18n.KeyCannotKeep, keep.Path, err))
					}
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
					switch {
					case errors.Is(result.Err, cleaner.ErrSelfDelete):
						firstErr = result.Path + ": " + selfDeleteMsg
					case isPermissionError(result.Err):
						firstErr = fmt.Sprintf(permissionDeleteFmt, result.Path)
					default:
						firstErr = result.Path + ": " + result.Err.Error()
					}
				}
			}
			if result.IndexErr != nil {
				failed++
				if firstErr == "" {
					if isPermissionError(result.IndexErr) {
						firstErr = result.Path + ": " + permissionIndexMsg
					} else {
						firstErr = result.Path + ": " + indexCleanupMsg + ": " + result.IndexErr.Error()
					}
				}
			}
		}
		s.cx.Post(func() {
			s.busy.Set(false)
			note := s.tf(i18n.KeyDeleted, s.fileCount(deleted), format.Bytes(freed))
			if missing > 0 {
				note += " " + s.tf(i18n.KeyAlreadyGone, missing)
			}
			if failed > 0 {
				note += " " + s.tf(i18n.KeyFailed, failed)
				s.message.Set(firstErr)
			}
			s.setNote(note, deleted > 0)
			s.refreshAsync(nil)
		})
	}()
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
