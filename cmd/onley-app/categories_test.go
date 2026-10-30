package main

import "testing"

func TestFileCategory(t *testing.T) {
	tests := []struct {
		name string
		want category
	}{
		{"photo.JPG", categoryImages},
		{"clip.mov", categoryVideos},
		{"report.pdf", categoryDocuments},
		{"backup.zip", categoryArchives},
		{"Onley.exe", categorySystem},
	}
	for _, test := range tests {
		if got := fileCategory(test.name); got != test.want {
			t.Errorf("fileCategory(%q) = %v, want %v", test.name, got, test.want)
		}
	}
}

func TestFormatCount(t *testing.T) {
	tests := map[int]string{
		0:      "0",
		42:     "42",
		999:    "999",
		1000:   "1,000",
		124082: "124,082",
	}
	for n, want := range tests {
		if got := formatCount(n); got != want {
			t.Errorf("formatCount(%d) = %q, want %q", n, got, want)
		}
	}
}
