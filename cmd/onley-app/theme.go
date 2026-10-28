package main

import "github.com/oneo-me/nsgo/core"

// midnightTheme is the Midnight Terminal palette. The light palette is the
// same, because the client always shows the dark appearance.
func midnightTheme() *core.Theme {
	palette := core.Palette{
		Background:     core.RGB(0x111318),
		Surface:        core.RGB(0x191D24),
		SurfaceHover:   core.RGB(0x252B35),
		SurfacePressed: core.RGB(0x2E3642),
		Border:         core.RGB(0x343C49),
		Text:           core.RGB(0xE6E8EC),
		TextMuted:      core.RGB(0x858D9B),
		Accent:         core.RGB(0x8BA4C1),
		AccentHover:    core.RGB(0x9FB6CF),
		AccentPressed:  core.RGB(0x7A93AE),
		AccentText:     core.RGB(0x111318),
		Danger:         core.RGB(0xD98282),
		Warning:        core.RGB(0xD8B477),
		Success:        core.RGB(0x8BC7A1),
		Selection:      core.RGBA(0x8BA4C1, 0.24),
		Focus:          core.RGBA(0x8BA4C1, 0.8),
		Scrollbar:      core.RGBA(0x858D9B, 0.35),
	}
	return &core.Theme{Light: palette, Dark: palette, Radius: 8}
}
