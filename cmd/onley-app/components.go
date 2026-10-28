package main

import (
	"fmt"
	"time"

	"github.com/oneo-me/nsgo/base"
	"github.com/oneo-me/nsgo/core"

	"onley/internal/db"
	"onley/internal/format"
)

type colors struct {
	background core.Color
	surface    core.Color
	border     core.Color
	text       core.Color
	muted      core.Color
	accent     core.Color
	accentText core.Color
	accentSoft core.Color
	danger     core.Color
	dangerSoft core.Color
	success    core.Color
}

func colorsOf(cx *core.Scope) colors {
	p := cx.Theme().Light
	if cx.Dark() {
		p = cx.Theme().Dark
	}
	return colors{
		background: p.Background,
		surface:    p.Surface,
		border:     p.Border,
		text:       p.Text,
		muted:      p.TextMuted,
		accent:     p.Accent,
		accentText: p.AccentText,
		accentSoft: p.Accent.Faded(0.12),
		danger:     p.Danger,
		dangerSoft: p.Danger.Faded(0.14),
		success:    p.Success,
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
	background, foreground, border := c.accent, c.accentText, c.accent
	switch kind {
	case buttonSecondary:
		background, foreground, border = c.background, c.text, c.border
	case buttonDanger:
		background, foreground, border = c.danger, c.background, c.danger
	}
	isDisabled := func() bool { return disabled != nil && disabled() }
	return base.Button{
		OnPress: func(core.PressEvent) {
			if onPress != nil {
				onPress()
			}
		},
		Disabled: isDisabled,
		Label:    label,
		TestID:   testID,
		Style: core.Style{
			Background: background, BorderWidth: 1, BorderColor: border,
			CornerRadius: 8, Padding: core.Symmetric(9, 14),
			AlignItems: core.AlignCenter, JustifyContent: core.JustifyCenter,
		},
		StyleFunc: func() core.Style {
			if isDisabled() {
				return core.Style{Opacity: core.Alpha(0.45)}
			}
			return core.Style{}
		},
		Children: []core.Node{line(label, 13, core.FontWeightSemibold, foreground)},
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

func fileCount(n int) string {
	if n == 1 {
		return "1 file"
	}
	return fmt.Sprintf("%d files", n)
}

func duplicateFileCount(n int) string {
	if n == 1 {
		return "1 duplicate file"
	}
	return fmt.Sprintf("%d duplicate files", n)
}

func groupCount(n int) string {
	if n == 1 {
		return "1 group"
	}
	return fmt.Sprintf("%d groups", n)
}

func copyCount(n int) string {
	if n == 1 {
		return "1 copy"
	}
	return fmt.Sprintf("%d copies", n)
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
						return "Group"
					}
					return fmt.Sprintf("Group %d", row.Index()+1)
				}, 15, core.FontWeightSemibold, c.text),
				linef(func() string {
					group, ok := groupAt()
					if !ok {
						return ""
					}
					return fmt.Sprintf("%s · %s", format.Bytes(group.Size), copyCount(len(group.Files)))
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
				return "Choose the copy to keep. A group without a choice is skipped."
			}, 12, core.FontWeightNormal, c.muted),
		},
	}
}

func (s *state) fileRow(cx *core.Scope, groupAt func() (db.DuplicateGroup, bool), file func() db.FileRecord) core.Node {
	c := colorsOf(cx)
	checked := func() bool {
		group, ok := groupAt()
		return ok && s.keep.Get()[group.MD5] == file().Path
	}
	onChange := func(bool) {
		group, ok := groupAt()
		if ok {
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
		if s.busy.Get() || s.scanning.Get() {
			style.Opacity = core.Alpha(0.6)
		}
		return style
	}
	return base.Radio{
		Checked:   checked,
		OnChange:  onChange,
		Disabled:  func() bool { return s.busy.Get() || s.scanning.Get() },
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
			core.Show{When: checked, Then: func(*core.Scope) core.Node {
				return badge(c, "keep", c.accent, c.accentText)
			}},
			core.Show{When: deletePending, Then: func(*core.Scope) core.Node {
				return badge(c, "delete", c.dangerSoft, c.danger)
			}},
		},
	}
}
