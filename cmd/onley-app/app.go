package main

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/oneo-me/nsgo/base"
	"github.com/oneo-me/nsgo/core"
	"github.com/oneo-me/nsgo/desktop"
	"github.com/oneo-me/nsgo/icons/lucide"

	"onley/internal/appmodel"
	"onley/internal/db"
	"onley/internal/format"
	"onley/internal/i18n"
)

// Root builds the app. Config is read once; tests pass a temporary store.
func Root(cfg Config) func(*core.Scope) core.Node {
	return func(cx *core.Scope) core.Node {
		cx.SetTheme(midnightTheme())
		cx.SetSubtreeAppearance(core.AppearanceDark)
		// The platform's title bar follows the app appearance, which needs
		// the mounted tree. The subtree appearance above only governs the
		// app's own views.
		cx.Effect(func() { cx.SetAppearance(core.AppearanceDark) })
		s := newState(cx, cfg)
		if desktop.IsAvailable(cx) {
			desktop.SetTitle(cx, "Onley")
			cx.Effect(func() {
				desktop.SetApplicationMenu(cx, s.menu())
				if cx.Env().OS() != core.OSMacOS {
					// Native menu bars are system-drawn. Hide the light bar
					// by default; Alt still reveals it with its shortcuts.
					desktop.SetMenuBar(cx, desktop.MenuBarAutoHide)
				}
			})
		}
		return core.Dynamic(func(cx *core.Scope) core.Node {
			_ = cx.Dark()
			_ = s.lang.Get()
			_ = s.keepPolicy.Get()
			return s.view(cx)
		})
	}
}

func (s *state) menu() []desktop.MenuItem {
	return []desktop.MenuItem{
		{Label: "Onley", Items: []desktop.MenuItem{
			{Role: desktop.MenuAbout},
			{Separator: true},
			{Role: desktop.MenuHide},
			{Role: desktop.MenuHideOthers},
			{Role: desktop.MenuShowAll},
			{Separator: true},
			{Role: desktop.MenuQuit},
		}},
		{Label: s.t(i18n.KeyFileMenu), Items: []desktop.MenuItem{
			{Label: s.t(i18n.KeyRefreshIndex), Shortcut: "mod+r", OnSelect: s.refresh},
			{Separator: true},
			{Role: desktop.MenuClose},
		}},
		{Label: s.t(i18n.KeyEditMenu), Items: []desktop.MenuItem{
			{Role: desktop.MenuUndo},
			{Role: desktop.MenuRedo},
			{Separator: true},
			{Role: desktop.MenuCut},
			{Role: desktop.MenuCopy},
			{Role: desktop.MenuPaste},
			{Role: desktop.MenuSelectAll},
		}},
	}
}

func (s *state) view(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{
		Style: core.Style{
			FlexGrow: 1, Width: core.Pct(100), Height: core.Pct(100),
			MinHeight: core.Pt(0), Background: c.background, Direction: core.Column,
		},
		Children: []core.Node{
			s.header(cx),
			s.banner(cx),
			core.View{Style: core.Style{
				FlexGrow: 1, FlexBasis: core.Pt(0), MinHeight: core.Pt(0),
				Direction: core.Row, Gap: 16, Padding: core.All(16),
			}, Children: []core.Node{
				s.sidebar(cx),
				s.results(cx),
			}},
			s.confirmDialog(cx),
		},
	}
}

func (s *state) header(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{
		Style: core.Style{
			FlexShrink: 0, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 12,
			Padding: core.Symmetric(12, 18), Background: c.surface,
			Borders: core.Edges{Bottom: 1}, BorderColor: c.border,
		},
		Children: []core.Node{
			core.View{Style: core.Style{
				Width: core.Pt(34), Height: core.Pt(34), CornerRadius: 10,
				Background: c.accent, AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			}, Children: []core.Node{line("O", 18, core.FontWeightBold, c.accentText)}},
			core.View{Style: core.Style{Gap: 1}, Children: []core.Node{
				line("Onley", 18, core.FontWeightBold, c.text),
				line(s.t(i18n.KeySubtitle), 11, core.FontWeightNormal, c.muted),
			}},
			spacer(),
			s.languageButton(cx),
			badge(c, s.t(i18n.KeyPreview), c.accentSoft, c.accent),
			line(s.t(i18n.KeyLocalOnly), 11, core.FontWeightNormal, c.muted),
		},
	}
}

func (s *state) languageButton(cx *core.Scope) core.Node {
	label := "中文"
	if s.lang.Get() == i18n.Chinese {
		label = "EN"
	}
	return appButton(cx, label, "language-toggle", buttonSecondary, nil, s.toggleLanguage)
}

func (s *state) banner(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.Show{When: func() bool { return s.message.Get() != "" }, Then: func(cx *core.Scope) core.Node {
		return core.View{
			Style: core.Style{
				FlexShrink: 0, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 10,
				Padding: core.Symmetric(8, 18), Background: c.dangerSoft,
				Borders: core.Edges{Bottom: 1}, BorderColor: c.danger.Faded(0.35),
			},
			Children: []core.Node{
				core.View{Style: core.Style{FlexGrow: 1, MinWidth: core.Pt(0)}, Children: []core.Node{
					core.Text{ContentFunc: func() string { return s.message.Get() }, MaxLines: 3, Selectable: core.SelectableOn,
						Style: core.Style{FontSize: 12, Color: c.text}},
				}},
				appButton(cx, s.t(i18n.KeyDismiss), "dismiss-error", buttonSecondary, nil, func() { s.message.Set("") }),
			},
		}
	}}
}

func (s *state) sidebar(cx *core.Scope) core.Node {
	return core.Scroll{
		Style: core.Style{Width: core.Pt(330), FlexShrink: 0},
		Children: []core.Node{core.View{Style: core.Style{Gap: 12, Padding: core.Edges{Bottom: 4}}, Children: []core.Node{
			s.statsCard(cx),
			s.scanCard(cx),
			s.actionsCard(cx),
			s.indexCard(cx),
			s.noteText(cx),
		}}},
	}
}

func (s *state) statCard(cx *core.Scope, title, testID string, value func() string) core.Node {
	c := colorsOf(cx)
	return core.View{
		Style: core.Style{
			FlexGrow: 1, MinWidth: core.Pt(0),
			Padding: core.Symmetric(4, 0), Gap: 2,
		},
		Children: []core.Node{
			core.Text{ContentFunc: value, TestID: testID, Style: core.Style{FontSize: 20, FontWeight: core.FontWeightBold, Color: c.text}},
			line(title, 11, core.FontWeightNormal, c.muted),
		},
	}
}

func (s *state) statsCard(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{Style: cardStyle(c), Children: []core.Node{
		line(s.t(i18n.KeyIndex), 13, core.FontWeightSemibold, c.text),
		core.View{Style: core.Style{Direction: core.Row, Gap: 8}, Children: []core.Node{
			s.statCard(cx, s.t(i18n.KeyIndexedFiles), "stat-total", func() string { return s.fileCount(s.total.Get()) }),
			s.statCard(cx, s.t(i18n.KeyDuplicateFiles), "stat-dupes", func() string { return s.fileCount(s.summary.Get().DuplicateFiles) }),
		}},
		core.View{Style: core.Style{Direction: core.Row, Gap: 8}, Children: []core.Node{
			s.statCard(cx, s.t(i18n.KeyGroups), "stat-groups", func() string { return s.groupCount(s.summary.Get().Groups) }),
			s.statCard(cx, s.t(i18n.KeyReclaimable), "stat-reclaim", func() string { return format.Bytes(s.summary.Get().Wasted) }),
		}},
	}}
}

func (s *state) scanCard(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	zoneStyle := func() core.Style {
		background, border := c.background, c.border
		if s.dragging.Get() {
			background, border = c.accentSoft, c.accent
		}
		return core.Style{
			Height: core.Pt(66), Background: background, BorderWidth: 1, BorderColor: border,
			CornerRadius: 8, AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter, Gap: 2,
		}
	}
	return core.View{Style: cardStyle(c), Children: []core.Node{
		line(s.t(i18n.KeyScanFolders), 15, core.FontWeightSemibold, c.text),
		line(s.t(i18n.KeySharedIndex), 11, core.FontWeightNormal, c.muted),
		core.View{Style: core.Style{Direction: core.Row, Gap: 8}, Children: []core.Node{
			core.View{Style: core.Style{FlexGrow: 1, MinWidth: core.Pt(0)}, Children: []core.Node{base.Input{
				Value:       s.scanPath.Get,
				OnChange:    s.scanPath.Set,
				OnSubmit:    func(string) { s.addFolder(s.scanPath.Get()) },
				Placeholder: s.t(i18n.KeyAddFolder),
				Disabled:    func() bool { return s.scanning.Get() || s.busy.Get() },
				TestID:      "scan-path",
				Style: core.Style{
					Height: core.Pt(36), Background: c.background, BorderWidth: 1, BorderColor: c.border,
					CornerRadius: 8, Padding: core.Symmetric(7, 10), FontSize: 13, Color: c.text,
					PlaceholderColor: c.muted,
				},
			}}},
			appButton(cx, s.t(i18n.KeyAdd), "add-folder", buttonSecondary, func() bool {
				return s.scanning.Get() || s.busy.Get() || strings.TrimSpace(s.scanPath.Get()) == ""
			}, func() { s.addFolder(s.scanPath.Get()) }),
		}},
		core.Each(
			func() []string { return s.scanDirs.Get() },
			func(root string) string { return root },
			func(cx *core.Scope, root func() string, _ func() int) core.Node {
				return core.View{Style: core.Style{
					Direction: core.Row, AlignItems: core.AlignCenter, Gap: 6,
					Padding: core.Symmetric(6, 4),
				}, Children: []core.Node{
					core.View{Style: core.Style{FlexGrow: 1, FlexShrink: 1, MinWidth: core.Pt(0)}, Children: []core.Node{
						core.Text{ContentFunc: func() string { return displayPath(root(), s.home) }, MaxLines: 1,
							Style: core.Style{FontSize: 11, Color: c.text, MinWidth: core.Pt(0)}},
					}},
					appButton(cx, s.t(i18n.KeyRemove), "remove-folder:"+root(), buttonSecondary,
						func() bool { return s.scanning.Get() || s.busy.Get() },
						func() { s.removeFolder(root()) }),
				}}
			},
		),
		line(s.t(i18n.KeyMergeHint), 11, core.FontWeightNormal, c.muted),
		core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8}, Children: []core.Node{
			core.Text{ContentFunc: func() string {
				return i18n.FolderCount(s.lang.Get(), len(s.scanDirs.Get()))
			}, TestID: "scan-root-count", Style: core.Style{FlexGrow: 1, FontSize: 11, Color: c.muted}},
			appButton(cx, s.t(i18n.KeyScanAll), "scan-button", buttonPrimary, func() bool {
				return s.scanning.Get() || s.busy.Get() || len(s.scanDirs.Get()) == 0
			}, s.startScan),
		}},
		core.View{
			OnFileDrop:      s.dropFiles,
			OnFileDragEnter: func() { s.dragging.Set(true) },
			OnFileDragLeave: func() { s.dragging.Set(false) },
			StyleFunc:       zoneStyle,
			TestID:          "drop-zone",
			Children: []core.Node{
				line(s.t(i18n.KeyDropFolders), 12, core.FontWeightMedium, c.text),
				line(s.t(i18n.KeyDropHint), 11, core.FontWeightNormal, c.muted),
			},
		},
	}}
}

func (s *state) scanningPanel(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	style := cardStyle(c)
	style.FlexGrow = 1
	style.MinHeight = core.Pt(0)
	style.AlignItems = core.AlignCenter
	style.JustifyContent = core.JustifyCenter
	style.Padding = core.All(24)
	style.Gap = 12
	return core.View{Style: style, Children: []core.Node{
		line(s.t(i18n.KeyScanning), 22, core.FontWeightSemibold, c.text),
		core.Text{Content: s.scanRootsLabel(), MaxLines: 1,
			Style: core.Style{FontSize: 12, Color: c.muted}},
		core.Canvas{
			Style:  core.Style{Width: core.Pct(100), Height: core.Pt(330)},
			Draw:   s.drawGauge(c),
			Label:  s.t(i18n.KeyScanSpeedGauge),
			TestID: "speed-gauge",
		},
		core.Text{ContentFunc: func() string { return speedText(s.progress.Get().Speed) },
			TestID: "scan-speed",
			Style:  core.Style{FontSize: 40, FontWeight: core.FontWeightBold, Color: c.text, TextAlign: core.TextAlignCenter}},
		linef(func() string {
			p := s.progress.Get()
			return s.tf(i18n.KeyScanStats, format.Bytes(p.Hashed), s.fileCount(p.Done), p.Skipped, p.Errors)
		}, 11, core.FontWeightNormal, c.muted),
		linef(func() string {
			return s.tf(i18n.KeyElapsed, formatDuration(s.progress.Get().Elapsed))
		}, 11, core.FontWeightNormal, c.muted),
		core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 10}, Children: []core.Node{
			appButton(cx, s.t(i18n.KeyCancelScan), "cancel-scan", buttonSecondary, nil, s.cancelScan),
			line(s.t(i18n.KeyCancelScanHint), 11, core.FontWeightNormal, c.muted),
		}},
	}}
}

func (s *state) actionsCard(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	disabled := func() bool { return s.scanning.Get() || s.busy.Get() }
	return core.View{Style: cardStyle(c), Children: []core.Node{
		line(s.t(i18n.KeyCleanup), 13, core.FontWeightSemibold, c.text),
		core.View{Style: core.Style{Direction: core.Row, Gap: 8}, Children: []core.Node{
			core.View{Style: core.Style{FlexGrow: 1}, Children: []core.Node{
				appButton(cx, s.t(i18n.KeyRefreshIndex), "refresh", buttonSecondary, disabled, s.refresh),
			}},
			core.View{Style: core.Style{FlexGrow: 1}, Children: []core.Node{
				appButton(cx, s.t(i18n.KeyClearChoices), "clear-keep", buttonSecondary,
					func() bool { return disabled() || len(s.keep.Get()) == 0 }, s.clearKeep),
			}},
		}},
	}}
}

func (s *state) indexCard(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	path := s.cfg.DBPath
	if path == "" {
		path = s.t(i18n.KeyNotAvailable)
	} else {
		path = displayPath(path, s.home)
	}
	return core.View{Style: core.Style{Gap: 2, Padding: core.Edges{Left: 4, Right: 4}}, Children: []core.Node{
		line(s.t(i18n.KeyIndexFile), 11, core.FontWeightMedium, c.muted),
		core.Text{Content: path, MaxLines: 1, Selectable: core.SelectableOn,
			Style: core.Style{FontSize: 11, Color: c.muted}},
	}}
}

func (s *state) noteText(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.Show{When: func() bool { return s.note.Get() != "" }, Then: func(*core.Scope) core.Node {
		color := c.muted
		if s.noteSuccess.Get() {
			color = c.success
		}
		return core.Text{ContentFunc: func() string { return s.note.Get() }, Style: core.Style{FontSize: 12, LineHeight: 17, Color: color}}
	}}
}

func (s *state) results(cx *core.Scope) core.Node {
	return core.Show{
		When: func() bool { return s.scanning.Get() },
		Then: s.scanningPanel,
		Else: s.groupsPanel,
	}
}

func (s *state) groupsPanel(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{Style: core.Style{FlexGrow: 1, MinWidth: core.Pt(0), Gap: 10}, Children: []core.Node{
		core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 10}, Children: []core.Node{
			line(s.t(i18n.KeyDuplicateGroups), 18, core.FontWeightBold, c.text),
			linef(func() string {
				summary := s.summary.Get()
				return fmt.Sprintf("%s · %s", s.groupCount(summary.Groups), s.duplicateFileCount(summary.DuplicateFiles))
			}, 12, core.FontWeightNormal, c.muted),
		}},
		core.List{
			Count: func() int { return len(s.groups.Get()) },
			Key: func(i int) any {
				groups := s.groups.Get()
				if i < 0 || i >= len(groups) {
					return i
				}
				return groups[i].MD5
			},
			Row:           s.groupRow,
			EstimatedSize: 150,
			Gap:           10,
			Style:         core.Style{FlexGrow: 1, MinHeight: core.Pt(0)},
			Empty:         s.emptyState(cx),
		},
		s.actionBar(cx),
	}}
}

func (s *state) emptyState(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	available := s.cfg.Store != nil
	title := s.t(i18n.KeyNoDuplicates)
	body := s.t(i18n.KeyEmptyBody)
	if !available {
		title = s.t(i18n.KeyIndexUnavailable)
		body = s.t(i18n.KeyCloseIndexHolder)
	}
	return core.View{
		TestID: "empty-state",
		Style: core.Style{
			FlexGrow: 1, MinHeight: core.Pt(280), AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			Gap: 8, Padding: core.All(28),
		},
		Children: []core.Node{
			core.View{Style: core.Style{
				Width: core.Pt(44), Height: core.Pt(44), CornerRadius: 22,
				Background: c.accentSoft, AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			}, Children: []core.Node{core.Icon{
				SVG:   lucide.Check(),
				Style: core.Style{Width: core.Pt(22), Height: core.Pt(22), Color: c.accent},
			}}},
			core.Text{Content: title, Style: core.Style{FontSize: 16, FontWeight: core.FontWeightSemibold, Color: c.text}},
			core.Text{Content: body, Style: core.Style{FontSize: 12, Color: c.muted}},
			core.Show{When: func() bool { return available }, Then: func(cx *core.Scope) core.Node {
				return appButton(cx, s.t(i18n.KeyScanDefaultFolder), "empty-scan", buttonSecondary, nil, func() {
					if s.addFolder(s.defaultDir) {
						s.startScan()
					}
				})
			}},
		},
	}
}

func (s *state) actionBar(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{
		Style: core.Style{
			FlexShrink: 0, Direction: core.Column, Gap: 8,
			Background: c.surface, BorderWidth: 1, BorderColor: c.border,
			CornerRadius: 12, Padding: core.All(12),
		},
		Children: []core.Node{
			core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 12}, Children: []core.Node{
				core.View{Style: core.Style{FlexGrow: 1, MinWidth: core.Pt(0), Gap: 2}, Children: []core.Node{
					core.Text{ContentFunc: func() string {
						plan := s.plan()
						if len(plan.Targets) == 0 {
							return s.t(i18n.KeyChooseCopyOrKeep)
						}
						return s.tf(i18n.KeySelection, s.fileCount(len(plan.Targets)), format.Bytes(plan.Bytes))
					}, Style: core.Style{FontSize: 13, FontWeight: core.FontWeightMedium, Color: c.text}},
					line(s.t(i18n.KeyPermanentHint), 11, core.FontWeightNormal, c.muted),
				}},
				appButton(cx, s.t(i18n.KeyReviewDelete), "review-delete", buttonDanger, func() bool {
					return s.scanning.Get() || s.busy.Get() || len(s.plan().Targets) == 0
				}, s.openConfirm),
			}},
			core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8}, Children: []core.Node{
				line(s.t(i18n.KeyKeepPolicy), 11, core.FontWeightMedium, c.muted),
				s.policyButton(cx, s.t(i18n.KeyNewest), "policy-newest", appmodel.KeepNewest),
				s.policyButton(cx, s.t(i18n.KeyOldest), "policy-oldest", appmodel.KeepOldest),
				line(s.t(i18n.KeyPolicyHint), 11, core.FontWeightNormal, c.muted),
				spacer(),
				appButton(cx, s.t(i18n.KeyKeepOne), "auto-keep", buttonSecondary, func() bool {
					return s.scanning.Get() || s.busy.Get() || len(s.groups.Get()) == 0
				}, s.autoKeep),
			}},
		},
	}
}

func (s *state) confirmDialog(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return base.Dialog{
		Open: s.confirm.Get,
		OnOpenChange: func(open bool, _ base.Reason) {
			s.confirm.Set(open)
		},
		BackdropStyle: core.Style{Background: core.RGBA(0x000000, 0.4)},
		PopupStyle: core.Style{
			Width: core.Pt(540), MaxWidth: core.Pct(90), Background: c.surface,
			BorderWidth: 1, BorderColor: c.border, CornerRadius: 14, Padding: core.All(22), Gap: 12,
		},
		Label:  s.t(i18n.KeyConfirmDelete),
		TestID: "confirm-dialog",
		Children: []core.Node{
			line(s.t(i18n.KeyDeleteDuplicates), 20, core.FontWeightBold, c.text),
			core.Text{ContentFunc: func() string {
				plan := s.plan()
				return s.tf(i18n.KeyDeleteConfirm, s.fileCount(len(plan.Targets)), format.Bytes(plan.Bytes))
			}, Style: core.Style{FontSize: 14, Color: c.text}},
			line(s.t(i18n.KeyCannotUndo), 12, core.FontWeightNormal, c.muted),
			core.Each(
				func() []db.FileRecord {
					plan := s.plan()
					if len(plan.Targets) > 5 {
						return plan.Targets[:5]
					}
					return plan.Targets
				},
				func(file db.FileRecord) string { return file.Path },
				func(cx *core.Scope, file func() db.FileRecord, _ func() int) core.Node {
					return core.Text{ContentFunc: func() string { return file().Path }, MaxLines: 2, Selectable: core.SelectableOn,
						Style: core.Style{FontSize: 11, Color: c.muted}}
				},
			),
			core.Show{When: func() bool { return len(s.plan().Targets) > 5 }, Then: func(*core.Scope) core.Node {
				return linef(func() string { return s.tf(i18n.KeyAndMore, len(s.plan().Targets)-5) }, 11, core.FontWeightNormal, c.muted)
			}},
			core.View{Style: core.Style{Direction: core.Row, JustifyContent: core.JustifyEnd, Gap: 8}, Children: []core.Node{
				appButton(cx, s.t(i18n.KeyCancel), "cancel-delete", buttonSecondary, func() bool { return s.busy.Get() }, func() { s.confirm.Set(false) }),
				appButton(cx, s.t(i18n.KeyDeletePermanently), "delete-confirm", buttonDanger, func() bool { return s.busy.Get() }, s.confirmDelete),
			}},
		},
	}
}

func displayPath(path, home string) string {
	if home != "" && strings.HasPrefix(path, home+string(filepath.Separator)) {
		return "~" + path[len(home):]
	}
	return path
}
