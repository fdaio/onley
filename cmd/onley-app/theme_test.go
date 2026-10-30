package main

import (
	"testing"

	"github.com/oneo-me/nsgo/core"
)

func TestBrutalistTheme(t *testing.T) {
	theme := brutalistTheme()
	p := theme.Light
	checks := []struct {
		name string
		got  core.Color
		want core.Color
	}{
		{"background", p.Background, core.RGB(0xF4F1EB)},
		{"surface", p.Surface, core.RGB(0xFFFFFF)},
		{"border", p.Border, core.RGB(0x111111)},
		{"text", p.Text, core.RGB(0x111111)},
		{"muted", p.TextMuted, core.RGB(0x6B6B6B)},
		{"accent", p.Accent, core.RGB(0xFF00A0)},
		{"success", p.Success, core.RGB(0x1F6B3A)},
		{"warning", p.Warning, core.RGB(0xFFB000)},
		{"danger", p.Danger, core.RGB(0xFF00A0)},
	}
	for _, check := range checks {
		if check.got != check.want {
			t.Errorf("%s = %#x, want %#x", check.name, uint32(check.got), uint32(check.want))
		}
	}
	if theme.Dark.Background != p.Background {
		t.Errorf("dark background = %#x, want the light one", uint32(theme.Dark.Background))
	}
}
