package main

import (
	"fmt"
	"time"

	"github.com/oneo-me/nsgo/base"
	"github.com/oneo-me/nsgo/core"

	"onley/internal/appmodel"
	"onley/internal/db"
	"onley/internal/format"
	"onley/internal/i18n"
	"onley/internal/selfpath"
)

type colors struct {
	background     core.Color
	surface        core.Color
	surfaceHover   core.Color
	surfacePressed core.Color
	border         core.Color
	text           core.Color
	muted          core.Color
	accent         core.Color
	accentHover    core.Color
	accentPressed  core.Color
	accentText     core.Color
	accentSoft     core.Color
	danger         core.Color
	dangerHover    core.Color
	dangerPressed  core.Color
	dangerSoft     core.Color
	success        core.Color
}

func colorsOf(cx *core.Scope) colors {
	p := cx.Theme().Light
	if cx.Dark() {
		p = cx.Theme().Dark
	}
	return colors{
		background:     p.Background,
		surface:        p.Surface,
		surfaceHover:   p.SurfaceHover,
		surfacePressed: p.SurfacePressed,
		border:         p.Border,
		text:           p.Text,
		muted:          p.TextMuted,
		accent:         p.Accent,
		accentHover:    p.AccentHover,
		accentPressed:  p.AccentPressed,
		accentText:     p.AccentText,
		accentSoft:     p.Accent.Faded(0.12),
		danger:         p.Danger,
		dangerHover:    p.Danger.Faded(0.82),
		dangerPressed:  p.Danger.Faded(0.7),
		dangerSoft:     p.Danger.Faded(0.14),
		success:        p.Success,
	}
}

func line(content string, size core.Points, weight core.FontWeight, color core.Color) core.Text {
	return core.Text{Content: content, Style: core.Style{FontSize: size, FontWeight: weight, Color: color}}
}

func linef(content func() string, size core.Points, weight core.FontWeight, color core.Color) core.Text {
	return core.Text{ContentFunc: content, Style: core.Style{FontSize: size, FontWeight: weight, Color: color}}
}

func spacer() core.Node {
	return core.View{Style: core.Style{FlexGrow: 1}}
}

func cardStyle(c colors) core.Style {
	return core.Style{
		Background: c.surface, CornerRadius: 12, BorderWidth: 1, BorderColor: c.border,
		Padding: core.All(14), Gap: 10,
	}
}

func badge(c colors, content string, background, foreground core.Color) core.Node {
	return core.View{
		Style:    core.Style{Background: background, CornerRadius: 6, Padding: core.Symmetric(3, 7)},
		Children: []core.Node{line(content, 11, core.FontWeightSemibold, foreground)},
	}
}

type buttonKind uint8

const (
	buttonPrimary buttonKind = iota
	buttonSecondary
	buttonDanger
)

func appButton(cx *core.Scope, label, testID string, kind buttonKind, disabled func() bool, onPress func()) core.Node {
	c := colorsOf(cx)
	background, hover, pressed, foreground, border := c.accent, c.accentHover, c.accentPressed, c.accentText, c.accent
	switch kind {
	case buttonSecondary:
		background, hover, pressed, foreground, border = c.background, c.surfaceHover, c.surfacePressed, c.text, c.border
	case buttonDanger:
		background, hover, pressed, foreground, border = c.danger, c.dangerHover, c.dangerPressed, c.background, c.danger
	}
	return base.Button{
		OnPress: func(core.PressEvent) {
			if onPress != nil {
				onPress()
			}
		},
		Disabled: disabled,
		Label:    label,
		TestID:   testID,
		Style: core.Style{
			Background: background, BorderWidth: 1, BorderColor: border,
			CornerRadius: 8, Padding: core.Symmetric(9, 14),
			AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			Hover:    &core.Style{Background: hover},
			Active:   &core.Style{Background: pressed},
			Disabled: &core.Style{Opacity: core.Alpha(0.45)},
		},
		Children: []core.Node{line(label, 13, core.FontWeightSemibold, foreground)},
	}
}

func (s *state) policyButton(cx *core.Scope, label, testID string, policy appmodel.KeepPolicy) core.Node {
	c := colorsOf(cx)
	selected := func() bool { return s.keepPolicy.Get() == policy }
	return base.Button{
		OnPress:  func(core.PressEvent) { s.setKeepPolicy(policy) },
		Disabled: func() bool { return s.scanning.Get() || s.busy.Get() },
		Selected: selected,
		Label:    label,
		TestID:   testID,
		Style: core.Style{
			Background: c.background, BorderWidth: 1, BorderColor: c.border,
			CornerRadius: 7, Padding: core.Symmetric(6, 10),
			AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			Hover:    &core.Style{Background: c.surfaceHover},
			Active:   &core.Style{Background: c.surfacePressed},
			Selected: &core.Style{Background: c.accentSoft, BorderColor: c.accent},
			Disabled: &core.Style{Opacity: core.Alpha(0.45)},
		},
		Children: []core.Node{line(label, 12, core.FontWeightMedium, c.text)},
	}
}

func radioDot(cx *core.Scope, checked func() bool) core.Node {
	c := colorsOf(cx)
	return core.View{
		StyleFunc: func() core.Style {
			border := c.border
			if checked() {
				border = c.accent
			}
			return core.Style{
				Width: core.Pt(18), Height: core.Pt(18), CornerRadius: 9,
				BorderWidth: 1.5, BorderColor: border,
				AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			}
		},
		Children: []core.Node{core.Show{
			When: checked,
			Then: func(*core.Scope) core.Node {
				return core.View{Style: core.Style{Width: core.Pt(8), Height: core.Pt(8), CornerRadius: 4, Background: c.accent}}
			},
		}},
	}
}

func speedText(bytesPerSecond float64) string {
	return format.Bytes(int64(bytesPerSecond)) + "/s"
}

func formatDuration(d time.Duration) string {
	if d < time.Minute {
		return fmt.Sprintf("%ds", int(d.Seconds()))
	}
	if d < time.Hour {
		return fmt.Sprintf("%d:%02d", int(d.Minutes()), int(d.Seconds())%60)
	}
	return fmt.Sprintf("%d:%02d:%02d", int(d.Hours()), int(d.Minutes())%60, int(d.Seconds())%60)
}

func shortMD5(md5 string) string {
	if len(md5) > 8 {
		return md5[:8]
	}
	return md5
}

func (s *state) groupRow(cx *core.Scope, row core.ListRow) core.Node {
	c := colorsOf(cx)
	groupAt := func() (db.DuplicateGroup, bool) {
		groups := s.groups.Get()
		i := row.Index()
		if i < 0 || i >= len(groups) {
			return db.DuplicateGroup{}, false
		}
		return groups[i], true
	}
	return core.View{
		Style: core.Style{
			Background: c.surface, CornerRadius: 12, BorderWidth: 1, BorderColor: c.border,
			Padding: core.All(14), Gap: 10,
		},
		Children: []core.Node{
			core.View{Style: core.Style{Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8}, Children: []core.Node{
				linef(func() string {
					if _, ok := groupAt(); !ok {
						return s.t(i18n.KeyGroupWord)
					}
					return s.tf(i18n.KeyGroup, row.Index()+1)
				}, 15, core.FontWeightSemibold, c.text),
				linef(func() string {
					group, ok := groupAt()
					if !ok {
						return ""
					}
					return fmt.Sprintf("%s · %s", format.Bytes(group.Size), s.copyCount(len(group.Files)))
				}, 12, core.FontWeightNormal, c.muted),
				spacer(),
				linef(func() string {
					group, ok := groupAt()
					if !ok {
						return ""
					}
					return shortMD5(group.MD5)
				}, 11, core.FontWeightNormal, c.muted),
			}},
			core.Each(
				func() []db.FileRecord {
					group, ok := groupAt()
					if !ok {
						return nil
					}
					return group.Files
				},
				func(file db.FileRecord) string { return file.Path },
				func(cx *core.Scope, file func() db.FileRecord, _ func() int) core.Node {
					return s.fileRow(cx, groupAt, file)
				},
			),
			linef(func() string {
				group, ok := groupAt()
				if !ok || s.keep.Get()[group.MD5] != "" {
					return ""
				}
				if _, found := appmodel.ProtectedFile(group, s.selfPath); found {
					return s.t(i18n.KeySelfKeepHint)
				}
				return s.t(i18n.KeyKeepHint)
			}, 12, core.FontWeightNormal, c.muted),
		},
	}
}

func (s *state) fileRow(cx *core.Scope, groupAt func() (db.DuplicateGroup, bool), file func() db.FileRecord) core.Node {
	c := colorsOf(cx)
	groupSelf := func() (db.FileRecord, bool) {
		group, ok := groupAt()
		if !ok {
			return db.FileRecord{}, false
		}
		return appmodel.ProtectedFile(group, s.selfPath)
	}
	isSelf := func() bool {
		self, ok := groupSelf()
		return ok && selfpath.Same(file().Path, self.Path)
	}
	groupHasSelf := func() bool {
		_, ok := groupSelf()
		return ok
	}
	checked := func() bool {
		group, ok := groupAt()
		return ok && s.keep.Get()[group.MD5] == file().Path
	}
	onChange := func(bool) {
		group, ok := groupAt()
		if ok && (!groupHasSelf() || isSelf()) {
			s.setKeep(group.MD5, file().Path)
		}
	}
	deletePending := func() bool {
		group, ok := groupAt()
		if !ok {
			return false
		}
		keep := s.keep.Get()[group.MD5]
		return keep != "" && keep != file().Path
	}
	rowStyle := func() core.Style {
		style := core.Style{
			Direction: core.Row, AlignItems: core.AlignCenter, Gap: 10,
			Background: c.background, BorderWidth: 1, BorderColor: c.border,
			CornerRadius: 8, Padding: core.Symmetric(8, 10),
		}
		if checked() {
			style.Background = c.accentSoft
			style.BorderColor = c.accent.Faded(0.45)
		}
		if groupHasSelf() && !isSelf() {
			style.Opacity = core.Alpha(0.55)
		}
		if s.busy.Get() || s.scanning.Get() {
			style.Opacity = core.Alpha(0.6)
		}
		return style
	}
	return base.Radio{
		Checked:  checked,
		OnChange: onChange,
		Disabled: func() bool {
			return s.busy.Get() || s.scanning.Get() || (groupHasSelf() && !isSelf())
		},
		Label:     file().Name,
		TestID:    "keep:" + file().Path,
		StyleFunc: rowStyle,
		Children: []core.Node{
			radioDot(cx, checked),
			core.View{Style: core.Style{FlexGrow: 1, FlexShrink: 1, MinWidth: core.Pt(0), Gap: 1}, Children: []core.Node{
				linef(func() string { return file().Name }, 13, core.FontWeightMedium, c.text),
				core.Text{
					ContentFunc: func() string { return file().Path },
					Style:       core.Style{FontSize: 11, Color: c.muted},
					MaxLines:    2,
				},
			}},
			linef(func() string { return format.Bytes(file().Size) }, 11, core.FontWeightNormal, c.muted),
			core.Show{When: isSelf, Then: func(*core.Scope) core.Node {
				return badge(c, s.t(i18n.KeyRunning), c.success.Faded(0.16), c.success)
			}},
			core.Show{When: checked, Then: func(*core.Scope) core.Node {
				return badge(c, s.t(i18n.KeyKeepBadge), c.accent, c.accentText)
			}},
			core.Show{When: deletePending, Then: func(*core.Scope) core.Node {
				return badge(c, s.t(i18n.KeyDeleteBadge), c.dangerSoft, c.danger)
			}},
		},
	}
}
