package main

import "testing"

func TestFilePath(t *testing.T) {
	tests := []struct {
		raw  string
		want string
		ok   bool
	}{
		{"file:///tmp/a%20b", "/tmp/a b", true},
		{"file:///tmp/plain", "/tmp/plain", true},
		{"https://example.com/a", "", false},
		{"not a url", "", false},
	}
	for _, tt := range tests {
		got, ok := filePath(tt.raw)
		if got != tt.want || ok != tt.ok {
			t.Errorf("filePath(%q) = %q, %v; want %q, %v", tt.raw, got, ok, tt.want, tt.ok)
		}
	}
}

func TestDisplayPath(t *testing.T) {
	if got := displayPath("/Users/alice/Downloads", "/Users/alice"); got != "~/Downloads" {
		t.Errorf("displayPath = %q, want ~/Downloads", got)
	}
	if got := displayPath("/Volumes/data", "/Users/alice"); got != "/Volumes/data" {
		t.Errorf("displayPath = %q, want unchanged", got)
	}
}
