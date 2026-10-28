package main

import (
	"testing"

	"github.com/oneo-me/nsgo/core"
)

func TestMidnightTheme(t *testing.T) {
	theme := midnightTheme()
	p := theme.Dark
	checks := []struct {
		name string
		got  core.Color
		want core.Color
	}{
		{"background", p.Background, core.RGB(0x111318)},
		{"surface", p.Surface, core.RGB(0x191D24)},
		{"border", p.Border, core.RGB(0x343C49)},
		{"text", p.Text, core.RGB(0xE6E8EC)},
		{"muted", p.TextMuted, core.RGB(0x858D9B)},
		{"accent", p.Accent, core.RGB(0x8BA4C1)},
		{"success", p.Success, core.RGB(0x8BC7A1)},
		{"warning", p.Warning, core.RGB(0xD8B477)},
		{"danger", p.Danger, core.RGB(0xD98282)},
	}
	for _, check := range checks {
		if check.got != check.want {
			t.Errorf("%s = %#x, want %#x", check.name, uint32(check.got), uint32(check.want))
		}
	}
	if theme.Light.Background != p.Background {
		t.Errorf("light background = %#x, want the dark one", uint32(theme.Light.Background))
	}
}
