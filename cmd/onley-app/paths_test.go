package main

import (
	"path/filepath"
	"testing"
)

func TestFilePath(t *testing.T) {
	tests := []struct {
		raw  string
		want string
		ok   bool
	}{
		{"file:///tmp/a%20b", filepath.FromSlash("/tmp/a b"), true},
		{"file:///tmp/plain", filepath.FromSlash("/tmp/plain"), true},
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
	home := filepath.Join(string(filepath.Separator), "Users", "alice")
	downloads := filepath.Join(home, "Downloads")
	if got, want := displayPath(downloads, home), "~"+string(filepath.Separator)+"Downloads"; got != want {
		t.Errorf("displayPath = %q, want %q", got, want)
	}
	other := filepath.Join(string(filepath.Separator), "Volumes", "data")
	if got := displayPath(other, home); got != other {
		t.Errorf("displayPath = %q, want unchanged", got)
	}
}
