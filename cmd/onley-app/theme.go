package main

import "github.com/oneo-me/nsgo/core"

// brutalistTheme is the black, paper and pink palette of the redesigned app.
// The light and dark palettes are the same, because the client always shows
// the light appearance.
func brutalistTheme() *core.Theme {
	palette := core.Palette{
		Background:     core.RGB(0xF4F1EB),
		Surface:        core.RGB(0xFFFFFF),
		SurfaceHover:   core.RGB(0xEEEAE2),
		SurfacePressed: core.RGB(0xE3DED4),
		Border:         core.RGB(0x111111),
		Text:           core.RGB(0x111111),
		TextMuted:      core.RGB(0x6B6B6B),
		Accent:         core.RGB(0xFF00A0),
		AccentHover:    core.RGB(0xFF33B3),
		AccentPressed:  core.RGB(0xD90088),
		AccentText:     core.RGB(0x111111),
		Danger:         core.RGB(0xFF00A0),
		Warning:        core.RGB(0xFFB000),
		Success:        core.RGB(0x1F6B3A),
		Selection:      core.RGBA(0xFF00A0, 0.18),
		Focus:          core.RGBA(0xFF00A0, 0.8),
		Scrollbar:      core.RGBA(0x111111, 0.6),
	}
	return &core.Theme{Light: palette, Dark: palette, Radius: 0}
}
