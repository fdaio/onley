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

const (
	displayFont = "Impact"
	monoFont    = "Courier New"
)

// Root builds the app. Config is read once; tests pass a temporary store.
func Root(cfg Config) func(*core.Scope) core.Node {
	return func(cx *core.Scope) core.Node {
		cx.SetTheme(brutalistTheme())
		cx.SetSubtreeAppearance(core.AppearanceLight)
		cx.Effect(func() { cx.SetAppearance(core.AppearanceLight) })
		s := newState(cx, cfg)
		if desktop.IsAvailable(cx) {
			desktop.SetTitle(cx, "Onley")
			cx.Effect(func() {
				desktop.SetApplicationMenu(cx, s.menu())
				if cx.Env().OS() != core.OSMacOS {
					desktop.SetMenuBar(cx, desktop.MenuBarAutoHide)
				}
			})
		}
		return core.Dynamic(func(cx *core.Scope) core.Node {
			_ = s.lang.Get()
			_ = s.keepPolicy.Get()
			_ = s.keep.Get()
			_ = s.groups.Get()
			_ = s.scoped.Get()
			_ = s.visible.Get()
			_ = s.summary.Get()
			_ = s.indexed.Get()
			_ = s.category.Get()
			_ = s.activeRoot.Get()
			_ = s.scanning.Get()
			_ = s.busy.Get()
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
			s.tabBar(cx),
			s.searchBar(cx),
			s.body(cx),
			s.bottomBar(cx),
			s.addFolderDialog(cx),
			s.confirmDialog(cx),
		},
	}
}

func (s *state) header(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{
		Style: core.Style{
			FlexShrink: 0, Direction: core.Row, Height: core.Pt(190),
			Background: c.surface, Borders: core.Edges{Bottom: 2}, BorderColor: c.border,
			Padding: core.Symmetric(16, 26), Gap: 24,
		},
		Children: []core.Node{
			core.View{Style: core.Style{
				FlexGrow: 1, FlexBasis: core.Pt(0), MinWidth: core.Pt(0),
				Direction: core.Column, JustifyContent: core.JustifyCenter, Gap: 6,
			}, Children: []core.Node{
				core.Text{Content: "ONLEY", Style: core.Style{FontFamily: displayFont, FontSize: 76, Color: c.text}},
				core.Text{Content: "NAVIGATING A WORLD OF\nSHIFTING DATA\nTRADITIONS.", MaxLines: 3,
					Style: core.Style{FontFamily: displayFont, FontSize: 17, LineHeight: 19, Color: c.text}},
			}},
			core.View{Style: core.Style{
				Width: core.Pt(300), FlexShrink: 0, Direction: core.Column, Gap: 4,
				Borders: core.Edges{Left: 2}, BorderColor: c.border, Padding: core.Edges{Left: 20},
			}, Children: []core.Node{
				s.stat(s.t(i18n.KeyIndexedFiles), formatCount(s.indexed.Get()), c.text, "indexed-files"),
				s.stat(s.t(i18n.KeyDuplicateGroups), formatCount(s.summary.Get().Groups), c.text, "group-summary"),
				s.stat(s.t(i18n.KeyReclaimableSpace), format.Bytes(s.summary.Get().Wasted), c.accent, "duplicate-summary"),
			}},
		},
	}
}

func (s *state) stat(label, value string, color core.Color, testID string) core.Node {
	c := colorsOf(s.cx)
	return core.View{Style: core.Style{
		FlexGrow: 1, Direction: core.Column, JustifyContent: core.JustifyEnd, Gap: 1,
		Borders: core.Edges{Top: 1}, BorderColor: c.border, Padding: core.Edges{Top: 4},
	}, Children: []core.Node{
		core.Text{Content: strings.ToUpper(label), Style: core.Style{FontFamily: monoFont, FontSize: 8, Color: c.muted}},
		core.Text{Content: value, TestID: testID,
			Style: core.Style{FontFamily: displayFont, FontSize: 28, Color: color, TextAlign: core.TextAlignEnd}},
	}}
}

func (s *state) tabBar(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	counts := s.categoryCounts()
	tabs := []category{categoryAll, categoryImages, categoryVideos, categoryDocuments, categoryArchives, categorySystem, categorySearch}
	children := make([]core.Node, 0, len(tabs))
	for _, cat := range tabs {
		label := s.t(cat.labelKey())
		if cat != categorySearch {
			label = fmt.Sprintf("%s (%02d)", label, counts[cat])
		}
		children = append(children, s.tab(cx, cat, label))
	}
	return base.Tabs{
		Style: core.Style{
			FlexShrink: 0, Direction: core.Row, Background: c.surface,
			Borders: core.Edges{Top: 1, Bottom: 1}, BorderColor: c.border,
		},
		Label:    s.t(i18n.KeyAllCopies),
		TestID:   "category-tabs",
		Children: children,
	}
}

func (s *state) tab(cx *core.Scope, cat category, label string) core.Node {
	c := colorsOf(cx)
	selected := func() bool { return s.category.Get() == cat }
	textColor := func() core.Color {
		if selected() {
			return c.surface
		}
		return c.text
	}
	children := []core.Node{core.Text{Content: strings.ToUpper(label),
		Style:     core.Style{FontFamily: monoFont, FontSize: 8, FontWeight: core.FontWeightBold},
		StyleFunc: func() core.Style { return core.Style{Color: textColor()} }}}
	if cat == categorySearch {
		children = []core.Node{core.View{Style: core.Style{
			Direction: core.Row, AlignItems: core.AlignCenter, Gap: 4,
		}, Children: []core.Node{
			core.Text{Content: strings.ToUpper(label),
				Style:     core.Style{FontFamily: monoFont, FontSize: 8, FontWeight: core.FontWeightBold},
				StyleFunc: func() core.Style { return core.Style{Color: textColor()} }},
			core.Icon{SVG: lucide.Search(), Style: core.Style{Width: core.Pt(10), Height: core.Pt(10)},
				StyleFunc: func() core.Style { return core.Style{Color: textColor()} }},
		}}}
	}
	return base.Tab{
		Selected: selected,
		OnPress:  func(core.PressEvent) { s.setCategory(cat) },
		Label:    label,
		TestID:   "tab:" + cat.keyName(),
		StyleFunc: func() core.Style {
			background := c.surface
			hover := c.surfaceHover
			if selected() {
				background = c.text
				hover = c.text
			}
			return core.Style{
				FlexGrow: 1, FlexBasis: core.Pt(0), MinWidth: core.Pt(0),
				AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
				Padding: core.Symmetric(11, 6), Background: background,
				Borders: core.Edges{Right: 1}, BorderColor: c.border,
				Hover: &core.Style{Background: hover},
			}
		},
		Children: children,
	}
}

func (s *state) searchBar(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.Show{When: func() bool { return s.category.Get() == categorySearch }, Then: func(*core.Scope) core.Node {
		return core.View{Style: core.Style{
			FlexShrink: 0, Padding: core.Symmetric(8, 20), Background: c.surface,
			Borders: core.Edges{Bottom: 1}, BorderColor: c.border,
		}, Children: []core.Node{
			base.Input{
				Value:       s.query.Get,
				OnChange:    s.setQuery,
				OnSubmit:    func(string) { s.applySearch() },
				Placeholder: s.t(i18n.KeySearchPlaceholder),
				TestID:      "search-input",
				Style: core.Style{
					Height: core.Pt(34), Background: c.background, BorderWidth: 1,
					BorderColor: c.border, Padding: core.Symmetric(6, 10),
					FontFamily: monoFont, FontSize: 11, Color: c.text,
					PlaceholderColor: c.muted,
				},
			},
		}}
	}}
}

func (s *state) body(cx *core.Scope) core.Node {
	return core.View{Style: core.Style{
		FlexGrow: 1, FlexBasis: core.Pt(0), MinHeight: core.Pt(0), Direction: core.Row,
	}, Children: []core.Node{
		s.sidebar(cx),
		s.mainPanel(cx),
	}}
}

func (s *state) sidebar(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{Style: core.Style{
		Width: core.Pt(260), FlexShrink: 0, Direction: core.Column,
		Background: c.surface, Borders: core.Edges{Right: 1}, BorderColor: c.border,
	}, Children: []core.Node{
		core.View{Style: core.Style{
			FlexShrink: 0, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8,
			Padding: core.Symmetric(9, 12), Background: c.text,
		}, Children: []core.Node{
			core.Text{Content: strings.ToUpper(s.t(i18n.KeySourceFolders)), Style: core.Style{FontFamily: monoFont,
				FontSize: 9, FontWeight: core.FontWeightBold, Color: c.surface}},
			spacer(),
			base.Button{
				OnPress: func(core.PressEvent) { s.setActiveRoot("") },
				Label:   s.t(i18n.KeyAllFolders),
				TestID:  "all-folders",
				Style:   core.Style{Padding: core.Symmetric(2, 4), Hover: &core.Style{Background: c.surfacePressed}},
				Children: []core.Node{core.Text{Content: strings.ToUpper(s.t(i18n.KeyAllFolders)),
					Style: core.Style{FontFamily: monoFont, FontSize: 7, Color: c.surface}}},
			},
			core.Text{Content: fmt.Sprintf("(%02d)", len(s.scanDirs.Get())),
				Style: core.Style{FontFamily: monoFont, FontSize: 9, Color: c.surface}},
		}},
		core.List{
			TestID: "source-folders",
			Count:  func() int { return len(s.scanDirs.Get()) },
			Key: func(i int) any {
				roots := s.scanDirs.Get()
				if i < 0 || i >= len(roots) {
					return i
				}
				return roots[i]
			},
			Row: func(cx *core.Scope, row core.ListRow) core.Node {
				roots := s.scanDirs.Get()
				i := row.Index()
				if i < 0 || i >= len(roots) {
					return core.View{}
				}
				return s.folderRow(cx, roots[i])
			},
			EstimatedSize: 46,
			Style:         core.Style{FlexGrow: 1, FlexBasis: core.Pt(0), MinHeight: core.Pt(0)},
			Empty:         line(s.t(i18n.KeyNoFolders), 9, core.FontWeightNormal, c.muted),
		},
		s.dropZone(cx),
		s.scanButton(cx),
		s.sidebarFooter(cx),
	}}
}

func (s *state) folderRow(cx *core.Scope, root string) core.Node {
	c := colorsOf(cx)
	return base.Button{
		OnPress: func(core.PressEvent) {
			if s.activeRoot.Get() == root {
				s.setActiveRoot("")
			} else {
				s.setActiveRoot(root)
			}
		},
		Label:  displayPath(root, s.home),
		TestID: "folder:" + root,
		StyleFunc: func() core.Style {
			background := c.surface
			if s.activeRoot.Get() == root {
				background = c.successSoft
			}
			return core.Style{
				FlexGrow: 1, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8,
				Padding: core.Symmetric(10, 12), Background: background,
				Borders: core.Edges{Bottom: 1}, BorderColor: c.border,
				Hover: &core.Style{Background: c.surfaceHover},
			}
		},
		Children: []core.Node{
			core.Text{Content: displayPath(root, s.home), MaxLines: 1, EllipsizeMode: core.EllipsizeMiddle,
				Style: core.Style{FlexGrow: 1, FlexShrink: 1, MinWidth: core.Pt(0), MaxWidth: core.Pt(170),
					FontFamily: monoFont, FontSize: 9, Color: c.text}},
			base.Button{
				OnPress:  func(core.PressEvent) { s.removeFolder(root) },
				Disabled: func() bool { return s.scanning.Get() || s.busy.Get() },
				Label:    s.t(i18n.KeyRemove),
				TestID:   "remove-folder:" + root,
				Style: core.Style{
					Padding: core.Symmetric(3, 6), BorderWidth: 1, BorderColor: c.border,
					Hover:    &core.Style{Background: c.surfaceHover},
					Disabled: &core.Style{Opacity: core.Alpha(0.4)},
				},
				Children: []core.Node{core.Text{Content: strings.ToUpper(s.t(i18n.KeyRemove)),
					Style: core.Style{FontFamily: monoFont, FontSize: 7, Color: c.text}}},
			},
		},
	}
}

func (s *state) dropZone(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{
		TestID: "drop-zone",
		Style: core.Style{
			FlexShrink: 0, Height: core.Pt(126), Margin: core.All(12), Direction: core.Column,
			Background: c.surface, BorderWidth: 1.5, BorderStyle: core.BorderDashed,
			BorderColor: c.border, CornerRadius: 999,
		},
		OnFileDrop: func(uris []string) {
			for _, uri := range uris {
				if path, ok := filePath(uri); ok {
					s.addFolder(path)
				}
			}
		},
		Children: []core.Node{
			base.Button{
				OnPress: func(core.PressEvent) { s.openAddFolder() },
				Label:   s.t(i18n.KeyAddFolder),
				TestID:  "add-folder",
				Style: core.Style{
					FlexGrow: 1, Width: core.Pct(100), Direction: core.Column,
					AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter, Gap: 6,
					Hover: &core.Style{Background: c.surfaceHover},
				},
				Children: []core.Node{
					core.Icon{SVG: lucide.Plus(), Style: core.Style{Width: core.Pt(18), Height: core.Pt(18), Color: c.text}},
					core.Text{Content: strings.ToUpper(s.t(i18n.KeyDropDirectory)), Style: core.Style{FontFamily: monoFont,
						FontSize: 8, FontWeight: core.FontWeightBold, Color: c.text, TextAlign: core.TextAlignCenter}},
					core.Text{Content: strings.ToUpper(s.t(i18n.KeyInitializeScan)), Style: core.Style{FontFamily: monoFont,
						FontSize: 7, Color: c.muted, TextAlign: core.TextAlignCenter}},
				},
			},
		},
	}
}

func (s *state) scanButton(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return base.Button{
		OnPress:  func(core.PressEvent) { s.startScan() },
		Disabled: func() bool { return s.scanning.Get() || s.busy.Get() || len(s.scanDirs.Get()) == 0 },
		Label:    s.t(i18n.KeyRunGlobalScan),
		TestID:   "scan-button",
		Style: core.Style{
			FlexShrink: 0, Height: core.Pt(44), Direction: core.Row, AlignItems: core.AlignCenter,
			JustifyContent: core.JustifyCenter, Gap: 6, Background: c.text,
			Hover: &core.Style{Background: c.accent}, Disabled: &core.Style{Opacity: core.Alpha(0.4)},
		},
		Children: []core.Node{core.Text{Content: strings.ToUpper(s.t(i18n.KeyRunGlobalScan)) + " ↗",
			Style: core.Style{FontFamily: monoFont, FontSize: 9, FontWeight: core.FontWeightBold, Color: c.surface}}},
	}
}

func (s *state) sidebarFooter(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	label := "中文"
	if s.lang.Get() == i18n.Chinese {
		label = "EN"
	}
	return core.View{Style: core.Style{
		FlexShrink: 0, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8,
		Padding: core.Symmetric(7, 12), Borders: core.Edges{Top: 1}, BorderColor: c.border,
	}, Children: []core.Node{
		core.Text{Content: strings.ToUpper(s.t(i18n.KeyLocalOnly)),
			Style: core.Style{FontFamily: monoFont, FontSize: 7, Color: c.muted}},
		spacer(),
		base.Button{
			OnPress: func(core.PressEvent) { s.toggleLanguage() },
			Label:   s.t(i18n.KeySwitchLanguage),
			TestID:  "language-toggle",
			Style:   core.Style{Padding: core.Symmetric(2, 4), Hover: &core.Style{Background: c.surfaceHover}},
			Children: []core.Node{core.Text{Content: label, Style: core.Style{FontFamily: monoFont,
				FontSize: 8, FontWeight: core.FontWeightBold, Color: c.text}}},
		},
	}}
}

func (s *state) mainPanel(cx *core.Scope) core.Node {
	return core.View{Style: core.Style{
		FlexGrow: 1, FlexBasis: core.Pt(0), MinWidth: core.Pt(0), MinHeight: core.Pt(0),
		Direction: core.Column, Padding: core.Symmetric(12, 16), Gap: 8,
	}, Children: []core.Node{
		s.banner(cx),
		s.results(cx),
		s.noteText(cx),
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
	return core.View{Style: core.Style{FlexGrow: 1, FlexBasis: core.Pt(0), MinHeight: core.Pt(0)}, Children: []core.Node{
		core.List{
			TestID: "duplicate-list",
			Count:  func() int { return len(s.visible.Get()) },
			Key: func(i int) any {
				groups := s.visible.Get()
				if i < 0 || i >= len(groups) {
					return i
				}
				return groups[i].MD5
			},
			Row:           s.groupRow,
			EstimatedSize: s.estimatedGroupHeight(),
			Footer:        core.View{Style: core.Style{Height: core.Pt(s.listFooterHeight())}},
			Gap:           12,
			Style:         core.Style{FlexGrow: 1, FlexBasis: core.Pt(0), MinHeight: core.Pt(0)},
			Empty:         s.emptyState(cx),
		},
	}}
}

const (
	groupRowMinHeight    = 190
	groupRowFileHeight   = 44
	listFooterBaseHeight = 120
	listFooterRowHeight  = 4
)

// estimatedGroupHeight returns the tallest visible group card estimate. A low
// estimate makes the list stop before the last row.
func (s *state) estimatedGroupHeight() core.Points {
	tallest := groupRowMinHeight
	for _, group := range s.visible.Get() {
		height := 20 + groupRowFileHeight*len(group.Files)
		if height > tallest {
			tallest = height
		}
	}
	return core.Points(tallest)
}

// listFooterHeight reserves scroll range below the last row. The native list
// can set its scroll range before it measures the last row.
func (s *state) listFooterHeight() core.Points {
	count := len(s.visible.Get())
	if count == 0 {
		return 0
	}
	return core.Points(listFooterBaseHeight + listFooterRowHeight*count)
}

func (s *state) bottomBar(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{
		TestID: "bottom-bar",
		Style: core.Style{
			FlexShrink: 0, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 14,
			Background: c.text, Padding: core.Symmetric(10, 20),
		},
		Children: []core.Node{
			core.Text{Content: strings.ToUpper(s.t(i18n.KeyAutoSelectPolicy)),
				Style: core.Style{FontFamily: monoFont, FontSize: 8, FontWeight: core.FontWeightBold, Color: c.surface}},
			s.policyToggle(cx, i18n.KeyKeepNewest, "policy-newest", appmodel.KeepNewest),
			s.policyToggle(cx, i18n.KeyKeepOldest, "policy-oldest", appmodel.KeepOldest),
			s.policyToggle(cx, i18n.KeyKeepByPath, "policy-path", appmodel.KeepByPath),
			base.Button{
				OnPress:  func(core.PressEvent) { s.clearKeep() },
				Disabled: func() bool { return s.scanning.Get() || s.busy.Get() || len(s.keep.Get()) == 0 },
				Label:    s.t(i18n.KeyClear),
				TestID:   "clear-choices",
				Style: core.Style{Padding: core.Symmetric(4, 6), Hover: &core.Style{Background: c.surfacePressed},
					Disabled: &core.Style{Opacity: core.Alpha(0.35)}},
				Children: []core.Node{core.Text{Content: strings.ToUpper(s.t(i18n.KeyClear)),
					Style: core.Style{FontFamily: monoFont, FontSize: 8, Color: c.surface}}},
			},
			spacer(),
			s.reviewButton(cx),
		},
	}
}

func (s *state) policyToggle(cx *core.Scope, key i18n.Key, testID string, policy appmodel.KeepPolicy) core.Node {
	c := colorsOf(cx)
	return base.Button{
		OnPress:  func(core.PressEvent) { s.applyKeepPolicy(policy) },
		Disabled: func() bool { return s.scanning.Get() || s.busy.Get() || len(s.groups.Get()) == 0 },
		Selected: func() bool { return s.policySelected(policy) },
		Label:    s.t(key),
		TestID:   testID,
		Style: core.Style{
			Direction: core.Row, AlignItems: core.AlignCenter, Gap: 6,
			Padding: core.Symmetric(4, 4), Hover: &core.Style{Background: c.surfacePressed},
			Active: &core.Style{Background: c.surfaceHover}, Disabled: &core.Style{Opacity: core.Alpha(0.35)},
		},
		Children: []core.Node{
			statusBox(cx, func() bool { return s.policySelected(policy) }),
			core.Text{Content: strings.ToUpper(s.t(key)), Style: core.Style{FontFamily: monoFont,
				FontSize: 8, FontWeight: core.FontWeightBold, Color: c.surface}},
		},
	}
}

func (s *state) reviewButton(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return base.Button{
		OnPress:  func(core.PressEvent) { s.openConfirm() },
		Disabled: func() bool { return s.scanning.Get() || s.busy.Get() || len(s.plan().Targets) == 0 },
		Label:    s.reviewLabel(),
		TestID:   "delete-files",
		Style: core.Style{
			FlexShrink: 0, Padding: core.Symmetric(12, 26), AlignItems: core.AlignCenter,
			JustifyContent: core.JustifyCenter, Background: c.accent, BorderWidth: 2, BorderColor: c.text,
			Hover: &core.Style{Background: c.accentHover}, Active: &core.Style{Background: c.accentPressed},
			Disabled: &core.Style{Opacity: core.Alpha(0.35)},
		},
		Children: []core.Node{core.Text{Content: s.reviewLabel(),
			Style: core.Style{FontFamily: displayFont, FontSize: 24, Color: c.text}}},
	}
}

func (s *state) reviewLabel() string {
	return strings.ToUpper(s.tf(i18n.KeyReviewDeleteN, s.fileCount(len(s.plan().Targets))))
}

func (s *state) banner(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.Show{When: func() bool { return s.message.Get() != "" }, Then: func(cx *core.Scope) core.Node {
		return core.View{
			Style: core.Style{
				FlexShrink: 0, Direction: core.Row, AlignItems: core.AlignCenter, Gap: 10,
				Padding: core.Symmetric(7, 10), Background: c.accentSoft,
				Borders: core.Edges{Bottom: 1}, BorderColor: c.accent,
			},
			Children: []core.Node{
				core.View{Style: core.Style{FlexGrow: 1, MinWidth: core.Pt(0)}, Children: []core.Node{
					core.Text{ContentFunc: func() string { return s.message.Get() }, MaxLines: 3,
						Selectable: core.SelectableOn, Style: core.Style{FontFamily: monoFont, FontSize: 10, Color: c.text}},
				}},
				appButton(cx, s.t(i18n.KeyDismiss), "dismiss-error", buttonSecondary, nil, func() { s.message.Set("") }),
			},
		}
	}}
}

func (s *state) scanningPanel(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.View{Style: core.Style{
		FlexGrow: 1, FlexBasis: core.Pt(0), MinHeight: core.Pt(0), Direction: core.Column,
		AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter, Gap: 10,
		Background: c.surface, BorderWidth: 2, BorderColor: c.border, Padding: core.All(18),
	}, Children: []core.Node{
		core.Text{Content: strings.ToUpper(s.t(i18n.KeyScanning)),
			Style: core.Style{FontFamily: displayFont, FontSize: 34, Color: c.text}},
		core.Text{Content: s.scanRootsLabel(), MaxLines: 1, TestID: "scan-root",
			Style: core.Style{FontFamily: monoFont, FontSize: 9, Color: c.muted}},
		core.Canvas{
			Style:  core.Style{Width: core.Pct(100), Height: core.Pt(280)},
			Draw:   s.drawGauge(c),
			Label:  s.t(i18n.KeyScanSpeedGauge),
			TestID: "speed-gauge",
		},
		core.Text{ContentFunc: func() string { return speedText(s.progress.Get().Speed) },
			TestID: "scan-speed",
			Style:  core.Style{FontFamily: displayFont, FontSize: 32, Color: c.text, TextAlign: core.TextAlignCenter}},
		linef(func() string {
			p := s.progress.Get()
			return s.tf(i18n.KeyScanStats, format.Bytes(p.Hashed), s.fileCount(p.Done), p.Skipped, p.Errors)
		}, 9, core.FontWeightNormal, c.muted),
		linef(func() string {
			return s.tf(i18n.KeyElapsed, formatDuration(s.progress.Get().Elapsed))
		}, 9, core.FontWeightNormal, c.muted),
		core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 10}, Children: []core.Node{
			appButton(cx, s.t(i18n.KeyCancelScan), "cancel-scan", buttonSecondary, nil, s.cancelScan),
			line(s.t(i18n.KeyCancelScanHint), 9, core.FontWeightNormal, c.muted),
		}},
	}}
}

func (s *state) noteText(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return core.Show{When: func() bool { return s.note.Get() != "" }, Then: func(*core.Scope) core.Node {
		color := c.muted
		if s.noteSuccess.Get() {
			color = c.success
		}
		return core.Text{ContentFunc: func() string { return s.note.Get() }, Style: core.Style{
			FontFamily: monoFont, FontSize: 9, Color: color}}
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
			FlexGrow: 1, MinHeight: core.Pt(240), AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			Gap: 8, Padding: core.All(24), Background: c.surface, BorderWidth: 2, BorderColor: c.border,
		},
		Children: []core.Node{
			core.Text{Content: strings.ToUpper(title), Style: core.Style{FontFamily: displayFont, FontSize: 30, Color: c.text}},
			core.Text{Content: body, MaxLines: 2, Style: core.Style{FontFamily: monoFont, FontSize: 10,
				Color: c.muted, TextAlign: core.TextAlignCenter}},
			core.Show{When: func() bool { return available }, Then: func(cx *core.Scope) core.Node {
				return appButton(cx, s.t(i18n.KeyScanDefaultFolder), "empty-scan", buttonPrimary, nil, func() {
					if s.addFolder(s.defaultDir) {
						s.startScan()
					}
				})
			}},
		},
	}
}

func (s *state) addFolderDialog(cx *core.Scope) core.Node {
	c := colorsOf(cx)
	return base.Dialog{
		Open: s.addingFolder.Get,
		OnOpenChange: func(open bool, _ base.Reason) {
			s.addingFolder.Set(open)
		},
		BackdropStyle: core.Style{Background: core.RGBA(0x111111, 0.55)},
		PopupStyle: core.Style{
			Width: core.Pt(500), MaxWidth: core.Pct(90), Background: c.surface,
			BorderWidth: 2, BorderColor: c.border, Padding: core.All(20), Gap: 10,
		},
		Label:  s.t(i18n.KeyAddFolder),
		TestID: "add-folder-dialog",
		Children: []core.Node{
			core.Text{Content: strings.ToUpper(s.t(i18n.KeyAddFolder)),
				Style: core.Style{FontFamily: displayFont, FontSize: 26, Color: c.text}},
			base.Input{
				Value:       s.scanPath.Get,
				OnChange:    s.scanPath.Set,
				OnSubmit:    func(string) { s.confirmAddFolder() },
				Placeholder: s.t(i18n.KeyAddFolder),
				Disabled:    func() bool { return s.scanning.Get() || s.busy.Get() },
				TestID:      "scan-path",
				Style: core.Style{
					Height: core.Pt(36), Background: c.background, BorderWidth: 1, BorderColor: c.border,
					Padding: core.Symmetric(7, 9), FontFamily: monoFont, FontSize: 10, Color: c.text,
					PlaceholderColor: c.muted,
				},
			},
			core.Show{When: func() bool { return s.message.Get() != "" }, Then: func(*core.Scope) core.Node {
				return core.Text{ContentFunc: func() string { return s.message.Get() }, TestID: "add-folder-error",
					Style: core.Style{FontFamily: monoFont, FontSize: 9, Color: c.accent}}
			}},
			core.View{Style: core.Style{Direction: core.Row, JustifyContent: core.JustifyEnd, Gap: 8}, Children: []core.Node{
				appButton(cx, s.t(i18n.KeyCancel), "cancel-add-folder", buttonSecondary, func() bool { return s.busy.Get() }, s.closeAddFolder),
				appButton(cx, s.t(i18n.KeyAdd), "add-folder-confirm", buttonPrimary, func() bool {
					return s.scanning.Get() || s.busy.Get() || strings.TrimSpace(s.scanPath.Get()) == ""
				}, s.confirmAddFolder),
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
		BackdropStyle: core.Style{Background: core.RGBA(0x111111, 0.55)},
		PopupStyle: core.Style{
			Width: core.Pt(560), MaxWidth: core.Pct(90), Background: c.surface,
			BorderWidth: 2, BorderColor: c.border, Padding: core.All(20), Gap: 10,
		},
		Label:  s.t(i18n.KeyConfirmDelete),
		TestID: "confirm-dialog",
		Children: []core.Node{
			core.Text{Content: strings.ToUpper(s.t(i18n.KeyDeleteDuplicates)),
				Style: core.Style{FontFamily: displayFont, FontSize: 30, Color: c.text}},
			core.Text{ContentFunc: func() string {
				plan := s.plan()
				return s.tf(i18n.KeyDeleteConfirm, s.fileCount(len(plan.Targets)), format.Bytes(plan.Bytes))
			}, Style: core.Style{FontFamily: monoFont, FontSize: 11, Color: c.text}},
			line(s.t(i18n.KeyCannotUndo), 9, core.FontWeightNormal, c.muted),
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
						Style: core.Style{FontFamily: monoFont, FontSize: 9, Color: c.muted}}
				},
			),
			core.Show{When: func() bool { return len(s.plan().Targets) > 5 }, Then: func(*core.Scope) core.Node {
				return linef(func() string { return s.tf(i18n.KeyAndMore, len(s.plan().Targets)-5) }, 9, core.FontWeightNormal, c.muted)
			}},
			core.View{Style: core.Style{Direction: core.Row, JustifyContent: core.JustifyEnd, Gap: 8}, Children: []core.Node{
				appButton(cx, s.t(i18n.KeyCancel), "cancel-delete", buttonSecondary, func() bool { return s.busy.Get() }, func() { s.confirm.Set(false) }),
				appButton(cx, s.t(i18n.KeyDeletePermanently), "delete-confirm", buttonDanger, func() bool { return s.busy.Get() }, s.confirmDelete),
			}},
		},
	}
}

func displayPath(path, home string) string {
	if home == "" {
		return path
	}
	if path == home {
		return "~"
	}
	if strings.HasPrefix(path, home+string(filepath.Separator)) {
		return "~" + path[len(home):]
	}
	return path
}
