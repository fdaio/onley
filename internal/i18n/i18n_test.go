package i18n

import "testing"

func TestDetect(t *testing.T) {
	tests := []struct {
		locale string
		want   Lang
	}{
		{"zh-Hans", Chinese},
		{"zh-CN", Chinese},
		{"ZH", Chinese},
		{"en-US", English},
		{"", English},
		{"ja-JP", English},
	}
	for _, tt := range tests {
		if got := Detect(tt.locale); got != tt.want {
			t.Errorf("Detect(%q) = %q, want %q", tt.locale, got, tt.want)
		}
	}
}

func TestParseAndNext(t *testing.T) {
	if lang, ok := Parse("zh"); !ok || lang != Chinese {
		t.Errorf("Parse(zh) = %q, %v", lang, ok)
	}
	if _, ok := Parse("fr"); ok {
		t.Errorf("Parse(fr) reported supported")
	}
	if got := Next(English); got != Chinese {
		t.Errorf("Next(en) = %q, want zh", got)
	}
	if got := Next(Chinese); got != English {
		t.Errorf("Next(zh) = %q, want en", got)
	}
}

func TestCatalogsComplete(t *testing.T) {
	for key := range messages[English] {
		if text, ok := messages[Chinese][key]; !ok || text == "" {
			t.Errorf("Chinese catalog missing %q", key)
		}
	}
	for lang, catalog := range messages {
		for key, text := range catalog {
			if text == "" {
				t.Errorf("%s catalog has empty %q", lang, key)
			}
		}
	}
}

func TestFallbackAndCounts(t *testing.T) {
	if got := T(Chinese, KeyScanning); got != "扫描中" {
		t.Errorf("Chinese scanning = %q", got)
	}
	if got := T("fr", KeyScanning); got != "Scanning" {
		t.Errorf("unknown language fallback = %q", got)
	}
	if got := FileCount(English, 1); got != "1 file" {
		t.Errorf("English singular = %q", got)
	}
	if got := FileCount(English, 2); got != "2 files" {
		t.Errorf("English plural = %q", got)
	}
	if got := FileCount(Chinese, 1); got != "1 个文件" {
		t.Errorf("Chinese count = %q", got)
	}
	if got := FolderCount(English, 1); got != "1 folder" {
		t.Errorf("folder singular = %q", got)
	}
	if got := FolderCount(Chinese, 2); got != "2 个文件夹" {
		t.Errorf("folder Chinese = %q", got)
	}
}
