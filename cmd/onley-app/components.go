package main

import (
	"fmt"
	"path/filepath"
	"strings"
	"time"

	"github.com/oneo-me/nsgo/base"
	"github.com/oneo-me/nsgo/core"
	"github.com/oneo-me/nsgo/icons/lucide"

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
	success        core.Color
	successSoft    core.Color
	danger         core.Color
	dangerSoft     core.Color
	warning        core.Color
}

func colorsOf(cx *core.Scope) colors {
	p := cx.Theme().Light
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
		success:        p.Success,
		successSoft:    core.RGB(0xB9E4C9),
		danger:         p.Danger,
		dangerSoft:     p.Danger.Faded(0.12),
		warning:        p.Warning,
	}
}

func line(content string, size core.Points, weight core.FontWeight, color core.Color) core.Text {
	return core.Text{Content: content, Style: core.Style{FontFamily: monoFont, FontSize: size,
		FontWeight: weight, Color: color}}
}

func linef(content func() string, size core.Points, weight core.FontWeight, color core.Color) core.Text {
	return core.Text{ContentFunc: content, Style: core.Style{FontFamily: monoFont, FontSize: size,
		FontWeight: weight, Color: color}}
}

func spacer() core.Node {
	return core.View{Style: core.Style{FlexGrow: 1}}
}

type buttonKind uint8

const (
	buttonPrimary buttonKind = iota
	buttonSecondary
	buttonDanger
	buttonGhost
)

func appButton(cx *core.Scope, label, testID string, kind buttonKind, disabled func() bool, onPress func()) core.Node {
	c := colorsOf(cx)
	background, hover, pressed, foreground, border := c.accent, c.accentHover, c.accentPressed, c.text, c.text
	switch kind {
	case buttonSecondary:
		background, hover, pressed, foreground, border = c.surface, c.surfaceHover, c.surfacePressed, c.text, c.text
	case buttonDanger:
		background, hover, pressed, foreground, border = c.accent, c.accentHover, c.accentPressed, c.text, c.text
	case buttonGhost:
		background, hover, pressed, foreground, border = c.surface, c.surfaceHover, c.surfacePressed, c.text, c.surface
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
			Padding: core.Symmetric(7, 12), AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			Hover:    &core.Style{Background: hover},
			Active:   &core.Style{Background: pressed},
			Disabled: &core.Style{Opacity: core.Alpha(0.4)},
		},
		Children: []core.Node{core.Text{Content: strings.ToUpper(label),
			Style: core.Style{FontFamily: monoFont, FontSize: 9, FontWeight: core.FontWeightBold, Color: foreground}}},
	}
}

func statusBox(cx *core.Scope, checked func() bool) core.Node {
	c := colorsOf(cx)
	return core.View{
		StyleFunc: func() core.Style {
			background := c.surface
			if checked() {
				background = c.successSoft
			}
			return core.Style{
				Width: core.Pt(16), Height: core.Pt(16), BorderWidth: 1, BorderColor: c.text,
				Background: background, AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
			}
		},
		Children: []core.Node{core.Show{When: checked, Then: func(*core.Scope) core.Node {
			return core.Icon{SVG: lucide.Check(), Style: core.Style{Width: core.Pt(10), Height: core.Pt(10), Color: c.text}}
		}}},
	}
}

func (s *state) policySelected(policy appmodel.KeepPolicy) bool {
	if s.keepPolicy.Get() != policy {
		return false
	}
	groups := s.scoped.Get()
	if len(groups) == 0 {
		return false
	}
	keep := s.keep.Get()
	for _, group := range groups {
		path := keep[group.MD5]
		if path == "" {
			return false
		}
		if self, found := appmodel.ProtectedFile(group, s.selfPath); found {
			if !selfpath.Same(path, self.Path) {
				return false
			}
			continue
		}
		if path != appmodel.ChooseKeep(group, policy).Path {
			return false
		}
	}
	return true
}

func (s *state) groupRow(cx *core.Scope, row core.ListRow) core.Node {
	c := colorsOf(cx)
	groupAt := func() (db.DuplicateGroup, bool) {
		groups := s.visible.Get()
		i := row.Index()
		if i < 0 || i >= len(groups) {
			return db.DuplicateGroup{}, false
		}
		return groups[i], true
	}
	if _, ok := groupAt(); !ok {
		return core.View{}
	}
	return core.View{
		Style: core.Style{
			Background: c.surface, BorderWidth: 2, BorderColor: c.border,
			Direction: core.Row, Padding: core.All(10), Gap: 12,
		},
		Children: []core.Node{
			s.preview(cx, groupAt),
			core.View{Style: core.Style{
				FlexGrow: 1, FlexBasis: core.Pt(0), MinWidth: core.Pt(0),
				Direction: core.Column, JustifyContent: core.JustifyCenter,
			}, Children: []core.Node{
				core.Each(
					func() []db.FileRecord {
						current, found := groupAt()
						if !found {
							return nil
						}
						return current.Files
					},
					func(file db.FileRecord) string { return file.Path },
					func(cx *core.Scope, file func() db.FileRecord, index func() int) core.Node {
						return s.fileRow(cx, groupAt, file, index)
					},
				),
			}},
		},
	}
}

func (s *state) preview(cx *core.Scope, groupAt func() (db.DuplicateGroup, bool)) core.Node {
	c := colorsOf(cx)
	file := func() db.FileRecord {
		group, ok := groupAt()
		if !ok || len(group.Files) == 0 {
			return db.FileRecord{}
		}
		return group.Files[0]
	}
	ext := func() string {
		value := strings.TrimPrefix(filepath.Ext(file().Name), ".")
		if value == "" {
			return "FILE"
		}
		return strings.ToUpper(value)
	}
	date := func() string {
		return strings.ToUpper(time.Unix(file().Mtime, 0).Format("Jan 02"))
	}
	return core.View{Style: core.Style{
		Width: core.Pt(180), FlexShrink: 0, Direction: core.Column, Gap: 5,
	}, Children: []core.Node{
		core.View{StyleFunc: func() core.Style {
			group, ok := groupAt()
			gradient := previewGradient("")
			if ok {
				gradient = previewGradient(group.MD5)
			}
			return core.Style{
				Width: core.Pct(100), Height: core.Pt(116), BorderWidth: 2, BorderColor: c.border,
				Gradients: []core.Gradient{gradient},
			}
		}},
		core.Text{ContentFunc: func() string { return file().Name }, MaxLines: 1,
			EllipsizeMode: core.EllipsizeMiddle,
			Style:         core.Style{FontFamily: monoFont, FontSize: 10, FontWeight: core.FontWeightBold, Color: c.text}},
		core.View{Style: core.Style{Direction: core.Row, Gap: 4, Wrap: core.Wrap}, Children: []core.Node{
			tag(cx, ext, c.accent),
			tag(cx, func() string { return format.Bytes(file().Size) }, c.surface),
			tag(cx, date, c.surface),
		}},
	}}
}

func tag(cx *core.Scope, content func() string, background core.Color) core.Node {
	c := colorsOf(cx)
	return core.View{Style: core.Style{
		Padding: core.Symmetric(2, 4), Background: background,
		BorderWidth: 1, BorderColor: c.border,
	}, Children: []core.Node{core.Text{ContentFunc: content, Style: core.Style{FontFamily: monoFont,
		FontSize: 7, FontWeight: core.FontWeightBold, Color: c.text}}}}
}

func previewGradient(md5 string) core.Gradient {
	palettes := [][]core.Color{
		{core.RGB(0x2E1065), core.RGB(0x9333EA), core.RGB(0xF59E0B)},
		{core.RGB(0x0F2027), core.RGB(0x203A43), core.RGB(0x2C5364)},
		{core.RGB(0xFF512F), core.RGB(0xDD2476), core.RGB(0xFF8A00)},
		{core.RGB(0x11998E), core.RGB(0x38EF7D)},
		{core.RGB(0x1E3A8A), core.RGB(0x3B82F6), core.RGB(0xF472B6)},
	}
	index := 0
	for i := 0; i < len(md5) && i < 3; i++ {
		index = index*16 + int(md5[i]%16)
	}
	return core.LinearGradient(135, palettes[index%len(palettes)]...)
}

func (s *state) fileRow(cx *core.Scope, groupAt func() (db.DuplicateGroup, bool), file func() db.FileRecord, index func() int) core.Node {
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
	onChange := func() {
		group, ok := groupAt()
		if ok && (!groupHasSelf() || isSelf()) {
			s.setKeep(group.MD5, file().Path)
		}
	}
	label := s.t(i18n.KeyOriginalCopy)
	if index() > 0 {
		label = s.tf(i18n.KeyDuplicateN, index())
	}
	status := s.t(i18n.KeyDeleteBadge)
	if checked() {
		status = s.t(i18n.KeyKeepBadge)
	}
	return base.Button{
		OnPress: func(core.PressEvent) { onChange() },
		Disabled: func() bool {
			return s.busy.Get() || s.scanning.Get() || (groupHasSelf() && !isSelf())
		},
		Label:    file().Name + ", " + file().Path,
		TestID:   "keep:" + file().Path,
		Selected: checked,
		StyleFunc: func() core.Style {
			background := c.surface
			if checked() {
				background = c.successSoft
			}
			style := core.Style{
				Direction: core.Row, AlignItems: core.AlignCenter, Gap: 8,
				Padding: core.Symmetric(7, 6), Background: background,
				Borders: core.Edges{Bottom: 1}, BorderColor: c.border,
				Hover: &core.Style{Background: c.surfaceHover},
			}
			if groupHasSelf() && !isSelf() {
				style.Opacity = core.Alpha(0.45)
			}
			if s.busy.Get() || s.scanning.Get() {
				style.Opacity = core.Alpha(0.6)
			}
			return style
		},
		Children: []core.Node{
			core.View{Style: core.Style{
				FlexGrow: 1, FlexBasis: core.Pt(0), MinWidth: core.Pt(0), Direction: core.Column, Gap: 1,
			}, Children: []core.Node{
				core.Text{Content: strings.ToUpper(label), Style: core.Style{FontFamily: monoFont,
					FontSize: 8, FontWeight: core.FontWeightBold, Color: c.text}},
				core.View{TestID: "file-dir:" + file().Path, Children: []core.Node{
					core.Text{Content: displayPath(file().Path, s.home), MaxLines: 1,
						EllipsizeMode: core.EllipsizeMiddle,
						Style:         core.Style{FontFamily: monoFont, FontSize: 8, Color: c.muted}},
				}},
			}},
			core.Text{Content: strings.ToUpper(status), Style: core.Style{FontFamily: monoFont,
				FontSize: 8, FontWeight: core.FontWeightBold, Color: c.text, TextAlign: core.TextAlignEnd}},
			statusBox(cx, checked),
		},
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
